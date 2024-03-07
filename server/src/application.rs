use serde::Deserialize;

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

impl From<&ApplicationResult> for Application {
    fn from(val: &ApplicationResult) -> Application {
        Application {
            app_name: val.app_name.clone(),
            entity_guid: val.entity_guid.clone(),
        }
    }
}
