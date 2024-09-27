use crate::{
    backend::{Bounds, PayloadType, UIEvent},
    dataset::{Dataset, Datasets, Logs},
    input::Inputs,
    query::NRQL,
    ui::{style_detail_line, ui},
    Config,
};

use chrono::{Timelike, Utc};
use crossbeam_channel::{Receiver as CrossBeamReceiver, Sender as CrossBeamSender};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use rand::{thread_rng, Rng};
use ratatui::{
    backend::Backend,
    style::Color,
    text::Line,
    widgets::{self, GraphType, ListState},
    Terminal,
};
use std::{
    collections::{
        btree_map::{Entry, OccupiedEntry},
        BTreeMap, HashSet, VecDeque,
    },
    fs::{self, OpenOptions},
    io::Write,
    sync::mpsc::Receiver,
    time::Duration,
};
use tokio::io;

pub struct UIFocus {
    pub tab: Tab,
    pub panel: Focus,
    pub input_mode: InputMode,
    pub loading: bool,
}

impl Default for UIFocus {
    fn default() -> Self {
        UIFocus {
            // tab: Tab::Graph,
            tab: Tab::Logs,
            panel: Focus::Default,
            input_mode: InputMode::Normal,
            loading: false,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
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

#[derive(Clone, Copy, PartialEq)]
pub enum InputMode {
    Normal,
    Input,
}

pub struct Theme {
    pub focus_fg: Color,
    pub chart_fg: Color,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Tab {
    Graph = 0,
    Logs = 1,
}

pub struct App {
    pub config: Box<Config>,
    pub inputs: Inputs,
    pub focus: UIFocus,
    pub tabs: Vec<String>,
    pub data_rx: Receiver<PayloadType>,
    pub ui_tx: CrossBeamSender<UIEvent>,
    pub list_state: ListState,
    pub datasets: Datasets,
    pub query_history: VecDeque<String>,
    pub logs: Logs,
    pub facet_colours: BTreeMap<String, Color>,
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
            focus: UIFocus::default(),
            list_state: ListState::default(),
            datasets: Datasets::new(),
            logs: Logs::default(),
            facet_colours: BTreeMap::default(),
            tabs: vec!["Logs".into()],
            query_history: VecDeque::default(),
        }
    }

    pub fn run<B: Backend>(mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        let mut rng = thread_rng();
        loop {
            terminal.draw(|f| ui(&mut self, f))?;

            // Session Load
            if !self.config.session.is_loaded {
                self.set_focus(UIFocus {
                    panel: Focus::SessionLoad,
                    input_mode: InputMode::Input,
                    ..self.focus
                });
            }

            // Event handlers
            if let Ok(true) = event::poll(Duration::from_millis(50)) {
                if let Event::Key(key) = event::read()? {
                    match self.focus.input_mode {
                        // Normal Mode
                        InputMode::Normal if key.kind == KeyEventKind::Press => match key.code {
                            KeyCode::Char('q') => {
                                self.set_focus(UIFocus {
                                    panel: Focus::SessionSave,
                                    input_mode: InputMode::Input,
                                    ..self.focus
                                });
                            }
                            KeyCode::Char('F') => self.set_focus(UIFocus {
                                panel: Focus::Search,
                                input_mode: InputMode::Input,
                                ..self.focus
                            }),
                            KeyCode::Char('e') => {
                                self.set_focus(UIFocus {
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
                                    if !self.datasets.is_empty() {
                                        self.set_focus(UIFocus {
                                            panel: Focus::Rename,
                                            input_mode: InputMode::Input,
                                            ..self.focus
                                        });
                                    }
                                }
                            },
                            KeyCode::Char('d') => match self.focus.panel {
                                Focus::Dashboard => self.set_focus(UIFocus {
                                    panel: Focus::Default,
                                    ..self.focus
                                }),
                                _ => self.set_focus(UIFocus {
                                    panel: Focus::Dashboard,
                                    ..self.focus
                                }),
                            },
                            KeyCode::Char('T') => self.next_tab(),
                            KeyCode::Char('C') => self.clear_filters(),
                            KeyCode::Esc => self.set_focus(UIFocus {
                                panel: Focus::Default,
                                ..self.focus
                            }),
                            KeyCode::Enter | KeyCode::Char(' ') => match self.focus.panel {
                                Focus::Log => self.set_focus(UIFocus {
                                    panel: Focus::LogDetail,
                                    ..self.focus
                                }),
                                Focus::LogDetail => {
                                    let key_idx = self.logs.log_item_list_state.selected().unwrap();
                                    let log = &self.logs.selected().unwrap()[key_idx].to_string();
                                    let correlation_id = log
                                        .split(' ')
                                        .last()
                                        .unwrap()
                                        .trim_matches(|p| char::is_ascii_punctuation(&p));
                                    let query = format!("SELECT * FROM Log WHERE allColumnSearch('{}', insensitive: true)", correlation_id);

                                    self.add_query(query);
                                    self.set_focus(UIFocus {
                                        panel: Focus::Default,
                                        ..self.focus
                                    });
                                }
                                Focus::Default => self.set_focus(UIFocus {
                                    panel: Focus::Log,
                                    ..self.focus
                                }),
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
                                        self.set_focus(UIFocus {
                                            loading: true,
                                            ..self.focus
                                        });
                                    }
                                    Focus::Rename => {
                                        self.rename_query(
                                            self.datasets.selected.to_owned(),
                                            self.inputs.get(Focus::Rename).to_owned(),
                                        );
                                    }
                                    Focus::Search => {
                                        let filter = self.inputs.get(Focus::Search);
                                        self.add_filter(filter.into());
                                        self.set_focus(UIFocus {
                                            panel: Focus::Default,
                                            ..self.focus
                                        });
                                    }
                                    Focus::SessionLoad => {
                                        match self.inputs.get(Focus::SessionLoad) {
                                            // Load session
                                            "y" | "Y" => {
                                                self.load_session();
                                            }
                                            // Don't load session
                                            _ => {
                                                self.config.session.is_loaded = true;
                                            }
                                        }
                                        // Update focus to default
                                        self.set_focus(UIFocus {
                                            panel: Focus::Default,
                                            ..self.focus
                                        });
                                    }
                                    Focus::SessionSave => {
                                        match self.inputs.get(Focus::SessionSave) {
                                            // Save session
                                            "y" | "Y" => {
                                                self.save_session();
                                            }
                                            _ => {}
                                        }
                                        return Ok(());
                                    }
                                    _ => {}
                                };
                                self.inputs.clear(self.focus.panel);
                                self.inputs.reset_cursor(self.focus.panel);
                                self.set_focus(UIFocus {
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
                                let query = self.query_history.pop_front().unwrap_or_default();
                                self.inputs.set(Focus::QueryInput, query.clone());
                                self.inputs.move_cursor_end(Focus::QueryInput);
                                self.query_history.push_back(query);
                            }
                            KeyCode::Down => {
                                let query = self.query_history.pop_back().unwrap_or_default();
                                self.inputs.set(Focus::QueryInput, query.clone());
                                self.inputs.move_cursor_end(Focus::QueryInput);
                                self.query_history.push_front(query);
                            }
                            KeyCode::Esc => match self.focus.panel {
                                Focus::SessionLoad => {}
                                _ => {
                                    self.set_focus(UIFocus {
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
                }
            }

            while let Some(payload) = self.data_rx.try_iter().next() {
                match payload {
                    PayloadType::None => self.set_focus(UIFocus {
                        panel: Focus::NoResult,
                        loading: false,
                        ..self.focus
                    }),
                    PayloadType::Timeseries(payload) => {
                        if let Entry::Vacant(e) = self.datasets.entry(payload.query.clone()) {
                            e.insert(Dataset {
                                query_alias: None,
                                facets: payload.data,
                                bounds: payload.bounds,
                                selection: payload.selection,
                                has_data: true,
                            });
                        } else {
                            _ = self
                                .datasets
                                .entry(payload.query.to_owned())
                                .and_modify(|data| {
                                    data.facets = payload.data;
                                    data.bounds = payload.bounds;
                                    data.has_data = true
                                })
                        }

                        for facet_key in payload.facets {
                            // Only add facet key if not present
                            if let Entry::Vacant(e) = self.facet_colours.entry(facet_key) {
                                e.insert(Color::Rgb(
                                    rng.gen::<u8>(),
                                    rng.gen::<u8>(),
                                    rng.gen::<u8>(),
                                ));
                            }
                        }
                    }
                    PayloadType::Log(payload) => {
                        let mut logs: BTreeMap<String, Vec<String>> = BTreeMap::new();
                        for (timestamp, log) in payload.logs {
                            logs.insert(timestamp, log.split('\n').map(|v| v.into()).collect());
                        }

                        if !logs.is_empty() {
                            self.logs = Logs {
                                selected: logs.first_entry().unwrap().key().into(),
                                logs,
                                log_item_list_state: ListState::default(),
                                chart_data: payload.chart_data,
                                bounds: payload.bounds,
                                filters: HashSet::default(),
                                log_list_state: ListState::default(),
                            };
                        }

                        self.set_focus(UIFocus {
                            loading: false,
                            ..self.focus
                        });
                    }
                }
            }
        }
    }

    // TODO
    fn add_filter(&mut self, filter: String) {
        self.logs.filters.insert(filter.clone());
        self.logs.logs.retain(|_key, value| {
            for line in value {
                if line.contains(&filter) {
                    return true;
                }
            }
            false
        })
    }

    fn rename_query(&mut self, query: String, alias: String) {
        if let Entry::Vacant(e) = self.datasets.entry(query.to_owned()) {
            e.insert(Dataset {
                has_data: false,
                query_alias: Some(alias),
                facets: BTreeMap::default(),
                bounds: Bounds::default(),
                selection: String::new(),
            });
        } else {
            _ = self.datasets.entry(query.to_owned()).and_modify(|data| {
                data.query_alias = Some(alias);
            })
        }
    }

    fn add_query(&mut self, query: String) {
        self.query_history.push_back(query.clone());
        _ = self.ui_tx.send(UIEvent::AddQuery(query));
    }

    pub fn set_focus(&mut self, focus: UIFocus) {
        self.focus = focus;
    }

    pub fn delete_query(&mut self) {
        let i = self.list_state.selected().unwrap();

        let removed = self.datasets.remove_entry(i);
        // TODO: Fix deleted queries reappearing on new data!
        _ = self.ui_tx.send(UIEvent::DeleteQuery(removed));
    }

    pub fn next(&mut self) {
        match self.focus.tab {
            Tab::Graph => {
                if self.datasets.is_empty() {
                    return;
                }

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
                self.datasets.select(i);
            }
            Tab::Logs => match self.focus.panel {
                Focus::Log => {
                    if self.logs.logs.is_empty() {
                        return;
                    }

                    let i = match self.logs.log_item_list_state.selected() {
                        Some(i) => {
                            if i >= self.logs.selected().unwrap().len() - 1 {
                                0
                            } else {
                                i + 1
                            }
                        }
                        None => 0,
                    };

                    self.logs.log_item_list_state.select(Some(i));
                    // self.logs.select(i);
                }
                _ => {
                    if self.logs.is_empty() {
                        return;
                    }

                    let i = match self.logs.log_list_state.selected() {
                        Some(i) => {
                            if i >= self.logs.len() - 1 {
                                0
                            } else {
                                i + 1
                            }
                        }
                        None => 0,
                    };

                    self.logs.log_list_state.select(Some(i));
                    self.logs.select(i);
                }
            },
        }
    }

    pub fn previous(&mut self) {
        match self.focus.tab {
            Tab::Graph => {
                if self.datasets.is_empty() {
                    return;
                }

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
                self.datasets.select(i);
            }
            Tab::Logs => match self.focus.panel {
                Focus::Log => {
                    if self.logs.logs.is_empty() {
                        return;
                    }

                    let i = match self.logs.log_item_list_state.selected() {
                        Some(i) => {
                            if i == 0 {
                                self.logs.selected().unwrap().len() - 1
                            } else {
                                i - 1
                            }
                        }
                        None => 0,
                    };
                    self.logs.log_item_list_state.select(Some(i));
                    // self.logs.select(i);
                }
                _ => {
                    if self.logs.is_empty() {
                        return;
                    }

                    let i = match self.logs.log_list_state.selected() {
                        Some(i) => {
                            if i == 0 {
                                self.logs.len() - 1
                            } else {
                                i - 1
                            }
                        }
                        None => 0,
                    };
                    self.logs.log_list_state.select(Some(i));
                    self.logs.select(i);
                }
            },
        }
    }

    pub fn load_session(&mut self) {
        let session_path = self.config.session.session_path.clone();
        let yaml = fs::read_to_string(session_path).expect("ERROR: Could not read session file!");
        let session_queries: Vec<String> =
            serde_yaml::from_str(&yaml).expect("ERROR: Could not deserialize session file!");

        self.query_history = VecDeque::from(session_queries);
        self.config.session.is_loaded = true;
    }
    // pub fn load_session(&mut self) {
    //     let session_path = self.config.session.session_path.clone();
    //     let yaml = fs::read_to_string(session_path).expect("ERROR: Could not read session file!");
    //     let session_queries: Option<BTreeMap<String, String>> =
    //         serde_yaml::from_str(&yaml).expect("ERROR: Could not deserialize session file!");

    //     dbg!(&session_queries);

    //     if let Some(queries) = session_queries {
    //         let iter = queries.into_iter();
    //         for (alias, query) in iter {
    //             // TODO: Avoid this
    //             let clean_query = query.replace("as value", "");
    //             if let Ok(parsed_query) = clean_query.trim().to_nrql() {
    //                 // TODO: Handle Log session
    //                 self.add_query(query);
    //                 self.rename_query(parsed_query.to_string().unwrap(), alias);
    //             }
    //         }
    //     }

    //     self.config.session.is_loaded = true;
    // }

    pub fn save_session(&self) {
        let mut out = String::new();

        let timeseries_queries = self
            .datasets
            .iter()
            .map(|(q, data)| {
                (
                    data.query_alias.clone().unwrap_or(q.to_owned()),
                    q.to_owned(),
                )
            })
            .collect::<BTreeMap<String, String>>();

        if !timeseries_queries.is_empty() {
            let yaml: String = serde_yaml::to_string(&timeseries_queries)
                .expect("ERROR: Could not serialize queries!");

            out += &yaml;
        }

        let log_queries = serde_yaml::to_string(&self.query_history)
            .expect("ERROR: Could not serialize query history!");

        out += &log_queries;

        let session_path = self.config.session.session_path.clone();
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .truncate(true)
            .create(true)
            .open(session_path)
            .expect("ERROR: Could not open session file!");
        file.write_all(out.as_bytes())
            .expect("ERROR: Could not write to session file!");
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
            // Tab::Logs => self.focus.tab = Tab::Graph,
            Tab::Logs => self.focus.tab = Tab::Logs,
        }
    }

    fn clear_filters(&mut self) {
        self.logs.filters.clear();
        self.add_query(
            self.query_history
                .back()
                .unwrap_or(&String::default())
                .to_owned(),
        );
    }
}
