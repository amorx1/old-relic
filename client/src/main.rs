use anyhow::{anyhow, Result};
use reqwest::Client;
use server::{
    timeseries::{Timeseries, TimeseriesResult},
    NewRelicClient,
};
use std::sync::OnceLock;

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

    let mut client = NewRelicClient::builder();
    client
        .url(ENDPOINT)
        .account(account)
        .api_key(api_key)
        .http_client(Client::builder());

    // let selected= client
    //     .query::<ApplicationResult>(
    //         "SELECT (appName, entityGuid) FROM Transaction WHERE appName LIKE 'fre-address-api-v2-prod'",
    //     )
    //     .await
    //     .and_then(|a| a.first().map(Application::from))
    //     .ok_or(anyhow!("No applications found!"))?;

    // let trace = client
    //     .query::<TraceResult>(format!(
    //         "SELECT traceId FROM Transaction WHERE entity.Guid = {} SINCE 1 minute ago",
    //         selected.entity_guid
    //     ))
    //     .await
    //     .and_then(|t| t.first().map(Trace::from))
    //     .ok_or(anyhow!("No traces found!"))?;

    let data = client.query::<TimeseriesResult>(
        "SELECT sum(newrelic.goldenmetrics.infra.awsapigatewayresourcewithmetrics.requests) AS 'Requests' FROM Metric WHERE entity.guid in ('MjU0MDc5MnxJTkZSQXxOQXw4MDE0OTk0OTg4MDIzNTAxOTQ0', 'MjU0MDc5MnxJTkZSQXxOQXw4Njc2NDEwODc3ODY4NDI2Mzcz', 'MjU0MDc5MnxJTkZSQXxOQXwtODA4OTQxNjQyMzkzODMwODg2NQ', 'MjU0MDc5MnxJTkZSQXxOQXwtMzMzMDQwNzA1ODI3MDQwODE5MA', 'MjU0MDc5MnxJTkZSQXxOQXw0NjY5MzQ3MTY5NTU5NDUyODc2', 'MjU0MDc5MnxJTkZSQXxOQXwxNTA2ODc2MTg0MjI0NTQyNjU3') FACET entity.name LIMIT MAX TIMESERIES SINCE 8 days ago UNTIL now")
        .await
        .ok_or(anyhow!("No timeseries data found!"))?;

    let mut dataset: Vec<_> = vec![];

    data.into_iter()
        .map(Timeseries::from)
        .map(|t| t.plot())
        .for_each(|(start, end)| {
            dataset.push(start);
            dataset.push(end);
        });

    dbg!(&dataset);

    Ok(())
}
