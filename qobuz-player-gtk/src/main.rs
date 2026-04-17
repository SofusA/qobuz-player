use qobuz_player_cli::{
    ConnectArgs, SharedArgs, SharedCommands, create_player, default_audio_quality, get_client,
    handle_shared_commands, spawn_clean_up,
};
use std::{fs, io, path::PathBuf, sync::Arc};
use tokio::sync::broadcast;

use clap::Parser;
use qobuz_player_controls::{
    AppResult, database::Database, error::Error, notification::NotificationBroadcast,
};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Arguments {
    #[clap(flatten)]
    shared: SharedArgs,

    #[clap(flatten)]
    connect: ConnectArgs,

    #[clap(subcommand)]
    command: Option<SharedCommands>,

    /// Install a user-level desktop entry
    #[arg(long)]
    install: bool,

    /// Uninstall the user-level desktop entry
    #[arg(long)]
    uninstall: bool,
}

#[tokio::main]
async fn main() {
    match run().await {
        Ok(()) => {}
        Err(err) => {
            error_exit(err);
        }
    }
}

pub async fn run() -> AppResult<()> {
    let args = Arguments::parse();

    if args.install {
        install_desktop_entry().expect("Failed to install desktop entry");
        return Ok(());
    }

    if args.uninstall {
        uninstall_desktop_entry().expect("Failed to uninstall desktop entry");
        return Ok(());
    }

    let database = Arc::new(Database::new().await?);

    if let Some(command) = args.command {
        handle_shared_commands(command, &database).await?;
        return Ok(());
    }

    let (exit_sender, exit_receiver) = broadcast::channel(5);

    let max_audio_quality = default_audio_quality(&database, args.shared.max_audio_quality).await?;
    let client = get_client(&database, max_audio_quality).await?;
    let client = Arc::new(client);

    let broadcast = Arc::new(NotificationBroadcast::new());

    let mut player = create_player(
        args.shared.audio_cache,
        database.clone(),
        client.clone(),
        broadcast.clone(),
        None,
        None,
        args.shared.output_device_id,
    )
    .await?;

    #[cfg(target_os = "linux")]
    {
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
                error_exit(e);
            }
        });
    }

    let client = client.clone();

    if args.connect.connect {
        let app_id = client.app_id().await?;
        let position_receiver = player.position();
        let tracklist_receiver = player.tracklist();
        let volume_receiver = player.volume();
        let status_receiver = player.status();
        let controls = player.controls();

        tokio::spawn(async move {
            if let Err(e) = qobuz_player_connect::init(
                &app_id,
                args.connect.name_args.connect_name,
                controls,
                position_receiver,
                tracklist_receiver,
                status_receiver,
                volume_receiver,
                max_audio_quality,
            )
            .await
            {
                error_exit(e);
            }
        });
    }

    let controls = player.controls();
    let tracklist_receiver = player.tracklist();
    let status_receiver = player.status();
    let position_receiver = player.position();
    tokio::task::spawn_blocking(move || {
        qobuz_player_gtk::init(
            client,
            tracklist_receiver,
            status_receiver,
            position_receiver,
            controls,
        );
    });

    spawn_clean_up(database, args.shared.audio_cache_time_to_live);
    player.player_loop(exit_receiver).await?;

    Ok(())
}

fn error_exit(error: Error) {
    eprintln!("{error}");
    std::process::exit(1);
}

const APP_ID: &str = "qobuz-player-gtk";
const DESKTOP_FILE: &str = "qobuz-player.desktop";
const ICON: &[u8] = include_bytes!("../../qobuz-player-web/assets/favicon.svg");

fn data_local_dir() -> PathBuf {
    dirs::data_local_dir().expect("Could not determine XDG data-local directory")
}

fn applications_dir() -> PathBuf {
    data_local_dir().join("applications")
}

fn icon_dir() -> PathBuf {
    data_local_dir().join("icons/qobuz-player")
}

fn icon_path() -> PathBuf {
    icon_dir().join("icon.svg")
}

fn desktop_entry_contents() -> String {
    format!(
        r#"[Desktop Entry]
Type=Application
Name=Qobuz Player
Comment=Qobuz desktop music player
Exec={app}
Icon={icon_path}
Terminal=false
Categories=Audio;Music;Player;
StartupNotify=true
"#,
        app = APP_ID,
        icon_path = icon_path().display()
    )
}

fn install_desktop_entry() -> io::Result<()> {
    let apps_dir = applications_dir();
    fs::create_dir_all(&apps_dir)?;

    let desktop_path = apps_dir.join(DESKTOP_FILE);
    fs::write(&desktop_path, desktop_entry_contents())?;

    let icons_dir = icon_dir();
    fs::create_dir_all(&icons_dir)?;

    let icon_path = icon_path();
    fs::write(&icon_path, ICON)?;

    println!("Desktop entry installed:");
    println!("{}", desktop_path.display());
    println!("Icon installed:");
    println!("{}", icon_path.display());

    Ok(())
}

fn uninstall_desktop_entry() -> io::Result<()> {
    let desktop_path = applications_dir().join(DESKTOP_FILE);
    let icon_path = icon_dir().join("icons.png");

    let _ = fs::remove_file(desktop_path);
    let _ = fs::remove_file(icon_path);

    println!("Desktop entry and icon removed");

    Ok(())
}
