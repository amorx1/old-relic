use std::{collections::BTreeMap, path::PathBuf};

pub struct Session {
    pub is_loaded: bool,
    pub queries: Option<BTreeMap<String, String>>,
    pub session_path: PathBuf,
}
