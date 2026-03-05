use std::sync::mpsc;

use ratatui::widgets::TableState;

use crate::db::{LoadMsg, Session};
use crate::fuzzy::{filter_sessions, ScoredSession};

/// The result of running the app — either the user selected a session or quit.
pub enum AppResult {
    Selected(Session),
    Quit,
}

pub struct App {
    pub sessions: Vec<Session>,
    pub filtered: Vec<ScoredSession>,
    pub query: String,
    pub selected: usize,
    pub sort_by_date: bool,
    pub result: Option<AppResult>,
    pub table_state: TableState,
    pub loading: bool,
    pub loading_messages: bool,
    pub load_error: Option<String>,
    receiver: mpsc::Receiver<LoadMsg>,
}

impl App {
    pub fn new(receiver: mpsc::Receiver<LoadMsg>) -> Self {
        App {
            sessions: Vec::new(),
            filtered: Vec::new(),
            query: String::new(),
            selected: 0,
            sort_by_date: false,
            result: None,
            table_state: TableState::default().with_selected(Some(0)),
            loading: true,
            loading_messages: false,
            load_error: None,
            receiver,
        }
    }

    /// Drain any pending messages from the background loader.
    /// Returns true if the display needs a redraw.
    pub fn poll_sessions(&mut self) -> bool {
        let mut needs_redraw = false;
        let mut sessions_added = false;
        loop {
            match self.receiver.try_recv() {
                Ok(LoadMsg::Batch(batch)) => {
                    self.sessions.extend(batch);
                    sessions_added = true;
                    needs_redraw = true;
                }
                Ok(LoadMsg::SessionsDone) => {
                    self.loading = false;
                    self.loading_messages = true;
                    needs_redraw = true;
                }
                Ok(LoadMsg::BackfillInput { index, last_input }) => {
                    if index < self.sessions.len() {
                        self.sessions[index].last_input = last_input;
                        // Update the matching filtered entry too
                        for scored in &mut self.filtered {
                            if scored.session.id == self.sessions[index].id {
                                scored.session.last_input = self.sessions[index].last_input.clone();
                                break;
                            }
                        }
                        needs_redraw = true;
                    }
                }
                Ok(LoadMsg::Done(err)) => {
                    self.loading = false;
                    self.loading_messages = false;
                    self.load_error = err;
                    needs_redraw = true;
                    break;
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.loading = false;
                    self.loading_messages = false;
                    needs_redraw = true;
                    break;
                }
            }
        }
        if sessions_added {
            self.update_filter();
        }
        needs_redraw
    }

    /// Re-filter sessions based on current query and sort mode.
    fn update_filter(&mut self) {
        self.filtered = filter_sessions(&self.sessions, &self.query, self.sort_by_date);
        // Clamp selection
        if self.filtered.is_empty() {
            self.selected = 0;
            self.table_state.select(None);
        } else if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len() - 1;
            self.table_state.select(Some(self.selected));
        }
    }

    /// Append a character to the query.
    pub fn type_char(&mut self, c: char) {
        self.query.push(c);
        self.update_filter();
    }

    /// Delete the last character from the query.
    pub fn backspace(&mut self) {
        self.query.pop();
        self.update_filter();
    }

    /// Toggle between sort-by-score and sort-by-date.
    pub fn toggle_sort(&mut self) {
        self.sort_by_date = !self.sort_by_date;
        self.update_filter();
    }

    /// Move selection up.
    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.table_state.select(Some(self.selected));
        }
    }

    /// Move selection down.
    pub fn move_down(&mut self) {
        if !self.filtered.is_empty() && self.selected < self.filtered.len() - 1 {
            self.selected += 1;
            self.table_state.select(Some(self.selected));
        }
    }

    /// Confirm the current selection.
    pub fn confirm(&mut self) {
        if let Some(scored) = self.filtered.get(self.selected) {
            self.result = Some(AppResult::Selected(scored.session.clone()));
        }
    }

    /// Quit without selecting.
    pub fn quit(&mut self) {
        self.result = Some(AppResult::Quit);
    }

    /// Whether the app should exit.
    pub fn should_exit(&self) -> bool {
        self.result.is_some()
    }
}
