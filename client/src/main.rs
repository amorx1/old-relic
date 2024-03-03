use anyhow::{self, Result};
use reqwest::Client;
use server::{
    query::Parameterized,
    trace::{Trace, TraceResult, QUERY as TRACE_QUERY},
    NRClient,
};
use std::sync::OnceLock;

use server::application::{Application, ApplicationResult, QUERY as APP_QUERY};

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

    let applications = client
        .query::<ApplicationResult>(APP_QUERY.param(["fre-address-api-v2-prod", ""]))
        .await
        .unwrap();

    let selected = applications.first().map(Application::from_result).unwrap();

    dbg!(&selected);

    let mut traces = client
        .query::<TraceResult>(TRACE_QUERY.param([&selected.entity_guid, "1 minute ago"]))
        .await
        .unwrap();

    let first_trace = traces
        .into_iter()
        .filter_map(|t| Trace::from_result(&t))
        .take(1)
        .last();

    dbg!(&first_trace);

    Ok(())
}
