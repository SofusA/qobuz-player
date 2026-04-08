use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use stream_download::{Settings, StreamDownload, storage::temp::TempStorageProvider};

use crate::{
    AppResult, cmaf, crypto,
    database::Database,
    error::Error,
    flac_source_stream::{
        FlacSourceParams, FlacSourceStream, SeekableStreamReader, SegmentByteInfo,
    },
    models::Track,
    notification::NotificationBroadcast,
};

use qobuz_player_client::qobuz_models::TrackURL;

pub enum DownloadResult {
    Cached(PathBuf),
    Streaming(SeekableStreamReader),
}

pub struct Downloader {
    audio_cache_dir: PathBuf,
    database: Arc<Database>,
    broadcast: Arc<NotificationBroadcast>,
}

impl Downloader {
    pub fn new(
        audio_cache_dir: PathBuf,
        broadcast: Arc<NotificationBroadcast>,
        database: Arc<Database>,
    ) -> Self {
        Self {
            audio_cache_dir,
            database,
            broadcast,
        }
    }

    pub async fn ensure_track_is_downloaded(
        &mut self,
        track_url: TrackURL,
        session_infos: Option<&str>,
        track: &Track,
    ) -> AppResult<DownloadResult> {
        let cache_path = cache_path(track, &track_url.mime_type, &self.audio_cache_dir);
        self.database.set_cache_entry(cache_path.as_path()).await;

        if cache_path.exists() {
            tracing::info!("Playing from cache: {}", cache_path.display());
            return Ok(DownloadResult::Cached(cache_path));
        }

        let n_segments = track_url.n_segments;

        tracing::info!("Streaming: {} ({n_segments} segments)", track.title);

        let content_key = match (&track_url.key, session_infos) {
            (Some(key_str), Some(infos)) => {
                let session_key = crypto::derive_session_key(infos)?;
                let content_key = crypto::unwrap_content_key(&session_key, key_str)?;
                tracing::debug!("Derived content key for key_id: {:?}", track_url.key_id);
                Some(content_key)
            }
            _ => {
                tracing::warn!("No encryption key available");
                None
            }
        };

        let seg0_url = track_url.url_template.replace("$SEGMENT$", "0");
        let init_bytes = fetch_segment(&seg0_url, 0).await?;
        let init_info = cmaf::parse_init_segment(&init_bytes)?;

        tracing::info!(
            "Init segment: {} bytes, FLAC header: {} bytes",
            init_bytes.len(),
            init_info.flac_header.len(),
        );

        // Segment table may list more audio segments than the API's n_segments-1.
        let audio_segments = init_info.segment_table.len() as u8;
        if audio_segments == 0 {
            return Err(Error::StreamError {
                message: "Track has no audio segments".to_string(),
            });
        }

        let flac_header_len = init_info.flac_header.len() as u64;
        let mut segment_map = Vec::new();
        let mut cumulative_offset: u64 = 0;
        for entry in &init_info.segment_table {
            segment_map.push(SegmentByteInfo {
                byte_offset: cumulative_offset,
                byte_len: entry.byte_len as u64,
            });
            cumulative_offset += entry.byte_len as u64;
        }
        let total_byte_len = flac_header_len + cumulative_offset;

        let n_segments_to_download = audio_segments + 1; // +1 for init segment

        tracing::info!(
            "Segment map: {} audio segments, total FLAC size: {} bytes",
            audio_segments,
            total_byte_len,
        );

        let params = FlacSourceParams {
            url_template: track_url.url_template,
            n_segments: n_segments_to_download,
            content_key,
            flac_header: init_info.flac_header,
            cache_path,
            broadcast: self.broadcast.clone(),
            segment_map: segment_map.clone(),
        };

        let reader = StreamDownload::new::<FlacSourceStream>(
            params,
            TempStorageProvider::default(),
            Settings::default().prefetch_bytes(4096),
        )
        .await
        .map_err(|e| Error::StreamError {
            message: format!("Failed to create stream: {e}"),
        })?;

        let seekable = SeekableStreamReader::new(reader, total_byte_len);

        Ok(DownloadResult::Streaming(seekable))
    }
}

async fn fetch_segment(url: &str, index: u8) -> AppResult<Vec<u8>> {
    let bytes = reqwest::get(url)
        .await
        .map_err(|e| Error::StreamError {
            message: format!("Failed to fetch segment {index}: {e}"),
        })?
        .bytes()
        .await
        .map_err(|e| Error::StreamError {
            message: format!("Failed to read segment {index} bytes: {e}"),
        })?;
    Ok(bytes.to_vec())
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
    let extension = if mime.contains("flac") || mime.contains("mp4") {
        "flac"
    } else {
        &guess_extension(mime)
    };
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
        m if m.contains("flac") => "flac".to_string(),
        m if m.contains("mpeg") => "mp3".to_string(),
        m if m.contains("mp3") => "mp3".to_string(),
        _ => "unknown".to_string(),
    }
}
