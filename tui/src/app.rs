use crate::{
    backend::{Backend as AppBackend, Bounds, UIEvent},
    query::{NRQLQuery, NRQL},
    ui::{
        render_dashboard, render_graph, render_load_session, render_loading, render_query_box,
        render_query_list, render_rename_dialog, render_splash,
    },
};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use rand::{thread_rng, Rng};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Layout},
    style::{palette::tailwind::Palette, Color},
    widgets::ListState,
    Frame, Terminal,
};
use std::{
    collections::{btree_map::Entry, BTreeMap},
    env,
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    time::Duration,
};
use tokio::io;

#[derive(Clone, Copy, PartialEq)]
pub enum Focus {
    QueryInput = 0,
    Rename = 1,
    Dashboard = 4,
    SessionLoad = 2,
    Default = 3,
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
    pub has_data: bool,
    pub query_alias: Option<String>,
    pub facets: BTreeMap<String, Vec<(f64, f64)>>,
    pub bounds: Bounds,
    pub selection: String,
}

pub struct Theme {
    pub focus_fg: Color,
    pub chart_fg: Color,
}

pub struct Datasets {
    datasets: BTreeMap<String, Dataset>,
    selected: String,
}

impl Datasets {
    pub fn new() -> Self {
        Datasets {
            datasets: BTreeMap::new(),
            selected: String::new(),
        }
    }

    pub fn entry(&mut self, entry: String) -> Entry<'_, String, Dataset> {
        self.datasets.entry(entry)
    }

    pub fn selected(&self) -> Option<&Dataset> {
        self.datasets.get(&self.selected)
    }

    pub fn remove_entry(&mut self, i: usize) -> String {
        let to_delete = self
            .datasets
            .keys()
            .nth(i)
            .cloned()
            .expect("ERROR: Could not index query for deletion!");

        let (removed, _) = self.datasets.remove_entry(&to_delete).unwrap();
        removed
    }

    pub fn iter(&self) -> std::collections::btree_map::Iter<'_, String, Dataset> {
        self.datasets.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.datasets.is_empty()
    }

    pub fn len(&self) -> usize {
        self.datasets.len()
    }

    pub fn select(&mut self, i: usize) {
        self.selected = self
            .datasets
            .keys()
            .nth(i)
            .expect("ERROR: Could not select query!")
            .to_owned();
    }
}

pub struct App {
    pub session: Option<BTreeMap<String, String>>,
    pub theme: Theme,
    pub inputs: Inputs,
    pub input_mode: InputMode,
    pub focus: Focus,
    pub backend: AppBackend,
    pub list_state: ListState,
    pub datasets: Datasets,
    pub facet_colours: BTreeMap<String, Color>,
}

pub struct Inputs {
    _inputs: [Input; 4],
}

impl Inputs {
    pub fn new() -> Self {
        Inputs {
            _inputs: [
                Input {
                    buffer: String::new(),
                    cursor_position: 0,
                },
                Input {
                    buffer: String::new(),
                    cursor_position: 0,
                },
                Input {
                    buffer: String::new(),
                    cursor_position: 0,
                },
                Input {
                    buffer: String::new(),
                    cursor_position: 0,
                },
            ],
        }
    }
    pub fn get(&self, focus: Focus) -> &str {
        &self._inputs[focus as usize].buffer
    }

    pub fn get_cursor_position(&self, focus: Focus) -> usize {
        self._inputs[focus as usize].cursor_position
    }

    pub fn len(&self, focus: Focus) -> usize {
        self._inputs[focus as usize].buffer.len()
    }

