use crate::query::QueryResponse;
use anyhow::{anyhow, Error};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, ClientBuilder, Method,
};
use serde::de::DeserializeOwned;

const QUERY_BASE: &str = r#"{ "query":  "{ actor { account(id: $account) { nrql(query: \"$query\") { results } } } }" }"#;

#[derive(Clone)]
pub struct NewRelicClient {
    url: Option<&'static str>,
    account: Option<String>,
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
        self.url = Some(url);
        self
    }

    pub fn account(&mut self, account: &str) -> &mut Self {
        self.account = Some(account.to_owned());
        self
    }

    pub fn api_key(&mut self, key: &str) -> &mut Self {
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
            HeaderValue::from_str(self.api_key.as_ref().expect("ERROR: No API Key provided!"))
                .unwrap(),
        );

        self.client = Some(client.default_headers(headers).build().unwrap());

        self
    }

    pub async fn query<T: DeserializeOwned + std::fmt::Debug + Default>(
        &self,
        query_str: impl AsRef<str>,
    ) -> Result<Vec<T>, Error> {
        let request_body = QUERY_BASE
            .replace("$account", self.account.as_ref().unwrap())
            .replace("$query", query_str.as_ref());
        let response = self
            .client
            .clone()
            .unwrap()
            .request(Method::POST, self.url.unwrap())
            .body(request_body)
            .send()
            .await;

        if let Ok(data) = response {
            let json = data
                .json::<QueryResponse<T>>()
                .await
                .map_err(|e| anyhow!(e))?;

            return Ok(json.data.actor.account.nrql.results);
        }

        Err(anyhow!("Query returned no results!"))
    }
}
