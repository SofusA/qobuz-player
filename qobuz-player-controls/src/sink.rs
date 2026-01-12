use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;
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
        })
    }

    pub fn track_finished(&self) -> Receiver<()> {
        self.track_finished.subscribe()
    }

    pub async fn clear(&mut self) -> Result<()> {
        self.sink = None;
        self.sender = None;
        self.stream_handle = None;
        self.track_handle = None;

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
            sink.try_seek(duration)?;
        }

        Ok(())
    }

    pub fn clear_queue(&mut self) {
        self.track_handle = None;

        if let Some(sender) = self.sender.as_ref() {
            sender.clear();
        }
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
        let signal = self.sender.as_ref().unwrap().append_with_signal(source);

        let track_handle = std::thread::spawn(move || {
            if signal.recv().is_ok() {
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
