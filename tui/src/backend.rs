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
use server::{
    timeseries::{Timeseries, TimeseriesResult},
    NewRelicClient,
};

use crate::query::NRQLQuery;

// pub struct GraphData {
//     mins: (f64, f64),
//     maxes: (f64, f64)
//     points: Vec<(f64, f64)>,
// }

pub struct Payload {
    pub query: String,
    pub data: BTreeMap<String, Vec<(f64, f64)>>,
}

pub struct Backend {
    pub client: NewRelicClient,
    pub runtime: Runtime,
    pub data_tx: Sender<Payload>,
    pub data_rx: Receiver<Payload>,
}

impl Backend {
    pub fn new(client: NewRelicClient) -> Self {
        let (data_tx, data_rx) = channel::<Payload>();
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
        }
    }

    pub fn add_query(&self, query: NRQLQuery) {
        let tx = self.data_tx.clone();
        let client = self.client.clone();
        self.runtime.spawn(async move {
            _ = refresh_timeseries(query, client, tx).await;
        });
    }
}

pub async fn refresh_timeseries(
    query: NRQLQuery,
    client: NewRelicClient,
    data_tx: Sender<Payload>,
) -> Result<()> {
    loop {
        if Utc::now().second() % 2 == 0 {
            let data = client
                .query::<TimeseriesResult>(query.to_string().unwrap())
                .await
                .unwrap_or_default();

            let mut datasets: BTreeMap<String, Vec<(f64, f64)>> = BTreeMap::default();

            for data in data.into_iter().map(Timeseries::from) {
                if datasets.contains_key(&data.facet) {
                    datasets
                        .get_mut(&data.facet)
                        .unwrap()
                        .push((data.begin_time_seconds, data.value));
                } else {
                    datasets.insert(
                        data.facet,
                        vec![(data.begin_time_seconds, data.end_time_seconds)],
                    );
                }
            }
            data_tx.send(Payload {
                query: query.to_string().unwrap(),
                data: datasets,
            })?
        }
        sleep(Duration::from_millis(50)).await;
    }
}
