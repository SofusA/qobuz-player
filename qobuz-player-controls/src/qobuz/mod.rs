use crate::{
    service::{Album, Artist, Favorites, MusicService, Playlist, SearchResults, Track},
    sql::db,
};
use async_trait::async_trait;
use qobuz_api::client::{
    album_suggestion::AlbumSuggestion,
    api::{self, Client as QobuzClient},
    favorites::Favorites as QobuzFavorites,
    release::{Release, Track as QobuzTrack},
    search_results::SearchAllResults,
};
use std::{collections::BTreeMap, str::FromStr};
use tracing::{debug, error, info};

pub type Result<T, E = qobuz_api::Error> = std::result::Result<T, E>;

pub mod album;
pub mod artist;
pub mod playlist;
pub mod track;

#[async_trait]
impl MusicService for QobuzClient {
    async fn login(&self, username: &str, password: &str) {
        self.login(username, password).await;
    }

    async fn album(&self, album_id: &str) -> Option<Album> {
        match self.album(album_id).await {
            Ok(album) => Some(album.into()),
            Err(err) => {
                error!("failed to get album: {}", err);
                None
            }
        }
    }

    async fn suggested_albums(&self, album_id: &str) -> Option<Vec<Album>> {
        match self.suggested_albums(album_id).await {
            Ok(album_suggestions) => Some(
                album_suggestions
                    .albums
                    .items
                    .into_iter()
                    .map(|x| x.into())
                    .collect(),
            ),
            Err(err) => {
                error!("failed to get album: {}", err);
                None
            }
        }
    }

    async fn track(&self, track_id: i32) -> Option<Track> {
        match self.track(track_id).await {
            Ok(track) => Some(track.into()),
            Err(_) => None,
        }
    }

    async fn artist(&self, artist_id: i32) -> Option<Artist> {
        match self.artist(artist_id, None).await {
            Ok(artist) => Some(artist.into()),
            Err(_) => None,
        }
    }

    async fn similar_artists(&self, artist_id: i32) -> Vec<Artist> {
        match self.similar_artists(artist_id, None).await {
            Ok(artists) => artists.items.into_iter().map(|x| x.into()).collect(),
            Err(err) => {
                error!("failed to get similar artists: {}", err);
                vec![]
            }
        }
    }

    async fn artist_releases(&self, artist_id: i32) -> Option<Vec<Album>> {
        match self.artist_releases(artist_id, None).await {
            Ok(artist_releases) => Some(artist_releases.into_iter().map(|x| x.into()).collect()),
            Err(_) => None,
        }
    }

    async fn playlist(&self, playlist_id: i64) -> Option<Playlist> {
        match self.playlist(playlist_id).await {
            Ok(playlist) => Some(playlist.into()),
            Err(_) => None,
        }
    }

    async fn search(&self, query: &str) -> Option<SearchResults> {
        match self.search_all(query, 20).await {
            Ok(results) => Some(results.into()),
            Err(_) => None,
        }
    }

    async fn favorites(&self) -> Option<Favorites> {
        match self.favorites(1000).await {
            Ok(results) => Some(results.into()),
            Err(_) => None,
        }
    }

    async fn add_favorite_album(&self, id: &str) {
        _ = self.add_favorite_album(id).await;
    }
    async fn remove_favorite_album(&self, id: &str) {
        _ = self.remove_favorite_album(id).await;
    }
    async fn add_favorite_artist(&self, id: &str) {
        _ = self.add_favorite_artist(id).await;
    }
    async fn remove_favorite_artist(&self, id: &str) {
        _ = self.remove_favorite_artist(id).await;
    }
    async fn add_favorite_playlist(&self, id: &str) {
        _ = self.add_favorite_playlist(id).await;
    }
    async fn remove_favorite_playlist(&self, id: &str) {
        _ = self.remove_favorite_playlist(id).await;
    }

    async fn track_url(&self, track_id: i32) -> Option<String> {
        match self.track_url(track_id, None).await {
            Ok(track_url) => Some(track_url.url),
            Err(_) => None,
        }
    }

    async fn user_playlists(&self) -> Option<Vec<Playlist>> {
        match self.user_playlists().await {
            Ok(up) => Some(
                up.playlists
                    .items
                    .into_iter()
                    .map(|p| p.into())
                    .collect::<Vec<Playlist>>(),
            ),
            Err(_) => None,
        }
    }
}

pub async fn make_client(username: Option<&str>, password: Option<&str>) -> Result<QobuzClient> {
    let mut client = api::new(None, None, None).await?;

    setup_client(&mut client, username, password).await
}

