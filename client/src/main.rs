use anyhow::{self, Result};
use reqwest::Client;
use server::{
    trace::{Trace, TraceResult},
    NRClient,
};
use std::sync::OnceLock;

use server::application::{Application, ApplicationResult};

const ENDPOINT: &str = "https://api.newrelic.com/graphql";
static ACCOUNT: OnceLock<i64> = OnceLock::new();
static API_KEY: OnceLock<String> = OnceLock::new();

#[tokio::main]
async fn main() -> Result<()> {
    let account = ACCOUNT.get_or_init(|| {
        std::env::var("NR_ACCOUNT")
            .expect("ERROR: No NR_ACCOUNT provided!")
            .parse::<i64>()
            .expect("ERROR: Provided NR_ACCOUNT is not valid! (Parse failure)")
    });
    let api_key = API_KEY
        .get_or_init(|| std::env::var("NR_API_KEY").expect("ERROR: No NR_API_KEY provided!"));

    let mut client = NRClient::builder();
    client
        .url(ENDPOINT)
        .account(account)
        .api_key(api_key)
        .http_client(Client::builder());

    let selected= client
        .query::<ApplicationResult>(
            "SELECT (appName, entityGuid) FROM Transaction WHERE appName LIKE 'fre-address-api-v2-prod'",
        )
        .await
        .and_then(|a| a.first().map(Application::from))
        .unwrap();

    let trace = client
        .query::<TraceResult>(format!(
            "SELECT traceId FROM Transaction WHERE entity.Guid = {} SINCE 1 minute ago",
            selected.entity_guid
        ))
        .await
        .and_then(|t| t.first().map(Trace::from))
        .unwrap();

    dbg!(&trace);

    Ok(())
}
