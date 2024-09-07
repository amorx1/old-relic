mod app;
mod backend;
mod client;
mod dataset;
mod input;
pub mod parser;
pub mod query;
mod ui;

use app::{App, Session};
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
    env,
    io::{self, stdout},
    path::PathBuf,
    sync::OnceLock,
};

static THEME: usize = 6;
const ENDPOINT: &str = "https://api.newrelic.com/graphql";
static ACCOUNT: OnceLock<i64> = OnceLock::new();
static API_KEY: OnceLock<String> = OnceLock::new();

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;

    let account = ACCOUNT.get_or_init(|| {
        std::env::var("NR_ACCOUNT")
            .expect("ERROR: No NR_ACCOUNT provided!")
            .parse::<i64>()
            .expect("ERROR: Provided NR_ACCOUNT is not valid! (Parse failure)")
    });
    let api_key = API_KEY
        .get_or_init(|| std::env::var("NR_API_KEY").expect("ERROR: No NR_API_KEY provided!"));

    let home_dir = match env::var("HOME") {
        Ok(val) => val,
        Err(_) => {
            eprintln!("Unable to determine home directory.");
            panic!()
        }
    };

    // Construct the path to Application Support directory
    let mut session_path = PathBuf::from(home_dir);
    // TODO: Implement for non-MacOS
    session_path.push("Library/Application Support/xrelic/session.yaml");
    let session = Session {
        queries: None,
        session_path: Box::new(session_path),
        is_loaded: false,
    };

    let mut client = NewRelicClient::builder();
    client
        .url(ENDPOINT)
        .account(account)
        .api_key(api_key)
        .http_client(Client::builder());

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.show_cursor()?;
    let backend = Backend::new(client);
    let app = App::new(&PALETTES[THEME], backend, session);

    app.run(&mut terminal).unwrap();

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}
