use chrono::{DateTime, Duration, Local};
use gstreamer::{ClockTime, State as GstState};
use qobuz_player_controls::{
    models::{Album, Track, TrackStatus},
    notification::Notification,
};
use std::collections::HashMap;
use tracing::debug;
use zbus::{interface, zvariant, Connection, ConnectionBuilder, SignalContext};

#[derive(Debug)]
struct Mpris {}

pub async fn init() {
    let mpris = Mpris {};
    let mpris_player = MprisPlayer {
        status: GstState::Null,
        total_tracks: 0,
        position: ClockTime::default(),
        position_ts: chrono::offset::Local::now(),
        can_play: true,
        can_pause: true,
        can_stop: true,
        can_next: true,
        can_previous: true,
    };
    let mpris_tracklist = MprisTrackList {};

    let conn = ConnectionBuilder::session()
        .unwrap()
        .serve_at("/org/mpris/MediaPlayer2", mpris)
        .unwrap()
        .serve_at("/org/mpris/MediaPlayer2", mpris_player)
        .unwrap()
        .serve_at("/org/mpris/MediaPlayer2", mpris_tracklist)
        .unwrap()
        .name("org.mpris.MediaPlayer2.qobuz-player")
        .unwrap()
        .build()
        .await
        .expect("There was an error with mpris and I must exit.");

    receive_notifications(&conn).await;
}

async fn receive_notifications(conn: &Connection) {
    let mut receiver = qobuz_player_controls::notify_receiver();
    let object_server = conn.object_server();

    loop {
        if let Ok(notification) = receiver.recv().await {
            match notification {
                Notification::Quit => {
                    return;
                }
                Notification::Status { status } => {
                    let iface_ref = object_server
                        .interface::<_, MprisPlayer>("/org/mpris/MediaPlayer2")
                        .await
                        .expect("failed to get object server");

                    let mut iface = iface_ref.get_mut().await;
                    iface.status = status;

                    match status {
                        GstState::Null => {
                            iface.can_play = true;
                            iface.can_pause = true;
                            iface.can_stop = false;
                        }
                        GstState::Paused => {
                            iface.can_play = true;
                            iface.can_pause = false;
                            iface.can_stop = true;
                        }
                        GstState::Playing => {
                            iface.position_ts = chrono::offset::Local::now();
                            iface.can_play = true;
                            iface.can_pause = true;
                            iface.can_stop = true;
                        }
                        _ => {
                            iface.can_play = true;
                            iface.can_pause = true;
                            iface.can_stop = true;
                        }
                    }

                    iface
                        .playback_status_changed(iface_ref.signal_context())
                        .await
                        .expect("failed to signal metadata change");
                }
                Notification::Position { clock } => {
                    let iface_ref = object_server
                        .interface::<_, MprisPlayer>("/org/mpris/MediaPlayer2")
                        .await
                        .expect("failed to get object server");

                    let mut iface = iface_ref.get_mut().await;
                    let now = chrono::offset::Local::now();
                    let diff = now.signed_duration_since(iface.position_ts);
                    let position_secs = clock.seconds();

                    if diff.num_seconds() != position_secs as i64 {
                        debug!("mpris clock drift, sending new position");
                        iface.position_ts =
                            chrono::offset::Local::now() - Duration::seconds(position_secs as i64);

                        MprisPlayer::seeked(iface_ref.signal_context(), clock.useconds() as i64)
                            .await
                            .expect("failed to send seeked signal");
                    }
                }
                Notification::CurrentTrackList { list } => {
                    let player_ref = object_server
                        .interface::<_, MprisPlayer>("/org/mpris/MediaPlayer2")
                        .await
                        .expect("failed to get object server");

                    let list_ref = object_server
                        .interface::<_, MprisTrackList>("/org/mpris/MediaPlayer2")
                        .await
                        .expect("failed to get object server");

                    let mut player_iface = player_ref.get_mut().await;

                    if let Some(album) = list.get_album() {
                        player_iface.total_tracks = album.total_tracks;
                    }

                    if let Some(current) = list.current_track() {
                        player_iface.can_previous = current.position != 0;

                        player_iface.can_next = !(player_iface.total_tracks != 0
                            && current.position == player_iface.total_tracks - 1);

                        let tracks = list
                            .queue
                            .values()
                            .map(|i| i.title.as_str())
                            .collect::<Vec<&str>>();

                        MprisTrackList::track_list_replaced(
                            list_ref.signal_context(),
                            tracks,
                            &current.title,
                        )
                        .await
                        .expect("failed to send track list replaced signal");
                    }

                    player_iface
                        .metadata_changed(player_ref.signal_context())
                        .await
                        .expect("failed to signal metadata change");
                }
                Notification::Error { error: _ } => {}
                Notification::Volume { volume: _ } => {}
            }
        }
    }
}

