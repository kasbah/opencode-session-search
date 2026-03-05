use chrono::DateTime;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::app::App;

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

/// Format epoch milliseconds as a human-readable local date string.
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

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // input
        Constraint::Min(5),    // table
        Constraint::Length(1), // status bar
    ])
    .split(frame.area());

    draw_input(frame, app, chunks[0]);
    draw_table(frame, app, chunks[1]);
    draw_status(frame, app, chunks[2]);
}

fn draw_input(frame: &mut Frame, app: &App, area: Rect) {
    let input = Paragraph::new(Line::from(vec![
        Span::styled("> ", Style::default().fg(Color::Magenta)),
        Span::raw(&app.query),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(" Search sessions "),
    );
    frame.render_widget(input, area);

    // Place cursor after the typed text
    frame.set_cursor_position((area.x + 3 + app.query.len() as u16, area.y + 1));
}

fn draw_table(frame: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec![
        Cell::from("Title"),
        Cell::from("Last Message"),
        Cell::from("Directory"),
        Cell::from("Date"),
    ])
    .style(
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    )
    .bottom_margin(1);

    let rows: Vec<Row> = app
        .filtered
        .iter()
        .enumerate()
        .map(|(i, scored)| {
            let s = &scored.session;
            let style = if i == app.selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(s.title.clone()),
                Cell::from(s.last_input.clone()),
                Cell::from(shorten_path(&s.directory)),
                Cell::from(format_date(s.time_created)),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Length(40),
        Constraint::Length(16),
    ];

    let table = Table::new(rows, widths).header(header).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(table, area);
}

fn draw_status(frame: &mut Frame, app: &App, area: Rect) {
    let count = app.filtered.len();
    let total = app.sessions.len();

    let sort_label = if app.sort_by_date { "date" } else { "score" };

    let status = Line::from(vec![
        Span::styled(
            format!(" {count}/{total}"),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            format!("  sort: {sort_label} (F2)"),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled("  title: mes: dir:", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "  Enter: open  Esc: quit",
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    frame.render_widget(Paragraph::new(status), area);
}
