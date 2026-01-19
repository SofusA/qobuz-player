use qobuz_player_models::{Album, Track, TrackStatus};
use rand::seq::SliceRandom;
use tokio::{
    select,
    sync::{
        mpsc,
        watch::{self, Receiver, Sender},
    },
};

use crate::{
    ExitReceiver, PositionReceiver, Result, Status, StatusReceiver, TracklistReceiver,
    VolumeReceiver,
    controls::{ControlCommand, Controls},
    database::Database,
    downloader::Downloader,
    notification::{Notification, NotificationBroadcast},
    sink::QueryTrackResult,
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
    position: Sender<Duration>,
    track_finished: Receiver<()>,
    done_buffering: Receiver<PathBuf>,
    controls_rx: mpsc::UnboundedReceiver<ControlCommand>,
    controls: Controls,
    database: Arc<Database>,
    next_track_is_queried: bool,
    next_track_in_sink_queue: bool,
    downloader: Downloader,
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
        let sink = Sink::new(volume_receiver)?;

        let downloader = Downloader::new(audio_cache_dir, broadcast.clone(), database.clone());

        let track_finished = sink.track_finished();
        let done_buffering = downloader.done_buffering();

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
            position,
            track_finished,
            done_buffering,
            database,
            next_track_in_sink_queue: false,
            next_track_is_queried: false,
            downloader,
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

    async fn play(&mut self) -> Result<()> {
        tracing::info!("Play");

        let track = self.tracklist_rx.borrow().current_track().cloned();

        if self.sink.is_empty()
            && let Some(current_track) = track
        {
            tracing::info!("Sink is empty. Query track from play");
            self.set_target_status(Status::Buffering);
            self.query_track(&current_track, false).await?;
        } else {
            self.set_target_status(Status::Playing);
            self.sink.play();
        }

        Ok(())
    }

    fn pause(&mut self) {
        self.set_target_status(Status::Paused);
        self.sink.pause();
    }

    fn set_target_status(&self, status: Status) {
        self.target_status.send(status).expect("infallible");
    }

    async fn query_track(&mut self, track: &Track, next_track: bool) -> Result<()> {
        tracing::info!(
            "Querying {} track: {}",
            if next_track { "next" } else { "current" },
            &track.title
        );

        if next_track {
            self.next_track_is_queried = true;
        }

        let track_url = self.client.track_url(track.id).await?;
        if let Some(track_path) = self
            .downloader
            .ensure_track_is_downloaded(track_url, track)
            .await
        {
            let query_result = self.sink.query_track(&track_path)?;

            if next_track {
                self.next_track_in_sink_queue = match query_result {
                    QueryTrackResult::Queued => {
                        tracing::info!("In queue");
                        true
                    }
                    QueryTrackResult::RecreateStreamRequired => {
                        tracing::info!("Not in queue");
                        false
                    }
                };
            }
            self.sink.play();
            self.set_target_status(Status::Playing);
        } else {
            tracing::info!("Buffering track: {}", &track.title);
            self.set_target_status(Status::Buffering);
        }

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
        self.sink.seek(duration)?;
        self.position.send(self.sink.position())?;
        Ok(())
    }

    fn jump_forward(&mut self) -> Result<()> {
        let duration = self
            .tracklist_rx
            .borrow()
            .current_track()
            .map(|x| Duration::from_secs(x.duration_seconds as u64));

        if let Some(duration) = duration {
            let ten_seconds = Duration::from_secs(10);
            let next_position = self.sink.position() + ten_seconds;

            if next_position < duration {
                self.seek(next_position)?;
            } else {
                self.seek(duration)?;
            }
        }

        Ok(())
    }

    fn jump_backward(&mut self) -> Result<()> {
        let current_position = self.sink.position();

        if current_position.as_millis() < 10000 {
            self.seek(Duration::default())?;
        } else {
            let ten_seconds = Duration::from_secs(10);
            let seek_position = current_position - ten_seconds;

            self.seek(seek_position)?;
        }
        Ok(())
    }

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

        self.position.send(Default::default())?;

        if tracklist.skip_to_track(new_position).is_some() {
            self.new_queue(tracklist).await?;
        } else {
            tracklist.reset();
            self.sink.clear()?;
            self.next_track_is_queried = false;
            self.set_target_status(Status::Paused);
            self.position.send(Default::default())?;
            self.broadcast_tracklist(tracklist).await?;
        }

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
        self.sink.clear()?;
        self.next_track_is_queried = false;
        self.next_track_in_sink_queue = false;

        if let Some(first_track) = tracklist.current_track() {
            tracing::info!("New queue starting with: {}", first_track.title);
            self.query_track(first_track, false).await?;
        }

        self.broadcast_tracklist(tracklist).await?;

        Ok(())
    }

    async fn update_queue(&mut self, tracklist: Tracklist) -> Result<()> {
        self.next_track_is_queried = false;
        self.sink.clear_queue()?;
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

    async fn play_album(&mut self, album_id: &str, index: usize) -> Result<()> {
        let album: Album = self.client.album(album_id).await?;

        let unstreamable_tracks_to_index = album
            .tracks
            .iter()
            .take(index)
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

        tracklist.skip_to_track(index as i32 - unstreamable_tracks_to_index);
        self.new_queue(tracklist).await
    }

    async fn play_top_tracks(&mut self, artist_id: u32, index: usize) -> Result<()> {
        let artist = self.client.artist_page(artist_id).await?;
        let tracks = artist.top_tracks;
        let unstreamable_tracks_to_index =
            tracks.iter().take(index).filter(|t| !t.available).count() as i32;

        let mut tracklist = Tracklist {
            queue: tracks.into_iter().filter(|t| t.available).collect(),
            list_type: TracklistType::TopTracks(tracklist::TopTracklist {
                artist_name: artist.name,
                id: artist_id,
                image: artist.image,
            }),
        };

        tracklist.skip_to_track(index as i32 - unstreamable_tracks_to_index);
        self.new_queue(tracklist).await
    }

    async fn play_playlist(&mut self, playlist_id: u32, index: usize, shuffle: bool) -> Result<()> {
        let playlist = self.client.playlist(playlist_id).await?;

        let unstreamable_tracks_to_index = playlist
            .tracks
            .iter()
            .take(index)
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

        tracklist.skip_to_track(index as i32 - unstreamable_tracks_to_index);
        self.new_queue(tracklist).await
    }

    async fn remove_index_from_queue(&mut self, index: usize) -> Result<()> {
        let mut tracklist = self.tracklist_rx.borrow().clone();

        tracklist.queue.remove(index);
        self.update_queue(tracklist).await?;
        let notification = Notification::Info("Queue updated".into());
        self.broadcast.send(notification);
        Ok(())
    }

    async fn add_track_to_queue(&mut self, id: u32) -> Result<()> {
        let mut tracklist = self.tracklist_rx.borrow().clone();
        let track = self.client.track(id).await?;

        let notification = Notification::Info(format!("{} added to queue", track.title.clone()));

        tracklist.queue.push(track);
        self.update_queue(tracklist).await?;
        self.broadcast.send(notification);
        Ok(())
    }

    async fn play_track_next(&mut self, id: u32) -> Result<()> {
        let mut tracklist = self.tracklist_rx.borrow().clone();
        let track = self.client.track(id).await?;

        let notification = Notification::Info(format!("{} playing next", track.title.clone()));

        let current_index = tracklist.current_position();
        tracklist.queue.insert(current_index + 1, track);
        self.update_queue(tracklist).await?;
        self.broadcast.send(notification);
        Ok(())
    }

    async fn reorder_queue(&mut self, new_order: Vec<usize>) -> Result<()> {
        if new_order.iter().enumerate().all(|(i, &v)| i == v) {
            return Ok(());
        }

        let mut tracklist = self.tracklist_rx.borrow().clone();

        let reordered: Vec<_> = new_order
            .iter()
            .map(|&i| tracklist.queue[i].clone())
            .collect();

        tracklist.queue = reordered;

        self.update_queue(tracklist).await?;
        let notification = Notification::Info("Queue updated".into());
        self.broadcast.send(notification);
        Ok(())
    }

    async fn tick(&mut self) -> Result<()> {
        if *self.target_status.borrow() != Status::Playing {
            return Ok(());
        }

        let position = self.sink.position();
        self.position.send(position)?;

        let duration = self
            .tracklist_rx
            .borrow()
            .current_track()
            .map(|x| x.duration_seconds);

        if let Some(duration) = duration {
            let position = position.as_secs();

            let track_about_to_finish = (duration as i16 - position as i16) < 60;

            if track_about_to_finish && !self.next_track_is_queried {
                tracing::info!("Track about to finish");

                let tracklist = self.tracklist_rx.borrow().clone();

                if let Some(next_track) = tracklist.next_track() {
                    tracing::info!("Query next track: {} from tick", &next_track.title);
                    self.query_track(next_track, true).await?;
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
                self.seek(time)?;
            }
            ControlCommand::SetVolume { volume } => {
                self.set_volume(volume).await?;
            }
            ControlCommand::AddTrackToQueue { id } => self.add_track_to_queue(id).await?,
            ControlCommand::RemoveIndexFromQueue { index } => {
                self.remove_index_from_queue(index).await?
            }
            ControlCommand::PlayTrackNext { id } => self.play_track_next(id).await?,
            ControlCommand::ReorderQueue { new_order } => self.reorder_queue(new_order).await?,
        }
        Ok(())
    }

    async fn track_finished(&mut self) -> Result<()> {
        let mut tracklist = self.tracklist_rx.borrow().clone();

        let current_position = tracklist.current_position();
        let new_position = current_position + 1;

        let next_track = tracklist.skip_to_track(new_position as i32);

        match next_track {
            Some(next_track) => {
                if !self.next_track_in_sink_queue {
                    tracing::info!(
                        "Track finished and next track is not in queue. Resetting queue"
                    );
                    self.sink.clear()?;
                    self.query_track(next_track, false).await?;
                }
            }
            None => {
                tracklist.reset();
                self.set_target_status(Status::Paused);
                self.sink.pause();
                self.sink.clear()?;
                self.position.send(Default::default())?;
            }
        }
        self.next_track_is_queried = false;
        self.broadcast_tracklist(tracklist).await?;
        Ok(())
    }

    fn done_buffering(&mut self, path: PathBuf) -> Result<()> {
        if *self.target_status.borrow() != Status::Playing {
            self.set_target_status(Status::Playing);
        }

        tracing::info!("Done buffering track: {}", path.to_string_lossy());

        self.next_track_in_sink_queue = match self.sink.query_track(&path)? {
            QueryTrackResult::Queued => true,
            QueryTrackResult::RecreateStreamRequired => false,
        };
        Ok(())
    }

    pub async fn player_loop(&mut self, mut exit_receiver: ExitReceiver) -> Result<()> {
        let mut interval = tokio::time::interval(Duration::from_millis(INTERVAL_MS));

        loop {
            select! {
                _ = interval.tick() => {
                    if let Err(err) = self.tick().await {
                        self.broadcast.send_error(err.to_string());
                    };
                }

                Some(notification) = self.controls_rx.recv() => {
                    if let Err(err) = self.handle_message(notification).await {
                        self.broadcast.send_error(err.to_string());
                    };
                }

                Ok(_) = self.track_finished.changed() => {
                    if let Err(err) = self.track_finished().await {
                        self.broadcast.send_error(err.to_string());
                    };
                }

                Ok(_) = self.done_buffering.changed() => {
                    let path = self.done_buffering.borrow_and_update().clone();
                    if let Err(err) = self.done_buffering(path) {
                        self.broadcast.send_error(err.to_string());
                    };
                }
                Ok(exit) = exit_receiver.recv() => {
                    if exit {
                        break Ok(());
                    }
                }
            }
        }
    }
}
