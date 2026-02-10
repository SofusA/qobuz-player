use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use rodio::Source;
use rodio::cpal::traits::HostTrait;
use rodio::{decoder::DecoderBuilder, queue::queue};
use tokio::sync::watch::{self, Receiver, Sender};
use tokio::task::JoinHandle;
use tokio::time::sleep;

use crate::error::Error;
use crate::{AppResult, VolumeReceiver};

pub struct Sink {
    output_stream: Option<rodio::OutputStream>,
    sink: Option<rodio::Sink>,
    sender: Option<Arc<rodio::queue::SourcesQueueInput>>,
    volume: VolumeReceiver,
    track_finished: Sender<()>,
    track_handle: Option<JoinHandle<()>>,
    duration_played: Arc<Mutex<Duration>>,
}

impl Sink {
    pub fn new(volume: VolumeReceiver) -> AppResult<Self> {
        let (track_finished, _) = watch::channel(());
        Ok(Self {
            sink: Default::default(),
            output_stream: Default::default(),
            sender: Default::default(),
            volume,
            track_finished,
            track_handle: Default::default(),
            duration_played: Default::default(),
        })
    }

    pub fn track_finished(&self) -> Receiver<()> {
        self.track_finished.subscribe()
    }

    pub fn position(&self) -> Duration {
        let position = self
            .sink
            .as_ref()
            .map(|sink| sink.get_pos())
            .unwrap_or_default();

        let duration_played = *self.duration_played.lock();

        if position < duration_played {
            return Default::default();
        }

        position - duration_played
    }

    pub fn play(&self) {
        if let Some(sink) = &self.sink {
            sink.play();
        }
    }

    pub fn pause(&self) {
        if let Some(sink) = &self.sink {
            sink.pause();
        }
    }

    pub fn seek(&self, duration: Duration) -> AppResult<()> {
        if let Some(sink) = &self.sink {
            match sink.try_seek(duration) {
                Ok(_) => {
                    *self.duration_played.lock() = Default::default();
                }
                Err(err) => return Err(err.into()),
            };
        }

        Ok(())
    }

    pub fn clear(&mut self) -> AppResult<()> {
        tracing::info!("Clearing sink");
        self.clear_queue()?;
        self.sink = None;
        self.sender = None;
        self.output_stream = None;
        *self.duration_played.lock() = Default::default();

        if let Some(handle) = self.track_handle.take() {
            handle.abort();
        }

        Ok(())
    }

    pub fn clear_queue(&mut self) -> AppResult<()> {
        tracing::info!("Clearing sink queue");
        *self.duration_played.lock() = Default::default();

        if let Some(sender) = self.sender.as_ref() {
            sender.clear();
        };
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.sink.is_none()
    }

    pub fn query_track(&mut self, track_path: &Path) -> AppResult<QueryTrackResult> {
        tracing::info!("Sink query track: {}", track_path.to_string_lossy());

        let file = fs::File::open(track_path).map_err(|err| Error::StreamError {
            message: format!("Failed to read file: {track_path:?}: {err}"),
        })?;
        let source = DecoderBuilder::new()
            .with_data(file)
            .with_seekable(true)
            .build()?;

        let sample_rate = source.sample_rate();
        let same_sample_rate = self
            .output_stream
            .as_ref()
            .map(|stream| stream.config().sample_rate() == sample_rate)
            .unwrap_or(true);

        if !same_sample_rate {
            return Ok(QueryTrackResult::RecreateStreamRequired);
        }

        let needs_stream =
            self.output_stream.is_none() || self.sink.is_none() || self.sender.is_none();

        if needs_stream {
            let mut stream_handle = open_default_stream(sample_rate)?;
            stream_handle.log_on_drop(false);

            let (sender, receiver) = queue(true);
            let sink = rodio::Sink::connect_new(stream_handle.mixer());
            sink.append(receiver);
            set_volume(&sink, &self.volume.borrow());

            self.sink = Some(sink);
            self.sender = Some(sender);
            self.output_stream = Some(stream_handle);
        }

        let track_finished = self.track_finished.clone();
        let track_duration = source.total_duration().unwrap_or_default();

        let duration_played = self.duration_played.clone();
        let signal = self.sender.as_ref().unwrap().append_with_signal(source);

        let track_handle = tokio::spawn(async move {
            loop {
                if signal.try_recv().is_ok() {
                    *duration_played.lock() += track_duration;
                    track_finished.send(()).expect("infallible");
                    break;
                }
                sleep(Duration::from_millis(200)).await;
            }
        });

        self.track_handle = Some(track_handle);

        Ok(QueryTrackResult::Queued)
    }

    pub fn sync_volume(&self) {
        if let Some(sink) = &self.sink {
            set_volume(sink, &self.volume.borrow());
        }
    }
}

fn set_volume(sink: &rodio::Sink, volume: &f32) {
    let volume = volume.clamp(0.0, 1.0).powi(3);
    sink.set_volume(volume);
}

fn open_default_stream(sample_rate: u32) -> AppResult<rodio::OutputStream> {
    rodio::OutputStreamBuilder::from_default_device()
        .and_then(|x| x.with_sample_rate(sample_rate).open_stream())
        .or_else(|original_err| {
            let mut devices = rodio::cpal::default_host().output_devices()?;

            Ok(devices
                .find_map(|d| {
                    rodio::OutputStreamBuilder::from_device(d)
                        .and_then(|x| x.with_sample_rate(sample_rate).open_stream_or_fallback())
                        .ok()
                })
                .ok_or(original_err)?)
        })
}

pub enum QueryTrackResult {
    Queued,
    RecreateStreamRequired,
}

impl Drop for Sink {
    fn drop(&mut self) {
        self.clear().unwrap();
    }
}
