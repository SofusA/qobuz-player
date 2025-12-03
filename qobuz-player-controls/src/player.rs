use qobuz_player_models::{Album, Track, TrackStatus};
use rand::seq::SliceRandom;
use tokio::{
    select,
    sync::watch::{self, Receiver, Sender},
};

use crate::{
    PositionReceiver, Result, Status, StatusReceiver, TracklistReceiver, VolumeReceiver,
    controls::{ControlCommand, Controls},
    database::Database,
    notification::NotificationBroadcast,
    timer::Timer,
    tracklist::{SingleTracklist, TracklistType},
};
use std::{path::PathBuf, sync::Arc, time::Duration};

use crate::{
    client::Client,
    sink::Sink,
    tracklist::{self, Tracklist},
};

const INTERVAL_MS: u64 = 500;

pub struct Player {
    broadcast: Arc<NotificationBroadcast>,
    tracklist_tx: Sender<Tracklist>,
    tracklist_rx: Receiver<Tracklist>,
    target_status: Sender<Status>,
    client: Arc<Client>,
    sink: Sink,
    volume: Sender<f32>,
    position_timer: Timer,
    position: Sender<Duration>,
    done_buffering: Receiver<()>,
    controls_rx: tokio::sync::mpsc::UnboundedReceiver<ControlCommand>,
    controls: Controls,
    database: Arc<Database>,
    next_track_is_queried: bool,
    first_track_queried: bool,
    next_track_in_queue: bool,
    next_track_is_ready: bool,
}

impl Player {
    pub fn new(
        tracklist: Tracklist,
        client: Arc<Client>,
        volume: f32,
        broadcast: Arc<NotificationBroadcast>,
        audio_cache_dir: PathBuf,
        database: Arc<Database>,
    ) -> Result<Self> {
        let (volume, volume_receiver) = watch::channel(volume);
        let sink = Sink::new(
            volume_receiver,
            broadcast.clone(),
            audio_cache_dir,
            database.clone(),
        )?;

        let done_buffering = sink.done_buffering();

        let (position, _) = watch::channel(Default::default());
        let (target_status, _) = watch::channel(Default::default());
        let (tracklist_tx, tracklist_rx) = watch::channel(tracklist);

        let (controls_tx, controls_rx) = tokio::sync::mpsc::unbounded_channel();
        let controls = Controls::new(controls_tx);

        Ok(Self {
            broadcast,
            tracklist_tx,
            tracklist_rx,
            controls_rx,
            controls,
            target_status,
            client,
            sink,
            volume,
            position_timer: Default::default(),
            position,
            done_buffering,
            database,
            next_track_in_queue: false,
            next_track_is_queried: false,
            first_track_queried: false,
            next_track_is_ready: false,
        })
    }

    pub fn controls(&self) -> Controls {
        self.controls.clone()
    }

    pub fn status(&self) -> StatusReceiver {
        self.target_status.subscribe()
    }

    pub fn volume(&self) -> VolumeReceiver {
        self.volume.subscribe()
    }

    pub fn position(&self) -> PositionReceiver {
        self.position.subscribe()
    }

    pub fn tracklist(&self) -> TracklistReceiver {
        self.tracklist_tx.subscribe()
    }

    async fn play_pause(&mut self) -> Result<()> {
        let target_status = *self.target_status.borrow();

        match target_status {
            Status::Playing | Status::Buffering => self.pause(),
            Status::Paused => self.play().await?,
        }

        Ok(())
    }

    fn start_timer(&mut self) {
        self.position_timer.start();
        self.position
            .send(self.position_timer.elapsed())
            .expect("infallible");
    }

    fn pause_timer(&mut self) {
        self.position_timer.pause();
        self.position
            .send(self.position_timer.elapsed())
            .expect("infallible");
    }

    fn stop_timer(&mut self) {
        self.position_timer.stop();
        self.position
            .send(self.position_timer.elapsed())
            .expect("infallible");
    }

