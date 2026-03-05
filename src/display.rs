use chrono::DateTime;
use std::fmt::Write;

use crate::db::Session;

/// Shorten an absolute path by replacing the home directory prefix with `~`.
fn shorten_path(path: &str) -> String {
    if let Some(home) = dirs::home_dir() {
        let home_str = home.to_string_lossy();
        if let Some(rest) = path.strip_prefix(home_str.as_ref()) {
            return format!("~{rest}");
        }
    }
    path.to_string()
}

/// Format epoch milliseconds as a human-readable date string.
fn format_date(epoch_ms: i64) -> String {
    let secs = epoch_ms / 1000;
    let nanos = ((epoch_ms % 1000) * 1_000_000) as u32;
    match DateTime::from_timestamp(secs, nanos) {
        Some(dt) => {
            let local: DateTime<chrono::Local> = dt.with_timezone(&chrono::Local);
            local.format("%Y-%m-%d %H:%M").to_string()
        }
        None => "unknown".to_string(),
    }
}

/// Print sessions as an aligned table to stdout.
pub fn print_table(sessions: &[Session]) {
    if sessions.is_empty() {
        println!("No sessions found.");
        return;
    }

    let headers = ["Session ID", "Title", "Directory", "Date"];

    // Precompute display values
    let rows: Vec<[String; 4]> = sessions
        .iter()
        .map(|s| {
            [
                s.id.clone(),
                s.title.clone(),
                shorten_path(&s.directory),
                format_date(s.time_created),
            ]
        })
        .collect();

    // Calculate column widths
    let mut widths = [0usize; 4];
    for (i, h) in headers.iter().enumerate() {
        widths[i] = h.len();
    }
    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(cell.len());
        }
    }

    // Print header
    let mut header_line = String::new();
    for (i, h) in headers.iter().enumerate() {
        if i > 0 {
            header_line.push_str("  ");
        }
        write!(header_line, "{:<width$}", h, width = widths[i]).unwrap();
    }
    println!("{header_line}");

    // Print separator
    let total_width = widths.iter().sum::<usize>() + (widths.len() - 1) * 2;
    println!("{}", "─".repeat(total_width));

    // Print rows
    for row in &rows {
        let mut line = String::new();
        for (i, cell) in row.iter().enumerate() {
            if i > 0 {
                line.push_str("  ");
            }
            write!(line, "{:<width$}", cell, width = widths[i]).unwrap();
        }
        println!("{line}");
    }
}
