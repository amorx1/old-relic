use serde::Deserialize;

#[derive(Default, Debug, Deserialize, PartialEq, PartialOrd)]
pub struct WebTransactionResult {
    #[serde(rename = "beginTimeSeconds")]
    begin_time_seconds: f64,
    #[serde(rename = "endTimeSeconds")]
    end_time_seconds: f64,
    #[serde(rename = "facet")]
    facet: String,
    #[serde(rename = "segmentName")]
    segment_name: String,
    #[serde(rename = "sum.apm.service.overview.web")]
    value: f64,
}

#[derive(Debug)]
pub struct WebTransaction {
    pub begin_time_seconds: f64,
    pub end_time_seconds: f64,
    pub facet: String,
    pub segment_name: String,
    pub value: f64,
}

impl WebTransaction {
    pub fn plot(&self) -> ((f64, f64), (f64, f64)) {
        (
            (self.begin_time_seconds, self.value),
            (self.end_time_seconds, self.value),
        )
    }
}

impl From<WebTransactionResult> for WebTransaction {
    fn from(val: WebTransactionResult) -> WebTransaction {
        WebTransaction {
            begin_time_seconds: val.begin_time_seconds,
            end_time_seconds: val.end_time_seconds,
            facet: val.facet.clone(),
            segment_name: val.segment_name.clone(),
            value: val.value,
        }
    }
}
