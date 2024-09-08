mod app;
mod backend;
mod client;
mod dataset;
mod input;
pub mod parser;
pub mod query;
mod ui;

use app::{App, Theme};
use backend::Backend;
use client::NewRelicClient;
use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use reqwest::Client;
use ui::PALETTES;

use std::{
    collections::BTreeMap,
    env,
    io::{self, stdout},
    path::PathBuf,
};

const DEFAULT_THEME: &str = "6";
const NEW_RELIC_ENDPOINT: &str = "https://api.newrelic.com/graphql";

pub struct Config {
    account: String,
    api_key: String,
    session: Session,
    theme: Theme,
}

impl Config {
    fn load() -> Box<Self> {
        let account = env::var("NR_ACCOUNT").expect("ERROR: No NR_ACCOUNT provided!");
        let api_key = env::var("NR_API_KEY").expect("ERROR: No NR_API_KEY provided!");
        let home_dir = env::var("HOME").expect("ERROR: $HOME could not be read");
        let palette = env::var("THEME")
            .unwrap_or(DEFAULT_THEME.into())
            .parse::<usize>()
            .expect("ERROR: Invalid THEME value provided!");
        let theme = Theme {
            focus_fg: PALETTES[palette].c200,
            chart_fg: PALETTES[palette].c400,
        };

        // Construct the path to Session directory
        let mut session_path = PathBuf::from(home_dir);
        // TODO: Implement for non-MacOS
        session_path.push("Library/Application Support/xrelic/session.yaml");

        let session = Session {
            queries: None,
            session_path,
            is_loaded: false,
        };

        Box::new(Config {
            account,
            api_key,
            session,
            theme,
        })
    }
}

pub struct Session {
    pub is_loaded: bool,
    pub queries: Option<BTreeMap<String, String>>,
    pub session_path: PathBuf,
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;

    let config = Config::load();
    let mut newrelic_client = NewRelicClient::builder();
    newrelic_client
        .url(NEW_RELIC_ENDPOINT)
        .account(&config.account)
        .api_key(&config.api_key)
        .http_client(Client::builder());

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.show_cursor()?;

    let backend = Backend::new(newrelic_client);
    let app = App::new(config, backend);

    app.run(&mut terminal).unwrap();

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}
