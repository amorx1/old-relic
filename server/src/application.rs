use serde::Deserialize;

use crate::query::Query;

pub static QUERY: Query = Query::Application(
    r#"{ "query":  "{ actor { account(id: $account) { nrql(query: \"SELECT (appName, entityGuid) FROM Transaction WHERE appName LIKE '$name'\") { results } } } }" }"#,
);

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationResult {
    pub app_name: String,
    pub entity_guid: String,
    pub timestamp: i64,
}

#[derive(Debug)]
pub struct Application {
    pub app_name: String,
    pub entity_guid: String,
}

impl Application {
    pub fn from_result(result: &ApplicationResult) -> Self {
        Application {
            app_name: result.app_name.clone(),
            entity_guid: result.entity_guid.clone(),
        }
    }
}
