mod app;
mod backend;
mod ui;

use app::App;
use backend::Backend;
use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use reqwest::Client;
use server::NewRelicClient;

use std::{
    io::{self, stdout},
    sync::OnceLock,
};

static THEME: usize = 8;
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

    let mut client = NewRelicClient::builder();
    client
        .url(ENDPOINT)
        .account(account)
        .api_key(api_key)
        .http_client(Client::builder());

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    let backend = Backend::new(client);
    let app = App::new(THEME, backend);

    app.run(&mut terminal).unwrap();

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}
