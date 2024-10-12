mod app;
mod backend;
mod client;
mod dataset;
mod input;
pub mod parser;
pub mod query;
mod session;
mod ui;

use anyhow::{anyhow, Error, Result};
use app::{App, Theme};
use backend::{query_log, query_timeseries, PayloadType, UIEvent};
use chrono::Offset;
use client::NewRelicClient;
use crossbeam_channel::{unbounded, Receiver as CrossBeamReceiver, Sender as CrossBeamSender};
use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use log::{error, info, warn, LevelFilter};
use query::{QueryType, NRQL};
use ratatui::{backend::CrosstermBackend, Terminal};
use reqwest::Client;
use session::Session;
use simplelog::{Config as LogConfig, ConfigBuilder, Level, WriteLogger};
use std::fs::File;
use tokio::{runtime, time};
use tokio_stream::{wrappers::IntervalStream, StreamExt};
use ui::PALETTES;

use std::{
    collections::HashSet,
    env,
    io::{self, stdout},
    path::PathBuf,
    sync::mpsc::{channel, Sender},
};

const DEFAULT_THEME: &str = "5";
const NEW_RELIC_ENDPOINT: &str = "https://api.newrelic.com/graphql";

pub struct Config {
    account: String,
    api_key: String,
    session: Session,
    theme: Theme,
}

impl Config {
    fn load() -> Result<Box<Self>> {
        let account = env::var("NR_ACCOUNT")?;
        let api_key = env::var("NR_API_KEY")?;
        let home_dir = env::var("HOME")?;
        let palette = env::var("THEME")
            .unwrap_or(DEFAULT_THEME.into())
            .parse::<usize>()?;
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

        Ok(Box::new(Config {
            account,
            api_key,
            session,
            theme,
        }))
    }
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    _ = setup_logging();

    let config = Config::load();

    match config {
        Ok(config) => {
            let mut newrelic_client = NewRelicClient::builder();
            newrelic_client
                .url(NEW_RELIC_ENDPOINT)
                .account(&config.account)
                .api_key(&config.api_key)
                .http_client(Client::builder());

            let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
            terminal.show_cursor()?;

            let backend = runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .thread_name("data")
                .enable_all()
                .build()?;
            let (data_tx, data_rx) = channel::<PayloadType>();
            let (ui_tx, ui_rx) = unbounded::<UIEvent>();
            {
                // Query events
                let newrelic_client = newrelic_client.clone();
                let data_tx = data_tx.clone();
                let _ui_tx = ui_tx.clone();
                backend.spawn(async move {
                    _ = listen(newrelic_client, data_tx, ui_rx).await;
                });
            }

            let app = App::new(config, data_rx, ui_tx);
            app.run(&mut terminal).unwrap();

            disable_raw_mode()?;
            stdout().execute(LeaveAlternateScreen)?;

            Ok(())
        }
        Err(e) => panic!("{}", e),
    }
}

async fn listen(
    client: NewRelicClient,
    data_tx: Sender<PayloadType>,
    ui_rx: CrossBeamReceiver<UIEvent>,
) -> Result<(), Error> {
    let mut queries: HashSet<String> = HashSet::new();
    for event in ui_rx {
        match event {
            UIEvent::AddQuery(query) => {
                queries.insert(query.to_owned());
                let query = query.to_nrql().map(QueryType::from);

                match query {
                    Ok(QueryType::Timeseries(q)) => {
                        info!("Dispatching Timeseries query: {}", &q.to_string().unwrap());

                        let result = query_timeseries(q, client.clone()).await;
                        if let Ok(data) = result {
                            if data.data.is_empty() {
                                data_tx.send(PayloadType::None)?;
                            } else {
                                let payload = PayloadType::Timeseries(data);
                                data_tx.send(payload)?;
                            }
                        }
                    }
                    Ok(QueryType::Log(q)) => {
                        info!("Dispatching Log query: {}", &q.to_string().unwrap());

                        let result = query_log(q.to_string()?, client.clone()).await;
                        if let Ok(data) = result {
                            if data.logs.is_empty() {
                                data_tx.send(PayloadType::None)?;
                            } else {
                                let payload = PayloadType::Log(data);
                                data_tx.send(payload)?;
                            }
                        }
                    }
                    Err(e) => panic!("{}", e),
                }
            }
            UIEvent::DeleteQuery(query) => {
                queries.remove(&query);
            }
        }
    }
    Ok(())
}

fn setup_logging() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConfigBuilder::new().set_time_format_rfc2822().build();

    WriteLogger::init(LevelFilter::Debug, config, File::create("app.log")?)?;

    Ok(())
}
