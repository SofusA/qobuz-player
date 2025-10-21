use moka::future::Cache;
use qobuz_player_client::{client::AudioQuality, qobuz_models::TrackURL};
use qobuz_player_models::{
    Album, AlbumSimple, Artist, ArtistPage, Favorites, Playlist, SearchResults, Track,
};
use std::sync::OnceLock;
use time::Duration;
use tokio::sync::Mutex;

use crate::{error::Error, simple_cache::SimpleCache};

type QobuzClient = qobuz_player_client::client::Client;
type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub struct Client {
    qobuz_client: OnceLock<QobuzClient>,
    username: String,
    password: String,
    max_audio_quality: AudioQuality,
    client_initiated: Mutex<bool>,
    favorites_cache: SimpleCache<Favorites>,
    featured_albums_cache: SimpleCache<Vec<(String, Vec<AlbumSimple>)>>,
    featured_playlists_cache: SimpleCache<Vec<(String, Vec<Playlist>)>>,
    album_cache: Cache<String, Album>,
    artist_cache: Cache<u32, ArtistPage>,
    artist_albums_cache: Cache<u32, Vec<AlbumSimple>>,
    playlist_cache: Cache<u32, Playlist>,
    similar_artists_cache: Cache<u32, Vec<Artist>>,
    suggested_albums_cache: Cache<String, Vec<AlbumSimple>>,
    search_cache: Cache<String, SearchResults>,
}

impl Client {
    pub fn new(username: String, password: String, max_audio_quality: AudioQuality) -> Self {
        let album_cache = moka::future::CacheBuilder::new(1000)
            .time_to_live(std::time::Duration::from_secs(60 * 60 * 24 * 7))
            .build();

        let artist_cache = moka::future::CacheBuilder::new(1000)
            .time_to_live(std::time::Duration::from_secs(60 * 60 * 24))
            .build();

        let artist_albums_cache = moka::future::CacheBuilder::new(1000)
            .time_to_live(std::time::Duration::from_secs(60 * 60 * 24))
            .build();

        let playlist_cache = moka::future::CacheBuilder::new(1000)
            .time_to_live(std::time::Duration::from_secs(60 * 60 * 24))
            .build();

        let similar_artists_cache = moka::future::CacheBuilder::new(1000)
            .time_to_live(std::time::Duration::from_secs(60 * 60 * 24 * 7))
            .build();

        let suggested_albums_cache = moka::future::CacheBuilder::new(1000)
            .time_to_live(std::time::Duration::from_secs(60 * 60 * 24 * 7))
            .build();

        let search_cache = moka::future::CacheBuilder::new(1000)
            .time_to_live(std::time::Duration::from_secs(60 * 60 * 24))
            .build();

        Self {
            qobuz_client: Default::default(),
            username,
            password,
            max_audio_quality,
            client_initiated: Mutex::new(false),
            favorites_cache: SimpleCache::new(Duration::days(1)),
            featured_albums_cache: SimpleCache::new(Duration::days(1)),
            featured_playlists_cache: SimpleCache::new(Duration::days(1)),
            album_cache,
            artist_cache,
            artist_albums_cache,
            playlist_cache,
            similar_artists_cache,
            suggested_albums_cache,
            search_cache,
        }
    }

    async fn init_client(&self) -> Result<QobuzClient> {
        let client = qobuz_player_client::client::new(
            &self.username,
            &self.password,
            self.max_audio_quality.clone(),
        )
        .await?;

        Ok(client)
    }

    async fn get_client(&self) -> Result<&QobuzClient> {
        if let Some(client) = self.qobuz_client.get() {
            return Ok(client);
        }

        let mut inititiated = self.client_initiated.lock().await;

        if !*inititiated {
            let client = self.init_client().await?;

            self.qobuz_client.set(client).or(Err(Error::Client {
                message: "Unable to set client".into(),
            }))?;
            *inititiated = true;
            drop(inititiated);
        }

        self.qobuz_client.get().ok_or_else(|| Error::Client {
            message: "Unable to acquire client lock".to_string(),
        })
    }

    pub(crate) async fn track_url(&self, track_id: u32) -> Result<TrackURL> {
        let client = self.get_client().await?;
        Ok(client.track_url(track_id).await?)
    }

    pub async fn album(&self, id: &str) -> Result<Album> {
        if let Some(cache) = self.album_cache.get(id).await {
            return Ok(cache);
        }

        let client = self.get_client().await?;
        let album = client.album(id).await?;

        self.album_cache.insert(id.to_string(), album.clone()).await;

        Ok(album)
    }

