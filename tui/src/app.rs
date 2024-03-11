use crate::{backend::Backend as AppBackend, ui::render_graph};
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

pub struct App {
    pub focus: Focus,
    pub backend: AppBackend,
    pub selected_query: String,
    pub datasets: BTreeMap<String, BTreeMap<String, Vec<(f64, f64)>>>,
}

impl App {
    pub fn new(theme: usize, backend: AppBackend) -> Self {
        backend.start();
        Self {
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
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('q') => return Ok(()),
                            _ => (),
                        }
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
