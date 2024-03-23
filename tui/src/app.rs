use crate::{
    backend::{Backend as AppBackend, Bounds},
    query::NRQL,
    ui::{
        render_dashboard, render_graph, render_query_box, render_query_list, render_rename_dialog,
    },
};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Layout},
    widgets::ListState,
    Frame, Terminal,
};
use std::{
    collections::{btree_map::Entry, BTreeMap},
    time::Duration,
};
use tokio::io;

const DEFAULT: isize = 0;
const QUERY: isize = 1;
const RENAME: isize = 2;
const DASHBOARD: isize = 4;

#[derive(Clone, Copy, PartialEq)]
pub enum Focus {
    QueryInput = QUERY,
    Rename = RENAME,
    Dashboard = DASHBOARD,
    Default = DEFAULT,
}

pub enum InputMode {
    Normal,
    Input,
}

pub struct Input {
    pub buffer: String,
    pub cursor_position: usize,
}

pub struct Dataset {
    pub query_alias: Option<String>,
    pub facets: BTreeMap<String, Vec<(f64, f64)>>,
    pub bounds: Bounds,
    pub selection: String,
}

pub struct App {
    pub inputs: [Input; 4],
    pub input_mode: InputMode,
    pub focus: Focus,
    pub backend: AppBackend,
    pub selected_query: String,
    pub list_state: ListState,
    pub datasets: BTreeMap<String, Dataset>,
}

impl App {
    pub fn new(theme: usize, backend: AppBackend) -> Self {
        Self {
            inputs: [
                Input {
                    buffer: "".to_owned(),
                    cursor_position: 0,
                },
                Input {
                    buffer: "".to_owned(),
                    cursor_position: 0,
                },
                Input {
                    buffer: "".to_owned(),
                    cursor_position: 0,
                },
                Input {
                    buffer: "".to_owned(),
                    cursor_position: 0,
                },
            ],
            input_mode: InputMode::Normal,
            focus: Focus::Default,
            backend,
            selected_query: String::new(),
            list_state: ListState::default(),
            datasets: BTreeMap::default(),
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
                                self.set_focus(Focus::QueryInput);
                                self.input_mode = InputMode::Input;
                            }
                            KeyCode::Char('j') => self.next(),
                            KeyCode::Char('k') => self.previous(),
                            KeyCode::Char('x') => self.delete(),
                            KeyCode::Char('r') => {
                                self.set_focus(Focus::Rename);
                                self.input_mode = InputMode::Input;
                            }
                            KeyCode::Char('d') => {
                                self.set_focus(Focus::Dashboard);
                            }
                            _ => (),
                        },
                        InputMode::Input if key.kind == KeyEventKind::Press => match key.code {
                            KeyCode::Enter => {
                                match self.focus {
                                    Focus::QueryInput => {
                                        if let Ok(query) = self.inputs[Focus::QueryInput as usize]
                                            .buffer
                                            .as_str()
                                            .to_nrql()
                                        {
                                            self.backend.add_query(query);
                                        }
                                    }
                                    Focus::Rename => {
                                        self.datasets
                                            .entry(self.selected_query.to_owned())
                                            .and_modify(|v| {
                                                v.query_alias = Some(
                                                    self.inputs[Focus::Rename as usize]
                                                        .buffer
                                                        .to_owned(),
                                                );
                                            });
                                    }
                                    _ => {}
                                };
                                self.inputs[self.focus as usize].buffer.clear();
                                self.reset_cursor();
                                self.set_focus(Focus::Default);
                                self.input_mode = InputMode::Normal;
                            }
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
                if let Entry::Vacant(e) = self.datasets.entry(payload.query.clone()) {
                    e.insert(Dataset {
                        query_alias: None,
                        facets: payload.data,
                        bounds: payload.bounds,
                        selection: payload.selection,
                    });
                } else {
                    _ = self
                        .datasets
                        .entry(payload.query.to_owned())
                        .and_modify(|data| {
                            data.facets = payload.data;
                            data.bounds = payload.bounds;
                        })
                }
            }
        }
    }

    pub fn ui(&mut self, frame: &mut Frame) {
        if self.focus == Focus::Dashboard {
            render_dashboard(self, frame, frame.size());
            return;
        }
        let area = frame.size();
        // TODO: Possible to pre-compute?
        let horizontal = Layout::horizontal([Constraint::Percentage(15), Constraint::Min(20)]);
        let vertical = Layout::vertical([Constraint::Length(3), Constraint::Min(20)]);
        let [input_area, rest] = vertical.areas(area);
        let [list_area, graph_area] = horizontal.areas(rest);

        render_query_box(self, frame, input_area);
        render_query_list(self, frame, list_area);
        match self.focus {
            Focus::Default | Focus::QueryInput => {
                render_graph(self, frame, graph_area);
            }
            Focus::Rename => {
                render_rename_dialog(self, frame, graph_area);
            }
            Focus::Dashboard => render_dashboard(self, frame, frame.size()),
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.inputs[self.focus as usize].buffer.len())
    }

    fn reset_cursor(&mut self) {
        self.inputs[self.focus as usize].cursor_position = 0;
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.inputs[self.focus as usize]
            .cursor_position
            .saturating_sub(1);
        self.inputs[self.focus as usize].cursor_position = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.inputs[self.focus as usize]
            .cursor_position
            .saturating_add(1);
        self.inputs[self.focus as usize].cursor_position = self.clamp_cursor(cursor_moved_right);
    }

    fn enter_char(&mut self, new_char: char) {
        let cursor_position = self.inputs[self.focus as usize].cursor_position;
        self.inputs[self.focus as usize]
            .buffer
            .insert(cursor_position, new_char);

        self.move_cursor_right();
    }

    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.inputs[self.focus as usize].cursor_position != 0;
        if is_not_cursor_leftmost {
            let current_index = self.inputs[self.focus as usize].cursor_position;
            let from_left_to_current_index = current_index - 1;

            let before_char_to_delete = self.inputs[self.focus as usize]
                .buffer
                .chars()
                .take(from_left_to_current_index);
            let after_char_to_delete = self.inputs[self.focus as usize]
                .buffer
                .chars()
                .skip(current_index);

            self.inputs[self.focus as usize].buffer =
                before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    pub fn set_focus(&mut self, focus: Focus) {
        self.focus = focus
    }

    pub fn delete(&mut self) {
        let i = self.list_state.selected().unwrap();
        let to_delete = self
            .datasets
            .keys()
            .nth(i)
            .cloned()
            .expect("ERROR: Could not index query for deletion!");

        let (removed, _) = self.datasets.remove_entry(&to_delete).unwrap();
        _ = self.backend.ui_tx.send(removed);
    }

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
