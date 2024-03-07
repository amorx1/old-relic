use serde::Deserialize;

#[derive(Default, Debug, Deserialize, PartialEq, PartialOrd)]
pub struct TimeseriesResult {
    #[serde(rename = "beginTimeSeconds")]
    begin_time_seconds: f64,
    #[serde(rename = "endTimeSeconds")]
    end_time_seconds: f64,
    #[serde(rename = "facet")]
    facet: String,
    #[serde(rename = "Requests")]
    requests: f64,
    #[serde(rename = "entity.name")]
    entity_name: String,
}

#[derive(Debug)]
pub struct Timeseries {
    begin_time_seconds: f64,
    end_time_seconds: f64,
    facet: String,
    requests: f64,
    entity_name: String,
}

impl Timeseries {
    pub fn plot(&self) -> ((f64, f64), (f64, f64)) {
        (
            (self.begin_time_seconds, self.requests),
            (self.end_time_seconds, self.requests),
        )
    }
}

impl From<TimeseriesResult> for Timeseries {
    fn from(val: TimeseriesResult) -> Timeseries {
        Timeseries {
            begin_time_seconds: val.begin_time_seconds,
            end_time_seconds: val.end_time_seconds,
            facet: val.facet.clone(),
            requests: val.requests,
            entity_name: val.entity_name.clone(),
        }
    }
}
