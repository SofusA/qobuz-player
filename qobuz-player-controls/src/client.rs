use qobuz_player_client::client::AudioQuality;
use std::sync::OnceLock;
use tokio::sync::Mutex;

use crate::{
    error::Error,
    models::{
        self, Album, AlbumSimple, Artist, ArtistPage, Favorites, Playlist, SearchResults, Track,
        parse_album, parse_album_simple, parse_track,
    },
};

type QobuzClient = qobuz_player_client::client::Client;
type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub struct Client {
    qobuz_client: OnceLock<QobuzClient>,
    username: String,
    password: String,
    max_audio_quality: AudioQuality,
    client_initiated: Mutex<bool>,
}

impl Client {
    pub fn new(username: String, password: String, max_audio_quality: AudioQuality) -> Self {
        Self {
            qobuz_client: Default::default(),
            username,
            password,
            max_audio_quality,
            client_initiated: Mutex::new(false),
        }
    }

    async fn init_client(&self) -> QobuzClient {
        let client = qobuz_player_client::client::new(
            &self.username,
            &self.password,
            self.max_audio_quality.clone(),
        )
        .await
        .expect("error making client");

        tracing::info!("Done");
        client
    }

    async fn get_client(&self) -> &QobuzClient {
        if let Some(client) = self.qobuz_client.get() {
            return client;
        }

        let mut inititiated = self.client_initiated.lock().await;

        if !*inititiated {
            let client = self.init_client().await;

            self.qobuz_client.set(client).unwrap();
            *inititiated = true;
            drop(inititiated);
        }

        self.qobuz_client.get().unwrap()
    }

    pub(crate) async fn track_url(
        &self,
        track_id: u32,
    ) -> Result<String, qobuz_player_client::Error> {
        let client = self.get_client().await;
        client.track_url(track_id).await
    }

    pub async fn album(&self, id: &str) -> Result<Album> {
        let client = self.get_client().await;
        let album = client.album(id).await?;
        Ok(parse_album(album, &self.max_audio_quality))
    }

    pub async fn search(&self, query: String) -> Result<SearchResults> {
        let client = self.get_client().await;
        let user_id = client.get_user_id();

        let results = client.search_all(&query, 20).await?;
        Ok(models::parse_search_results(
            results,
            user_id,
            &self.max_audio_quality,
        ))
    }

    pub async fn artist_page(&self, artist_id: u32) -> Result<ArtistPage> {
        let client = self.get_client().await;
        let artist = client.artist(artist_id).await?;
        Ok(artist.into())
    }

    pub async fn similar_artists(&self, artist_id: u32) -> Result<Vec<Artist>> {
        let client = self.get_client().await;
        let similar_artists = client.similar_artists(artist_id, None).await?;

        Ok(similar_artists
            .items
            .into_iter()
            .map(|s_a| s_a.into())
            .collect())
    }

    pub async fn track(&self, id: u32) -> Result<Track> {
        let client = self.get_client().await;
        Ok(parse_track(
            client.track(id).await?,
            &self.max_audio_quality,
        ))
    }

    pub async fn suggested_albums(&self, album_id: &str) -> Result<Vec<AlbumSimple>> {
        let client = self.get_client().await;
        let suggested_albums = client.suggested_albums(album_id).await?;

        Ok(suggested_albums
            .albums
            .items
            .into_iter()
            .map(|x| parse_album_simple(x, &self.max_audio_quality))
            .collect())
    }

    pub async fn featured_albums(&self) -> Result<Vec<(String, Vec<AlbumSimple>)>> {
        let client = self.get_client().await;
        let featured = client.featured_albums().await?;

        Ok(featured
            .into_iter()
            .map(|featured| {
                let featured_type = featured.0;

                let albums = featured
                    .1
                    .albums
                    .items
                    .into_iter()
                    .map(|value| AlbumSimple {
                        id: value.id,
                        title: value.title,
                        artist: value.artist.into(),
                        hires_available: value.hires_streamable,
                        explicit: value.parental_warning,
                        available: value.streamable,
                        image: value.image.large,
                    })
                    .collect::<Vec<_>>();

                (featured_type, albums)
            })
            .collect())
    }

