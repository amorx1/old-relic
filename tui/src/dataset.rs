use std::collections::{btree_map::Entry, BTreeMap};

use ratatui::{text::Line, widgets::ListState};

use crate::backend::Bounds;

pub struct Dataset {
    pub has_data: bool,
    pub query_alias: Option<String>,
    pub facets: BTreeMap<String, Vec<(f64, f64)>>,
    pub bounds: Bounds,
    pub selection: String,
}

pub struct Datasets {
    pub datasets: BTreeMap<String, Dataset>,
    pub selected: String,
}

#[derive(Clone, Default)]
pub enum LogState {
    #[default]
    None,
    Loading,
    Show,
}

#[derive(Default, Clone)]
pub struct Logs<'a> {
    pub state: LogState,
    pub logs: BTreeMap<String, Vec<Line<'a>>>,
    pub log_item_list_state: ListState,
    pub selected: String,
}

impl Logs<'_> {
    pub fn selected(&self) -> Option<&Vec<Line<'_>>> {
        self.logs.get(&self.selected)
    }

    pub fn iter(&self) -> std::collections::btree_map::Iter<'_, String, Vec<Line<'_>>> {
        self.logs.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.logs.is_empty()
    }

    pub fn len(&self) -> usize {
        self.logs.len()
    }

    pub fn select(&mut self, i: usize) {
        self.selected = self
            .logs
            .keys()
            .nth(i)
            .expect("ERROR: Could not select query!")
            .to_owned();
    }
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
