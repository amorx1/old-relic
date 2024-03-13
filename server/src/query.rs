use anyhow::{bail, Result};

pub enum QueryTypes<'a> {
    Timeseries(TimeseriesQuery<'a>),
}

#[derive(Debug, Default)]
pub struct TimeseriesQuery<'a> {
    source: Option<&'a str>,
    selection: Option<&'a str>,
    r#where: Option<&'a str>,
    facet: Option<&'a str>,
    since: Option<&'a str>,
    until: Option<&'a str>,
    limit: Option<&'a str>,
}

pub trait Timeseries: Send + Sync {
    fn timeseries(&self) -> Result<String> {
        bail!("Default implementation should not be used!")
    }
}

impl<'a> Timeseries for TimeseriesQuery<'a> {
    fn timeseries(&self) -> Result<String> {
        let mut query = String::new();

        if let Some(source) = self.source {
            query.push_str("FROM ");
            query.push_str(source);
            query.push(' ');
        }

        if let Some(selection) = self.selection {
            query.push_str("SELECT ");
            query.push_str(selection);
            query.push(' ');
        }
        if let Some(r#where) = self.r#where {
            query.push_str("WHERE ");
            query.push_str(r#where);
            query.push(' ');
        }
        if let Some(facet) = self.facet {
            query.push_str("FACET ");
            query.push_str(facet);
            query.push(' ');
        }
        if let Some(since) = self.since {
            query.push_str("SINCE ");
            query.push_str(since);
            query.push(' ');
        }
        if let Some(until) = self.until {
            query.push_str("UNTIL ");
            query.push_str(until);
            query.push(' ');
        }
        if let Some(limit) = self.limit {
            query.push_str("LIMIT ");
            query.push_str(limit);
            query.push(' ');
        }

        query.push_str("TIMESERIES");

        Ok(query.to_string())
    }
}

impl<'a> TimeseriesQuery<'a> {
    pub fn from(&mut self, arg: &'a str) -> &mut Self {
        self.source = Some(arg);
        self
    }
    pub fn select(&mut self, arg: &'a str) -> &mut Self {
        self.selection = Some(arg);
        self
    }
    pub fn r#where(&mut self, arg: &'a str) -> &mut Self {
        self.r#where = Some(arg);
        self
    }
    pub fn facet(&mut self, arg: &'a str) -> &mut Self {
        self.facet = Some(arg);
        self
    }
    pub fn since(&mut self, arg: &'a str) -> &mut Self {
        self.since = Some(arg);
        self
    }
    pub fn until(&mut self, arg: &'a str) -> &mut Self {
        self.until = Some(arg);
        self
    }
    pub fn limit(&mut self, arg: &'a str) -> &mut Self {
        self.limit = Some(arg);
        self
    }
}
