use std::{path::PathBuf, sync::Arc};

use clap::{Parser, Subcommand};
use qobuz_player_controls::{
    AudioQuality, client::Client, database::Database, notification::NotificationBroadcast,
    player::Player,
};
use qobuz_player_rfid::RfidState;
use snafu::prelude::*;
use tokio_schedule::{Job, every};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(short, long)]
    /// Log level
    verbosity: Option<tracing::Level>,

    #[clap(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Default. Starts the player
    Open {
        /// Provide a username. (overrides any configured value)
        #[clap(short, long)]
        username: Option<String>,

        #[clap(short, long)]
        /// Provide a password. (overrides any configured value)
        password: Option<String>,

        #[clap(short, long)]
        /// Provide max audio quality. (overrides any configured value)
        max_audio_quality: Option<AudioQuality>,

        #[clap(short, long, default_value_t = false)]
        /// Disable the TUI interface.
        disable_tui: bool,

        #[cfg(target_os = "linux")]
        #[clap(long, default_value_t = false)]
        /// Disable the mpris interface.
        disable_mpris: bool,

        #[clap(short, long, default_value_t = false)]
        /// Start web server with websocket API and embedded UI.
        web: bool,

        #[clap(long)]
        /// Secret used for web ui auth.
        web_secret: Option<String>,

        #[clap(long, default_value_t = false)]
        /// Enable rfid interface.
        rfid: bool,

        #[cfg(feature = "gpio")]
        #[clap(long, default_value_t = false)]
        /// Enable gpio interface for raspberry pi. Pin 16 (gpio-23) will be high when playing.
        gpio: bool,

        #[clap(long, default_value_t = 9888)]
        /// Specify port for the web server.
        port: u16,

        #[clap(long)]
        /// Cache audio files in directory.
        audio_cache: Option<PathBuf>,

        #[clap(long, default_value_t = false)]
        /// Do not clean up audio cache
        no_clean_up_audio_cache: bool,
    },
    /// Persist configurations
    Config {
        #[clap(subcommand)]
        command: ConfigCommands,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Set username.
    #[clap(value_parser)]
    Username { username: String },
    /// Set password.
    #[clap(value_parser)]
    Password { password: String },
    /// Set max audio quality.
    #[clap(value_parser)]
    MaxAudioQuality {
        #[clap(value_enum)]
        quality: AudioQuality,
    },
}

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("{error}"))]
    ClientError { error: String },
    #[snafu(display("{error}"))]
    PlayerError { error: String },
    #[snafu(display("{error}"))]
    TerminalError { error: String },
    #[snafu(display("No username found. Set with config or arguments"))]
    UsernameMissing,
    #[snafu(display("No password found. Set with config or arguments"))]
    PasswordMissing,
}

impl From<qobuz_player_client::Error> for Error {
    fn from(error: qobuz_player_client::Error) -> Self {
        Error::ClientError {
            error: error.to_string(),
        }
    }
}

impl From<qobuz_player_controls::error::Error> for Error {
    fn from(error: qobuz_player_controls::error::Error) -> Self {
        Error::PlayerError {
            error: error.to_string(),
        }
    }
}

