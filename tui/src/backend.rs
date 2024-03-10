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
use server::{webtransaction::*, NewRelicClient};

pub struct Backend {
    pub client: NewRelicClient,
    pub runtime: Runtime,
    pub data_tx: Sender<BTreeMap<String, Vec<(f64, f64)>>>,
    pub data_rx: Receiver<BTreeMap<String, Vec<(f64, f64)>>>,
}

impl Backend {
    pub fn new(client: NewRelicClient) -> Self {
        let (data_tx, data_rx) = channel::<BTreeMap<String, Vec<(f64, f64)>>>();
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

    pub fn start(&self) {
        // Auth thread
        println!("Backend starting ...");
        let tx = self.data_tx.clone();
        let client = self.client.clone();
        self.runtime.spawn(async move {
            _ = refresh(client, tx).await;
        });
    }
}

pub async fn refresh(
    client: NewRelicClient,
    data_tx: Sender<BTreeMap<String, Vec<(f64, f64)>>>,
) -> Result<()> {
    loop {
        if Utc::now().second() % 5 == 0 {
            let data = client
                // .query::<TimeseriesResult>("SELECT sum(newrelic.goldenmetrics.infra.awsapigatewayresourcewithmetrics.requests) AS 'Requests' FROM Metric WHERE entity.guid in ('MjU0MDc5MnxJTkZSQXxOQXw4MDE0OTk0OTg4MDIzNTAxOTQ0', 'MjU0MDc5MnxJTkZSQXxOQXw4Njc2NDEwODc3ODY4NDI2Mzcz', 'MjU0MDc5MnxJTkZSQXxOQXwtODA4OTQxNjQyMzkzODMwODg2NQ', 'MjU0MDc5MnxJTkZSQXxOQXwtMzMzMDQwNzA1ODI3MDQwODE5MA', 'MjU0MDc5MnxJTkZSQXxOQXw0NjY5MzQ3MTY5NTU5NDUyODc2', 'MjU0MDc5MnxJTkZSQXxOQXwxNTA2ODc2MTg0MjI0NTQyNjU3') FACET entity.name LIMIT MAX TIMESERIES SINCE 8 days ago UNTIL now")
                .query::<WebTransactionResult>("SELECT sum(apm.service.overview.web * 1000) FROM Metric WHERE (entity.guid = 'MjU0MDc5MnxBUE18QVBQTElDQVRJT058OTI2ODAyNzcw') FACET `segmentName` LIMIT MAX SINCE 30 minutes ago TIMESERIES UNTIL now")
                .await
                .unwrap_or_default();

            let mut datasets: BTreeMap<String, Vec<(f64, f64)>> = BTreeMap::default();

            for (i, data) in data.into_iter().map(WebTransaction::from).enumerate() {
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

            // let facets = data
            //     .into_iter()
            //     .map(WebTransaction::from)
            //     .map(|t| (t.segment_name, (t.begin_time_seconds, t.value)))
            //     .collect::<BTreeMap<String, Vec<(f64, f64)>>>();

            // facets.into_iter().map(
            //     |(facet, (time, value))| Dataset {
            //         facet,
            //         data
            //     }
            // )
            // .enumerate()
            // .for_each(|(i, (start, end))| {
            //     let idx = i as u16;
            //     let idx = std::convert::Into::<f64>::into(idx);
            //     points.push((idx, start.1));
            //     points.push((idx, end.1));
            // });

            data_tx.send(datasets)?
        }
        sleep(Duration::from_millis(16)).await;
    }
}