    fn set_timer(&mut self, duration: Duration) {
        self.position_timer.set_time(duration);
        self.position
            .send(self.position_timer.elapsed())
            .expect("infallible");
    }

    async fn play(&mut self) -> Result<()> {
        let track = self.tracklist_rx.borrow().current_track().cloned();

        if !self.first_track_queried
            && let Some(current_track) = track
        {
            self.set_target_status(Status::Buffering);
            self.query_track(&current_track).await?;
            self.first_track_queried = true;
        }

        self.set_target_status(Status::Playing);
        self.sink.play();
        self.start_timer();
        Ok(())
    }

    fn pause(&mut self) {
        self.set_target_status(Status::Paused);
        self.sink.pause();
        self.pause_timer();
    }

    fn set_target_status(&self, status: Status) {
        self.target_status.send(status).expect("infallible");
    }

    async fn query_track(&mut self, track: &Track) -> Result<()> {
        let track_url = self.client.track_url(track.id).await?;
        let next_track_has_other_sample_rate = self.sink.query_track(track_url, track)?;
        self.next_track_is_ready = false;

        self.next_track_in_queue = match next_track_has_other_sample_rate {
            crate::sink::QueryTrackResult::Queued => true,
            crate::sink::QueryTrackResult::NotQueued => false,
        };

        Ok(())
    }

    async fn set_volume(&self, volume: f32) -> Result<()> {
        self.volume.send(volume)?;
        self.sink.sync_volume();
        self.database.set_volume(volume).await?;
        Ok(())
    }

    async fn broadcast_tracklist(&self, tracklist: Tracklist) -> Result<()> {
        self.database.set_tracklist(&tracklist).await?;
        self.tracklist_tx.send(tracklist)?;
        Ok(())
    }

    fn seek(&mut self, duration: Duration) -> Result<()> {
        self.set_timer(duration);
        self.sink.seek(duration)
    }

    fn jump_forward(&mut self) -> Result<()> {
        let duration = self
            .tracklist_rx
            .borrow()
            .current_track()
            .map(|x| Duration::from_secs(x.duration_seconds as u64));

        if let Some(duration) = duration {
            let ten_seconds = Duration::from_secs(10);
            let next_position = self.position_timer.elapsed() + ten_seconds;

            if next_position < duration {
                self.seek(next_position)?;
            } else {
                self.seek(duration)?;
            }
        }

        Ok(())
    }

    fn jump_backward(&mut self) -> Result<()> {
        let current_position = self.position_timer.elapsed();

        if current_position.as_millis() < 10000 {
            self.seek(Duration::default())?;
        } else {
            let ten_seconds = Duration::from_secs(10);
            let seek_position = current_position - ten_seconds;

            self.seek(seek_position)?;
        }
        Ok(())
    }

    /// Skip to a specific track in the tracklist.
    async fn skip_to_position(&mut self, new_position: i32, force: bool) -> Result<()> {
        let mut tracklist = self.tracklist_rx.borrow().clone();
        let current_position = tracklist.current_position();

        // Typical previous skip functionality where if,
        // the track is greater than 1 second into playing,
        // then it goes to the beginning. If triggered again
        // within a second after playing, it will skip to the previous track.
        if !force
            && new_position < current_position as i32
            && self.position.borrow().as_millis() > 1000
        {
            self.seek(Duration::default())?;
            return Ok(());
        }

        self.stop_timer();
        self.set_target_status(Status::Buffering);

        self.position.send(Default::default())?;

        if let Some(next_track) = tracklist.skip_to_track(new_position) {
            self.sink.clear().await?;
            self.next_track_is_queried = false;
            self.query_track(next_track).await?;
            self.first_track_queried = true;
            self.start_timer();
        } else {
            tracklist.reset();
            self.sink.clear().await?;
            self.next_track_is_queried = false;
            self.first_track_queried = false;
            self.set_target_status(Status::Paused);
            self.sink.pause();
            self.position.send(Default::default())?;
        }

        self.broadcast_tracklist(tracklist).await?;

        Ok(())
    }