    pub async fn search(&self, query: String) -> Result<SearchResults> {
        if let Some(cache) = self.search_cache.get(&query).await {
            return Ok(cache);
        }

        let client = self.get_client().await?;
        let results = client.search_all(&query, 20).await?;

        self.search_cache.insert(query, results.clone()).await;
        Ok(results)
    }

    pub async fn artist_page(&self, id: u32) -> Result<ArtistPage> {
        if let Some(cache) = self.artist_cache.get(&id).await {
            return Ok(cache);
        }

        let client = self.get_client().await?;
        let artist = client.artist(id).await?;

        self.artist_cache.insert(id, artist.clone()).await;
        Ok(artist)
    }

    pub async fn similar_artists(&self, id: u32) -> Result<Vec<Artist>> {
        if let Some(cache) = self.similar_artists_cache.get(&id).await {
            return Ok(cache);
        }

        let client = self.get_client().await?;
        Ok(client.similar_artists(id, None).await?)
    }

    pub async fn track(&self, id: u32) -> Result<Track> {
        let client = self.get_client().await?;
        Ok(client.track(id).await?)
    }

    pub async fn suggested_albums(&self, id: &str) -> Result<Vec<AlbumSimple>> {
        if let Some(cache) = self.suggested_albums_cache.get(id).await {
            return Ok(cache);
        }

        let client = self.get_client().await?;
        let suggested_albums = client.suggested_albums(id).await?;

        self.suggested_albums_cache
            .insert(id.to_string(), suggested_albums.clone())
            .await;

        Ok(suggested_albums)
    }

    pub async fn featured_albums(&self) -> Result<Vec<(String, Vec<AlbumSimple>)>> {
        if let Some(cache) = self.featured_albums_cache.get().await {
            return Ok(cache);
        }

        let client = self.get_client().await?;
        let featured = client.featured_albums().await?;

        self.featured_albums_cache.set(featured.clone()).await;

        Ok(featured)
    }

    pub async fn featured_playlists(&self) -> Result<Vec<(String, Vec<Playlist>)>> {
        if let Some(cache) = self.featured_playlists_cache.get().await {
            return Ok(cache);
        }

        let client = self.get_client().await?;
        let featured = client.featured_playlists().await?;

        self.featured_playlists_cache.set(featured.clone()).await;

        Ok(featured)
    }

    pub async fn playlist(&self, id: u32) -> Result<Playlist> {
        if let Some(cache) = self.playlist_cache.get(&id).await {
            return Ok(cache);
        }

        let client = self.get_client().await?;
        let playlist = client.playlist(id).await?;

        self.playlist_cache.insert(id, playlist.clone()).await;
        Ok(playlist)
    }

    pub async fn artist_albums(&self, id: u32) -> Result<Vec<AlbumSimple>> {
        if let Some(cache) = self.artist_albums_cache.get(&id).await {
            return Ok(cache);
        }

        let client = self.get_client().await?;
        let albums = client.artist_releases(id, None).await?;

        self.artist_albums_cache.insert(id, albums.clone()).await;

        Ok(albums)
    }

    pub async fn add_favorite_album(&self, id: &str) -> Result<()> {
        let client = self.get_client().await?;
        client.add_favorite_album(id).await?;
        self.favorites_cache.clear().await;
        Ok(())
    }

    pub async fn remove_favorite_album(&self, id: &str) -> Result<()> {
        let client = self.get_client().await?;
        client.remove_favorite_album(id).await?;
        self.favorites_cache.clear().await;
        Ok(())
    }

    pub async fn add_favorite_artist(&self, id: &str) -> Result<()> {
        let client = self.get_client().await?;
        client.add_favorite_artist(id).await?;
        self.favorites_cache.clear().await;
        Ok(())
    }

    pub async fn remove_favorite_artist(&self, id: &str) -> Result<()> {
        let client = self.get_client().await?;
        client.remove_favorite_artist(id).await?;
        self.favorites_cache.clear().await;
        Ok(())
    }

    pub async fn add_favorite_playlist(&self, id: &str) -> Result<()> {
        let client = self.get_client().await?;
        client.add_favorite_playlist(id).await?;
        self.favorites_cache.clear().await;
        Ok(())
    }

    pub async fn remove_favorite_playlist(&self, id: &str) -> Result<()> {
        let client = self.get_client().await?;
        client.remove_favorite_playlist(id).await?;
        self.favorites_cache.clear().await;
        Ok(())
    }

    pub async fn favorites(&self) -> Result<Favorites> {
        if let Some(cache) = self.favorites_cache.get().await {
            return Ok(cache);
        }

        let client = self.get_client().await?;

        let favorites = client.favorites(1000).await?;

        self.favorites_cache.set(favorites.clone()).await;
        Ok(favorites)
    }
}
