use std::{
    collections::{self, btree_map::Entry, BTreeMap, HashSet, VecDeque},
    string::String,
    vec::Vec,
};

use ratatui::{style::Color, widgets::ListState};

use crate::backend::{Bounds, ChartData};

#[derive(Default)]
pub struct Data {
    pub timeseries: Datasets,
    pub logs: Logs,
    pub query_history: VecDeque<String>,
    pub facet_colours: BTreeMap<String, Color>,
}

#[derive(Default)]
pub struct Dataset {
    pub has_data: bool,
    pub query_alias: Option<String>,
    pub facets: BTreeMap<String, Vec<(f64, f64)>>,
    pub bounds: Bounds,
    pub selection: String,
}

#[derive(Default)]
pub struct Datasets {
    pub datasets: BTreeMap<String, Dataset>,
    pub list_state: ListState,
    pub selected: String,
}

#[derive(Default, Clone)]
pub struct Logs {
    pub logs: BTreeMap<String, Vec<String>>,
    pub chart_data: ChartData,
    pub bounds: Bounds,
    pub log_list_state: ListState,
    pub log_item_list_state: ListState,
    pub selected: String,
    pub filters: HashSet<String>,
}

impl Logs {
    pub fn selected(&self) -> Option<&Vec<String>> {
        self.logs.get(&self.selected)
    }

    pub fn iter(&self) -> collections::btree_map::Iter<'_, String, Vec<String>> {
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
            list_state: ListState::default(),
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

    pub fn iter(&self) -> collections::btree_map::Iter<'_, String, Dataset> {
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
