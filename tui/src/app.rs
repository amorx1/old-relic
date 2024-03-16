use crate::{backend::Backend as AppBackend, query::NRQL, ui::render_graph};
use anyhow::anyhow;
use chrono::{Timelike, Utc};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{backend::Backend, Frame, Terminal};
use std::{collections::BTreeMap, time::Duration};
use tokio::io;

#[derive(Clone, Copy)]
pub enum Focus {
    Graph,
    Popup,
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
    pub datasets: BTreeMap<String, BTreeMap<String, Vec<(f64, f64)>>>,
}

impl App {
    pub fn new(theme: usize, backend: AppBackend) -> Self {
        // backend.start();
        Self {
            input: String::new(),
            input_mode: InputMode::Normal,
            cursor_position: 0,
            focus: Focus::Graph,
            backend,
            selected_query: String::new(),
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
                                self.input_mode = InputMode::Input;
                            }
                            KeyCode::Char('a') => {
                                let query = "FROM Metric SELECT sum(apm.service.overview.web) WHERE (appName = 'fre-consignment-api-v2-prod') FACET `segmentName` SINCE 30 minutes ago UNTIL now LIMIT MAX TIMESERIES".to_nrql().expect("ERROR: Invalid NRQL query!");
                                self.backend.add_query(query);
                            }
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
                self.selected_query = payload.query.to_owned();
                self.datasets.insert(payload.query, payload.data);
            }
        }
    }

    pub fn ui(&mut self, frame: &mut Frame) {
        let area = frame.size();

        match self.focus {
            // Alert for upcoming event
            Focus::Graph => {
                render_graph(self, frame, area);
            }
            _ => todo!(), // Detailed view for selected event
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.len())
    }

    fn reset_cursor(&mut self) {
        self.cursor_position = 0;
    }

    // TODO: Save entered query
    fn submit_message(&mut self) {
        // self.messages.push(self.input.clone());
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

    // pub fn next(&mut self) {
    //     let i = match self.table_state.selected() {
    //         Some(i) => {
    //             if i >= self.events.len() - 1 {
    //                 0
    //             } else {
    //                 i + 1
    //             }
    //         }
    //         None => 0,
    //     };
    //     self.table_state.select(Some(i));
    // }

    // pub fn previous(&mut self) {
    //     let i = match self.table_state.selected() {
    //         Some(i) => {
    //             if i == 0 {
    //                 self.events.len() - 1
    //             } else {
    //                 i - 1
    //             }
    //         }
    //         None => 0,
    //     };
    //     self.table_state.select(Some(i));
    // }
}
