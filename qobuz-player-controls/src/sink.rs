use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use rodio::Source;
use rodio::cpal::traits::HostTrait;
use rodio::{decoder::DecoderBuilder, queue::queue};
use tokio::sync::watch::{self, Receiver, Sender};

use crate::error::Error;
use crate::{Result, VolumeReceiver};

pub struct Sink {
    stream_handle: Option<rodio::OutputStream>,
    sink: Option<rodio::Sink>,
    sender: Option<Arc<rodio::queue::SourcesQueueInput>>,
    volume: VolumeReceiver,
    track_finished: Sender<()>,
    track_handle: Option<JoinHandle<()>>,
    duration_played: Arc<Mutex<Option<Duration>>>,
}

impl Sink {
    pub fn new(volume: VolumeReceiver) -> Result<Self> {
        let (track_finished, _) = watch::channel(());
        Ok(Self {
            sink: Default::default(),
            stream_handle: Default::default(),
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

        let duration_played = self
            .duration_played
            .lock()
            .map(|lock| lock.unwrap_or_default())
            .unwrap_or_default();

        if position < duration_played {
            return Default::default();
        }

        position - duration_played
    }

    pub async fn clear(&mut self) -> Result<()> {
        self.clear_queue()?;
        self.sink = None;
        self.sender = None;
        self.stream_handle = None;
        self.track_handle = None;
        *self.duration_played.lock()? = None;

        Ok(())
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

    pub fn seek(&self, duration: Duration) -> Result<()> {
        if let Some(sink) = &self.sink {
            match sink.try_seek(duration) {
                Ok(_) => {
                    *self.duration_played.lock()? = None;
                }
                Err(err) => return Err(err.into()),
            };
        }

        Ok(())
    }

    pub fn clear_queue(&mut self) -> Result<()> {
        self.track_handle = None;
        *self.duration_played.lock()? = None;

        if let Some(sender) = self.sender.as_ref() {
            sender.clear();
        };
        Ok(())
    }

    pub fn query_track(&mut self, track_url: &Path) -> Result<QueryTrackResult> {
        let bytes = fs::read(track_url).map_err(|_| Error::StreamError {
            message: "File not found".into(),
        })?;

        let cursor = Cursor::new(bytes);
        let source = DecoderBuilder::new()
            .with_data(cursor)
            .with_seekable(true)
            .build()
            .map_err(|_| Error::StreamError {
                message: "Unable to decode audio file".into(),
            })?;

        let sample_rate = source.sample_rate();

        if self.stream_handle.is_none() || self.sink.is_none() || self.sender.is_none() {
            let mut stream_handle = open_default_stream(sample_rate)?;
            stream_handle.log_on_drop(false);

            let (sender, receiver) = queue(true);
            let sink = rodio::Sink::connect_new(stream_handle.mixer());
            sink.append(receiver);
            set_volume(&sink, &self.volume.borrow());
            self.sink = Some(sink);
            self.sender = Some(sender);
            self.stream_handle = Some(stream_handle);
        }

        let track_finished = self.track_finished.clone();

        let track_duration = {
            let previously_played = self.duration_played.lock()?.unwrap_or_default();

            source
                .total_duration()
                .map(|duration| duration + previously_played)
        };

        let duration_played = self.duration_played.clone();
        let signal = self.sender.as_ref().unwrap().append_with_signal(source);

        let track_handle = std::thread::spawn(move || {
            if signal.recv().is_ok() {
                if let Ok(mut duration_played) = duration_played.lock() {
                    *duration_played = track_duration;
                };
                track_finished.send(()).expect("infallible");
            }
        });

        self.track_handle = Some(track_handle);

        let same_sample_rate =
            sample_rate == self.stream_handle.as_ref().unwrap().config().sample_rate();

        Ok(match same_sample_rate {
            true => QueryTrackResult::Queued,
            false => QueryTrackResult::NotQueued,
        })
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

fn open_default_stream(sample_rate: u32) -> Result<rodio::OutputStream> {
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
    NotQueued,
}