    async fn next(&mut self) -> Result<()> {
        let current_position = self.tracklist_rx.borrow().current_position();
        self.skip_to_position((current_position + 1) as i32, true)
            .await
    }

    async fn previous(&mut self) -> Result<()> {
        let current_position = self.tracklist_rx.borrow().current_position();
        self.skip_to_position(current_position as i32 - 1, false)
            .await
    }

    async fn new_queue(&mut self, tracklist: Tracklist) -> Result<()> {
        self.stop_timer();
        self.sink.clear().await?;
        self.next_track_is_queried = false;
        self.set_target_status(Status::Buffering);

        if let Some(first_track) = tracklist.current_track() {
            self.query_track(first_track).await?;
            self.first_track_queried = true;
        }

        self.broadcast_tracklist(tracklist).await?;

        Ok(())
    }

    async fn play_track(&mut self, track_id: u32) -> Result<()> {
        let mut track: Track = self.client.track(track_id).await?;
        track.status = TrackStatus::Playing;

        let tracklist = Tracklist {
            list_type: TracklistType::Track(SingleTracklist {
                track_title: track.title.clone(),
                album_id: track.album_id.clone(),
                image: track.image.clone(),
            }),
            queue: vec![track],
        };

        self.new_queue(tracklist).await
    }

    async fn play_album(&mut self, album_id: &str, index: u32) -> Result<()> {
        let album: Album = self.client.album(album_id).await?;

        let unstreambale_tracks_to_index = album
            .tracks
            .iter()
            .take(index as usize)
            .filter(|t| !t.available)
            .count() as i32;

        let mut tracklist = Tracklist {
            queue: album.tracks.into_iter().filter(|t| t.available).collect(),
            list_type: TracklistType::Album(tracklist::AlbumTracklist {
                title: album.title,
                id: album.id,
                image: Some(album.image),
            }),
        };

        tracklist.skip_to_track(index as i32 - unstreambale_tracks_to_index);
        self.new_queue(tracklist).await
    }

    async fn play_top_tracks(&mut self, artist_id: u32, index: u32) -> Result<()> {
        let artist = self.client.artist_page(artist_id).await?;
        let tracks = artist.top_tracks;
        let unstreambale_tracks_to_index = tracks
            .iter()
            .take(index as usize)
            .filter(|t| !t.available)
            .count() as i32;

        let mut tracklist = Tracklist {
            queue: tracks.into_iter().filter(|t| t.available).collect(),
            list_type: TracklistType::TopTracks(tracklist::TopTracklist {
                artist_name: artist.name,
                id: artist_id,
                image: artist.image,
            }),
        };

        tracklist.skip_to_track(index as i32 - unstreambale_tracks_to_index);
        self.new_queue(tracklist).await
    }

    async fn play_playlist(&mut self, playlist_id: u32, index: u32, shuffle: bool) -> Result<()> {
        let playlist = self.client.playlist(playlist_id).await?;

        let unstreambale_tracks_to_index = playlist
            .tracks
            .iter()
            .take(index as usize)
            .filter(|t| !t.available)
            .count() as i32;

        let mut tracks: Vec<Track> = playlist
            .tracks
            .into_iter()
            .filter(|t| t.available)
            .collect();

        if shuffle {
            tracks.shuffle(&mut rand::rng());
        }

        let mut tracklist = Tracklist {
            queue: tracks,
            list_type: TracklistType::Playlist(tracklist::PlaylistTracklist {
                title: playlist.title,
                id: playlist.id,
                image: playlist.image,
            }),
        };

        tracklist.skip_to_track(index as i32 - unstreambale_tracks_to_index);
        self.new_queue(tracklist).await
    }

