use crate::{
    backend::{Bounds, PayloadType, UIEvent},
    dataset::{Data, Dataset, Logs},
    input::Inputs,
    ui::ui,
    Config,
};

use anyhow::Result;
use crossbeam_channel::Sender as CrossBeamSender;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use log::{error, info};
use rand::{thread_rng, Rng};
use ratatui::{backend::Backend, style::Color, widgets::ListState, Terminal};
use std::{
    collections::{btree_map::Entry, BTreeMap, HashSet, VecDeque},
    fs::{self, OpenOptions},
    io::Write,
    sync::mpsc::Receiver,
    time::Duration,
};
use tokio::io;

pub const ALL_COLUMN_SEARCH: &str =
    "SELECT * FROM Log WHERE allColumnSearch('$', insensitive: true)";

#[derive(Debug)]
pub struct UI {
    pub tab: Tab,
    pub panel: Focus,
    pub input_mode: InputMode,
    pub loading: bool,
}

impl Default for UI {
    fn default() -> Self {
        UI {
            tab: Tab::Logs,
            panel: Focus::Default,
            input_mode: InputMode::Normal,
            loading: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    QueryInput = 0,
    Rename = 1,
    Dashboard = 4,
    SessionLoad = 2,
    SessionSave = 5,
    Default = 3,
    Log = 6,
    LogDetail = 7,
    Search = 8,
    NoResult = 9,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    Normal,
    Input,
}

pub struct Theme {
    pub focus_fg: Color,
    pub chart_fg: Color,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Graph = 0,
    Logs = 1,
}

pub struct App {
    pub config: Box<Config>,
    pub inputs: Inputs,
    pub focus: UI,
    pub tabs: Vec<String>,
    pub data_rx: Receiver<PayloadType>,
    pub ui_tx: CrossBeamSender<UIEvent>,
    pub data: Data,
    pub redraw: bool,
}

impl App {
    pub fn new(
        config: Box<Config>,
        data_rx: Receiver<PayloadType>,
        ui_tx: CrossBeamSender<UIEvent>,
    ) -> Self {
        Self {
            inputs: Inputs::new(),
            config,
            data_rx,
            ui_tx,
            focus: UI::default(),
            data: Data::default(),
            tabs: vec!["Logs".into()],
            redraw: true,
        }
    }

    pub fn run<B: Backend>(mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        let mut rng = thread_rng();
        loop {
            if self.redraw ||
                // TODO: Currently redrawing on each tick to keep graph live, should be on an interval
                // TODO: Redraw on window resizing
                (!self.data.timeseries.is_empty() && self.focus.tab == Tab::Graph)
            {
                terminal.draw(|f| ui(&mut self, f))?;
            }

            self.redraw = false;

            // Session Load
            if !self.config.session.is_loaded {
                self.set_focus(UI {
                    panel: Focus::SessionLoad,
                    input_mode: InputMode::Input,
                    ..self.focus
                });
                self.redraw = true;
            }

            // Event handlers
            if let Ok(true) = event::poll(Duration::from_millis(50)) {
                if let Event::Key(key) = event::read()? {
                    match self.focus.input_mode {
                        // Normal Mode
                        InputMode::Normal if key.kind == KeyEventKind::Press => match key.code {
                            KeyCode::Char('q') => {
                                self.set_focus(UI {
                                    panel: Focus::SessionSave,
                                    input_mode: InputMode::Input,
                                    ..self.focus
                                });
                            }
                            KeyCode::Char('F') => self.set_focus(UI {
                                panel: Focus::Search,
                                input_mode: InputMode::Input,
                                ..self.focus
                            }),
                            KeyCode::Char('e') => {
                                self.set_focus(UI {
                                    panel: Focus::QueryInput,
                                    input_mode: InputMode::Input,
                                    ..self.focus
                                });
                            }
                            KeyCode::Char('j') | KeyCode::Down => match self.focus.panel {
                                Focus::LogDetail => {}
                                _ => self.next(),
                            },
                            KeyCode::Char('k') | KeyCode::Up => match self.focus.panel {
                                Focus::LogDetail => {}
                                _ => self.previous(),
                            },
                            KeyCode::Char('x') => self.delete_query(),
                            KeyCode::Char('r') => match self.focus.panel {
                                Focus::QueryInput => {}
                                _ => {
                                    if !self.data.timeseries.is_empty() {
                                        self.set_focus(UI {
                                            panel: Focus::Rename,
                                            input_mode: InputMode::Input,
                                            ..self.focus
                                        });
                                    }
                                }
                            },
                            KeyCode::Char('d') => match self.focus.panel {
                                Focus::Dashboard => self.set_focus(UI {
                                    panel: Focus::Default,
                                    ..self.focus
                                }),
                                _ => self.set_focus(UI {
                                    panel: Focus::Dashboard,
                                    ..self.focus
                                }),
                            },
                            KeyCode::Char('i') => self.rehydrate_query(),
                            KeyCode::Char('T') => self.next_tab(),
                            KeyCode::Char('C') => self.clear_filters(),
                            KeyCode::Esc => self.set_focus(UI {
                                panel: Focus::Default,
                                ..self.focus
                            }),
                            KeyCode::Enter | KeyCode::Char(' ') => match self.focus.panel {
                                Focus::Log => self.set_focus(UI {
                                    panel: Focus::LogDetail,
                                    ..self.focus
                                }),
                                Focus::LogDetail => {
                                    let key_idx =
                                        self.data.logs.log_item_list_state.selected().unwrap();
                                    let log =
                                        &self.data.logs.selected().unwrap()[key_idx].to_string();
                                    let value = log
                                        .split(' ')
                                        .last()
                                        .unwrap()
                                        .trim_matches(|p| char::is_ascii_punctuation(&p));

                                    self.add_query(value.to_owned());
                                    self.set_focus(UI {
                                        panel: Focus::Default,
                                        ..self.focus
                                    });
                                }
                                Focus::Default => {
                                    if self.focus.tab != Tab::Graph {
                                        self.set_focus(UI {
                                            panel: Focus::Log,
                                            ..self.focus
                                        })
                                    }
                                }
                                _ => {}
                            },
                            _ => (),
                        },

                        // Input Mode
                        InputMode::Input if key.kind == KeyEventKind::Press => match key.code {
                            KeyCode::Enter => {
                                match self.focus.panel {
                                    Focus::QueryInput => {
                                        let raw_query = self.inputs.get(Focus::QueryInput);
                                        self.add_query(raw_query.to_owned());
                                        self.set_focus(UI {
                                            loading: true,
                                            ..self.focus
                                        });
                                    }
                                    Focus::Rename => {
                                        self.rename_query(
                                            self.data.timeseries.selected.to_owned(),
                                            self.inputs.get(Focus::Rename).to_owned(),
                                        );
                                    }
                                    Focus::Search => {
                                        let filter = self.inputs.get(Focus::Search);
                                        self.add_filter(filter.into());
                                        self.set_focus(UI {
                                            panel: Focus::Default,
                                            ..self.focus
                                        });
                                    }
                                    Focus::SessionLoad => {
                                        match self.inputs.get(Focus::SessionLoad) {
                                            // Load session
                                            "y" | "Y" => {
                                                if let Err(e) = self.load_session() {
                                                    error!("{}", e);
                                                }
                                            }
                                            // Don't load session
                                            _ => {}
                                        }
                                        self.config.session.is_loaded = true;
                                        // Update focus to default
                                        self.set_focus(UI {
                                            panel: Focus::Default,
                                            ..self.focus
                                        });
                                    }
                                    Focus::SessionSave => {
                                        match self.inputs.get(Focus::SessionSave) {
                                            // Save session
                                            "y" | "Y" => {
                                                if let Err(e) = self.save_session() {
                                                    error!("{}", e);
                                                }
                                            }
                                            _ => {}
                                        }
                                        return Ok(());
                                    }
                                    _ => {}
                                };
                                self.inputs.clear(self.focus.panel);
                                self.inputs.reset_cursor(self.focus.panel);
                                self.set_focus(UI {
                                    panel: Focus::Default,
                                    input_mode: InputMode::Normal,
                                    ..self.focus
                                });
                            }
                            KeyCode::Char(to_insert) => {
                                self.inputs.enter_char(self.focus.panel, to_insert);
                            }
                            KeyCode::Backspace => {
                                self.inputs.delete_char(self.focus.panel);
                            }
                            KeyCode::Left => {
                                self.inputs.move_cursor_left(self.focus.panel);
                            }
                            KeyCode::Right => {
                                self.inputs.move_cursor_right(self.focus.panel);
                            }
                            KeyCode::Up => {
                                self.inputs.clear(Focus::QueryInput);
                                let query = self.data.query_history.pop_front().unwrap_or_default();
                                self.inputs.set(Focus::QueryInput, query.clone());
                                self.inputs.move_cursor_end(Focus::QueryInput);
                                self.data.query_history.push_back(query);
                            }
                            KeyCode::Down => {
                                let query = self.data.query_history.pop_back().unwrap_or_default();
                                self.inputs.set(Focus::QueryInput, query.clone());
                                self.inputs.move_cursor_end(Focus::QueryInput);
                                self.data.query_history.push_front(query);
                            }
                            KeyCode::Esc => match self.focus.panel {
                                Focus::SessionLoad => {}
                                _ => {
                                    self.set_focus(UI {
                                        panel: Focus::Default,
                                        input_mode: InputMode::Normal,
                                        ..self.focus
                                    });
                                }
                            },
                            _ => {}
                        },
                        _ => {}
                    }
                    // On any key event, we probably want to redraw
                    self.redraw = true;
                }
            }

            while let Some(payload) = self.data_rx.try_iter().next() {
                match payload {
                    PayloadType::None => self.set_focus(UI {
                        panel: Focus::NoResult,
                        loading: false,
                        ..self.focus
                    }),
                    PayloadType::Timeseries(payload) => {
                        info!("Received payload from backend of type: TimeseriesPayload");

                        if let Entry::Vacant(e) = self.data.timeseries.entry(payload.query.clone())
                        {
                            // New query
                            e.insert(Dataset {
                                query_alias: None,
                                facets: payload.data,
                                bounds: payload.bounds,
                                selection: payload.selection,
                                has_data: true,
                            });

                            // Only switch focus to Graph for new query
                            self.set_focus(UI {
                                loading: false,
                                tab: Tab::Graph,
                                ..self.focus
                            });
                        } else {
                            // Update data for existing query
                            _ = self
                                .data
                                .timeseries
                                .entry(payload.query)
                                .and_modify(|data| {
                                    data.facets = payload.data;
                                    data.bounds = payload.bounds;
                                    data.has_data = true
                                });

                            self.set_focus(UI {
                                loading: false,
                                ..self.focus
                            });
                        }

                        for facet_key in payload.facets {
                            // Only add facet key if not present
                            if let Entry::Vacant(e) = self.data.facet_colours.entry(facet_key) {
                                e.insert(Color::Rgb(
                                    rng.gen::<u8>(),
                                    rng.gen::<u8>(),
                                    rng.gen::<u8>(),
                                ));
                            }
                        }
                    }
                    PayloadType::Log(payload) => {
                        info!("Received payload from backend of type: LogPayload");

                        let mut logs: BTreeMap<String, Vec<String>> = BTreeMap::new();
                        for (timestamp, log) in payload.logs {
                            logs.insert(timestamp, log.split('\n').map(|v| v.into()).collect());
                        }

                        if !logs.is_empty() {
                            self.data.logs = Logs {
                                selected: logs.first_entry().unwrap().key().into(),
                                logs,
                                log_item_list_state: ListState::default(),
                                chart_data: payload.chart_data,
                                bounds: payload.bounds,
                                filters: HashSet::default(),
                                log_list_state: ListState::default(),
                            };
                        }

                        self.set_focus(UI {
                            loading: false,
                            tab: Tab::Logs,
                            ..self.focus
                        });
                    }
                }

                // Redraw on new data
                self.redraw = true;
            }
        }
    }

    fn add_filter(&mut self, filter: String) {
        info!("Filtering logs by: {}", &filter);

        self.data.logs.filters.insert(filter.clone());
        self.data.logs.logs.retain(|_key, value| {
            for line in value {
                if line.contains(&filter) {
                    return true;
                }
            }
            false
        });

        // Reset list
        if !self.data.logs.is_empty() {
            self.data.logs.select(0);
            self.data.logs.log_list_state.select(Some(0));
        }
    }

    fn rename_query(&mut self, query: String, alias: String) {
        info!("Renaming query: {} -> {}", &query, &alias);

        if let Entry::Vacant(e) = self.data.timeseries.entry(query.to_owned()) {
            e.insert(Dataset {
                has_data: false,
                query_alias: Some(alias),
                facets: BTreeMap::default(),
                bounds: Bounds::default(),
                selection: String::new(),
            });
        } else {
            _ = self
                .data
                .timeseries
                .entry(query.to_owned())
                .and_modify(|data| {
                    data.query_alias = Some(alias);
                })
        }
    }

    fn add_query(&mut self, query: String) {
        self.data.query_history.push_back(query.clone());
        _ = self.ui_tx.send(UIEvent::AddQuery(query));
    }

    pub fn set_focus(&mut self, focus: UI) {
        self.focus = focus;
    }

    pub fn delete_query(&mut self) {
        let i = self.data.timeseries.list_state.selected().unwrap();

        let removed = self.data.timeseries.remove_entry(i);
        // TODO: Fix deleted queries reappearing on new data!
        _ = self.ui_tx.send(UIEvent::DeleteQuery(removed));
    }

    pub fn next(&mut self) {
        match self.focus.tab {
            Tab::Graph => {
                if self.data.timeseries.is_empty() {
                    return;
                }

                let i = match self.data.timeseries.list_state.selected() {
                    Some(i) => {
                        if i >= self.data.timeseries.len() - 1 {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };

                self.data.timeseries.list_state.select(Some(i));
                self.data.timeseries.select(i);
            }
            Tab::Logs => match self.focus.panel {
                Focus::Log => {
                    if self.data.logs.is_empty() {
                        return;
                    }

                    let i = match self.data.logs.log_item_list_state.selected() {
                        Some(i) => {
                            if i >= self.data.logs.selected().unwrap().len() - 1 {
                                0
                            } else {
                                i + 1
                            }
                        }
                        None => 0,
                    };

                    self.data.logs.log_item_list_state.select(Some(i));
                    // self.logs.select(i);
                }
                _ => {
                    if self.data.logs.is_empty() {
                        return;
                    }

                    let i = match self.data.logs.log_list_state.selected() {
                        Some(i) => {
                            if i >= self.data.logs.len() - 1 {
                                0
                            } else {
                                i + 1
                            }
                        }
                        None => 0,
                    };

                    self.data.logs.log_list_state.select(Some(i));
                    self.data.logs.select(i);
                }
            },
        }
    }

    pub fn previous(&mut self) {
        match self.focus.tab {
            Tab::Graph => {
                if self.data.timeseries.is_empty() {
                    return;
                }

                let i = match self.data.timeseries.list_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            self.data.timeseries.len() - 1
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.data.timeseries.list_state.select(Some(i));
                self.data.timeseries.select(i);
            }
            Tab::Logs => match self.focus.panel {
                Focus::Log => {
                    if self.data.logs.is_empty() {
                        return;
                    }

                    let i = match self.data.logs.log_item_list_state.selected() {
                        Some(i) => {
                            if i == 0 {
                                self.data.logs.selected().unwrap().len() - 1
                            } else {
                                i - 1
                            }
                        }
                        None => 0,
                    };
                    self.data.logs.log_item_list_state.select(Some(i));
                    // self.logs.select(i);
                }
                _ => {
                    if self.data.logs.is_empty() {
                        return;
                    }

                    let i = match self.data.logs.log_list_state.selected() {
                        Some(i) => {
                            if i == 0 {
                                self.data.logs.len() - 1
                            } else {
                                i - 1
                            }
                        }
                        None => 0,
                    };
                    self.data.logs.log_list_state.select(Some(i));
                    self.data.logs.select(i);
                }
            },
        }
    }

    pub fn load_session(&mut self) -> Result<()> {
        let session_path = self.config.session.session_path.clone();
        let yaml = fs::read_to_string(session_path)?;
        let session_queries: Vec<String> = serde_yaml::from_str(&yaml)?;

        info!(
            "Successfully loaded session: {} queries",
            &session_queries.len()
        );

        self.data.query_history = VecDeque::from(session_queries);
        self.config.session.is_loaded = true;

        Ok(())
    }

    pub fn save_session(&self) -> Result<()> {
        let mut out = String::new();

        let timeseries_queries = self
            .data
            .timeseries
            .iter()
            .map(|(q, data)| {
                (
                    data.query_alias.clone().unwrap_or(q.to_owned()),
                    q.to_owned(),
                )
            })
            .collect::<BTreeMap<String, String>>();

        if !timeseries_queries.is_empty() {
            let yaml: String =
                serde_yaml::to_string(&timeseries_queries.values().collect::<Vec<_>>())?;

            out += &yaml;
        }

        let log_queries = serde_yaml::to_string(&self.data.query_history)?;

        out += &log_queries;

        let session_path = self.config.session.session_path.clone();
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .truncate(true)
            .create(true)
            .open(session_path)?;
        file.write_all(out.as_bytes())?;

        info!("Successfully saved session");
        Ok(())
    }

    fn previous_tab(&mut self) {
        match self.focus.tab {
            Tab::Graph => self.focus.tab = Tab::Logs,
            // Tab::Logs => self.focus.tab = Tab::Graph,
            Tab::Logs => self.focus.tab = Tab::Logs,
        }
    }

    fn next_tab(&mut self) {
        // TODO: Handle n tabs
        match self.focus.tab {
            Tab::Graph => self.focus.tab = Tab::Logs,
            Tab::Logs => self.focus.tab = Tab::Graph,
            // Tab::Logs => self.focus.tab = Tab::Logs,
        }
    }

    fn clear_filters(&mut self) {
        self.data.logs.filters.clear();
        self.add_query(
            self.data
                .query_history
                .back()
                .unwrap_or(&String::default())
                .to_owned(),
        );

        // Reset list
        if !self.data.logs.is_empty() {
            self.data.logs.select(0);
            self.data.logs.log_list_state.select(Some(0));
        }
    }

    fn rehydrate_query(&mut self) {
        let selected_query = self.data.timeseries.selected.to_owned();
        self.inputs.set(Focus::QueryInput, selected_query);
    }
}
