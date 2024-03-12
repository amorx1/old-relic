use anyhow::Result;
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fmt::{self, Debug},
    sync::mpsc::{channel, Receiver, Sender},
    time::Duration,
};
use tokio::{
    runtime::{self, Runtime},
    time::sleep,
};

use chrono::{Timelike, Utc};
use server::{query::Timeseries, webtransaction::*, NewRelicClient};

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

    // pub fn start(&self) {
    //     println!("Backend starting ...");
    //     let tx = self.data_tx.clone();
    //     let client = self.client.clone();
    //     self.runtime.spawn(async move {
    //         _ = refresh(client, tx).await;
    //     });
    // }

    pub fn add_timeseries(&self, query: &dyn Timeseries) {
        let tx = self.data_tx.clone();
        let client = self.client.clone();
        let query = query.timeseries().unwrap();
        self.runtime.spawn(async move {
            _ = refresh(query, client, tx).await;
        });
    }
}
// pub async fn refresh_timeseries<T>(
//     query: &dyn Timeseries,
//     client: NewRelicClient,
//     data_tx: Sender<T>,
// ) where
//     T: Debug + Default + for<'de> Deserialize<'de>,
// {
//     loop {
//         if Utc::now().second() % 2 == 0 {
//             // to avoid CPU burn
//             let q = query.timeseries().unwrap();
//             let data = client.query::<T>(&q).await.unwrap_or_default();
//             let mut datasets: BTreeMap<String, Vec<(f64, f64)>> = BTreeMap::default();
//             for data in data.into_iter().map(T::from) {
//                 if datasets.contains_key(&data.segment_name) {
//                     datasets
//                         .get_mut(&data.segment_name)
//                         .unwrap()
//                         .push((data.begin_time_seconds, data.value));
//                 } else {
//                     datasets.insert(
//                         data.segment_name,
//                         vec![(data.begin_time_seconds, data.end_time_seconds)],
//                     );
//                 }
//             }
//             data_tx.send(Payload {
//                 query: query.into(),
//                 data: datasets,
//             })?

//             sleep(Duration::from_millis(16)).await;
//         }
//     }
// }

pub async fn refresh(
    query: String,
    client: NewRelicClient,
    data_tx: Sender<Payload>,
) -> Result<()> {
    loop {
        if Utc::now().second() % 2 == 0 {
            let data = client
                // .query::<TimeseriesResult>("SELECT sum(newrelic.goldenmetrics.infra.awsapigatewayresourcewithmetrics.requests) AS 'Requests' FROM Metric WHERE entity.guid in ('MjU0MDc5MnxJTkZSQXxOQXw4MDE0OTk0OTg4MDIzNTAxOTQ0', 'MjU0MDc5MnxJTkZSQXxOQXw4Njc2NDEwODc3ODY4NDI2Mzcz', 'MjU0MDc5MnxJTkZSQXxOQXwtODA4OTQxNjQyMzkzODMwODg2NQ', 'MjU0MDc5MnxJTkZSQXxOQXwtMzMzMDQwNzA1ODI3MDQwODE5MA', 'MjU0MDc5MnxJTkZSQXxOQXw0NjY5MzQ3MTY5NTU5NDUyODc2', 'MjU0MDc5MnxJTkZSQXxOQXwxNTA2ODc2MTg0MjI0NTQyNjU3') FACET entity.name LIMIT MAX TIMESERIES SINCE 8 days ago UNTIL now")
                .query::<WebTransactionResult>(&query)
                .await
                .unwrap_or_default();

            let mut datasets: BTreeMap<String, Vec<(f64, f64)>> = BTreeMap::default();

            for data in data.into_iter().map(WebTransaction::from) {
                if datasets.contains_key(&data.segment_name) {
                    datasets
                        .get_mut(&data.segment_name)
                        .unwrap()
                        .push((data.begin_time_seconds, data.value));
                } else {
                    datasets.insert(
                        data.segment_name,
                        vec![(data.begin_time_seconds, data.end_time_seconds)],
                    );
                }
            }
            data_tx.send(Payload {
                query: query.clone(),
                data: datasets,
            })?
        }
        sleep(Duration::from_millis(16)).await;
    }
}