    fn clamp_cursor(&self, focus: Focus, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.len(focus))
    }

    fn move_cursor_left(&mut self, focus: Focus) {
        let cursor_moved_left = self._inputs[focus as usize]
            .cursor_position
            .saturating_sub(1);
        self._inputs[focus as usize].cursor_position = self.clamp_cursor(focus, cursor_moved_left);
    }

    fn move_cursor_right(&mut self, focus: Focus) {
        let cursor_moved_right = self._inputs[focus as usize]
            .cursor_position
            .saturating_add(1);
        self._inputs[focus as usize].cursor_position = self.clamp_cursor(focus, cursor_moved_right);
    }

    fn enter_char(&mut self, focus: Focus, new_char: char) {
        let cursor_position = self.get_cursor_position(focus);
        self._inputs[focus as usize]
            .buffer
            .insert(cursor_position, new_char);

        self.move_cursor_right(focus);
    }

    fn delete_char(&mut self, focus: Focus) {
        let is_not_cursor_leftmost = self.get_cursor_position(focus) != 0;
        if is_not_cursor_leftmost {
            let current_index = self.get_cursor_position(focus);
            let from_left_to_current_index = current_index - 1;

            let before_char_to_delete = self._inputs[focus as usize]
                .buffer
                .chars()
                .take(from_left_to_current_index);
            let after_char_to_delete = self._inputs[focus as usize]
                .buffer
                .chars()
                .skip(current_index);

            self._inputs[focus as usize].buffer =
                before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left(focus);
        }
    }

    pub fn clear(&mut self, focus: Focus) {
        self._inputs[focus as usize].buffer.clear();
    }

    pub fn reset_cursor(&mut self, focus: Focus) {
        self._inputs[focus as usize].cursor_position = 0;
    }
}

impl App {
    pub fn new(
        palette: &Palette,
        backend: AppBackend,
        session: Option<BTreeMap<String, String>>,
    ) -> Self {
        Self {
            inputs: Inputs::new(),
            session,
            theme: Theme {
                focus_fg: palette.c500,
                chart_fg: palette.c900,
            },
            input_mode: InputMode::Normal,
            focus: Focus::Default,
            backend,
            list_state: ListState::default(),
            datasets: Datasets::new(),
            facet_colours: BTreeMap::default(),
        }
    }

