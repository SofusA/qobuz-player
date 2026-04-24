use clap::{Args, Subcommand};
use qobuz_player_controls::{
    AppResult, AudioQuality, client::Client, database::Database,
    notification::NotificationBroadcast, player::Player,
};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio_schedule::{Job, every};

#[derive(Args, Debug)]
pub struct SharedArgs {
    #[clap(long)]
    pub audio_cache: Option<PathBuf>,

    #[clap(long, default_value_t = 1)]
    pub audio_cache_time_to_live: u32,

    #[clap(short, long)]
    /// Provide max audio quality (overrides any configured value)
    pub max_audio_quality: Option<AudioQuality>,

    #[clap(long)]
    /// Use provided device for audio output, instead of default.
    /// Use qobuz-player list-devices for output device list
    pub output_device_id: Option<String>,
}

#[derive(Args, Debug)]
pub struct ConnectArgs {
    #[clap(long)]
    pub connect: bool,

    #[clap(flatten)]
    pub name_args: ConnectNameArgs,
}

#[derive(Args, Debug)]
pub struct RfidArgs {
    #[clap(long)]
    /// Use other qobuz-player with web for rfid database
    pub rfid_server_base_address: Option<String>,

    #[clap(long)]
    /// Secret for optional qobuz-player rfid server
    pub rfid_server_secret: Option<String>,
}

#[derive(Args, Debug)]
pub struct ConnectNameArgs {
    #[clap(long, default_value = "qobuz-player")]
    pub connect_name: String,
}

#[derive(Args, Debug)]
pub struct GpioArgs {
    #[clap(long, default_value_t = false)]
    /// Enable gpio interface for raspberry pi. Pin 16 (gpio-23) will be high when playing
    pub gpio: bool,
}

#[derive(Args, Debug)]
pub struct DelayArgs {
    #[clap(long)]
    /// Delay playback when changing state from paused to playing in milliseconds
    pub state_change_delay_ms: Option<u64>,

    #[clap(long)]
    /// Delay playback when changing sample rate in milliseconds
    pub sample_rate_change_delay_ms: Option<u64>,
}

#[derive(Subcommand, Debug)]
pub enum SharedCommands {
    /// Authenticate with Qobuz via browser
    Login,

    /// Logout from Qobuz
    Logout,

    /// Persistently set the maximum audio quality
    SetMaxAudioQuality {
        #[clap(value_enum)]
        quality: AudioQuality,
    },
}

pub async fn handle_shared_commands(
    command: SharedCommands,
    database: &Database,
    headless: bool,
) -> AppResult<()> {
    match command {
        SharedCommands::Login => {
            let (_client, oauth_result) =
                Client::new_with_oauth_login(AudioQuality::Mp3, headless).await?;

            database
                .set_user_auth_token(oauth_result.user_auth_token, oauth_result.user_id)
                .await?;
            println!("Login successful! You can now run qobuz-player.");
            Ok(())
        }
        SharedCommands::Logout => {
            database.clear_user_auth_token().await?;
            println!("Logout successful!");
            Ok(())
        }
        SharedCommands::SetMaxAudioQuality { quality } => {
            database.set_max_audio_quality(quality).await?;

            println!("Max audio quality saved.");
            Ok(())
        }
    }
}

pub async fn get_client(
    database: &Database,
    max_audio_quality: AudioQuality,
    headless: bool,
) -> AppResult<Client> {
    let database_credentials = database.get_credentials().await?;

    let client = match (
        database_credentials.user_auth_token,
        database_credentials.user_id,
    ) {
        (Some(token), Some(user_id)) => Client::new(token, user_id, max_audio_quality),
        _ => {
            let (client, oauth_result) =
                Client::new_with_oauth_login(max_audio_quality, headless).await?;

            database
                .set_user_auth_token(oauth_result.user_auth_token, oauth_result.user_id)
                .await?;

            client
        }
    };

    Ok(client)
}

pub fn spawn_clean_up(database: Arc<Database>, audio_cache_time_to_live: u32) {
    if audio_cache_time_to_live != 0 {
        let clean_up_schedule = every(1).hour().perform(move || {
            let database = database.clone();
            async move {
                if let Ok(deleted_paths) = database
                    .clean_up_cache_entries(time::Duration::hours(audio_cache_time_to_live.into()))
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
}

fn default_audio_cache(path: Option<PathBuf>) -> PathBuf {
    path.unwrap_or_else(|| {
        let mut cache_dir = std::env::temp_dir();
        cache_dir.push("qobuz-player-cache");
        cache_dir
    })
}

pub async fn default_audio_quality(
    database: &Database,
    args: Option<AudioQuality>,
) -> AppResult<AudioQuality> {
    match args {
        Some(quality) => Ok(quality),
        None => {
            let database_configuration = database.get_configuration().await?;
            Ok(database_configuration.max_audio_quality.into())
        }
    }
}

pub async fn create_player(
    audio_cache: Option<PathBuf>,
    database: Arc<Database>,
    client: Arc<Client>,
    broadcast: Arc<NotificationBroadcast>,
    state_change_delay_ms: Option<u64>,
    sample_rate_change_delay_ms: Option<u64>,
    output_device_id: Option<String>,
) -> AppResult<Player> {
    let tracklist = database.get_tracklist().await.unwrap_or_default();
    let volume = database.get_volume().await.unwrap_or(1.0);
    let audio_cache = default_audio_cache(audio_cache);

    let state_change_delay = state_change_delay_ms.map(Duration::from_millis);
    let sample_rate_change_delay = sample_rate_change_delay_ms.map(Duration::from_millis);

    let player = Player::new(
        tracklist,
        client,
        volume,
        broadcast,
        audio_cache,
        database,
        state_change_delay,
        sample_rate_change_delay,
        output_device_id,
    )?;

    Ok(player)
}
