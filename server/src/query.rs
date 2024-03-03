#[derive(Debug)]
pub enum Query {
    Application(&'static str),
    Trace(&'static str),
}

impl Parameterized for Query {
    fn param(&self, p: [&str; 2]) -> String {
        match self {
            Query::Application(base) => base.replacen("$name", p[0], 1),
            Query::Trace(base) => base.replace("$entity", p[0]).replace("$since", p[1]),
        }
    }
}

pub trait Parameterized {
    fn param(&self, p: [&str; 2]) -> String;
}
