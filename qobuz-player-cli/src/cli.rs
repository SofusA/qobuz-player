use std::{
    collections::{HashMap, HashSet},
    io::{Write, stdin, stdout},
    net::TcpListener,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use axum::{extract::Query, response::Html, routing::get};
use clap::{Parser, Subcommand};
#[cfg(any(windows, target_os = "linux", target_os = "macos"))]
use futures::executor::block_on;
use qobuz_player_client::client as qobuz_api;
use qobuz_player_controls::{
    AudioQuality, client::Client, database::Database, notification::NotificationBroadcast,
    player::Player,
};
#[cfg(any(windows, target_os = "linux", target_os = "macos"))]
use qobuz_player_controls::{Status, StatusReceiver};
use qobuz_player_rfid::RfidState;
use rodio::{DeviceTrait, cpal::traits::HostTrait};
use snafu::prelude::*;
use tokio::sync::broadcast;
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
#[allow(clippy::large_enum_variant)]
enum Commands {
    /// Default. Starts the player
    Open {
        #[clap(short, long)]
        /// Provide max audio quality (overrides any configured value)
        max_audio_quality: Option<AudioQuality>,

        #[clap(long)]
        /// Use provided device for audio output, instead of default.
        /// Use qobuz-player list-devices for output device list
        output_device_id: Option<String>,

        #[clap(long)]
        /// Delay playback when changing state from paused to playing in milliseconds
        state_change_delay_ms: Option<u64>,

        #[clap(long)]
        /// Delay playback when changing sample rate in milliseconds
        sample_rate_change_delay_ms: Option<u64>,

        #[clap(short, long, default_value_t = false)]
        /// Disable the TUI interface
        disable_tui: bool,

        #[clap(long, default_value_t = false)]
        /// Disable the album cover image in TUI
        disable_tui_album_cover: bool,

        #[cfg(target_os = "linux")]
        #[clap(long, default_value_t = false)]
        /// Disable the mpris interface
        disable_mpris: bool,

        #[clap(long, default_value_t = false)]
        /// Enable qobuz connect (experimental)
        connect: bool,

        #[clap(long, default_value_t = String::from("qobuz-player"))]
        /// Set qobuz connect device name
        connect_name: String,

        #[clap(short, long, default_value_t = false)]
        /// Start web server with web api and ui
        web: bool,

        #[clap(long)]
        /// Secret used for web ui auth
        web_secret: Option<String>,

        #[clap(long, default_value_t = 9888)]
        /// Specify port for the web server
        port: u16,

        #[clap(long, default_value_t = false)]
        /// Enable rfid interface
        rfid: bool,

        #[clap(long)]
        /// Use other qobuz-player with web for rfid database
        rfid_server_base_address: Option<String>,

        #[clap(long)]
        /// Secret for optional qobuz-player rfid server
        rfid_server_secret: Option<String>,

        #[cfg(feature = "gpio")]
        #[clap(long, default_value_t = false)]
        /// Enable gpio interface for raspberry pi. Pin 16 (gpio-23) will be high when playing
        gpio: bool,

        #[clap(long)]
        /// Cache audio files in directory [default: Temporary directory]
        audio_cache: Option<PathBuf>,

        #[clap(long, default_value_t = 1)]
        /// Hours before audio cache is cleaned. 0 for disable
        audio_cache_time_to_live: u32,

        #[cfg(any(windows, target_os = "linux", target_os = "macos"))]
        #[clap(long, default_value_t = false)]
        /// Disable sleep inhibitor
        disable_sleep_inhibitor: bool,
    },
    /// Authenticate with Qobuz via browser
    Login,
    /// Persist configurations
    Config {
        #[clap(subcommand)]
        command: ConfigCommands,
    },

    /// List available output devices
    ListDevices,
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Set username.
    #[clap(value_parser)]
    Username { username: String },
    /// Set password. Leave empty to get a password prompt.
    #[clap(value_parser)]
    Password { password: Option<String> },
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
    PlayerError { error: String },
    #[snafu(display("{error}"))]
    TerminalError { error: String },
    #[snafu(display("Not logged in. Run `qobuz-player login` to authenticate."))]
    LoginRequired,
    #[snafu(display("Login failed"))]
    LoginFailed,
    #[snafu(display("{error}"))]
    ConnectError { error: String },
}

impl From<qobuz_player_controls::error::Error> for Error {
    fn from(error: qobuz_player_controls::error::Error) -> Self {
        Error::PlayerError {
            error: error.to_string(),
        }
    }
}

impl From<qobuz_player_connect::Error> for Error {
    fn from(error: qobuz_player_connect::Error) -> Self {
        Error::ConnectError {
            error: error.to_string(),
        }
    }
}

pub async fn run() -> Result<(), Error> {
    let cli = Cli::parse();

    let database = Arc::new(Database::new().await?);

    let verbosity = match &cli.command {
        Some(Commands::Open {
            disable_tui,
            rfid,
            web,
            ..
        }) => {
            if cli.verbosity.is_none() && *disable_tui && !*rfid && *web {
                Some(tracing::Level::INFO)
            } else {
                cli.verbosity
            }
        }
        _ => cli.verbosity,
    };

    {
        let level_str = match verbosity {
            Some(tracing::Level::TRACE) => "trace",
            Some(tracing::Level::DEBUG) => "debug",
            Some(tracing::Level::INFO) => "info",
            Some(tracing::Level::WARN) => "warn",
            Some(tracing::Level::ERROR) => "error",
            None => "none",
        };

        let filter = match verbosity {
            Some(_) => {
                format!("{level_str},stream_download=warn,hyper=warn,reqwest=warn,rustls=warn")
            }
            None => level_str.to_string(),
        };

        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .compact()
            .init();
    }

    match cli.command.unwrap_or(Commands::Open {
        max_audio_quality: Default::default(),
        output_device_id: None,
        state_change_delay_ms: Default::default(),
        sample_rate_change_delay_ms: Default::default(),
        disable_tui: Default::default(),
        #[cfg(target_os = "linux")]
        disable_mpris: Default::default(),
        connect: Default::default(),
        connect_name: Default::default(),
        web: Default::default(),
        web_secret: Default::default(),
        rfid: Default::default(),
        rfid_server_base_address: Default::default(),
        rfid_server_secret: Default::default(),
        port: Default::default(),
        #[cfg(feature = "gpio")]
        gpio: Default::default(),
        audio_cache: Default::default(),
        audio_cache_time_to_live: Default::default(),
        disable_tui_album_cover: false,
        #[cfg(any(windows, target_os = "linux", target_os = "macos"))]
        disable_sleep_inhibitor: false,
    }) {
        Commands::Open {
            max_audio_quality,
            output_device_id,
            state_change_delay_ms,
            sample_rate_change_delay_ms,
            disable_tui,
            #[cfg(target_os = "linux")]
            disable_mpris,
            connect,
            connect_name,
            web,
            web_secret,
            rfid,
            rfid_server_base_address,
            rfid_server_secret,
            port,
            #[cfg(feature = "gpio")]
            gpio,
            audio_cache,
            audio_cache_time_to_live,
            disable_tui_album_cover,
            #[cfg(any(windows, target_os = "linux", target_os = "macos"))]
            disable_sleep_inhibitor,
        } => {
            let database_credentials = database.get_credentials().await?;
            let database_configuration = database.get_configuration().await?;
            let tracklist = database.get_tracklist().await.unwrap_or_default();
            let volume = database.get_volume().await.unwrap_or(1.0);

            let (exit_sender, exit_receiver) = broadcast::channel(5);

            let audio_cache = audio_cache.unwrap_or_else(|| {
                let mut cache_dir = std::env::temp_dir();
                cache_dir.push("qobuz-player-cache");
                cache_dir
            });

            let user_auth_token = match database_credentials.user_auth_token {
                Some(token) => token,
                None => {
                    tracing::info!("No auth token found, starting browser login...");
                    browser_oauth_login(&database).await?
                }
            };

            let state_change_delay = state_change_delay_ms.map(Duration::from_millis);
            let sample_rate_change_delay = sample_rate_change_delay_ms.map(Duration::from_millis);

            let max_audio_quality = max_audio_quality.unwrap_or_else(|| {
                database_configuration
                    .max_audio_quality
                    .try_into()
                    .expect("This should always convert")
            });

            let client = Arc::new(Client::new(user_auth_token, max_audio_quality.clone()));

            let broadcast = Arc::new(NotificationBroadcast::new());
            let mut player = Player::new(
                tracklist,
                client.clone(),
                volume,
                broadcast.clone(),
                audio_cache,
                database.clone(),
                state_change_delay,
                sample_rate_change_delay,
                output_device_id,
            )?;

            if connect {
                let app_id = client.app_id().await?;
                let position_receiver = player.position();
                let tracklist_receiver = player.tracklist();
                let volume_receiver = player.volume();
                let status_receiver = player.status();
                let controls = player.controls();

                tokio::spawn(async move {
                    if let Err(e) = qobuz_player_connect::init(
                        &app_id,
                        connect_name,
                        controls,
                        position_receiver,
                        tracklist_receiver,
                        status_receiver,
                        volume_receiver,
                        max_audio_quality,
                    )
                    .await
                    {
                        error_exit(e.into());
                    }
                });
            }

            let rfid_state = rfid.then(RfidState::default);

            #[cfg(target_os = "linux")]
            if !disable_mpris {
                let position_receiver = player.position();
                let tracklist_receiver = player.tracklist();
                let volume_receiver = player.volume();
                let status_receiver = player.status();
                let controls = player.controls();
                let exit_sender = exit_sender.clone();
                tokio::spawn(async move {
                    if let Err(e) = qobuz_player_mpris::init(
                        position_receiver,
                        tracklist_receiver,
                        volume_receiver,
                        status_receiver,
                        controls,
                        exit_sender,
                    )
                    .await
                    {
                        error_exit(e.into());
                    }
                });
            }

            #[cfg(any(windows, target_os = "linux", target_os = "macos"))]
            if !disable_sleep_inhibitor {
                let status_receiver = player.status();

                sleep_inhibitor(status_receiver);
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
                let database = database.clone();

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
                        database,
                    )
                    .await
                    {
                        error_exit(e.into());
                    }
                });
            }

            #[cfg(feature = "gpio")]
            if gpio {
                let status_receiver = player.status();
                tokio::spawn(async move {
                    if let Err(e) = qobuz_player_gpio::init(status_receiver).await {
                        error_exit(e.into());
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
                        rfid_server_base_address,
                        rfid_server_secret,
                    )
                    .await
                    {
                        error_exit(e.into());
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
                        exit_sender,
                        disable_tui_album_cover,
                    )
                    .await
                    {
                        error_exit(e.into());
                    };
                });
            };

            if audio_cache_time_to_live != 0 {
                let clean_up_schedule = every(1).hour().perform(move || {
                    let database = database.clone();
                    async move {
                        if let Ok(deleted_paths) = database
                            .clean_up_cache_entries(time::Duration::hours(
                                audio_cache_time_to_live.into(),
                            ))
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

            player.player_loop(exit_receiver).await?;
            Ok(())
        }
        Commands::Login => {
            browser_oauth_login(&database).await?;
            println!("Login successful! You can now run qobuz-player.");
            Ok(())
        }
        Commands::Config { command } => match command {
            ConfigCommands::Username { username } => {
                database.set_username(username).await?;
                println!("Username saved.");
                Ok(())
            }
            ConfigCommands::Password { password } => {
                let password = match password {
                    Some(password) => password,
                    None => {
                        print!("Password: ");
                        stdout().flush().or(Err(Error::LoginFailed))?;
                        stdin()
                            .lines()
                            .next()
                            .expect("encountered EOF")
                            .or(Err(Error::LoginFailed))?
                    }
                };
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
        Commands::ListDevices => {
            let Ok(devices) = rodio::cpal::default_host().output_devices() else {
                println!("Unable to find available devices");
                return Ok(());
            };

            let entries: HashSet<String> = devices
                .filter_map(|x| x.description().ok().map(|x| x.to_string()))
                .collect();

            if entries.is_empty() {
                println!("No output devices found");
                return Ok(());
            }

            println!("Available output devices:");

            for name in entries {
                println!("{name}");
            }

            Ok(())
        }
    }
}

async fn browser_oauth_login(database: &Database) -> Result<String, Error> {
    let app_id = qobuz_api::get_app_id()
        .await
        .map_err(|_| Error::LoginFailed)?;

    let listener = TcpListener::bind("127.0.0.1:0").map_err(|_| Error::LoginFailed)?;
    let port = listener
        .local_addr()
        .map_err(|_| Error::LoginFailed)?
        .port();
    drop(listener);

    let oauth_url = qobuz_api::build_oauth_url(&app_id, port);

    let manual_oauth_url = format!(
        "https://www.qobuz.com/signin/oauth?ext_app_id={app_id}&redirect_url=http%3A%2F%2Flocalhost"
    );

    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(1);

    let app = axum::Router::new().route(
        "/",
        get(move |Query(params): Query<HashMap<String, String>>| {
            let tx = tx.clone();
            async move {
                if let Some(code) = params.get("code_autorisation") {
                    let _ = tx.send(code.clone()).await;
                    Html("<html><body><h2>Login successful!</h2><p>You can close this tab and return to the player.</p></body></html>".to_string())
                } else {
                    Html("<html><body><h2>Login failed</h2><p>No authorization code received.</p></body></html>".to_string())
                }
            }
        }),
    );

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
        .await
        .map_err(|_| Error::LoginFailed)?;

    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });

    println!("Opening browser for Qobuz login...");
    println!();
    println!("  {oauth_url}");
    println!();
    println!("Headless? Open this URL on another device instead:");
    println!();
    println!("  {manual_oauth_url}");
    println!();
    println!("After login, copy the code_autorisation value from the URL bar and paste it here.");
    println!("Or if on the same network, the redirect will be captured automatically.");
    println!();
    let _ = open::that(&oauth_url);

    let code = tokio::select! {
        result = async {
            tokio::time::timeout(Duration::from_secs(300), rx.recv())
                .await
                .ok()
                .flatten()
        } => {
            match result {
                Some(code) => code,
                None => return Err(Error::LoginFailed),
            }
        }
        result = read_code_from_stdin() => {
            result?
        }
    };

    server.abort();

    tracing::debug!("Received authorization code: {}", code);

    let result = qobuz_api::exchange_oauth_code(&code, &app_id)
        .await
        .map_err(|e| {
            tracing::error!("OAuth code exchange failed: {:?}", e);
            Error::LoginFailed
        })?;

    database
        .set_user_auth_token(result.user_auth_token.clone())
        .await?;

    tracing::info!("Auth token saved.");
    Ok(result.user_auth_token)
}

async fn read_code_from_stdin() -> Result<String, Error> {
    tokio::task::spawn_blocking(|| {
        print!("Paste code: ");
        stdout().flush().ok();
        let mut input = String::new();
        stdin()
            .read_line(&mut input)
            .map_err(|_| Error::LoginFailed)?;
        let input = input.trim();
        // Accept either raw code or full URL containing code_autorisation=
        if let Some(pos) = input.find("code_autorisation=") {
            let code = &input[pos + "code_autorisation=".len()..];
            let code = code.split(['&', ' ', '#']).next().unwrap_or(code);
            Ok(code.to_string())
        } else {
            Ok(input.to_string())
        }
    })
    .await
    .map_err(|_| Error::LoginFailed)?
}

fn error_exit(error: Error) {
    eprintln!("{error}");
    std::process::exit(1);
}

#[cfg(any(windows, target_os = "linux", target_os = "macos"))]
pub fn sleep_inhibitor(mut status_receiver: StatusReceiver) {
    std::thread::spawn(move || {
        let mut sleep_inhibitor = SleepInhibitor::new();

        loop {
            let changed = block_on(async { status_receiver.changed().await });
            if changed.is_err() {
                sleep_inhibitor.restore_sleep();
                break;
            }

            let status = *status_receiver.borrow_and_update();
            match status {
                Status::Paused => sleep_inhibitor.restore_sleep(),
                Status::Playing | Status::Buffering => sleep_inhibitor.block_sleep(),
            }
        }
    });
}

#[cfg(any(windows, target_os = "linux", target_os = "macos"))]
struct SleepInhibitor {
    awake: Option<keepawake::KeepAwake>,
}

#[cfg(any(windows, target_os = "linux", target_os = "macos"))]
impl SleepInhibitor {
    fn new() -> Self {
        Self { awake: None }
    }

    fn block_sleep(&mut self) {
        if self.awake.is_none() {
            let mut builder = keepawake::Builder::default();
            builder
                .idle(true)
                .sleep(true)
                .reason("Audio playback")
                .app_name("qobuz-player");

            if let Ok(awake) = builder.create() {
                self.awake = Some(awake);
            }
        }
    }

    fn restore_sleep(&mut self) {
        let _ = self.awake.take();
    }
}