#[interface(name = "org.mpris.MediaPlayer2")]
impl Mpris {
    #[zbus(property, name = "CanQuit")]
    fn can_quit(&self) -> bool {
        true
    }
    #[zbus(property, name = "CanSetFullscreen")]
    fn can_set_fullscreen(&self) -> bool {
        false
    }
    #[zbus(property, name = "CanRaise")]
    fn can_raise(&self) -> bool {
        false
    }
    #[zbus(property, name = "SupportedMimeTypes")]
    fn supported_mime_types(&self) -> Vec<&str> {
        vec!["audio/mpeg", "audio/x-flac", "audio/flac"]
    }
    #[zbus(property, name = "SupportedUriSchemes")]
    fn supported_uri_schemes(&self) -> Vec<&str> {
        vec!["http"]
    }
    #[zbus(property)]
    fn identity(&self) -> &str {
        "qobuz-player"
    }
    #[zbus(property)]
    fn has_track_list(&self) -> bool {
        true
    }
}

#[derive(Debug)]
struct MprisPlayer {
    status: GstState,
    position: ClockTime,
    position_ts: DateTime<Local>,
    total_tracks: u32,
    can_play: bool,
    can_pause: bool,
    can_stop: bool,
    can_next: bool,
    can_previous: bool,
}

#[interface(name = "org.mpris.MediaPlayer2.Player")]
impl MprisPlayer {
    async fn play(&self) {
        if let Err(error) = qobuz_player_controls::play().await {
            debug!(?error);
        }
    }
    async fn pause(&self) {
        if let Err(error) = qobuz_player_controls::pause().await {
            debug!(?error);
        }
    }
    async fn stop(&self) {
        if let Err(error) = qobuz_player_controls::stop().await {
            debug!(?error);
        }
    }
    async fn play_pause(&self) {
        if let Err(error) = qobuz_player_controls::play_pause().await {
            debug!(?error);
        }
    }
    async fn next(&self) {
        if let Err(error) = qobuz_player_controls::next().await {
            debug!(?error);
        }
    }
    async fn previous(&self) {
        if let Err(error) = qobuz_player_controls::previous().await {
            debug!(?error);
        }
    }
    #[zbus(property, name = "PlaybackStatus")]
    async fn playback_status(&self) -> &str {
        match self.status {
            GstState::Playing => "Playing",
            GstState::Paused => "Paused",
            GstState::Null => "Stopped",
            GstState::VoidPending => "Stopped",
            GstState::Ready => "Ready",
        }
    }
    #[zbus(property, name = "LoopStatus")]
    fn loop_status(&self) -> &str {
        "None"
    }
    #[zbus(property, name = "Rate")]
    fn rate(&self) -> f64 {
        1.0
    }
    #[zbus(property, name = "Shuffle")]
    fn shuffle(&self) -> bool {
        false
    }
    #[zbus(property, name = "Metadata")]
    async fn metadata(&self) -> HashMap<&str, zvariant::Value> {
        debug!("signal metadata refresh");
        if let Some(current_track) = qobuz_player_controls::current_track().await {
            track_to_meta(
                current_track,
                qobuz_player_controls::current_tracklist()
                    .await
                    .get_album()
                    .cloned(),
            )
        } else {
            HashMap::default()
        }
    }
    #[zbus(property, name = "Volume")]
    fn volume(&self) -> f64 {
        1.0
    }
    #[zbus(property, name = "Position")]
    async fn position(&self) -> i64 {
        self.position.useconds() as i64
    }
    #[zbus(signal, name = "Seeked")]
    async fn seeked(
        #[zbus(signal_context)] ctxt: &SignalContext<'_>,
        message: i64,
    ) -> zbus::Result<()>;
    #[zbus(property, name = "MinimumRate")]
    fn minimum_rate(&self) -> f64 {
        1.0
    }
    #[zbus(property, name = "MaximumRate")]
    fn maximum_rate(&self) -> f64 {
        1.0
    }
    #[zbus(property, name = "CanGoNext")]
    fn can_go_next(&self) -> bool {
        self.can_next
    }
    #[zbus(property, name = "CanGoPrevious")]
    fn can_go_previous(&self) -> bool {
        self.can_previous
    }
    #[zbus(property, name = "CanPlay")]
    fn can_play(&self) -> bool {
        self.can_play
    }
    #[zbus(property, name = "CanPause")]
    fn can_pause(&self) -> bool {
        self.can_pause
    }
    #[zbus(property, name = "CanStop")]
    fn can_stop(&self) -> bool {
        self.can_stop
    }
    #[zbus(property, name = "CanSeek")]
    fn can_seek(&self) -> bool {
        true
    }
    #[zbus(property, name = "CanControl")]
    fn can_control(&self) -> bool {
        true
    }
}

