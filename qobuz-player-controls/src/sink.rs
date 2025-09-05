use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;

use crate::{Broadcast, Time, error::Error};
use rodio::{Source, decoder::DecoderBuilder, queue::queue};
use tokio::sync::RwLock;

pub struct Sink {
    stream_handle: rodio::OutputStream,
    sink: rodio::Sink,
    sender: Arc<rodio::queue::SourcesQueueInput>,
    broadcast: Arc<Broadcast>,
    duration_played: Arc<RwLock<Duration>>,
}

type Result<T, E = Error> = std::result::Result<T, E>;

impl Sink {
    pub fn new(broadcast: Arc<Broadcast>) -> Result<Self> {
        let stream_handle = rodio::OutputStreamBuilder::open_default_stream()?;
        let (sender, receiver) = queue(true);

        let sink = rodio::Sink::connect_new(stream_handle.mixer());
        sink.append(receiver);

        Ok(Self {
            sink,
            stream_handle,
            sender,
            broadcast,
            duration_played: Default::default(),
        })
    }

    pub async fn clear(&mut self) -> Result<()> {
        let (sender, receiver) = queue(true);
        let sink = rodio::Sink::connect_new(self.stream_handle.mixer());
        sink.append(receiver);

        self.sink = sink;
        self.sender = sender;

        let mut duration_played = self.duration_played.write().await;
        *duration_played = Duration::default();

        Ok(())
    }

    pub fn play(&self) {
        self.sink.play();
    }

    pub fn pause(&self) {
        self.sink.pause();
    }

    pub fn seek(&self, time: Time) -> Result<()> {
        let duration = Duration::from_millis(time.mseconds());
        self.sink.try_seek(duration)?;
        Ok(())
    }

    pub async fn position(&self) -> Time {
        let position = self.sink.get_pos();
        let duration_played = self.duration_played.read().await;
        let current_track_position = position - *duration_played;
        Time::from_mseconds(current_track_position.as_millis() as u64)
    }

    pub fn is_playing(&self) -> bool {
        !self.sink.is_paused()
    }

    pub async fn query_track_url(&self, track_url: &str) -> Result<()> {
        let track_url = track_url.to_string();
        let sender = self.sender.clone();
        let broadcast = self.broadcast.clone();
        let duration_played = self.duration_played.clone();

        // TODO: Cancelation signal. Cancel download when new download starts and when dropping
        // Cancel thread or request?
        tokio::spawn(async move {
            let resp = reqwest::get(track_url).await.unwrap();
            let cursor = Cursor::new(resp.bytes().await.unwrap());

            let source = DecoderBuilder::new()
                .with_data(cursor)
                .with_seekable(true)
                .build()
                .unwrap();

            let source_duration = source.total_duration();

            let signal = sender.append_with_signal(source);

            if signal.iter().next().is_some() {
                if let Some(source_duration) = source_duration {
                    let mut duration_played = duration_played.write().await;
                    *duration_played += source_duration;
                }
                broadcast.track_finished();
            }
        });
        self.play();

        Ok(())
    }

    pub fn set_volume(&self, volume: f64) {
        let volume_pow = volume.clamp(0.0, 1.0).powi(3);
        self.sink.set_volume(volume_pow as f32);
    }

    pub fn volume(&self) -> f64 {
        self.sink.volume() as f64
    }
}

impl Drop for Sink {
    fn drop(&mut self) {
        self.stream_handle.log_on_drop(false);
        self.sink.clear();
    }
}