    async fn tick(&mut self) -> Result<()> {
        if *self.target_status.borrow() != Status::Playing {
            return Ok(());
        }

        let position = self.position_timer.elapsed();

        self.position.send(position)?;

        let duration = self
            .tracklist_rx
            .borrow()
            .current_track()
            .map(|x| x.duration_seconds);

        if let Some(duration) = duration {
            let position = position.as_secs();

            if duration as i16 <= position as i16 {
                self.track_finished().await?;
                return Ok(());
            }

            let track_about_to_finish = (duration as i16 - position as i16) < 60;

            if track_about_to_finish && !self.next_track_is_queried {
                let tracklist = self.tracklist_rx.borrow().clone();

                if let Some(next_track) = tracklist.next_track() {
                    self.query_track(next_track).await?;
                    self.first_track_queried = true;
                    self.next_track_is_queried = true;
                }
            }
        }

        Ok(())
    }

    async fn handle_message(&mut self, notification: ControlCommand) -> Result<()> {
        match notification {
            ControlCommand::Album { id, index } => {
                self.play_album(&id, index).await?;
            }
            ControlCommand::Playlist { id, index, shuffle } => {
                self.play_playlist(id, index, shuffle).await?;
            }
            ControlCommand::ArtistTopTracks { artist_id, index } => {
                self.play_top_tracks(artist_id, index).await?;
            }
            ControlCommand::Track { id } => {
                self.play_track(id).await?;
            }
            ControlCommand::Next => {
                self.next().await?;
            }
            ControlCommand::Previous => {
                self.previous().await?;
            }
            ControlCommand::PlayPause => {
                self.play_pause().await?;
            }
            ControlCommand::Play => {
                self.play().await?;
            }
            ControlCommand::Pause => {
                self.pause();
            }
            ControlCommand::SkipToPosition {
                new_position,
                force,
            } => {
                self.skip_to_position(new_position as i32, force).await?;
            }
            ControlCommand::JumpForward => {
                self.jump_forward()?;
            }
            ControlCommand::JumpBackward => {
                self.jump_backward()?;
            }
            ControlCommand::Seek { time } => {
                self.set_timer(time);
                self.seek(time)?;
            }
            ControlCommand::SetVolume { volume } => {
                self.set_volume(volume).await?;
            }
        }
        Ok(())
    }

    async fn track_finished(&mut self) -> Result<()> {
        self.stop_timer();
        let mut tracklist = self.tracklist_rx.borrow().clone();

        let current_position = tracklist.current_position();
        let new_position = current_position + 1;

        let next_track = tracklist.skip_to_track(new_position as i32);

        match next_track {
            Some(next_track) => {
                if !self.next_track_in_queue {
                    self.sink.clear().await?;
                    self.query_track(next_track).await?;
                }

                if self.next_track_is_ready {
                    self.start_timer();
                } else {
                    self.set_target_status(Status::Buffering);
                }
            }
            None => {
                tracklist.reset();
                self.set_target_status(Status::Paused);
                self.sink.pause();
                self.position_timer.stop();
                self.sink.clear().await?;
            }
        }
        self.next_track_is_queried = false;
        self.broadcast_tracklist(tracklist).await?;
        Ok(())
    }

    pub async fn player_loop(&mut self) -> Result<()> {
        let mut interval = tokio::time::interval(Duration::from_millis(INTERVAL_MS));

        loop {
            select! {
                _ = interval.tick() => {
                    if let Err(err) = self.tick().await {
                        self.broadcast.send_error(format!("{err}"));
                    };
                }

                Some(notification) = self.controls_rx.recv() => {
                    if let Err(err) = self.handle_message(notification).await {
                        self.broadcast.send_error(format!("{err}"));
                    };
                }

                Ok(_) = self.done_buffering.changed() => {
                    self.next_track_is_ready = true;
                    if *self.target_status.borrow() != Status::Playing {
                        self.position_timer.reset();
                        self.start_timer();
                        self.set_target_status(Status::Playing);
                    }
                }
            }
        }
    }
}