#[derive(Debug)]
struct MprisTrackList {}

#[interface(name = "org.mpris.MediaPlayer2.TrackList")]
impl MprisTrackList {
    #[zbus(signal, name = "TrackListReplaced")]
    async fn track_list_replaced(
        #[zbus(signal_context)] ctxt: &SignalContext<'_>,
        tracks: Vec<&str>,
        current: &str,
    ) -> zbus::Result<()>;

    #[zbus(property, name = "Tracks")]
    async fn tracks(&self) -> Vec<String> {
        qobuz_player_controls::current_tracklist()
            .await
            .queue
            .iter()
            .filter_map(|t| {
                if t.1.status == TrackStatus::Unplayed {
                    Some(t.1)
                } else {
                    None
                }
            })
            .map(|i| i.position.to_string())
            .collect::<Vec<String>>()
    }

    #[zbus(property, name = "CanEditTracks")]
    async fn can_edit_tracks(&self) -> bool {
        false
    }
}

fn track_to_meta<'a>(
    playlist_track: Track,
    album: Option<Album>,
) -> HashMap<&'a str, zvariant::Value<'a>> {
    let mut meta = HashMap::new();

    meta.insert(
        "mpris:trackid",
        zvariant::Value::new(format!(
            "/org/qobuz-player/Player/TrackList/{}",
            playlist_track.id
        )),
    );
    meta.insert(
        "xesam:title",
        zvariant::Value::new(playlist_track.title.trim().to_string()),
    );
    meta.insert(
        "xesam:trackNumber",
        zvariant::Value::new(playlist_track.position as i32),
    );

    meta.insert(
        "mpris:length",
        zvariant::Value::new(
            ClockTime::from_seconds(playlist_track.duration_seconds as u64).useconds() as i64,
        ),
    );

    if let Some(artist) = &playlist_track.artist {
        meta.insert(
            "xesam:artist",
            zvariant::Value::new(artist.name.trim().to_string()),
        );
    }

    if let Some(album) = album {
        meta.insert(
            "mpris:artUrl",
            zvariant::Value::new(album.cover_art.clone()),
        );
        meta.insert(
            "xesam:album",
            zvariant::Value::new(album.title.trim().to_string()),
        );
        meta.insert(
            "xesam:albumArtist",
            zvariant::Value::new(album.artist.name.trim().to_string()),
        );
        meta.insert(
            "xesam:artist",
            zvariant::Value::new(album.artist.name.trim().to_string()),
        );
    }

    meta
}
