use anyhow::{anyhow, Result};
use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Arc,
};
use tokio::runtime::{self, Runtime};

use chrono::{Timelike, Utc};
use server::{
    timeseries::{Timeseries, TimeseriesResult},
    NewRelicClient,
};

pub struct Backend {
    pub client: NewRelicClient,
    pub runtime: Runtime,
    pub data_tx: Sender<Vec<(f64, f64)>>,
    pub data_rx: Receiver<Vec<(f64, f64)>>,
}

impl Backend {
    pub fn new(client: NewRelicClient) -> Self {
        let (data_tx, data_rx) = channel::<Vec<(f64, f64)>>();
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

pub async fn refresh(client: NewRelicClient, data_tx: Sender<Vec<(f64, f64)>>) -> Result<()> {
    loop {
        if Utc::now().second() % 5 == 0 {
            let data = client
                .query::<TimeseriesResult>("SELECT sum(newrelic.goldenmetrics.infra.awsapigatewayresourcewithmetrics.requests) AS 'Requests' FROM Metric WHERE entity.guid in ('MjU0MDc5MnxJTkZSQXxOQXw4MDE0OTk0OTg4MDIzNTAxOTQ0', 'MjU0MDc5MnxJTkZSQXxOQXw4Njc2NDEwODc3ODY4NDI2Mzcz', 'MjU0MDc5MnxJTkZSQXxOQXwtODA4OTQxNjQyMzkzODMwODg2NQ', 'MjU0MDc5MnxJTkZSQXxOQXwtMzMzMDQwNzA1ODI3MDQwODE5MA', 'MjU0MDc5MnxJTkZSQXxOQXw0NjY5MzQ3MTY5NTU5NDUyODc2', 'MjU0MDc5MnxJTkZSQXxOQXwxNTA2ODc2MTg0MjI0NTQyNjU3') FACET entity.name LIMIT MAX TIMESERIES SINCE 8 days ago UNTIL now")
                .await
                .unwrap_or_default();

            let mut points: Vec<_> = vec![];

            data.into_iter()
                .map(Timeseries::from)
                .map(|t| t.plot())
                .enumerate()
                .for_each(|(i, (start, end))| {
                    let idx = i as u16;
                    let idx = std::convert::Into::<f64>::into(idx);
                    points.push((idx, start.1));
                    points.push((idx, end.1));
                });

            data_tx.send(points)?
        }
    }
}
