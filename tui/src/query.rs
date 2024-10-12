use anyhow::Result;
use serde::Deserialize;

use crate::parser::parse_nrql;

#[derive(Debug, Deserialize, Clone)]
pub enum QueryType {
    Timeseries(NRQLQuery),
    Log(NRQLQuery),
}

impl QueryType {
    pub fn from(nrql: NRQLQuery) -> Self {
        match nrql.mode {
            Some(_) => QueryType::Timeseries(nrql),
            None => QueryType::Log(nrql),
        }
    }
}

#[derive(Default, Debug, Deserialize, Clone)]
pub struct NRQLQuery {
    pub from: String,
    pub select: String,
    pub r#where: String,
    pub facet: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub limit: Option<String>,
    pub mode: Option<String>,
}

impl NRQLQuery {
    pub fn from_captures(captures: regex::Captures) -> Result<Self> {
        Ok(NRQLQuery {
            from: match captures.name("from") {
                Some(m) => m.as_str().to_string(),
                None => panic!(),
            },
            select: match captures.name("select") {
                Some(m) => m.as_str().to_string(),
                None => panic!(),
            },
            r#where: match captures.name("where") {
                Some(m) => m.as_str().to_string(),
                None => panic!(),
            },
            facet: captures.name("facet").map(|m| m.as_str().to_string()),
            since: captures.name("since").map(|m| m.as_str().to_string()),
            until: captures.name("until").map(|m| m.as_str().to_string()),
            limit: captures.name("limit").map(|m| m.as_str().to_string()),
            mode: captures.name("mode").map(|m| m.as_str().to_string()),
        })
    }
    pub fn to_string(&self) -> Result<String> {
        let mut query = String::new();
        query += format!("SELECT {}", self.select).as_str();

        if self.mode == Some("TIMSERIES".into()) {
            query += " as value";
        }

        query += format!(" FROM {}", self.from).as_str();
        // TODO: Fix 'as value' duplication on save/load session
        query += format!(" WHERE {}", self.r#where).as_str();
        if let Some(facet) = &self.facet {
            query += format!(" FACET {}", facet).as_str();
        }
        if let Some(since) = &self.since {
            query += format!(" since {}", since).as_str();
        }
        if let Some(until) = &self.until {
            query += format!(" until {}", until).as_str();
        }
        if let Some(limit) = &self.limit {
            query += format!(" limit {}", limit).as_str();
        }
        if let Some(mode) = &self.mode {
            query += " ";
            query += mode.to_string().as_str();
        }

        Ok(query.to_string())
    }
}

impl NRQL for &str {
    fn to_nrql(self) -> Result<NRQLQuery> {
        parse_nrql(self)
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
