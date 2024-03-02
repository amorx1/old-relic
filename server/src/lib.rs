/* Flow
    - Search for an application & select one
    - Store that application
        - appName
        - entityGuid
    - Select some time period
    - Get Traces for selected application within the specified time period
    - Get Trace data for found traces
*/

use anyhow::anyhow;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, ClientBuilder, Method,
};

pub mod application;
use application::{ApplicationResult, ApplicationSearchResult, QUERY as APP_QUERY};
use serde::Deserialize;

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
                    .expect("ERROR: API Key must be provided first!")
                    .as_ref(),
            )
            .unwrap(),
        );

        self.client = Some(client.default_headers(headers).build().unwrap());

        self
    }

    // pub async fn get_traces(&self, guid: &str) {
    //     Some(
    //         self.
    //     )
    // }

    pub async fn search_application(&self, name: &str) -> Option<Vec<ApplicationResult>> {
        Some(
            self.query::<ApplicationSearchResult>(APP_QUERY.replace("$name", name))
                .await?
                .data
                .actor
                .account
                .nrql
                .results,
        )
    }

    pub async fn query<T: for<'de> Deserialize<'de>>(&self, query_str: String) -> Option<T> {
        let response = self
            .client
            .clone()?
            .request(Method::POST, self.url?)
            .body(query_str)
            .send()
            .await;

        if let Ok(data) = response {
            let json = data.json::<T>().await.map_err(|e| anyhow!(e));

            return Some(json.unwrap());
        }

        None
    }
}
