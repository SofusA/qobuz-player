use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

use crate::{database::Database, notification::NotificationBroadcast};
use qobuz_player_client::qobuz_models::TrackURL;
use qobuz_player_models::Track;
use tokio::{
    sync::watch::{self, Receiver, Sender},
    task::JoinHandle,
};

/// min bytes to buffer before signaling playback can start
/// could be tweaked more than 1Mb, it's purely arbitrary for testing
const INITIAL_BUFFER_BYTES: u64 = 1_024 * 1_024;

/// shared state between the download task and the StreamingFile reader
pub struct StreamState {
    pub download_complete: AtomicBool,
    pub total_size: AtomicU64,
}

/// send through the watch channel when enough data is buffered for playback
#[derive(Clone)]
pub struct BufferReady {
    pub path: PathBuf,
    pub state: Arc<StreamState>,
}

impl Default for BufferReady {
    fn default() -> Self {
        Self {
            path: PathBuf::default(),
            state: Arc::new(StreamState {
                download_complete: AtomicBool::new(true),
                total_size: AtomicU64::new(0),
            }),
        }
    }
}

pub struct Downloader {
    audio_cache_dir: PathBuf,
    database: Arc<Database>,
    broadcast: Arc<NotificationBroadcast>,
    done_buffering_tx: Sender<BufferReady>,
    download_handle: Option<JoinHandle<()>>,
    /// stream state for the current download used to unblock readers on abort
    current_stream_state: Option<Arc<StreamState>>,
}

impl Downloader {
    pub fn new(
        audio_cache_dir: PathBuf,
        broadcast: Arc<NotificationBroadcast>,
        database: Arc<Database>,
    ) -> Self {
        let (done_buffering_tx, _) = watch::channel(Default::default());

        Self {
            audio_cache_dir,
            done_buffering_tx,
            database,
            broadcast,
            download_handle: None,
            current_stream_state: None,
        }
    }

    pub fn done_buffering(&self) -> Receiver<BufferReady> {
        self.done_buffering_tx.subscribe()
    }

    pub async fn ensure_track_is_downloaded(
        &mut self,
        track_url: TrackURL,
        track: &Track,
    ) -> Option<PathBuf> {
        if let Some(handle) = &self.download_handle {
            // unblock any active StreamingFile reader before aborting
            if let Some(state) = &self.current_stream_state {
                state.download_complete.store(true, Ordering::Release);
            }
            handle.abort();
            self.download_handle = None;
            self.current_stream_state = None;
        };

        let cache_path = cache_path(track, &track_url.mime_type, &self.audio_cache_dir);
        self.database.set_cache_entry(cache_path.as_path()).await;

        if cache_path.exists() {
            return Some(cache_path);
        }

        let done_buffering = self.done_buffering_tx.clone();
        let broadcast = self.broadcast.clone();

        let stream_state = Arc::new(StreamState {
            download_complete: AtomicBool::new(false),
            total_size: AtomicU64::new(0),
        });
        self.current_stream_state = Some(stream_state.clone());

        tracing::info!("Downloading: {}", track.title);
        let handle = tokio::spawn(async move {
            let Ok(resp) = reqwest::get(&track_url.url).await else {
                broadcast.send_error("Unable to get track audio file".to_string());
                stream_state
                    .download_complete
                    .store(true, Ordering::Release);
                return;
            };

            let total_size = resp.content_length().unwrap_or(u64::MAX);
            stream_state.total_size.store(total_size, Ordering::Release);

            if let Some(parent) = cache_path.parent()
                && let Err(e) = fs::create_dir_all(parent)
            {
                broadcast.send_error(format!("Unable to create cache directory: {e}"));
                stream_state
                    .download_complete
                    .store(true, Ordering::Release);
                return;
            }

            let tmp = cache_path.with_extension("partial");
            let Ok(mut file) = fs::File::create(&tmp) else {
                broadcast.send_error("Unable to create cache temp file".to_string());
                stream_state
                    .download_complete
                    .store(true, Ordering::Release);
                return;
            };

            let mut downloaded: u64 = 0;
            let mut signaled = false;
            let mut resp = resp;

            loop {
                match resp.chunk().await {
                    Ok(Some(chunk)) => {
                        if let Err(e) = file.write_all(&chunk) {
                            broadcast.send_error(format!("Unable to write cache temp file: {e}"));
                            stream_state
                                .download_complete
                                .store(true, Ordering::Release);
                            return;
                        }

                        downloaded += chunk.len() as u64;

                        if !signaled && downloaded >= INITIAL_BUFFER_BYTES {
                            tracing::info!(
                                "Initial buffer ready ({downloaded} bytes), starting playback"
                            );
                            let _ = done_buffering.send(BufferReady {
                                path: tmp.clone(),
                                state: stream_state.clone(),
                            });
                            signaled = true;
                        }
                    }
                    Ok(None) => break, // download complete jump
                    Err(e) => {
                        broadcast.send_error(format!("Unable to get audio file bytes: {e}"));
                        stream_state
                            .download_complete
                            .store(true, Ordering::Release);
                        return;
                    }
                }
            }

            // flush to ensure all data is visible to readers
            let _ = file.flush();
            drop(file);

            // mark download complete before rename so StreamingFile sees all data
            stream_state
                .download_complete
                .store(true, Ordering::Release);

            if let Err(e) = fs::rename(&tmp, &cache_path) {
                let _ = fs::remove_file(&tmp);
                broadcast.send_error(format!("Unable to finalize cache file: {e}"));
            }

            // signal for small files that completed before reaching the threshold
            if !signaled {
                let _ = done_buffering.send(BufferReady {
                    path: cache_path,
                    state: stream_state,
                });
            }
        });

        self.download_handle = Some(handle);
        None
    }
}

fn cache_path(track: &Track, mime: &str, audio_cache_dir: &Path) -> PathBuf {
    let artist_name = track.artist_name.as_deref().unwrap_or("unknown");
    let artist_id = track
        .artist_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let album_title = track.album_title.as_deref().unwrap_or("unknown");
    let album_id = track.album_id.as_deref().unwrap_or("unknown");
    let track_title = &track.title;

    let artist_dir = format!(
        "{} ({})",
        sanitize_name(artist_name),
        sanitize_name(&artist_id),
    );
    let album_dir = format!(
        "{} ({})",
        sanitize_name(album_title),
        sanitize_name(album_id),
    );
    let extension = guess_extension(mime);
    let track_file = format!(
        "{}_{}.{extension}",
        track.number,
        sanitize_name(track_title)
    );

    audio_cache_dir
        .join(artist_dir)
        .join(album_dir)
        .join(track_file)
}

fn sanitize_name(input: &str) -> String {
    let mut s: String = input
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '_',
            c if c.is_control() => '_',
            _ => c,
        })
        .collect();

    s = s.trim_matches([' ', '.']).to_string();

    let mut out = String::with_capacity(s.len());
    let mut prev_underscore = false;
    for ch in s.chars() {
        let ch2 = if ch == ' ' { '_' } else { ch };
        if ch2 == '_' {
            if prev_underscore {
                continue;
            }
            prev_underscore = true;
        } else {
            prev_underscore = false;
        }
        out.push(ch2);
    }

    if out.is_empty() {
        return "unknown".to_string();
    }

    const MAX: usize = 100;
    out.chars().take(MAX).collect()
}

fn guess_extension(mime: &str) -> String {
    match mime {
        m if m.contains("mp4") => "mp4".to_string(),
        m if m.contains("mp3") => "mp3".to_string(),
        m if m.contains("flac") => "flac".to_string(),
        _ => "unknown".to_string(),
    }
}
