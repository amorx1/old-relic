use crate::query::{Timeseries, TimeseriesResult};
use anyhow::{Error, Result};

use std::{
    collections::BTreeMap,
    sync::mpsc::{channel, Receiver, Sender},
};
use tokio::runtime::{self, Runtime};

use crossbeam_channel::{unbounded, Receiver as MReceiver, Sender as MSender};

use crate::client::NewRelicClient;
use crate::query::NRQLQuery;

#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub mins: (f64, f64),
    pub maxes: (f64, f64),
}

impl Default for Bounds {
    fn default() -> Self {
        Bounds {
            mins: (0 as f64, 0 as f64),
            maxes: (0 as f64, 0 as f64),
        }
    }
}

#[derive(Debug)]
pub enum PayloadType {
    Timeseries(TimseriesPayload),
    Log(LogPayload),
    None, // No data
}

#[derive(Debug)]
pub struct LogPayload {
    pub logs: BTreeMap<String, String>,
    pub chart_data: ChartData,
    pub bounds: Bounds,
}

#[derive(Debug)]
pub struct Bins {}

#[derive(Default, Debug)]
pub struct TimseriesPayload {
    pub query: String,
    pub facets: Vec<String>,
    pub data: BTreeMap<String, Vec<(f64, f64)>>,
    pub bounds: Bounds,
    pub selection: String,
}

pub struct Backend {
    pub client: NewRelicClient,
    pub runtime: Runtime,
    pub data_tx: Sender<PayloadType>,
    pub data_rx: Receiver<PayloadType>,
    pub ui_tx: MSender<UIEvent>,
    pub ui_rx: MReceiver<UIEvent>,
}

pub enum UIEvent {
    AddQuery(String),
    DeleteQuery(String),
}

impl Backend {
    pub fn new(client: NewRelicClient) -> Self {
        let (data_tx, data_rx) = channel::<PayloadType>();
        let (ui_tx, ui_rx) = unbounded::<UIEvent>();
        let runtime = runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .thread_name("data")
            .enable_all()
            .build()
            .unwrap();

        Self {
            client,
            runtime,
            data_tx,
            data_rx,
            ui_tx,
            ui_rx,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChartData {
    pub info: Vec<(f64, f64)>,
    pub error: Vec<(f64, f64)>,
    pub debug: Vec<(f64, f64)>,
}

impl ChartData {
    pub fn new() -> Self {
        ChartData {
            info: vec![],
            error: vec![],
            debug: vec![],
        }
    }
}

impl Default for ChartData {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn query_log(query: String, client: NewRelicClient) -> Result<LogPayload, Error> {
    let data: Vec<serde_json::Value> = client
        .query::<serde_json::Value>(query)
        .await
        .unwrap_or_default();

    let mut logs: BTreeMap<String, String> = BTreeMap::new();
    let mut chart_data = ChartData::default();
    let mut min_bounds: (f64, f64) = (f64::MAX, f64::MAX);
    let mut max_bounds: (f64, f64) = (0 as f64, 0 as f64);

    for log in data {
        let timestamp = log
            .get("timestamp")
            .expect("ERROR: Log had no timestamp")
            .to_string();

        let level = if let Some(val) = log.get("level") {
            val.to_string()
        } else if let Some(val) = log.get("severity.text") {
            val.to_string()
        } else {
            "Information".into()
        };

        match level.trim_matches('\"') {
            "Information" => chart_data.info.push((timestamp.parse::<f64>()?, 1_f64)),
            "Error" => chart_data.error.push((timestamp.parse::<f64>()?, 1_f64)),
            "Debug" => chart_data.debug.push((timestamp.parse::<f64>()?, 1_f64)),
            // TODO: How to handle none
            _ => {}
        }

        min_bounds.0 = f64::min(min_bounds.0, timestamp.parse::<f64>().unwrap());
        min_bounds.1 = f64::min(min_bounds.1, 1.0);

        max_bounds.0 = f64::max(max_bounds.0, timestamp.parse::<f64>().unwrap());
        max_bounds.1 = f64::max(max_bounds.1, 1.0);

        logs.insert(timestamp, serde_json::to_string_pretty(&log).unwrap());
    }

    Ok(LogPayload {
        logs,
        chart_data,
        bounds: Bounds {
            mins: min_bounds,
            maxes: max_bounds,
        },
    })
}

pub async fn query_timeseries(
    query: NRQLQuery,
    client: NewRelicClient,
) -> Result<TimseriesPayload, Error> {
    let data = client
        .query::<TimeseriesResult>(query.to_string().unwrap())
        .await
        .unwrap_or_default();

    let mut min_bounds: (f64, f64) = (f64::MAX, f64::MAX);
    let mut max_bounds: (f64, f64) = (0 as f64, 0 as f64);

    for point in &data {
        min_bounds.0 = f64::min(min_bounds.0, point.end_time_seconds);
        min_bounds.1 = f64::min(min_bounds.1, point.value);

        max_bounds.0 = f64::max(max_bounds.0, point.end_time_seconds);
        max_bounds.1 = f64::max(max_bounds.1, point.value);
    }

    let mut facets: BTreeMap<String, Vec<(f64, f64)>> = BTreeMap::default();
    let mut facet_keys: Vec<String> = vec![];

    for data in data.into_iter().map(Timeseries::from) {
        let facet = &data.facet.unwrap_or(String::from("value"));
        facet_keys.push(facet.to_owned());
        if facets.contains_key(facet) {
            facets
                .get_mut(facet)
                .unwrap()
                .extend_from_slice(&[(data.end_time_seconds, data.value)]);
        } else {
            facets.insert(
                facet.to_owned(),
                vec![(data.begin_time_seconds, data.value)],
            );
        }
    }

    Ok(TimseriesPayload {
        query: query.to_string().unwrap(),
        facets: facet_keys,
        data: facets,
        bounds: Bounds {
            mins: min_bounds,
            maxes: max_bounds,
        },
        selection: query.select.to_owned(),
    })
}
