mod app;
mod db;
mod fuzzy;
mod ui;

use std::io;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

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
    let db_override: Option<PathBuf> = if let Some(pos) = args.iter().position(|a| a == "--db") {
        args.get(pos + 1).map(|p| PathBuf::from(p))
    } else {
        None
    };

    // Set up channel and spawn background loader
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        db::stream_sessions(db_override, tx);
    });

    let mut app = App::new(rx);

    // Set up terminal
    enable_raw_mode().expect("failed to enable raw mode");
    crossterm::execute!(io::stdout(), EnterAlternateScreen).expect("failed to enter alt screen");
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend).expect("failed to create terminal");

    // Main loop — poll with a short timeout so we can receive new session batches
    loop {
        // Drain any pending sessions from the background thread
        app.poll_sessions();

        terminal
            .draw(|f| ui::draw(f, &mut app))
            .expect("draw failed");

        if app.should_exit() {
            break;
        }

        // Poll for keyboard events with a short timeout to stay responsive
        // to incoming session data
        if event::poll(Duration::from_millis(50)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Esc => app.quit(),
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.quit()
                    }
                    KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.cursor = 0;
                    }
                    KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.cursor = app.query.len();
                    }
                    KeyCode::Enter => app.confirm(),
                    KeyCode::Backspace => app.backspace(),
                    KeyCode::Left => app.move_cursor_left(),
                    KeyCode::Right => app.move_cursor_right(),
                    KeyCode::Up => app.move_up(),
                    KeyCode::Down => app.move_down(),
                    KeyCode::F(2) => app.toggle_sort(),
                    KeyCode::Char(c) => app.type_char(c),
                    _ => {}
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode().expect("failed to disable raw mode");
    crossterm::execute!(io::stdout(), LeaveAlternateScreen).expect("failed to leave alt screen");

    // Act on result
    match app.result {
        Some(AppResult::Selected(session)) => {
            let err = Command::new("opencode").arg("-s").arg(&session.id).exec();
            eprintln!("Failed to exec opencode: {err}");
            std::process::exit(1);
        }
        _ => {}
    }
}