pub async fn run() -> Result<(), Error> {
    let cli = Cli::parse();

    let database = Arc::new(Database::new().await?);

    tracing_subscriber::fmt()
        .with_max_level(cli.verbosity)
        .with_target(false)
        .compact()
        .init();

    match cli.command.unwrap_or(Commands::Open {
        username: Default::default(),
        password: Default::default(),
        max_audio_quality: Default::default(),
        disable_tui: Default::default(),
        #[cfg(target_os = "linux")]
        disable_mpris: Default::default(),
        web: Default::default(),
        web_secret: Default::default(),
        rfid: Default::default(),
        port: Default::default(),
        #[cfg(feature = "gpio")]
        gpio: Default::default(),
        audio_cache: Default::default(),
        no_clean_up_audio_cache: Default::default(),
    }) {
        Commands::Open {
            username,
            password,
            max_audio_quality,
            disable_tui,
            #[cfg(target_os = "linux")]
            disable_mpris,
            web,
            web_secret,
            rfid,
            port,
            #[cfg(feature = "gpio")]
            gpio,
            audio_cache,
            no_clean_up_audio_cache,
        } => {
            let database_credentials = database.get_credentials().await?;
            let database_configuration = database.get_configuration().await?;
            let tracklist = database.get_tracklist().await.unwrap_or_default();
            let volume = database.get_volume().await.unwrap_or(1.0);

            let audio_cache = audio_cache.unwrap_or_else(|| {
                let mut cache_dir = std::env::temp_dir();
                cache_dir.push("qobuz-player-cache");
                cache_dir
            });

            let username = match username {
                Some(username) => username,
                None => database_credentials
                    .username
                    .ok_or(Error::UsernameMissing)?,
            };

            let password = match password {
                Some(p) => p,
                None => database_credentials
                    .password
                    .ok_or(Error::PasswordMissing)?,
            };

            let max_audio_quality = max_audio_quality.unwrap_or_else(|| {
                database_configuration
                    .max_audio_quality
                    .try_into()
                    .expect("This should always convert")
            });

            let client = Arc::new(Client::new(username, password, max_audio_quality));

            let broadcast = Arc::new(NotificationBroadcast::new());
            let mut player = Player::new(
                tracklist,
                client.clone(),
                volume,
                broadcast.clone(),
                audio_cache,
                database.clone(),
            )?;

            let rfid_state = rfid.then(RfidState::default);

            #[cfg(target_os = "linux")]
            if !disable_mpris {
                let position_receiver = player.position();
                let tracklist_receiver = player.tracklist();
                let volume_receiver = player.volume();
                let status_receiver = player.status();
                let controls = player.controls();
                tokio::spawn(async move {
                    if let Err(e) = qobuz_player_mpris::init(
                        position_receiver,
                        tracklist_receiver,
                        volume_receiver,
                        status_receiver,
                        controls,
                    )
                    .await
                    {
                        exit(!disable_tui && !rfid, e.into());
                    }
                });
            }

            if web {
                let position_receiver = player.position();
                let tracklist_receiver = player.tracklist();
                let volume_receiver = player.volume();
                let status_receiver = player.status();
                let controls = player.controls();
                let rfid_state = rfid_state.clone();
                let broadcast = broadcast.clone();
                let client = client.clone();

                tokio::spawn(async move {
                    if let Err(e) = qobuz_player_web::init(
                        controls,
                        position_receiver,
                        tracklist_receiver,
                        volume_receiver,
                        status_receiver,
                        port,
                        web_secret,
                        rfid_state,
                        broadcast,
                        client,
                    )
                    .await
                    {
                        exit(!disable_tui && !rfid, e.into());
                    }
                });
            }

            #[cfg(feature = "gpio")]
            if gpio {
                let status_receiver = player.status();
                tokio::spawn(async move {
                    if let Err(e) = qobuz_player_gpio::init(status_receiver).await {
                        exit(!disable_tui && !rfid, e.into());
                    }
                });
            }

            if let Some(rfid_state) = rfid_state {
                let tracklist_receiver = player.tracklist();
                let controls = player.controls();
                let database = database.clone();
                tokio::spawn(async move {
                    if let Err(e) = qobuz_player_rfid::init(
                        rfid_state,
                        tracklist_receiver,
                        controls,
                        database,
                        broadcast,
                    )
                    .await
                    {
                        exit(!disable_tui && !rfid, e.into());
                    }
                });
            } else if !disable_tui {
                let position_receiver = player.position();
                let tracklist_receiver = player.tracklist();
                let status_receiver = player.status();
                let controls = player.controls();
                let client = client.clone();
                let broadcast = broadcast.clone();
                tokio::spawn(async move {
                    if let Err(e) = qobuz_player_tui::init(
                        client,
                        broadcast,
                        controls,
                        position_receiver,
                        tracklist_receiver,
                        status_receiver,
                    )
                    .await
                    {
                        exit(!disable_tui && !rfid, e.into());
                    };
                });
            };

            if !no_clean_up_audio_cache {
                let clean_up_schedule = every(1).hour().perform(move || {
                    let database = database.clone();
                    async move {
                        if let Ok(deleted_paths) = database
                            .clean_up_cache_entries(time::Duration::hours(1))
                            .await
                        {
                            for path in deleted_paths {
                                _ = tokio::fs::remove_file(path.as_path()).await;
                            }
                        };
                    }
                });

                tokio::spawn(clean_up_schedule);
            }

            player.player_loop().await?;
            Ok(())
        }
        Commands::Config { command } => match command {
            ConfigCommands::Username { username } => {
                database.set_username(username).await?;
                println!("Username saved.");
                Ok(())
            }
            ConfigCommands::Password { password } => {
                database.set_password(password).await?;
                println!("Password saved.");
                Ok(())
            }
            ConfigCommands::MaxAudioQuality { quality } => {
                database.set_max_audio_quality(quality).await?;

                println!("Max audio quality saved.");

                Ok(())
            }
        },
    }
}

fn exit(cli: bool, error: Error) {
    if cli {
        ratatui::restore();
    }

    eprintln!("{error}");
    std::process::exit(1);
}
