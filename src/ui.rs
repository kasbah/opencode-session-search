use chrono::DateTime;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::app::App;

const MATCH_BG: Color = Color::DarkGray;
const SELECTED_BG: Color = Color::Rgb(180, 90, 0); // dark orange
const SELECTED_MATCH_BG: Color = Color::Rgb(120, 60, 0); // darker orange for matches on selected row

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

/// Build a Line with matched character indices highlighted.
/// `base_style` is applied to non-highlighted characters.
fn highlighted_line<'a>(
    text: &str,
    indices: &[usize],
    base_style: Style,
    selected: bool,
) -> Line<'a> {
    let match_bg = if selected {
        SELECTED_MATCH_BG
    } else {
        MATCH_BG
    };
    let highlight_style = base_style
        .bg(match_bg)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);

    if indices.is_empty() {
        return Line::from(Span::styled(text.to_string(), base_style));
    }

    let mut spans = Vec::new();
    let mut last = 0;

    for &idx in indices {
        // Translate byte-safe: indices are char indices, so iterate chars
        let char_start = text.char_indices().nth(idx);
        if let Some((byte_pos, ch)) = char_start {
            // Add any text before this match
            if byte_pos > last {
                spans.push(Span::styled(text[last..byte_pos].to_string(), base_style));
            }
            spans.push(Span::styled(ch.to_string(), highlight_style));
            last = byte_pos + ch.len_utf8();
        }
    }

    // Remainder after last match
    if last < text.len() {
        spans.push(Span::styled(text[last..].to_string(), base_style));
    }

    Line::from(spans)
}

pub fn draw(frame: &mut Frame, app: &mut App) {
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

fn draw_table(frame: &mut Frame, app: &mut App, area: Rect) {
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
            let is_selected = i == app.selected;
            let base_style = if is_selected {
                Style::default().bg(SELECTED_BG).fg(Color::White)
            } else {
                Style::default()
            };

            let dir_display = shorten_path(&s.directory);
            // Directory indices are against the original path; remap to shortened
            let dir_indices =
                remap_dir_indices(&s.directory, &dir_display, &scored.indices.directory);

            Row::new(vec![
                Cell::from(highlighted_line(
                    &s.title,
                    &scored.indices.title,
                    base_style,
                    is_selected,
                )),
                Cell::from(highlighted_line(
                    &s.last_input,
                    &scored.indices.last_input,
                    base_style,
                    is_selected,
                )),
                Cell::from(highlighted_line(
                    &dir_display,
                    &dir_indices,
                    base_style,
                    is_selected,
                )),
                Cell::from(Span::styled(format_date(s.time_created), base_style)),
            ])
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

    frame.render_stateful_widget(table, area, &mut app.table_state);
}

/// Remap match indices from the original directory path to the shortened display path.
/// The shortened path replaces `/home/user` with `~`, so indices shift accordingly.
fn remap_dir_indices(original: &str, shortened: &str, indices: &[usize]) -> Vec<usize> {
    if indices.is_empty() || original == shortened {
        return indices.to_vec();
    }

    // Figure out how many chars were removed by the ~ substitution
    let orig_char_count = original.chars().count();
    let short_char_count = shortened.chars().count();
    let offset = orig_char_count.saturating_sub(short_char_count);

    // The ~ replaces the home directory prefix. Indices into the home prefix part
    // map to index 0 (~). Indices after the prefix shift left by offset.
    // offset = home_prefix_len - 1 (since ~ is 1 char replacing home_prefix_len chars)
    // Characters at indices 0..=offset are in the home prefix, map to 0 (~).
    // Characters at indices > offset map to idx - offset in the shortened string.
    indices
        .iter()
        .filter_map(|&idx| {
            if idx <= offset {
                Some(0)
            } else {
                let new_idx = idx - offset;
                if new_idx < short_char_count {
                    Some(new_idx)
                } else {
                    None
                }
            }
        })
        .collect()
}

fn draw_status(frame: &mut Frame, app: &App, area: Rect) {
    let count = app.filtered.len();
    let total = app.sessions.len();

    let sort_label = if app.sort_by_date { "date" } else { "score" };

    let loading_indicator = if app.loading {
        "  loading sessions..."
    } else if app.loading_messages {
        "  loading messages..."
    } else if let Some(ref err) = app.load_error {
        return draw_error_status(frame, err, area);
    } else {
        ""
    };

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
        Span::styled(
            loading_indicator.to_string(),
            Style::default().fg(Color::Yellow),
        ),
    ]);
    frame.render_widget(Paragraph::new(status), area);
}

fn draw_error_status(frame: &mut Frame, error: &str, area: Rect) {
    let status = Line::from(vec![Span::styled(
        format!(" Error: {error}"),
        Style::default().fg(Color::Red),
    )]);
    frame.render_widget(Paragraph::new(status), area);
}
