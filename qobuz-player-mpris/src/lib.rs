use mpris_server::{
    zbus::{self, fdo},
    LoopStatus, Metadata, PlaybackRate, PlaybackStatus, PlayerInterface, Property, RootInterface,
    Server, Time, TrackId, Volume,
};
use qobuz_player_controls::{models::Track, notification::Notification, ClockTime, State};

struct MprisPlayer;

impl RootInterface for MprisPlayer {
    async fn identity(&self) -> fdo::Result<String> {
        Ok("Quboz-player".into())
    }
    async fn raise(&self) -> fdo::Result<()> {
        Err(fdo::Error::NotSupported("Not supported".into()))
    }
    async fn quit(&self) -> fdo::Result<()> {
        match qobuz_player_controls::quit().await {
            Ok(_) => Ok(()),
            Err(err) => Err(fdo::Error::Failed(err.to_string())),
        }
    }
    async fn can_quit(&self) -> fdo::Result<bool> {
        Ok(true)
    }
    async fn fullscreen(&self) -> fdo::Result<bool> {
        Err(fdo::Error::NotSupported("Not supported".into()))
    }
    async fn set_fullscreen(&self, _fullscreen: bool) -> zbus::Result<()> {
        Err(zbus::Error::Unsupported)
    }
    async fn can_set_fullscreen(&self) -> fdo::Result<bool> {
        Ok(false)
    }
    async fn can_raise(&self) -> fdo::Result<bool> {
        Ok(false)
    }
    async fn has_track_list(&self) -> fdo::Result<bool> {
        Ok(false)
    }
    async fn desktop_entry(&self) -> fdo::Result<String> {
        Ok("com.github.sofusa-quboz-player".into())
    }
    async fn supported_uri_schemes(&self) -> fdo::Result<Vec<String>> {
        Ok(vec![])
    }
    async fn supported_mime_types(&self) -> fdo::Result<Vec<String>> {
        Ok(vec![])
    }
}

impl PlayerInterface for MprisPlayer {
    async fn next(&self) -> fdo::Result<()> {
        match qobuz_player_controls::next().await {
            Ok(()) => Ok(()),
            Err(err) => Err(fdo::Error::Failed(err.to_string())),
        }
    }

    async fn previous(&self) -> fdo::Result<()> {
        match qobuz_player_controls::previous().await {
            Ok(()) => Ok(()),
            Err(err) => Err(fdo::Error::Failed(err.to_string())),
        }
    }

    async fn pause(&self) -> fdo::Result<()> {
        match qobuz_player_controls::pause().await {
            Ok(()) => Ok(()),
            Err(err) => Err(fdo::Error::Failed(err.to_string())),
        }
    }

    async fn play_pause(&self) -> fdo::Result<()> {
        match qobuz_player_controls::play_pause().await {
            Ok(()) => Ok(()),
            Err(err) => Err(fdo::Error::Failed(err.to_string())),
        }
    }

    async fn stop(&self) -> fdo::Result<()> {
        match qobuz_player_controls::stop().await {
            Ok(()) => Ok(()),
            Err(err) => Err(fdo::Error::Failed(err.to_string())),
        }
    }

    async fn play(&self) -> fdo::Result<()> {
        match qobuz_player_controls::play().await {
            Ok(()) => Ok(()),
            Err(err) => Err(fdo::Error::Failed(err.to_string())),
        }
    }

    async fn seek(&self, offset: Time) -> fdo::Result<()> {
        let clock = ClockTime::from_seconds(offset.as_secs() as u64);

        match qobuz_player_controls::seek(clock, None).await {
            Ok(()) => Ok(()),
            Err(err) => Err(fdo::Error::Failed(err.to_string())),
        }
    }

    async fn set_position(&self, _track_id: TrackId, _position: Time) -> fdo::Result<()> {
        Err(fdo::Error::NotSupported("Not supported".into()))
    }

    async fn open_uri(&self, _uri: String) -> fdo::Result<()> {
        Err(fdo::Error::NotSupported("Not supported".into()))
    }

    async fn playback_status(&self) -> fdo::Result<PlaybackStatus> {
        let current_status = qobuz_player_controls::current_state();

        let status = match current_status {
            State::VoidPending => PlaybackStatus::Stopped,
            State::Null => PlaybackStatus::Stopped,
            State::Ready => PlaybackStatus::Stopped,
            State::Paused => PlaybackStatus::Paused,
            State::Playing => PlaybackStatus::Playing,
        };

        Ok(status)
    }