    pub fn run<B: Backend>(mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        let mut rng = thread_rng();
        loop {
            terminal.draw(|f| self.ui(f))?;

            // Session Load
            if self.session.is_some() {
                self.focus = Focus::SessionLoad;
                self.input_mode = InputMode::Input;
            }

            // Event handlers
            if let Ok(true) = event::poll(Duration::from_millis(50)) {
                if let Event::Key(key) = event::read()? {
                    match self.input_mode {
                        InputMode::Normal if key.kind == KeyEventKind::Press => match key.code {
                            KeyCode::Char('q') => {
                                self.save_session();
                                return Ok(());
                            }
                            KeyCode::Char('e') => {
                                self.set_focus(Focus::QueryInput);
                                self.set_input_mode(InputMode::Input);
                            }
                            KeyCode::Char('j') => self.next(),
                            KeyCode::Char('k') => self.previous(),
                            KeyCode::Char('x') => self.delete_query(),
                            KeyCode::Char('r') => match self.focus {
                                Focus::QueryInput => {}
                                _ => {
                                    if !self.datasets.is_empty() {
                                        self.set_focus(Focus::Rename);
                                        self.set_input_mode(InputMode::Input);
                                    }
                                }
                            },
                            KeyCode::Char('d') => match self.focus {
                                Focus::Dashboard => self.set_focus(Focus::Default),
                                _ => self.set_focus(Focus::Dashboard),
                            },
                            _ => (),
                        },
                        InputMode::Input if key.kind == KeyEventKind::Press => match key.code {
                            KeyCode::Enter => {
                                match self.focus {
                                    Focus::QueryInput => {
                                        if let Ok(query) =
                                            self.inputs.get(Focus::QueryInput).to_nrql()
                                        {
                                            self.add_query(query);
                                        }
                                    }
                                    Focus::Rename => {
                                        self.rename_query(
                                            self.datasets.selected.to_owned(),
                                            self.inputs.get(Focus::Rename).to_owned(),
                                        );
                                    }
                                    Focus::SessionLoad => {
                                        match self.inputs.get(Focus::SessionLoad) {
                                            // Load session
                                            "y" | "Y" => {
                                                let session =
                                                    self.session.clone().unwrap().into_iter();
                                                for (alias, query) in session {
                                                    // TODO: Avoid
                                                    let clean_query = query.replace("as value", "");
                                                    if let Ok(parsed_query) =
                                                        clean_query.trim().to_nrql()
                                                    {
                                                        self.add_query(parsed_query.clone());
                                                        self.rename_query(
                                                            parsed_query.to_string().unwrap(),
                                                            alias,
                                                        );
                                                        // self.set_focus(Focus::Loading);
                                                    }
                                                }
                                                // };
                                            }
                                            // Don't load session
                                            _ => {}
                                        }
                                        // Clear previous session once loaded
                                        self.session = None;

                                        // Update focus to home
                                        self.set_focus(Focus::Default);
                                    }
                                    _ => {}
                                };
                                self.inputs.clear(self.focus);
                                self.inputs.reset_cursor(self.focus);
                                self.set_focus(Focus::Default);
                                self.set_input_mode(InputMode::Normal);
                            }
                            KeyCode::Char(to_insert) => {
                                self.inputs.enter_char(self.focus, to_insert);
                            }
                            KeyCode::Backspace => {
                                self.inputs.delete_char(self.focus);
                            }
                            KeyCode::Left => {
                                self.inputs.move_cursor_left(self.focus);
                            }
                            KeyCode::Right => {
                                self.inputs.move_cursor_right(self.focus);
                            }
                            KeyCode::Esc => {
                                self.set_focus(Focus::Default);
                                self.set_input_mode(InputMode::Normal);
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
        }
    }

    pub fn ui(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let horizontal = Layout::horizontal([Constraint::Percentage(15), Constraint::Min(20)]);
        let vertical = Layout::vertical([Constraint::Length(3), Constraint::Min(20)]);
        let [input_area, rest] = vertical.areas(area);
        let [list_area, graph_area] = horizontal.areas(rest);

        match self.focus {
            Focus::SessionLoad => {
                render_load_session(self, frame, area);
            }
            Focus::Dashboard => {
                render_dashboard(self, frame, area);
            }
            Focus::Rename => {
                render_query_box(self, frame, input_area);
                render_query_list(self, frame, list_area);
                render_rename_dialog(self, frame, graph_area);
            }
            Focus::Default | Focus::QueryInput => {
                render_query_box(self, frame, input_area);
                render_query_list(self, frame, list_area);
                if let Some(dataset) = self.datasets.selected() {
                    if dataset.has_data {
                        render_graph(self, frame, graph_area);
                    } else {
                        render_loading(self, frame, graph_area);
                    }
                } else {
                    render_splash(self, frame, graph_area);
                }
            }
        }
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

    fn add_query(&self, query: NRQLQuery) {
        self.backend.add_query(query);
    }

    pub fn set_focus(&mut self, focus: Focus) {
        self.focus = focus
    }

    pub fn set_input_mode(&mut self, mode: InputMode) {
        self.input_mode = mode;
    }

    pub fn delete_query(&mut self) {
        let i = self.list_state.selected().unwrap();

        let removed = self.datasets.remove_entry(i);
        // TODO: Fix deleted queries reappearing on new data!
        _ = self.backend.ui_tx.send(UIEvent::DeleteQuery(removed));
    }

    pub fn next(&mut self) {
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

    pub fn previous(&mut self) {
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

    // TODO: Prompt user for session save on exit (q)
    pub fn save_session(&self) {
        // TODO: Dedup code
        let home_dir = match env::var("HOME") {
            Ok(val) => val,
            Err(_) => {
                eprintln!("Unable to determine home directory.");
                panic!()
            }
        };

        let output = self
            .datasets
            .iter()
            .map(|(q, data)| {
                (
                    data.query_alias.clone().unwrap_or(q.to_owned()),
                    q.to_owned(),
                )
            })
            .collect::<BTreeMap<String, String>>();

        let yaml: String =
            serde_yaml::to_string(&output).expect("ERROR: Could not serialize queries!");
        let mut session_path = PathBuf::from(home_dir);
        session_path.push("Library/Application Support/xrelic/session.yaml");
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .truncate(true)
            .create(true)
            .open(session_path)
            .expect("ERROR: Could not open file!");
        file.write_all(yaml.as_bytes())
            .expect("ERROR: Could not write to file!");
    }
}
