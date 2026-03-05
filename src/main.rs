mod app;
mod db;
mod fuzzy;
mod ui;

use std::io;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::CrosstermBackend;
use ratatui::Terminal;

use app::{App, AppResult};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Check for --version / -v flag
    if args.iter().any(|a| a == "--version" || a == "-v") {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return;
    }

    // Check for --db <path> argument
    let db_override = if let Some(pos) = args.iter().position(|a| a == "--db") {
        args.get(pos + 1).map(|p| PathBuf::from(p))
    } else {
        None
    };

    let sessions = match db::query_sessions(db_override.as_deref()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    if sessions.is_empty() {
        eprintln!("No sessions found.");
        std::process::exit(0);
    }

    let mut app = App::new(sessions);

    // Set up terminal
    enable_raw_mode().expect("failed to enable raw mode");
    crossterm::execute!(io::stdout(), EnterAlternateScreen).expect("failed to enter alt screen");
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend).expect("failed to create terminal");

    // Main loop
    loop {
        terminal.draw(|f| ui::draw(f, &app)).expect("draw failed");

        if app.should_exit() {
            break;
        }

        if let Ok(Event::Key(key)) = event::read() {
            // Only handle Press events (ignore Release/Repeat on some terminals)
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Esc => app.quit(),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => app.quit(),
                KeyCode::Enter => app.confirm(),
                KeyCode::Backspace => app.backspace(),
                KeyCode::Up => app.move_up(),
                KeyCode::Down => app.move_down(),
                KeyCode::F(2) => app.toggle_sort(),
                KeyCode::Char(c) => app.type_char(c),
                _ => {}
            }
        }
    }

    // Restore terminal
    disable_raw_mode().expect("failed to disable raw mode");
    crossterm::execute!(io::stdout(), LeaveAlternateScreen).expect("failed to leave alt screen");

    // Act on result
    match app.result {
        Some(AppResult::Selected(session)) => {
            // exec replaces this process with opencode
            let err = Command::new("opencode").arg("-s").arg(&session.id).exec();
            // exec only returns on error
            eprintln!("Failed to exec opencode: {err}");
            std::process::exit(1);
        }
        _ => {
            // User quit, exit cleanly
        }
    }
}