    pub async fn featured_playlists(&self) -> Result<Vec<(String, Vec<Playlist>)>> {
        let client = self.get_client().await;
        let user_id = client.get_user_id();
        let featured = client.featured_playlists().await?;

        Ok(featured
            .into_iter()
            .map(|featured| {
                let featured_type = featured.0;
                let playlists = featured
                    .1
                    .playlists
                    .items
                    .into_iter()
                    .map(|playlist| {
                        models::parse_playlist(playlist, user_id, &self.max_audio_quality)
                    })
                    .collect();

                (featured_type, playlists)
            })
            .collect())
    }

    pub async fn playlist(&self, id: u32) -> Result<Playlist> {
        let client = self.get_client().await;
        let user_id = client.get_user_id();
        let playlist = client.playlist(id).await?;

        Ok(models::parse_playlist(
            playlist,
            user_id,
            &self.max_audio_quality,
        ))
    }

    pub async fn artist_albums(&self, artist_id: u32) -> Result<Vec<AlbumSimple>> {
        let client = self.get_client().await;
        let albums = client.artist_releases(artist_id, None).await?;

        Ok(albums.into_iter().map(|release| release.into()).collect())
    }

    pub async fn add_favorite_album(&self, id: &str) -> Result<()> {
        let client = self.get_client().await;
        client.add_favorite_album(id).await?;
        Ok(())
    }

    pub async fn remove_favorite_album(&self, id: &str) -> Result<()> {
        let client = self.get_client().await;
        client.remove_favorite_album(id).await?;
        Ok(())
    }

    pub async fn add_favorite_artist(&self, id: &str) -> Result<()> {
        let client = self.get_client().await;
        client.add_favorite_artist(id).await?;
        Ok(())
    }

    pub async fn remove_favorite_artist(&self, id: &str) -> Result<()> {
        let client = self.get_client().await;
        client.remove_favorite_artist(id).await?;
        Ok(())
    }

    pub async fn add_favorite_playlist(&self, id: &str) -> Result<()> {
        let client = self.get_client().await;
        client.add_favorite_playlist(id).await?;
        Ok(())
    }

    pub async fn remove_favorite_playlist(&self, id: &str) -> Result<()> {
        let client = self.get_client().await;
        client.remove_favorite_playlist(id).await?;
        Ok(())
    }

    pub async fn favorites(&self) -> Result<Favorites> {
        let client = self.get_client().await;
        let (favorites, favorite_playlists) = tokio::join!(
            client.favorites(1000),
            user_playlists(client, &self.max_audio_quality)
        );

        let mut favorite_playlists = favorite_playlists.unwrap_or_default();

        let qobuz_player_client::qobuz_models::favorites::Favorites {
            albums,
            tracks: _,
            artists,
        } = favorites?;
        let mut albums = albums.items;
        albums.sort_by(|a, b| a.artist.name.cmp(&b.artist.name));

        let mut artists = artists.items;
        artists.sort_by(|a, b| a.name.cmp(&b.name));

        favorite_playlists.sort_by(|a, b| a.title.cmp(&b.title));

        Ok(Favorites {
            albums: albums
                .into_iter()
                .map(|x| parse_album(x, &self.max_audio_quality))
                .collect(),
            artists: artists.into_iter().map(|x| x.into()).collect(),
            playlists: favorite_playlists,
        })
    }
}

async fn user_playlists(
    client: &QobuzClient,
    max_audio_quality: &AudioQuality,
) -> Result<Vec<Playlist>> {
    let user_id = client.get_user_id();
    let playlists = client.user_playlists().await?;

    Ok(playlists
        .playlists
        .items
        .into_iter()
        .map(|playlist| models::parse_playlist(playlist, user_id, max_audio_quality))
        .collect())
}
