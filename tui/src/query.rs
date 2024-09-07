use anyhow::Result;
use serde::Deserialize;

use crate::parser::parse_nrql;

#[derive(Debug, Deserialize, Clone)]
pub enum QueryType {
    Timeseries(NRQLQuery),
    Log(String),
}

pub struct NRQLResult {}

#[derive(Default, Debug, Deserialize, Clone)]
pub struct NRQLQuery {
    pub from: String,
    pub select: String,
    pub r#where: String,
    pub facet: String,
    pub since: String,
    pub until: String,
    pub limit: String,
    pub mode: String,
}

impl NRQLQuery {
    pub fn to_string(&self) -> Result<String> {
        let mut query = String::new();
        query += format!("FROM {} ", self.from).as_str();
        // TODO: Fix 'as value' duplication on save/load session
        query += format!("SELECT {} as value ", self.select).as_str();
        query += format!("WHERE {} ", self.r#where).as_str();
        if !String::is_empty(&self.facet) {
            query += format!("FACET {} ", self.facet).as_str();
        }
        query += format!("SINCE {} ", self.since).as_str();
        query += format!("UNTIL {} ", self.until).as_str();
        query += format!("LIMIT {} ", self.limit).as_str();
        query += self.mode.to_string().as_str();

        Ok(query.to_string())
    }
}

impl NRQL for &str {
    fn to_nrql(self) -> Result<NRQLQuery> {
        let parts = parse_nrql(self)?;
        let mut nrql = NRQLQuery::default();
        parts.iter().for_each(|(key, value)| match key.as_ref() {
            "FROM" => nrql.from = value.to_owned(),
            "SELECT" => nrql.select = value.to_owned(),
            "WHERE" => nrql.r#where = value.to_owned(),
            "FACET" => nrql.facet = value.to_owned(),
            "SINCE" => nrql.since = value.to_owned(),
            "UNTIL" => nrql.until = value.to_owned(),
            "LIMIT" => nrql.limit = value.to_owned(),
            "MODE" => nrql.mode = value.to_owned(),
            _ => panic!(),
        });
        Ok(nrql)
    }
}

pub trait NRQL {
    fn to_nrql(self) -> Result<NRQLQuery>;
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResponse<T> {
    pub data: Data<T>,
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Data<T> {
    pub actor: Actor<T>,
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Actor<T> {
    pub account: Account<T>,
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account<T> {
    pub nrql: Nrql<T>,
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Nrql<T> {
    pub results: Vec<T>,
}

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
