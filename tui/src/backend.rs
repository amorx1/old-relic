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
use server::{
    timeseries::{Timeseries, TimeseriesResult},
    NewRelicClient,
};

use crate::query::NRQLQuery;

pub struct Bounds {
    pub mins: (f64, f64),
    pub maxes: (f64, f64),
}

pub struct Payload {
    pub query: String,
    pub data: BTreeMap<String, Vec<(f64, f64)>>,
    pub bounds: Bounds,
}

pub struct Backend {
    pub client: NewRelicClient,
    pub runtime: Runtime,
    pub data_tx: Sender<Payload>,
    pub data_rx: Receiver<Payload>,
    pub ui_tx: MSender<String>,
    pub ui_rx: MReceiver<String>,
}

impl Backend {
    pub fn new(client: NewRelicClient) -> Self {
        let (data_tx, data_rx) = channel::<Payload>();
        let (ui_tx, ui_rx) = unbounded();
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

    pub fn add_query(&self, query: NRQLQuery) {
        let tx = self.data_tx.clone();
        let rx = self.ui_rx.clone();
        let client = self.client.clone();
        self.runtime.spawn(async move {
            _ = refresh_timeseries(query, client, tx, rx).await;
        });
    }
}

pub async fn refresh_timeseries(
    query: NRQLQuery,
    client: NewRelicClient,
    data_tx: Sender<Payload>,
    ui_rx: MReceiver<String>,
) -> Result<()> {
    loop {
        while let Some(q) = ui_rx.try_iter().next() {
            if query.to_string().unwrap() == q {
                return Ok(());
            }
        }
        if Utc::now().second() % 5 == 0 {
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

            for data in data.into_iter().map(Timeseries::from) {
                let facet = &data.facet.unwrap_or(String::from("value"));
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

            data_tx.send(Payload {
                query: query.to_string().unwrap(),
                data: facets,
                bounds: Bounds {
                    mins: min_bounds,
                    maxes: max_bounds,
                },
            })?
        }
        sleep(Duration::from_millis(16)).await;
    }
}
