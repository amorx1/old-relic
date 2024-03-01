use serde::{Deserialize, Serialize};

use crate::NodeList;

pub static QUERY: &str = r#"
    {
        actor {
            account(id: $account) {
                nrql(query: "SELECT (appName, entityGuid) FROM Transaction WHERE appName LIKE '$name'") {
                    results
                }
            }
        }
    }"#;

#[derive(Deserialize)]
pub struct Application {
    pub app_name: String,
    pub entity_guid: String,
}

#[derive(Serialize)]
pub struct ApplicationQuery<'a> {
    pub account: i64,
    pub name: &'a str,
}

#[derive(Deserialize)]
pub struct ApplicationSearchResult {
    pub applications: NodeList<Application>,
}
