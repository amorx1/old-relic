use crate::query::{QueryType, Timeseries, TimeseriesResult};
use anyhow::Result;

use std::{
    collections::BTreeMap,
    sync::mpsc::{channel, Receiver, Sender},
    time::Duration,
};
use tokio::{
    runtime::{self, Runtime},
    time::sleep,
};

use chrono::{Timelike, Utc};
use crossbeam_channel::{unbounded, Receiver as MReceiver, Sender as MSender};

use crate::client::NewRelicClient;
use crate::query::NRQLQuery;

#[derive(Clone, Copy)]
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

pub enum PayloadType {
    Timeseries(Payload),
    Log(LogPayload),
}

pub struct LogPayload {
    pub logs: BTreeMap<String, String>,
}

pub struct Payload {
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

    pub fn add_query(&self, query: QueryType) {
        let tx = self.data_tx.clone();
        let rx = self.ui_rx.clone();
        let client = self.client.clone();
        self.runtime.spawn(async move {
            match query {
                QueryType::Timeseries(query) => _ = refresh_timeseries(query, client, tx, rx).await,
                QueryType::Log(query) => _ = query_log(query, client, tx, rx).await,
            }
        });
    }
}

pub async fn query_log(
    query: String,
    client: NewRelicClient,
    data_tx: Sender<PayloadType>,
    _ui_rx: MReceiver<UIEvent>,
) -> Result<()> {
    let data: Vec<serde_json::Value> = client
        .query::<serde_json::Value>(query)
        .await
        .unwrap_or_default();

    let mut logs: BTreeMap<String, String> = BTreeMap::new();
    for log in data {
        logs.insert(
            log.get("timestamp")
                .expect("ERROR: Log had no timestamp")
                .to_string(),
            serde_json::to_string_pretty(&log).unwrap(),
        );
    }

    data_tx.send(PayloadType::Log(LogPayload { logs }))?;
    Ok(())
}

pub async fn refresh_timeseries(
    query: NRQLQuery,
    client: NewRelicClient,
    data_tx: Sender<PayloadType>,
    ui_rx: MReceiver<UIEvent>,
) -> Result<()> {
    loop {
        while let Some(event) = ui_rx.try_iter().next() {
            match event {
                UIEvent::DeleteQuery(q) => {
                    if query.to_string().unwrap() == q {
                        return Ok(());
                    }
                }
            }
        }

        if Utc::now().second() % 10 == 0 {
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

            data_tx.send(PayloadType::Timeseries(Payload {
                query: query.to_string().unwrap(),
                facets: facet_keys,
                data: facets,
                bounds: Bounds {
                    mins: min_bounds,
                    maxes: max_bounds,
                },
                selection: query.select.to_owned(),
            }))?
        }
        sleep(Duration::from_millis(16)).await;
    }
}
