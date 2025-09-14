use std::{sync::Arc, time::Duration};

use mpris_server::{
    LoopStatus, Metadata, PlaybackRate, PlaybackStatus, PlayerInterface, Property, RootInterface,
    Server, Time, TrackId, Volume,
    zbus::{self, fdo},
};
use qobuz_player_controls::{
    PositionReceiver, Status, TracklistReceiver, models::Track, notification::Notification,
};
use qobuz_player_state::State;

struct MprisPlayer {
    state: Arc<State>,
    position_receiver: PositionReceiver,
    tracklist_receiver: TracklistReceiver,
}

impl RootInterface for MprisPlayer {
    async fn identity(&self) -> fdo::Result<String> {
        Ok("Quboz-player".into())
    }
    async fn raise(&self) -> fdo::Result<()> {
        Err(fdo::Error::NotSupported("Not supported".into()))
    }
    async fn quit(&self) -> fdo::Result<()> {
        self.state.broadcast.quit();
        Ok(())
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
        self.state.broadcast.next();
        Ok(())
    }

    async fn previous(&self) -> fdo::Result<()> {
        self.state.broadcast.previous();
        Ok(())
    }

    async fn pause(&self) -> fdo::Result<()> {
        self.state.broadcast.pause();
        Ok(())
    }

    async fn play_pause(&self) -> fdo::Result<()> {
        self.state.broadcast.play_pause();
        Ok(())
    }

    async fn stop(&self) -> fdo::Result<()> {
        self.state.broadcast.pause();
        Ok(())
    }

    async fn play(&self) -> fdo::Result<()> {
        self.state.broadcast.play();
        Ok(())
    }

    async fn seek(&self, offset: Time) -> fdo::Result<()> {
        let clock = Duration::from_secs(offset.as_secs() as u64);
        self.state.broadcast.seek(clock);
        Ok(())
    }

    async fn set_position(&self, _track_id: TrackId, _position: Time) -> fdo::Result<()> {
        Err(fdo::Error::NotSupported("Not supported".into()))
    }

    async fn open_uri(&self, _uri: String) -> fdo::Result<()> {
        Err(fdo::Error::NotSupported("Not supported".into()))
    }

    async fn playback_status(&self) -> fdo::Result<PlaybackStatus> {
        let status = match *self.state.target_status.read().await {
            Status::Paused | Status::Buffering => PlaybackStatus::Paused,
            Status::Playing => PlaybackStatus::Playing,
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
        let tracklist = self.tracklist_receiver.borrow();
        let current_track = tracklist.current_track();

        if let Some(current_track) = current_track {
            return Ok(track_to_metadata(current_track));
        };

        Ok(Metadata::new())
    }

    async fn volume(&self) -> fdo::Result<Volume> {
        let volume = self.state.volume.read().await;
        Ok(*volume as f64)
    }

    async fn set_volume(&self, volume: Volume) -> zbus::Result<()> {
        self.state.broadcast.set_volume(volume as f32);
        Ok(())
    }

    async fn position(&self) -> fdo::Result<Time> {
        let position_millis = self.position_receiver.borrow().as_millis();
        let time = Time::from_millis(position_millis as i64);
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

pub async fn init(
    state: Arc<State>,
    position_receiver: PositionReceiver,
    mut tracklist_receiver: TracklistReceiver,
) {
    let mut receiver = state.broadcast.notify_receiver();

    let server = Server::new(
        "com.github.sofusa-quboz-player",
        MprisPlayer {
            state,
            position_receiver,
            tracklist_receiver: tracklist_receiver.clone(),
        },
    )
    .await
    .unwrap();

    loop {
        tokio::select! {
            Ok(_) = tracklist_receiver.changed() => {
                let tracklist = tracklist_receiver.borrow_and_update().clone();
                let current_track = tracklist.current_track();

                if let Some(current_track) = current_track {
                    let metadata = track_to_metadata(current_track);

                    let current_position = tracklist.current_position();
                    let total_tracks = tracklist.total();

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
            },
            notification = receiver.recv() => {
                if let Ok(notification) = notification {
                    match notification {
                        Notification::Quit => return,
                        Notification::Status { status } => {
                            let (can_play, can_pause) = match status {
                                Status::Buffering => (false, false),
                                Status::Paused => (true, true),
                                Status::Playing => (true, true),
                            };

                            let playback_status = match status {
                                Status::Paused | Status::Buffering => PlaybackStatus::Paused,
                                Status::Playing => PlaybackStatus::Playing,
                            };

                            server
                                .properties_changed([
                                    Property::CanPlay(can_play),
                                    Property::CanPause(can_pause),
                                    Property::PlaybackStatus(playback_status),
                                ])
                                .await
                                .unwrap();
                        }
                        Notification::Message { message: _ } => {}
                        Notification::Volume { volume } => {
                            server
                                .properties_changed([Property::Volume(volume.into())])
                                .await
                                .unwrap();
                        }
                        Notification::Play(_play_notification) => (),
                    }
                }
            }
        }
    }
}

fn track_to_metadata(track: &Track) -> Metadata {
    let mut metadata = Metadata::new();
    let duration = mpris_server::Time::from_secs(track.duration_seconds as i64);
    metadata.set_length(Some(duration));

    metadata.set_album(track.album_title.clone());
    metadata.set_art_url(track.image.clone());

    // artist
    let artist_name = track.artist_name.clone();

    metadata.set_artist(artist_name.as_ref().map(|a| vec![a]));
    metadata.set_album_artist(artist_name.as_ref().map(|a| vec![a]));

    // track
    metadata.set_title(Some(track.title.clone()));
    metadata.set_track_number(Some(track.number as i32));

    metadata
}
