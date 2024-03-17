use serde::Deserialize;

#[derive(Default, Debug, Deserialize, PartialEq, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct TimeseriesResult {
    pub begin_time_seconds: f64,
    pub end_time_seconds: f64,
    pub facet: Option<String>,
    pub value: f64,
}

#[derive(Debug)]
pub struct Timeseries {
    pub begin_time_seconds: f64,
    pub end_time_seconds: f64,
    pub facet: Option<String>,
    pub value: f64,
}

impl Timeseries {
    pub fn plot(&self) -> ((f64, f64), (f64, f64)) {
        (
            (self.begin_time_seconds, self.value),
            (self.end_time_seconds, self.value),
        )
    }
}

impl From<TimeseriesResult> for Timeseries {
    fn from(val: TimeseriesResult) -> Timeseries {
        Timeseries {
            begin_time_seconds: val.begin_time_seconds,
            end_time_seconds: val.end_time_seconds,
            facet: val.facet.clone(),
            value: val.value,
        }
    }
}
