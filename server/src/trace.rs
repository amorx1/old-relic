use serde::Deserialize;

#[derive(Default, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct TraceResult {
    pub trace_id: Option<String>,
    pub timestamp: i64,
}

#[derive(Debug)]
pub struct Trace {
    id: String,
}

impl From<&TraceResult> for Trace {
    fn from(val: &TraceResult) -> Trace {
        Trace {
            id: val
                .trace_id
                .as_ref()
                .expect("ERROR: Result had no traceId")
                .to_string(),
        }
    }
}
