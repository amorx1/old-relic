use std::str::FromStr;

pub trait Parameterized {
    fn param(&self, p: [&str; 2]) -> String;
}

pub struct NRQL {
    // *
    selection: String,
    // FROM
    source: String,
    // WHERE
    condition: String,
}

pub struct NRQLParseError;
static N_SELECT: usize = "SELECT".len();

impl FromStr for NRQL {
    type Err = NRQLParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        //
        let select_end = s
            .find("SELECT")
            .map(|idx| idx + N_SELECT + 1)
            .ok_or(NRQLParseError)?;

        Ok(NRQL {
            selection: "".to_string(),
            source: "".to_string(),
            condition: "".to_string(),
        })
    }
}
