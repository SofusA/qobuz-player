use async_trait::async_trait;
use qobuz_api::client::Image;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt::Debug};

#[async_trait]
pub trait MusicService: Send + Sync + Debug {
    async fn login(&self, username: &str, password: &str);
    async fn album(&self, album_id: &str) -> Option<Album>;
    async fn suggested_albums(&self, album_id: &str) -> Option<Vec<Album>>;
    async fn track(&self, track_id: i32) -> Option<Track>;
    async fn artist(&self, artist_id: i32) -> Option<Artist>;
    async fn artist_releases(&self, artist_id: i32) -> Option<Vec<Album>>;
    async fn similar_artists(&self, artist_id: i32) -> Vec<Artist>;
    async fn playlist(&self, playlist_id: i64) -> Option<Playlist>;
    async fn search(&self, query: &str) -> Option<SearchResults>;
    async fn track_url(&self, track_id: i32) -> Option<String>;
    async fn user_playlists(&self) -> Option<Vec<Playlist>>;
    async fn favorites(&self) -> Option<Favorites>;
    async fn add_favorite_album(&self, id: &str);
    async fn remove_favorite_album(&self, id: &str);
    async fn add_favorite_artist(&self, id: &str);
    async fn remove_favorite_artist(&self, id: &str);
    async fn add_favorite_playlist(&self, id: &str);
    async fn remove_favorite_playlist(&self, id: &str);
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrackStatus {
    Played,
    Playing,
    #[default]
    Unplayed,
    Unplayable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: u32,
    pub number: u32,
    pub title: String,
    pub album: Option<Album>,
    pub artist: Option<Artist>,
    pub duration_seconds: u32,
    pub explicit: bool,
    pub hires_available: bool,
    pub sampling_rate: f32,
    pub bit_depth: u32,
    pub status: TrackStatus,
    #[serde(skip)]
    pub track_url: Option<String>,
    pub available: bool,
    pub cover_art: Option<String>,
    pub position: u32,
    pub media_number: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    pub id: String,
    pub title: String,
    pub artist: Artist,
    pub release_year: u32,
    pub hires_available: bool,
    pub explicit: bool,
    pub total_tracks: u32,
    pub tracks: BTreeMap<u32, Track>,
    pub available: bool,
    pub cover_art: String,
    pub cover_art_small: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    pub query: String,
    pub albums: Vec<Album>,
    pub tracks: Vec<Track>,
    pub artists: Vec<Artist>,
    pub playlists: Vec<Playlist>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Favorites {
    pub albums: Vec<Album>,
    pub tracks: Vec<Track>,
    pub artists: Vec<Artist>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Artist {
    pub id: u32,
    pub name: String,
    pub image: Option<Image>,
    pub albums: Option<Vec<Album>>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub title: String,
    pub duration_seconds: u32,
    pub tracks_count: u32,
    pub id: u32,
    pub cover_art: Option<String>,
    pub tracks: BTreeMap<u32, Track>,
}
