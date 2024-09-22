mod app;
mod backend;
mod client;
mod dataset;
mod input;
pub mod parser;
pub mod query;
mod session;
mod ui;

use anyhow::Error;
use app::{App, Theme};
use backend::{query_log, query_timeseries, PayloadType, UIEvent};
use client::NewRelicClient;
use crossbeam_channel::{unbounded, Receiver as CrossBeamReceiver, Sender as CrossBeamSender};
use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use query::{QueryType, NRQL};
use ratatui::{backend::CrosstermBackend, Terminal};
use reqwest::Client;
use session::Session;
use tokio::{runtime, time};
use tokio_stream::{wrappers::IntervalStream, StreamExt};
use ui::PALETTES;

use std::{
    collections::HashSet,
    env,
    io::{self, stdout},
    path::PathBuf,
    sync::mpsc::{channel, Sender},
    time::Duration,
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
        let ui_tx = ui_tx.clone();
        backend.spawn(async move {
            _ = listen(newrelic_client, data_tx, ui_rx).await;
        });

        // Refresh events
        // backend.spawn(async move {
        //     let mut stream = IntervalStream::new(time::interval(Duration::from_secs(5)));
        //     while let Some(_ts) = stream.next().await {
        //         _ = ui_tx.send(UIEvent::RefreshData);
        //     }
        // });
    }

    let app = App::new(config, data_rx, ui_tx);
    app.run(&mut terminal).unwrap();

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}

async fn listen(
    client: NewRelicClient,
    data_tx: Sender<PayloadType>,
    ui_rx: CrossBeamReceiver<UIEvent>,
) -> Result<(), Error> {
    let mut queries: HashSet<String> = HashSet::new();
    loop {
        while let Some(event) = ui_rx.try_iter().next() {
            match event {
                UIEvent::AddQuery(query) => {
                    queries.insert(query.to_owned());
                    let q = query
                        .to_nrql()
                        .map_or_else(|_| QueryType::Log(query.to_owned()), QueryType::Timeseries);

                    let data = match q {
                        QueryType::Timeseries(x) => PayloadType::Timeseries(
                            query_timeseries(x, client.clone()).await.unwrap(),
                        ),
                        QueryType::Log(x) => {
                            PayloadType::Log(query_log(x, client.clone()).await.unwrap())
                        }
                    };
                    data_tx.send(data)?
                }
                UIEvent::DeleteQuery(query) => {
                    queries.remove(&query);
                }
                UIEvent::RefreshData => {} // UIEvent::RefreshData => {
                                           //     for query in &queries {
                                           //         let q = query.to_nrql().map_or_else(
                                           //             |_| QueryType::Log(query.to_owned()),
                                           //             QueryType::Timeseries,
                                           //         );

                                           //         let data = match q {
                                           //             QueryType::Timeseries(x) => {
                                           //                 PayloadType::Timeseries(query_timeseries(x, client.clone()).await?)
                                           //             }
                                           //             QueryType::Log(query) => {
                                           //                 PayloadType::Log(query_log(query, client.clone()).await?)
                                           //                 // Mock data
                                           //                 // PayloadType::Log(LogPayload {
                                           //                 //     // logs: BTreeMap::from([("Testing".to_string(), "".to_string())]),
                                           //                 // })
                                           //                 // PayloadType::Log(query_log(x, client.clone()).await.unwrap())
                                           //             }
                                           //         };

                                           //         data_tx.send(data)?
                                           //     }
                                           // }
            }
        }
        time::sleep(Duration::from_millis(30)).await;
    }
}
