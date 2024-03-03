/* Flow
    - Search for an application & select one OK
    - Store that application OK
        - appName OK
        - entityGuid OK
    - Select some time period OK
    - Get Traces for selected application within the specified time period OK
    - Get Trace data for found traces ...
*/

use anyhow::anyhow;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, ClientBuilder, Method,
};

pub mod application;
pub mod newrelic;
pub mod query;
pub mod trace;
use serde::de::DeserializeOwned;

use newrelic::QueryResponse;
use trace::Trace;

pub struct NRClient<'a> {
    url: Option<&'a str>,
    account: Option<&'a i64>,
    api_key: Option<&'a str>,
    client: Option<Client>,
}

impl<'a> NRClient<'a> {
    pub fn builder() -> Self {
        NRClient {
            url: None,
            account: None,
            api_key: None,
            client: None,
        }
    }

    pub fn url(&mut self, url: &'static str) -> &mut Self {
        self.url = Some(url);
        self
    }

    pub fn account(&mut self, account: &'static i64) -> &mut Self {
        self.account = Some(account);
        self
    }

    pub fn api_key(&mut self, key: &'static str) -> &mut Self {
        self.api_key = Some(key);
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

    pub async fn trace_metrics(traces: Vec<Trace>) {}

    pub async fn query<T: DeserializeOwned + std::fmt::Debug + Default>(
        &self,
        query_str: String,
    ) -> Option<Vec<T>> {
        // dbg!(&query_str);

        let response = self
            .client
            .clone()?
            .request(Method::POST, self.url?)
            .body(
                query_str.replace(
                    "$account",
                    &self
                        .account
                        .expect("ERROR: No account number linked to client!")
                        .to_string(),
                ),
            )
            .send()
            .await;

        if let Ok(data) = response {
            let json = data
                .json::<QueryResponse<T>>()
                .await
                .map_err(|e| anyhow!(e))
                .unwrap_or_default();

            // dbg!(&json);
            return Some(json.data.actor.account.nrql.results);
        }

        None
    }
}
