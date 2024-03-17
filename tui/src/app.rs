use crate::{
    backend::{Backend as AppBackend, Bounds},
    query::NRQL,
    ui::{render_graph, render_query_box, render_query_list},
};
use anyhow::anyhow;
use chrono::{Timelike, Utc};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Layout},
    widgets::ListState,
    Frame, Terminal,
};
use std::{
    collections::{BTreeMap, HashSet},
    time::Duration,
};
use tokio::io;

#[derive(Clone, Copy)]
pub enum Focus {
    Graph,
    QueryList,
}

pub enum InputMode {
    Normal,
    Input,
}

pub struct App {
    pub input: String,
    pub input_mode: InputMode,
    pub cursor_position: usize,
    pub focus: Focus,
    pub backend: AppBackend,
    pub selected_query: String,
    // pub queries: HashSet<String>,
    pub list_state: ListState,
    pub datasets: BTreeMap<String, BTreeMap<String, Vec<(f64, f64)>>>,
    pub bounds: BTreeMap<String, Bounds>,
}

impl App {
    pub fn new(theme: usize, backend: AppBackend) -> Self {
        Self {
            input: String::new(),
            input_mode: InputMode::Normal,
            cursor_position: 0,
            focus: Focus::Graph,
            backend,
            selected_query: String::new(),
            // queries: HashSet::from(["Query 1".into(), "Query 2".into(), "Query 3".into()]),
            list_state: ListState::default(),
            datasets: BTreeMap::default(),
            bounds: BTreeMap::default(),
        }
    }

    pub fn run<B: Backend>(mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            // Manual event handlers.
            if let Ok(true) = event::poll(Duration::from_millis(50)) {
                if let Event::Key(key) = event::read()? {
                    match self.input_mode {
                        InputMode::Normal if key.kind == KeyEventKind::Press => match key.code {
                            KeyCode::Char('q') => return Ok(()),
                            KeyCode::Char('e') => {
                                self.input_mode = InputMode::Input;
                            }
                            KeyCode::Char('j') => self.next(),
                            KeyCode::Char('k') => self.previous(),
                            _ => (),
                        },
                        InputMode::Input if key.kind == KeyEventKind::Press => match key.code {
                            KeyCode::Enter => self.submit_message(),
                            KeyCode::Char(to_insert) => {
                                self.enter_char(to_insert);
                            }
                            KeyCode::Backspace => {
                                self.delete_char();
                            }
                            KeyCode::Left => {
                                self.move_cursor_left();
                            }
                            KeyCode::Right => {
                                self.move_cursor_right();
                            }
                            KeyCode::Esc => {
                                self.input_mode = InputMode::Normal;
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }
            }

            while let Some(payload) = self.backend.data_rx.try_iter().next() {
                // TODO: Fix selection
                self.bounds.insert(payload.query.to_owned(), payload.bounds);
                self.datasets.insert(payload.query, payload.data);
            }
        }
    }

    pub fn ui(&mut self, frame: &mut Frame) {
        let area = frame.size();
        // TODO: Possible to pre-compute?
        let horizontal = Layout::horizontal([Constraint::Percentage(15), Constraint::Min(20)]);
        let vertical = Layout::vertical([Constraint::Length(3), Constraint::Min(20)]);
        let [input_area, rest] = vertical.areas(area);
        let [list_area, graph_area] = horizontal.areas(rest);

        render_query_box(self, frame, input_area);
        render_query_list(self, frame, list_area);
        match self.focus {
            Focus::Graph => {
                render_graph(self, frame, graph_area);
            }
            _ => todo!(),
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.len())
    }

    fn reset_cursor(&mut self) {
        self.cursor_position = 0;
    }

    fn submit_message(&mut self) {
        let query = self.input.as_str().to_nrql().unwrap();
        self.backend.add_query(query);
        self.input.clear();
        self.reset_cursor();
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.cursor_position.saturating_sub(1);
        self.cursor_position = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.cursor_position.saturating_add(1);
        self.cursor_position = self.clamp_cursor(cursor_moved_right);
    }

    fn enter_char(&mut self, new_char: char) {
        self.input.insert(self.cursor_position, new_char);

        self.move_cursor_right();
    }

    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.cursor_position != 0;
        if is_not_cursor_leftmost {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.

            let current_index = self.cursor_position;
            let from_left_to_current_index = current_index - 1;

            // Getting all characters before the selected character.
            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            // Getting all characters after selected character.
            let after_char_to_delete = self.input.chars().skip(current_index);

            // Put all characters together except the selected one.
            // By leaving the selected one out, it is forgotten and therefore deleted.
            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    // pub fn set_focus(&mut self, focus: Focus) {
    //     self.focus = focus;
    // }

    // pub fn popup(&mut self) {
    //     self.focus = Focus::Popup;
    //     _ = Command::new("zellij")
    //         .args(["action", "toggle-floating-panes"])
    //         .status()
    //         .expect("ERROR: Could not send command to Zellij");
    // }

    pub fn next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.datasets.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
        self.selected_query = self
            .datasets
            .keys()
            .nth(i)
            .expect("ERROR: Could not select query!")
            .to_owned();
    }

    pub fn previous(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.datasets.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
        self.selected_query = self
            .datasets
            .keys()
            .nth(i)
            .expect("ERROR: Could not select query!")
            .to_owned();
    }
}