    async fn loop_status(&self) -> fdo::Result<LoopStatus> {
        Err(fdo::Error::NotSupported("Not supported".into()))
    }

    async fn set_loop_status(&self, _loop_status: LoopStatus) -> zbus::Result<()> {
        Err(zbus::Error::Unsupported)
    }

    async fn rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(1.0)
    }

    async fn set_rate(&self, _rate: PlaybackRate) -> zbus::Result<()> {
        Err(zbus::Error::Unsupported)
    }

    async fn shuffle(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn set_shuffle(&self, _shuffle: bool) -> zbus::Result<()> {
        Err(zbus::Error::Unsupported)
    }

    async fn metadata(&self) -> fdo::Result<Metadata> {
        match qobuz_player_controls::current_track().await {
            Ok(current_track) => Ok(track_to_metadata(current_track)),
            Err(_) => Ok(Metadata::new()),
        }
    }

    async fn volume(&self) -> fdo::Result<Volume> {
        Ok(qobuz_player_controls::volume())
    }

    async fn set_volume(&self, volume: Volume) -> zbus::Result<()> {
        qobuz_player_controls::set_volume(volume);
        Ok(())
    }

    async fn position(&self) -> fdo::Result<Time> {
        let position_mseconds = qobuz_player_controls::position()
            .map(|position| position.mseconds())
            .map_or(0, |p| p as i64);
        let time = Time::from_micros(position_mseconds);

        Ok(time)
    }

    async fn minimum_rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(1.0)
    }

    async fn maximum_rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(1.0)
    }

    async fn can_go_next(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn can_go_previous(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn can_play(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn can_pause(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn can_seek(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn can_control(&self) -> fdo::Result<bool> {
        Ok(true)
    }
}

pub async fn init() {
    let server = Server::new("com.github.sofusa-quboz-player", MprisPlayer)
        .await
        .unwrap();

    let mut receiver = qobuz_player_controls::notify_receiver();

    loop {
        if let Ok(notification) = receiver.recv().await {
            match notification {
                Notification::Quit => return,
                Notification::Status { status } => {
                    let (can_play, can_pause) = match status {
                        State::VoidPending => (false, false),
                        State::Null => (false, false),
                        State::Ready => (false, false),
                        State::Paused => (true, true),
                        State::Playing => (true, true),
                    };

                    server
                        .properties_changed([
                            Property::CanPlay(can_play),
                            Property::CanPause(can_pause),
                        ])
                        .await
                        .unwrap();
                }
                Notification::Position { clock: _ } => {}
                Notification::CurrentTrackList { list } => {
                    let current_track = qobuz_player_controls::current_track().await.unwrap();
                    let metadata = track_to_metadata(current_track);

                    let current_position = list.current_position();
                    let total_tracks = list.total();

                    let can_previous = current_position != 0;
                    let can_next = !(total_tracks != 0 && current_position == total_tracks - 1);

                    server
                        .properties_changed([
                            Property::Metadata(metadata),
                            Property::CanGoPrevious(can_previous),
                            Property::CanGoNext(can_next),
                        ])
                        .await
                        .unwrap();
                }
                Notification::Error { error: _ } => {}
                Notification::Volume { volume } => {
                    server
                        .properties_changed([Property::Volume(volume)])
                        .await
                        .unwrap();
                }
            }
        }
    }
}

fn track_to_metadata(track: Option<Track>) -> Metadata {
    let mut metadata = Metadata::new();
    let duration = track
        .as_ref()
        .map(|ct| mpris_server::Time::from_secs(ct.duration_seconds as i64));
    metadata.set_length(duration);

    // album
    let (album_title, album_image) = track.as_ref().map_or((None, None), |ct| {
        ct.album.as_ref().map_or((None, None), |a| {
            (Some(a.title.clone()), Some(a.image.clone()))
        })
    });

    metadata.set_album(album_title);
    metadata.set_art_url(album_image);

    // artist
    let artist_name = track
        .as_ref()
        .and_then(|ct| ct.artist.as_ref().map(|a| a.name.clone()));

    metadata.set_artist(artist_name.as_ref().map(|a| vec![a]));
    metadata.set_album_artist(artist_name.as_ref().map(|a| vec![a]));

    // track
    metadata.set_title(track.as_ref().map(|ct| ct.title.clone()));
    metadata.set_track_number(track.map(|ct| ct.number as i32));

    metadata
}
