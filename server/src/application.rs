use serde::Deserialize;

pub static QUERY: &str = r#"{ "query":  "{ actor { account(id: 2540792) { nrql(query: \"SELECT (appName, entityGuid) FROM Transaction WHERE appName LIKE '$name'\") { results } } } }" }"#;

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationSearchResult {
    pub data: Data,
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    pub actor: Actor,
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Actor {
    pub account: Account,
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub nrql: Nrql,
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Nrql {
    pub results: Vec<ApplicationResult>,
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationResult {
    pub app_name: String,
    pub entity_guid: String,
    pub timestamp: i64,
}

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
