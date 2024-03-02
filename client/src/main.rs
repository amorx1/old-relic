use anyhow::{self, bail, Result};
use reqwest::Client;
use server::NRClient;
use std::sync::OnceLock;
use tokio;

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

    let applications = client.search_application("fre-address-api-v2-prod").await;

    dbg!(&applications);

    let selected_app = applications
        .expect("No applications found!")
        .first()
        .map(|app| Application::from_result(app))
        .unwrap();

    Ok(())
}
