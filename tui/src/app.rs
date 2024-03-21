use crate::{
    backend::{Backend as AppBackend, Bounds},
    query::NRQL,
    ui::{render_graph, render_query_box, render_query_list, render_rename_dialog},
};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Layout},
    widgets::ListState,
    Frame, Terminal,
};
use std::{
    collections::{
        btree_map::{self, Entry},
        BTreeMap, HashMap,
    },
    time::Duration,
};
use tokio::io;

#[derive(Clone, Copy)]
pub enum Focus {
    QueryInput,
    Rename,
    Default,
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
}

pub struct App {
    pub inputs: BTreeMap<String, Input>,
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
            // input: String::new(),
            inputs: BTreeMap::from([
                (
                    "query".to_owned(),
                    Input {
                        buffer: String::new(),
                        cursor_position: 0,
                    },
                ),
                (
                    "rename".to_owned(),
                    Input {
                        buffer: String::new(),
                        cursor_position: 0,
                    },
                ),
            ]),
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
                    let buffer = match self.focus {
                        Focus::QueryInput => "query",
                        Focus::Rename => "rename",
                        _ => "",
                    };
                    match self.input_mode {
                        InputMode::Normal if key.kind == KeyEventKind::Press => match key.code {
                            KeyCode::Char('q') => return Ok(()),
                            KeyCode::Char('e') => {
                                self.focus = Focus::QueryInput;
                                self.input_mode = InputMode::Input;
                            }
                            KeyCode::Char('j') => self.next(),
                            KeyCode::Char('k') => self.previous(),
                            KeyCode::Char('x') => self.delete(),
                            KeyCode::Char('r') => {
                                self.focus = Focus::Rename;
                                self.input_mode = InputMode::Input;
                            }
                            _ => (),
                        },
                        InputMode::Input if key.kind == KeyEventKind::Press => match key.code {
                            KeyCode::Enter => match buffer {
                                "query" => {
                                    if let Ok(query) =
                                        self.inputs.get(buffer).unwrap().buffer.as_str().to_nrql()
                                    {
                                        self.backend.add_query(query);
                                    }
                                    self.inputs.get_mut(buffer).unwrap().buffer.clear();
                                    self.reset_cursor(buffer);
                                    self.focus = Focus::Default;
                                    self.input_mode = InputMode::Normal;
                                }
                                "rename" => {
                                    self.datasets
                                        .entry(self.selected_query.to_owned())
                                        .and_modify(|v| {
                                            v.query_alias = Some(
                                                self.inputs.get(buffer).unwrap().buffer.to_owned(),
                                            );
                                        });
                                    self.inputs.get_mut(buffer).unwrap().buffer.clear();
                                    self.reset_cursor(buffer);
                                    self.focus = Focus::Default;
                                    self.input_mode = InputMode::Normal;
                                }
                                _ => {}
                            },
                            KeyCode::Char(to_insert) => {
                                self.enter_char(buffer, to_insert);
                            }
                            KeyCode::Backspace => {
                                self.delete_char(buffer);
                            }
                            KeyCode::Left => {
                                self.move_cursor_left(buffer);
                            }
                            KeyCode::Right => {
                                self.move_cursor_right(buffer);
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
        }
    }

    fn clamp_cursor(&self, buffer: &str, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.inputs.get(buffer).unwrap().buffer.len())
    }

    fn reset_cursor(&mut self, buffer: &str) {
        self.inputs.get_mut(buffer).unwrap().cursor_position = 0;
    }

    // fn submit(&mut self, buffer: &str) {
    //     let query = self
    //         .inputs
    //         .get(buffer)
    //         .unwrap()
    //         .buffer
    //         .as_str()
    //         .to_nrql()
    //         .unwrap();
    // }

    fn move_cursor_left(&mut self, buffer: &str) {
        let cursor_moved_left = self
            .inputs
            .get(buffer)
            .unwrap()
            .cursor_position
            .saturating_sub(1);
        self.inputs.get_mut(buffer).unwrap().cursor_position =
            self.clamp_cursor(buffer, cursor_moved_left);
    }

    fn move_cursor_right(&mut self, buffer: &str) {
        let cursor_moved_right = self
            .inputs
            .get(buffer)
            .unwrap()
            .cursor_position
            .saturating_add(1);
        self.inputs.get_mut(buffer).unwrap().cursor_position =
            self.clamp_cursor(buffer, cursor_moved_right);
    }

    fn enter_char(&mut self, buffer: &str, new_char: char) {
        let cursor_position = self.inputs.get(buffer).unwrap().cursor_position;
        self.inputs
            .get_mut(buffer)
            .unwrap()
            .buffer
            .insert(cursor_position, new_char);

        self.move_cursor_right(buffer);
    }

    fn delete_char(&mut self, buffer: &str) {
        let is_not_cursor_leftmost = self.inputs.get(buffer).unwrap().cursor_position != 0;
        if is_not_cursor_leftmost {
            let current_index = self.inputs.get(buffer).unwrap().cursor_position;
            let from_left_to_current_index = current_index - 1;

            let before_char_to_delete = self
                .inputs
                .get(buffer)
                .unwrap()
                .buffer
                .chars()
                .take(from_left_to_current_index);
            let after_char_to_delete = self
                .inputs
                .get(buffer)
                .unwrap()
                .buffer
                .chars()
                .skip(current_index);

            self.inputs.get_mut(buffer).unwrap().buffer =
                before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left(buffer);
        }
    }

    // pub fn set_focus(&mut self, focus: Focus) {
    //     self.focus = focus;
    // }

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
