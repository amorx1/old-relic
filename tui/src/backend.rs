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

use anyhow::anyhow;
use chrono::{Timelike, Utc};
use crossbeam_channel::{unbounded, Receiver as MReceiver, Sender as MSender};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, ClientBuilder, Method,
};

use serde::{de::DeserializeOwned, Deserialize};

use crate::query::NRQLQuery;

#[derive(Clone, Copy)]
pub struct Bounds {
    pub mins: (f64, f64),
    pub maxes: (f64, f64),
}

pub struct Payload {
    pub query: String,
    pub data: BTreeMap<String, Vec<(f64, f64)>>,
    pub bounds: Bounds,
    pub selection: String,
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
                selection: query.select.to_owned(),
            })?
        }
        sleep(Duration::from_millis(16)).await;
    }
}

static QUERY_BASE: &str = r#"{ "query":  "{ actor { account(id: $account) { nrql(query: \"$query\") { results } } } }" }"#;

#[derive(Clone)]
pub struct NewRelicClient {
    url: Option<String>,
    account: Option<i64>,
    api_key: Option<String>,
    client: Option<Client>,
}

impl NewRelicClient {
    pub fn builder() -> Self {
        NewRelicClient {
            url: None,
            account: None,
            api_key: None,
            client: None,
        }
    }

    pub fn url(&mut self, url: &'static str) -> &mut Self {
        self.url = Some(url.to_owned());
        self
    }

    pub fn account(&mut self, account: &'static i64) -> &mut Self {
        self.account = Some(account.to_owned());
        self
    }

    pub fn api_key(&mut self, key: &'static str) -> &mut Self {
        self.api_key = Some(key.to_owned());
        self
    }

    pub fn http_client(&mut self, client: ClientBuilder) -> &Self {
        let mut headers = HeaderMap::new();
        headers.append(
            "Content-Type",
            HeaderValue::from_str("application/json").unwrap(),
        );
        headers.append(
            "API-Key",
            HeaderValue::from_str(
                self.api_key
                    .as_ref()
                    .expect("ERROR: API Key must be provided first!"),
            )
            .unwrap(),
        );

        self.client = Some(client.default_headers(headers).build().unwrap());

        self
    }

    pub async fn query<T: DeserializeOwned + std::fmt::Debug + Default>(
        &self,
        query_str: impl AsRef<str>,
    ) -> Option<Vec<T>> {
        // dbg!(&query_str);

        let response = self
            .client
            .clone()?
            .request(Method::POST, self.url.clone()?)
            .body(
                QUERY_BASE
                    .replace(
                        "$account",
                        &self
                            .account
                            .expect("ERROR: No account number linked to client!")
                            .to_string(),
                    )
                    .replace("$query", query_str.as_ref()),
            )
            .send()
            .await;

        if let Ok(data) = response {
            let json = data
                .json::<QueryResponse<T>>()
                .await
                .map_err(|e| anyhow!(e))
                .expect("ERROR: Error in response deserialization schema");

            // dbg!(&json);
            return Some(json.data.actor.account.nrql.results);
        }

        None
    }
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResponse<T> {
    pub data: Data<T>,
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Data<T> {
    pub actor: Actor<T>,
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Actor<T> {
    pub account: Account<T>,
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account<T> {
    pub nrql: Nrql<T>,
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Nrql<T> {
    pub results: Vec<T>,
}

#[derive(Default, Debug, Deserialize, PartialEq, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct TimeseriesResult {
    pub begin_time_seconds: f64,
    pub end_time_seconds: f64,
    pub facet: Option<String>,
    pub value: f64,
}

#[derive(Debug)]
pub struct Timeseries {
    pub begin_time_seconds: f64,
    pub end_time_seconds: f64,
    pub facet: Option<String>,
    pub value: f64,
}

impl Timeseries {
    pub fn plot(&self) -> ((f64, f64), (f64, f64)) {
        (
            (self.begin_time_seconds, self.value),
            (self.end_time_seconds, self.value),
        )
    }
}

impl From<TimeseriesResult> for Timeseries {
    fn from(val: TimeseriesResult) -> Timeseries {
        Timeseries {
            begin_time_seconds: val.begin_time_seconds,
            end_time_seconds: val.end_time_seconds,
            facet: val.facet.clone(),
            value: val.value,
        }
    }
}
