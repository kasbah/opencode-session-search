use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};
use std::sync::mpsc;

#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub title: String,
    pub directory: String,
    pub time_created: i64, // epoch milliseconds
    pub last_input: String,
}

/// Locate the opencode SQLite database.
pub fn db_path() -> Result<PathBuf, String> {
    let data_dir = dirs::data_dir().ok_or("Could not determine XDG data directory")?;
    let path = data_dir.join("opencode").join("opencode.db");
    if path.exists() {
        Ok(path)
    } else {
        Err(format!("Database not found at {}", path.display()))
    }
}

/// Messages sent from the background loading thread.
pub enum LoadMsg {
    /// Phase 1: a batch of sessions (metadata only, no last_input yet).
    Batch(Vec<Session>),
    /// Phase 1 complete — all session metadata has been sent.
    SessionsDone,
    /// Phase 2: backfill last_input for a session by index.
    BackfillInput { index: usize, last_input: String },
    /// Everything is done. Contains an error message if one occurred.
    Done(Option<String>),
}

const BATCH_SIZE: usize = 50;

/// Stream sessions from the database in two phases:
/// 1. Quickly load session metadata (no expensive JOINs) in batches.
/// 2. Backfill last_input for each session via a second query.
pub fn stream_sessions(db_override: Option<PathBuf>, tx: mpsc::Sender<LoadMsg>) {
    let result = stream_sessions_inner(db_override.as_deref(), &tx);
    let err = result.err();
    let _ = tx.send(LoadMsg::Done(err));
}

fn resolve_path(db_override: Option<&Path>) -> Result<PathBuf, String> {
    match db_override {
        Some(p) => {
            if p.exists() {
                Ok(p.to_path_buf())
            } else {
                Err(format!("Database not found at {}", p.display()))
            }
        }
        None => db_path(),
    }
}

fn stream_sessions_inner(
    db_override: Option<&Path>,
    tx: &mpsc::Sender<LoadMsg>,
) -> Result<(), String> {
    let path = resolve_path(db_override)?;
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("Failed to open database: {e}"))?;

    // Phase 1: fast query — just the session table, no JOINs.
    let session_count = load_session_metadata(&conn, tx)?;
    if tx.send(LoadMsg::SessionsDone).is_err() {
        return Ok(());
    }

    // Phase 2: backfill last_input for each session.
    backfill_last_inputs(&conn, tx, session_count)?;

    Ok(())
}

/// Phase 1: load session metadata quickly from the session table only.
fn load_session_metadata(conn: &Connection, tx: &mpsc::Sender<LoadMsg>) -> Result<usize, String> {
    let query = "
        SELECT id, title, directory, time_created
        FROM session
        WHERE parent_id IS NULL
        ORDER BY time_created DESC
    ";

    let mut stmt = conn
        .prepare(query)
        .map_err(|e| format!("Query error: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(Session {
                id: row.get(0)?,
                title: row.get(1)?,
                directory: row.get(2)?,
                time_created: row.get(3)?,
                last_input: String::new(),
            })
        })
        .map_err(|e| format!("Query error: {e}"))?;

    let mut batch = Vec::with_capacity(BATCH_SIZE);
    let mut count = 0;
    for row in rows {
        let session = row.map_err(|e| format!("Row error: {e}"))?;
        batch.push(session);
        count += 1;
        if batch.len() >= BATCH_SIZE {
            if tx.send(LoadMsg::Batch(std::mem::take(&mut batch))).is_err() {
                return Ok(count);
            }
            batch = Vec::with_capacity(BATCH_SIZE);
        }
    }
    if !batch.is_empty() {
        let _ = tx.send(LoadMsg::Batch(batch));
    }

    Ok(count)
}

/// Phase 2: for each session (by index), query the last user message text.
fn backfill_last_inputs(
    conn: &Connection,
    tx: &mpsc::Sender<LoadMsg>,
    session_count: usize,
) -> Result<(), String> {
    // Get session IDs in the same order (newest first) so indices match.
    let query = "
        SELECT id FROM session
        WHERE parent_id IS NULL
        ORDER BY time_created DESC
    ";
    let mut id_stmt = conn
        .prepare(query)
        .map_err(|e| format!("Query error: {e}"))?;
    let ids: Vec<String> = id_stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| format!("Query error: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Row error: {e}"))?;

    // Prepare a statement to get the last user message for a given session.
    let msg_query = "
        SELECT COALESCE(json_extract(p.data, '$.text'), '')
        FROM message m
        JOIN part p ON p.message_id = m.id
        WHERE m.session_id = ?1
          AND json_extract(m.data, '$.role') = 'user'
          AND json_extract(p.data, '$.type') = 'text'
        ORDER BY m.time_created DESC, p.time_created ASC
        LIMIT 1
    ";
    let mut msg_stmt = conn
        .prepare(msg_query)
        .map_err(|e| format!("Query error: {e}"))?;

    for (index, session_id) in ids.iter().enumerate() {
        if index >= session_count {
            break;
        }
        let last_input: String = msg_stmt
            .query_row([session_id], |row| row.get(0))
            .unwrap_or_default();

        let first_line = last_input.lines().next().unwrap_or("").to_string();
        if !first_line.is_empty() {
            if tx
                .send(LoadMsg::BackfillInput {
                    index,
                    last_input: first_line,
                })
                .is_err()
            {
                return Ok(()); // receiver dropped
            }
        }
    }

    Ok(())
}
