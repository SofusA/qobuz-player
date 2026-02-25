use std::fs;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use rodio::cpal::traits::HostTrait;
use rodio::queue::queue;
use rodio::{Decoder, DeviceTrait, Source};
use tokio::sync::watch::{self, Receiver, Sender};
use tokio::task::JoinHandle;
use tokio::time::sleep;

use crate::downloader::StreamState;
use crate::error::Error;
use crate::stderr_redirect::silence_stderr;
use crate::{AppResult, VolumeReceiver};

/// just a file wrapper that blocks on EOF while download is still in progress
/// allows the decoder to read from a file that is being written to concurrently by the download task
struct StreamingFile {
    file: fs::File,
    state: Arc<StreamState>,
}

impl Read for StreamingFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            let n = self.file.read(buf)?;
            if n > 0 {
                return Ok(n);
            }
            // got 0, if the download is complete, this is real EOF.
            if self.state.download_complete.load(Ordering::Acquire) {
                return Ok(0);
            }
            // if download still in progress just wait and retry
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Seek for StreamingFile {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        match pos {
            // translate SeekFrom::End using the known total file sizze since the file may not be fully written yet
            SeekFrom::End(offset) => {
                let total = self.state.total_size.load(Ordering::Acquire);
                if total == u64::MAX {
                    // no content-length available, delegate to the file
                    return self.file.seek(pos);
                }
                let target = total as i64 + offset;
                self.file.seek(SeekFrom::Start(target.max(0) as u64))
            }
            other => self.file.seek(other),
        }
    }
}

pub struct Sink {
    sink: Option<rodio::Sink>,
    output_stream: Option<rodio::OutputStream>,
    sender: Option<Arc<rodio::queue::SourcesQueueInput>>,
    volume: VolumeReceiver,
    track_finished: Sender<()>,
    track_handle: Option<JoinHandle<()>>,
    duration_played: Arc<Mutex<Duration>>,
    preferred_device_id: Option<String>,
}

impl Sink {
    pub fn new(volume: VolumeReceiver, preferred_device_id: Option<String>) -> AppResult<Self> {
        let (track_finished, _) = watch::channel(());
        Ok(Self {
            sink: None,
            output_stream: None,
            sender: None,
            volume,
            track_finished,
            track_handle: Default::default(),
            duration_played: Default::default(),
            preferred_device_id,
        })
    }

    pub fn track_finished(&self) -> Receiver<()> {
        self.track_finished.subscribe()
    }

    pub fn position(&self) -> Duration {
        let position = self.sink.as_ref().map(|x| x.get_pos()).unwrap_or_default();

        let duration_played = *self.duration_played.lock();

        if position < duration_played {
            return Default::default();
        }

        position - duration_played
    }

    pub fn play(&self) {
        if let Some(player) = &self.sink {
            player.play();
        }
    }

    pub fn pause(&self) {
        if let Some(player) = &self.sink {
            player.pause();
        }
    }

    pub fn seek(&self, duration: Duration) -> AppResult<()> {
        if let Some(player) = &self.sink {
            match player.try_seek(duration) {
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
        self.output_stream = None;
        self.sender = None;

        *self.duration_played.lock() = Default::default();

        if let Some(handle) = self.track_handle.take() {
            handle.abort();
        }

        Ok(())
    }

    pub fn clear_queue(&mut self) -> AppResult<()> {
        tracing::info!("Clearing sink queue");
        *self.duration_played.lock() = Default::default();

        if let Some(player) = self.sink.as_ref() {
            player.clear();
        };
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.sink.is_none()
    }

    pub fn query_track(
        &mut self,
        track_path: &Path,
        stream_state: Option<Arc<StreamState>>,
    ) -> AppResult<QueryTrackResult> {
        tracing::info!("Sink query track: {}", track_path.to_string_lossy());

        let file = fs::File::open(track_path).map_err(|err| Error::StreamError {
            message: format!("Failed to read file: {track_path:?}: {err}"),
        })?;

        let state = stream_state.unwrap_or_else(|| {
            let metadata = file.metadata().ok();
            let size = metadata.map(|m| m.len()).unwrap_or(0);
            Arc::new(StreamState {
                download_complete: AtomicBool::new(true),
                total_size: AtomicU64::new(size),
            })
        });

        let streaming = StreamingFile { file, state };
        let source = Decoder::try_from(BufReader::new(streaming))?;

        let sample_rate = source.sample_rate();
        let same_sample_rate = self
            .output_stream
            .as_ref()
            .map(|mixer| mixer.config().sample_rate() == sample_rate)
            .unwrap_or(true);

        if !same_sample_rate {
            return Ok(QueryTrackResult::RecreateStreamRequired);
        }

        let needs_stream = self.output_stream.is_none() || self.sink.is_none();

        if needs_stream {
            let mut mixer = if let Some(preferred_device_name) = self.preferred_device_id.as_deref()
            {
                silence_stderr(|| open_preferred_stream(sample_rate, preferred_device_name))?
            } else {
                open_default_stream(sample_rate)?
            };
            mixer.log_on_drop(false);

            let (sender, receiver) = queue(true);
            let player = rodio::Sink::connect_new(mixer.mixer());
            player.append(receiver);
            set_volume(&player, &self.volume.borrow());

            self.sink = Some(player);
            self.sender = Some(sender);
            self.output_stream = Some(mixer);
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
        if let Some(player) = &self.sink {
            set_volume(player, &self.volume.borrow());
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

fn open_preferred_stream(
    sample_rate: u32,
    preferred_device_name: &str,
) -> AppResult<rodio::OutputStream> {
    let devices = rodio::cpal::default_host().output_devices()?;

    for device in devices {
        if device.name().map(|x| x.to_string()).ok().as_deref() == Some(preferred_device_name) {
            let Ok(stream) = rodio::OutputStreamBuilder::from_device(device)
                .and_then(|x| x.with_sample_rate(sample_rate).open_stream_or_fallback())
            else {
                break;
            };

            return Ok(stream);
        }
    }

    let devices = rodio::cpal::default_host().output_devices()?;
    let available_devices: Vec<String> = devices
        .flat_map(|x| x.name().map(|x| x.to_string()))
        .collect();
    let available_devices = available_devices.join(", ");

    Err(Error::SinkDeviceError {
        message: format!("Unable to find device. Available devices: {available_devices}"),
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
