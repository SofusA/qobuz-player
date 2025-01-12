use clap::{Parser, Subcommand};
use dialoguer::{Input, Password};
use hifirs_player::mpris;
use hifirs_player::sql::db;
use hifirs_qobuz_api::client::api::OutputFormat;
use snafu::prelude::*;
use tokio::task::JoinHandle;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::{fmt, prelude::*};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// Provide a username. (overrides any database value)
    #[clap(short, long)]
    pub username: Option<String>,

    #[clap(short, long)]
    /// Provide a password. (overrides any database value)
    pub password: Option<String>,

    #[clap(short, long, default_value_t = false)]
    /// Disable the TUI interface.
    pub disable_tui: bool,

    #[clap(short, long, default_value_t = false)]
    /// Start web server with websocket API and embedded UI.
    pub web: bool,

    #[clap(long, default_value = "0.0.0.0:9888")]
    /// Specify a different interface and port for the web server to listen on.
    pub interface: String,

    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Open the player
    Open {},
    /// Set configuration options
    Config {
        #[clap(subcommand)]
        command: ConfigCommands,
    },
}

#[derive(Subcommand)]
pub enum ApiCommands {
    /// Search for tracks, albums, artists and playlists
    Search {
        #[clap(value_parser)]
        query: String,
        #[clap(long, short)]
        limit: Option<i32>,
        #[clap(short, long = "output", value_enum)]
        output_format: Option<OutputFormat>,
    },
    /// Search for albums in the Qobuz database
    SearchAlbums {
        #[clap(value_parser)]
        query: String,
        #[clap(long, short)]
        limit: Option<i32>,
        #[clap(short, long = "output", value_enum)]
        output_format: Option<OutputFormat>,
    },
    /// Search for artists in the Qobuz database
    SearchArtists {
        #[clap(value_parser)]
        query: String,
        #[clap(long, short)]
        limit: Option<i32>,
        #[clap(short, long = "output", value_enum)]
        output_format: Option<OutputFormat>,
    },
    Album {
        #[clap(value_parser)]
        id: String,
        #[clap(short, long = "output", value_enum)]
        output_format: Option<OutputFormat>,
    },
    Artist {
        #[clap(value_parser)]
        id: i32,
        #[clap(short, long = "output", value_enum)]
        output_format: Option<OutputFormat>,
    },
    Track {
        #[clap(value_parser)]
        id: i32,
        #[clap(short, long = "output", value_enum)]
        output_format: Option<OutputFormat>,
    },
    /// Retrieve information about a specific playlist.
    Playlist {
        #[clap(value_parser)]
        id: i64,
        #[clap(short, long = "output", value_enum)]
        output_format: Option<OutputFormat>,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Save username to database.
    #[clap(value_parser)]
    Username {},
    /// Save password to database.
    #[clap(value_parser)]
    Password {},
}

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("{error}"))]
    ClientError { error: String },
    #[snafu(display("{error}"))]
    PlayerError { error: String },
    #[snafu(display("{error}"))]
    TerminalError { error: String },
}

impl From<hifirs_qobuz_api::Error> for Error {
    fn from(error: hifirs_qobuz_api::Error) -> Self {
        Error::ClientError {
            error: error.to_string(),
        }
    }
}

impl From<hifirs_player::error::Error> for Error {
    fn from(error: hifirs_player::error::Error) -> Self {
        Error::PlayerError {
            error: error.to_string(),
        }
    }
}

async fn setup_player(
    web: bool,
    interface: String,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<Vec<JoinHandle<()>>, Error> {
    hifirs_player::init(username, password).await?;

    let mut handles: Vec<JoinHandle<()>> = Vec::new();

    #[cfg(target_os = "linux")]
    {
        let conn = mpris::init().await;

        handles.push(tokio::spawn(async move {
            mpris::receive_notifications(&conn).await;
        }));
    }

    if web {
        handles.push(tokio::spawn(
            async move { hifirs_web::init(interface).await },
        ));
    }

    handles.push(tokio::spawn(async {
        match hifirs_player::player_loop().await {
            Ok(_) => debug!("player loop exited successfully"),
            Err(error) => debug!("player loop error {error}"),
        }
    }));

    Ok(handles)
}

pub async fn run() -> Result<(), Error> {
    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .compact()
                .with_file(false)
                .with_writer(std::io::stderr),
        )
        .with(EnvFilter::from_env("HIFIRS_LOG"))
        .init();

    // PARSE CLI ARGS
    let cli = Cli::parse();

    // INIT DB
    db::init().await;

    // CLI COMMANDS
    match cli.command {
        Commands::Open {} => {
            let mut handles = setup_player(
                cli.web,
                cli.interface,
                cli.username.as_deref(),
                cli.password.as_deref(),
            )
            .await?;

            if !(cli.disable_tui) {
                let mut tui = hifirs_tui::CursiveUI::new();
                handles.push(tokio::spawn(async {
                    hifirs_tui::receive_notifications().await
                }));
                tui.run().await;
                debug!("tui exited, quitting");
                hifirs_player::quit().await?;
                for h in handles {
                    match h.await {
                        Ok(_) => debug!("task exited"),
                        Err(error) => debug!("task error {error}"),
                    };
                }
            } else {
                debug!("waiting for ctrlc");
                tokio::signal::ctrl_c()
                    .await
                    .expect("error waiting for ctrlc");
                debug!("ctrlc received, quitting");
                hifirs_player::quit().await?;
                for h in handles {
                    match h.await {
                        Ok(_) => debug!("task exited"),
                        Err(error) => debug!("task error {error}"),
                    };
                }
            };

            Ok(())
        }
        Commands::Config { command } => match command {
            ConfigCommands::Username {} => {
                if let Ok(username) = Input::new()
                    .with_prompt("Enter your username / email")
                    .interact_text()
                {
                    db::set_username(username).await;

                    println!("Username saved.");
                }
                Ok(())
            }
            ConfigCommands::Password {} => {
                if let Ok(password) = Password::new()
                    .with_prompt("Enter your password (hidden)")
                    .interact()
                {
                    let md5_pw = format!("{:x}", md5::compute(password));

                    debug!("saving password to database: {}", md5_pw);

                    db::set_password(md5_pw).await;

                    println!("Password saved.");
                }
                Ok(())
            }
        },
    }
}
