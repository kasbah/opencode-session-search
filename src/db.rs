use rusqlite::{Connection, OpenFlags};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub title: String,
    pub directory: String,
    pub time_created: i64, // epoch milliseconds
    #[allow(dead_code)]
    pub parent_id: Option<String>,
}

/// Locate the opencode SQLite database.
/// Checks `~/.local/share/opencode/opencode.db` (XDG data dir).
pub fn db_path() -> Result<PathBuf, String> {
    let data_dir = dirs::data_dir().ok_or("Could not determine XDG data directory")?;
    let path = data_dir.join("opencode").join("opencode.db");
    if path.exists() {
        Ok(path)
    } else {
        Err(format!("Database not found at {}", path.display()))
    }
}

/// Query all sessions from the database.
/// If `include_subagents` is false, sessions with a non-null parent_id are excluded.
pub fn query_sessions(include_subagents: bool) -> Result<Vec<Session>, String> {
    let path = db_path()?;
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("Failed to open database: {e}"))?;

    let query = if include_subagents {
        "SELECT id, title, directory, time_created, parent_id FROM session ORDER BY time_created DESC"
    } else {
        "SELECT id, title, directory, time_created, parent_id FROM session WHERE parent_id IS NULL ORDER BY time_created DESC"
    };

    let mut stmt = conn
        .prepare(query)
        .map_err(|e| format!("Query error: {e}"))?;

    let sessions = stmt
        .query_map([], |row| {
            Ok(Session {
                id: row.get(0)?,
                title: row.get(1)?,
                directory: row.get(2)?,
                time_created: row.get(3)?,
                parent_id: row.get(4)?,
            })
        })
        .map_err(|e| format!("Query error: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Row error: {e}"))?;

    Ok(sessions)
}
