/* Flow
    - Search for an application & select one
    - Store that application
        - appName
        - entityGuid
    - Select some time period
    - Get Traces for selected application within the specified time period
    - Get Trace data for found traces
*/

use gql_client::{Client, GraphQLError};
use serde::Deserialize;

mod application;
use application::{Application, ApplicationQuery, ApplicationSearchResult, QUERY as APP_QUERY};

#[derive(Deserialize)]
pub struct NodeList<T> {
    data: Vec<T>,
}

pub async fn search_application(
    account: i64,
    name: &str,
    client: Client,
) -> Result<Vec<Application>, GraphQLError> {
    let param = ApplicationQuery { account, name };
    let response = client
        .query_with_vars::<ApplicationSearchResult, ApplicationQuery>(APP_QUERY, param)
        .await?;

    match response {
        Some(res) => Ok(res.applications.data),
        None => Err(GraphQLError::with_text("ERROR: No application found!")),
    }
}
