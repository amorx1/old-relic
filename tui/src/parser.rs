use anyhow::{anyhow, Result};

use log::debug;
use regex::RegexBuilder;

use crate::query::NRQLQuery;

const NRQL_REGEX: &str = r"^SELECT\s+(?P<select>.+?)\s+FROM\s+(?P<from>\w+)(?:\s+WHERE\s+(?P<where>.+?))?(?:\s+(?:SINCE\s+(?P<since>.+?)|UNTIL\s+(?P<until>.+?)|FACET\s+(?P<facet>.+?)|LIMIT\s+(?P<limit>\S+)))*(?:\s+(?P<mode>TIMESERIES)(?:\s+(?P<timeseries_args>.+))?)?$";

pub fn parse_nrql(input: &str) -> Result<NRQLQuery> {
    let re = RegexBuilder::new(NRQL_REGEX)
        .case_insensitive(true)
        .build()
        .unwrap();
    let captures = re.captures(input);
    if let Some(captures) = captures {
        NRQLQuery::from_captures(captures)
    } else {
        Err(anyhow!("Query {input:?} could not be parsed!"))
    }
}
