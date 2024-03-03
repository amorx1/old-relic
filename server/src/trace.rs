use serde::Deserialize;

use crate::query::Query;

pub static QUERY: Query = Query::Trace(
    r#"{ "query":  "{ actor { account(id: $account) { nrql(query: \"SELECT traceId FROM Transaction WHERE entity.Guid = $entity SINCE $since\") { results } } } }" }"#,
);

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

impl Trace {
    pub fn from_result(result: &TraceResult) -> Option<Self> {
        result.trace_id.as_ref().map(|id| Trace { id: id.clone() })
    }
}