/// Setup app_id, secret and user credentials for authentication
pub async fn setup_client(
    client: &mut QobuzClient,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<QobuzClient> {
    info!("setting up the api client");

    if let Some(config) = db::get_config().await {
        let mut refresh_config = false;

        if let Some(app_id) = config.app_id {
            debug!("using app_id from cache");
            client.set_app_id(app_id);
        } else {
            debug!("app_id not found, will have to refresh config");
            refresh_config = true;
        }

        if let Some(secret) = config.active_secret {
            debug!("using active secret from cache");
            client.set_active_secret(secret);
        } else {
            debug!("active_secret not found, will have to refresh config");
            refresh_config = true;
        }

        if refresh_config {
            client.refresh().await?;

            if let Some(id) = client.get_app_id() {
                db::set_app_id(id).await;
            }

            if let Some(secret) = client.get_active_secret() {
                db::set_active_secret(secret).await;
            }
        }

        if let Some(token) = config.user_token {
            info!("using token from cache");
            client.set_token(token);
        } else {
            let (username, password): (Option<String>, Option<String>) =
                if let (Some(u), Some(p)) = (username, password) {
                    (Some(u.to_string()), Some(p.to_string()))
                } else if let (Some(u), Some(p)) = (config.username, config.password) {
                    (Some(u), Some(p))
                } else {
                    (None, None)
                };

            if let (Some(username), Some(password)) = (username, password) {
                info!("setting auth using username and password from cache");
                client.login(&username, &password).await?;
                client.test_secrets().await?;

                if let Some(token) = client.get_token() {
                    db::set_user_token(token).await;
                }

                if let Some(secret) = client.get_active_secret() {
                    db::set_active_secret(secret).await;
                }
            }
        }
    }

    Ok(client.clone())
}

impl From<SearchAllResults> for SearchResults {
    fn from(s: SearchAllResults) -> Self {
        Self {
            query: s.query,
            albums: s
                .albums
                .items
                .into_iter()
                .map(|a| a.into())
                .collect::<Vec<Album>>(),
            tracks: s
                .tracks
                .items
                .into_iter()
                .map(|t| t.into())
                .collect::<Vec<Track>>(),
            artists: s
                .artists
                .items
                .into_iter()
                .map(|a| a.into())
                .collect::<Vec<Artist>>(),
            playlists: s
                .playlists
                .items
                .into_iter()
                .map(|p| p.into())
                .collect::<Vec<Playlist>>(),
        }
    }
}

impl From<QobuzFavorites> for Favorites {
    fn from(s: QobuzFavorites) -> Self {
        Self {
            albums: s
                .albums
                .items
                .into_iter()
                .map(|a| a.into())
                .collect::<Vec<Album>>(),
            tracks: s
                .tracks
                .items
                .into_iter()
                .map(|t| t.into())
                .collect::<Vec<Track>>(),
            artists: s
                .artists
                .items
                .into_iter()
                .map(|a| a.into())
                .collect::<Vec<Artist>>(),
        }
    }
}

impl From<QobuzTrack> for Track {
    fn from(s: QobuzTrack) -> Self {
        Self {
            id: s.id,
            number: s.physical_support.track_number as u32,
            title: s.title,
            album: None,
            artist: Some(Artist {
                id: s.artist.id as u32,
                name: s.artist.name.display,
                ..Default::default()
            }),
            duration_seconds: s.duration as u32,
            explicit: s.parental_warning,
            hires_available: s.rights.streamable,
            sampling_rate: s.audio_info.maximum_sampling_rate,
            bit_depth: s.audio_info.maximum_bit_depth,
            status: crate::service::TrackStatus::Unplayed,
            track_url: None,
            available: s.rights.streamable,
            cover_art: None,
            position: s.physical_support.track_number as u32,
            media_number: s.physical_support.media_number as u32,
        }
    }
}

impl From<Release> for Album {
    fn from(s: Release) -> Self {
        let year = chrono::NaiveDate::from_str(&s.dates.original)
            .expect("failed to parse date")
            .format("%Y");

        let tracks = if let Some(tracks) = s.tracks {
            let mut position = 1_u32;

            tracks
                .items
                .into_iter()
                .filter_map(|t| {
                    if t.rights.streamable {
                        let mut track: Track = t.into();

                        let next_position = position;
                        track.position = next_position;

                        position += 1;

                        Some((next_position, track))
                    } else {
                        None
                    }
                })
                .collect::<BTreeMap<u32, Track>>()
        } else {
            BTreeMap::new()
        };

        Self {
            id: s.id,
            title: s.title,
            artist: Artist {
                id: s.artist.id as u32,
                name: s.artist.name.display,
                ..Default::default()
            },
            release_year: year
                .to_string()
                .parse::<u32>()
                .expect("error converting year"),
            hires_available: s.rights.hires_streamable,
            explicit: s.parental_warning,
            total_tracks: s.tracks_count as u32,
            tracks,
            available: s.rights.streamable,
            cover_art: s.image.large,
            cover_art_small: s.image.small,
        }
    }
}

impl From<AlbumSuggestion> for Album {
    fn from(s: AlbumSuggestion) -> Self {
        let year = chrono::NaiveDate::from_str(&s.dates.original)
            .expect("failed to parse date")
            .format("%Y");

        let tracks = BTreeMap::new();

        let artist = s.artists.and_then(|vec| vec.into_iter().next());
        let (artist_id, artist_name) = artist.map_or((0, "Unknown".into()), |artist| {
            (artist.id as u32, artist.name)
        });

        Self {
            id: s.id,
            title: s.title,
            artist: Artist {
                id: artist_id,
                name: artist_name,
                ..Default::default()
            },
            release_year: year
                .to_string()
                .parse::<u32>()
                .expect("error converting year"),
            hires_available: s.rights.hires_streamable,
            explicit: s.parental_warning,
            total_tracks: s.track_count as u32,
            tracks,
            available: s.rights.streamable,
            cover_art: s.image.large,
            cover_art_small: s.image.small,
        }
    }
}
