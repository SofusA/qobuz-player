use crate::{
    Error, Result,
    qobuz_models::{
        TrackURL,
        album::Album,
        album_suggestion::{AlbumOfTheWeekQuery, AlbumSuggestionResponse, ReleaseQuery},
        artist::{Artist, ArtistsResponse},
        artist_page::ArtistPage,
        favorites::Favorites,
        featured::{FeaturedAlbumsResponse, FeaturedPlaylistsResponse},
        genre::{GenreFeaturedPlaylists, GenreResponse},
        playlist::{Playlist, UserPlaylistsResult},
        search_results::SearchAllResults,
        track::Track,
    },
};
use regex::Regex;
use reqwest::{
    Method, Response, StatusCode,
    header::{HeaderMap, HeaderValue},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashMap},
    fmt::Display,
    time::{SystemTime, UNIX_EPOCH},
};

const RNG_INIT: &str = "abb21364945c0583309667d13ca3d93a";

#[derive(Debug)]
pub struct Client {
    session: Option<StartResponse>,
    app_id: String,
    base_url: String,
    http_client: reqwest::Client,
    user_token: String,
    user_id: i64,
    max_audio_quality: AudioQuality,
}

#[derive(Clone, Debug, clap::ValueEnum)]
pub enum AudioQuality {
    Mp3 = 5,
    CD = 6,
    HIFI96 = 7,
    HIFI192 = 27,
}

impl Display for AudioQuality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            AudioQuality::Mp3 => "5",
            AudioQuality::CD => "6",
            AudioQuality::HIFI96 => "7",
            AudioQuality::HIFI192 => "27",
        })
    }
}

impl TryFrom<i64> for AudioQuality {
    type Error = ();

    fn try_from(value: i64) -> std::result::Result<Self, Self::Error> {
        match value {
            5 => Ok(AudioQuality::Mp3),
            6 => Ok(AudioQuality::CD),
            7 => Ok(AudioQuality::HIFI96),
            27 => Ok(AudioQuality::HIFI192),
            _ => Err(()),
        }
    }
}

pub enum ReleaseType {
    Albums,
    EPsAndSingles,
    Live,
    Compilations,
    // Other,
}

impl ReleaseType {
    fn as_str(&self) -> &'static str {
        match self {
            ReleaseType::Albums => "album",
            ReleaseType::EPsAndSingles => "epSingle",
            ReleaseType::Live => "live",
            ReleaseType::Compilations => "compilation",
            // ReleaseType::Other => "other",
        }
    }
}

pub enum FeaturedAlbumType {
    PressAwards,
    MostStreamed,
    NewReleases,
    Qobuzissims,
    IdealDiscography,
}

impl FeaturedAlbumType {
    fn as_str(&self) -> &'static str {
        match self {
            FeaturedAlbumType::PressAwards => "press-awards",
            FeaturedAlbumType::MostStreamed => "most-streamed",
            FeaturedAlbumType::NewReleases => "new-releases-full",
            FeaturedAlbumType::Qobuzissims => "qobuzissims",
            FeaturedAlbumType::IdealDiscography => "ideal-discography",
        }
    }
}

pub enum FeaturedPlaylistType {
    EditorsPick,
}

impl FeaturedPlaylistType {
    fn as_str(&self) -> &'static str {
        match self {
            FeaturedPlaylistType::EditorsPick => "editor-picks",
        }
    }
}

pub enum FeaturedGenreAlbumType {
    PressAwards,
    MostStreamed,
    NewReleases,
    Qobuzissims,
    BestSellers,
}

impl FeaturedGenreAlbumType {
    fn as_str(&self) -> &'static str {
        match self {
            FeaturedGenreAlbumType::PressAwards => "press-awards",
            FeaturedGenreAlbumType::MostStreamed => "most-streamed",
            FeaturedGenreAlbumType::NewReleases => "new-releases-full",
            FeaturedGenreAlbumType::Qobuzissims => "qobuzissims",
            FeaturedGenreAlbumType::BestSellers => "best-sellers",
        }
    }
}

pub struct FavoriteCollection {
    pub albums: Vec<Album>,
    pub artists: Vec<Artist>,
    pub playlists: Vec<Playlist>,
    pub tracks: Vec<Track>,
}

enum Endpoint {
    Album,
    ArtistPage,
    SimilarArtists,
    ArtistReleases,
    UserPlaylist,
    Track,
    TrackURL,
    Playlist,
    PlaylistCreate,
    PlaylistDelete,
    PlaylistAddTracks,
    PlaylistDeleteTracks,
    PlaylistUpdatePosition,
    Search,
    SessionStart,
    Favorites,
    FavoriteAdd,
    FavoriteRemove,
    FavoritePlaylistAdd,
    FavoritePlaylistRemove,
    AlbumSuggest,
    AlbumFeatured,
    AlbumOfTheWeek,
    PlaylistFeatured,
    GenreList,
    GenreFeatured,
    GenrePlaylists,
}

impl Display for Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let endpoint = match self {
            Endpoint::Album => "album/get",
            Endpoint::ArtistPage => "artist/page",
            Endpoint::ArtistReleases => "artist/getReleasesList",
            Endpoint::SimilarArtists => "artist/getSimilarArtists",
            Endpoint::Playlist => "playlist/get",
            Endpoint::PlaylistCreate => "playlist/create",
            Endpoint::PlaylistDelete => "playlist/delete",
            Endpoint::PlaylistAddTracks => "playlist/addTracks",
            Endpoint::PlaylistDeleteTracks => "playlist/deleteTracks",
            Endpoint::PlaylistUpdatePosition => "playlist/updateTracksPosition",
            Endpoint::Search => "catalog/search",
            Endpoint::SessionStart => "session/start",
            Endpoint::Track => "track/get",
            Endpoint::TrackURL => "file/url",
            Endpoint::UserPlaylist => "playlist/getUserPlaylists",
            Endpoint::Favorites => "favorite/getUserFavorites",
            Endpoint::FavoriteAdd => "favorite/create",
            Endpoint::FavoriteRemove => "favorite/delete",
            Endpoint::FavoritePlaylistAdd => "playlist/subscribe",
            Endpoint::FavoritePlaylistRemove => "playlist/unsubscribe",
            Endpoint::AlbumSuggest => "album/suggest",
            Endpoint::AlbumFeatured => "album/getFeatured",
            Endpoint::AlbumOfTheWeek => "discover/albumOfTheWeek",
            Endpoint::PlaylistFeatured => "playlist/getFeatured",
            Endpoint::GenreList => "genre/list",
            Endpoint::GenreFeatured => "album/getFeatured",
            Endpoint::GenrePlaylists => "discover/playlists",
        };

        f.write_str(endpoint)
    }
}

impl Client {
    pub async fn new(user_auth_token: &str, max_audio_quality: AudioQuality) -> Result<Client> {
        let http_client = reqwest::Client::builder()
            .cookie_store(true)
            .build()
            .expect("infallible");

        let Secrets { app_id } = get_secrets(&http_client).await?;

        tracing::debug!("Got login secrets, app_id: {}", app_id);

        let base_url = "https://www.qobuz.com/api.json/0.2/".to_string();

        let client = Client {
            http_client,
            session: None,
            user_token: user_auth_token.to_string(),
            user_id: 0,
            app_id,
            base_url,
            max_audio_quality,
        };

        Ok(client)
    }

    pub fn app_id(&self) -> &str {
        &self.app_id
    }

    pub fn user_id(&self) -> i64 {
        self.user_id
    }

    pub async fn featured_albums(
        &self,
        featured_type: FeaturedAlbumType,
    ) -> Result<FeaturedAlbumsResponse> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::AlbumFeatured);
        let type_string = featured_type.as_str();
        let params = vec![("type", type_string), ("offset", "0"), ("limit", "20")];
        self.get(&endpoint, Some(&params)).await
    }

    pub async fn album_of_the_week(&self) -> Result<AlbumOfTheWeekQuery> {
        self.get(
            &format!("{}{}", self.base_url, Endpoint::AlbumOfTheWeek),
            None,
        )
        .await
    }

    pub async fn featured_playlists(
        &self,
        featured_type: FeaturedPlaylistType,
    ) -> Result<FeaturedPlaylistsResponse> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::PlaylistFeatured);
        let type_string = featured_type.as_str();
        let params = vec![("type", type_string), ("offset", "0"), ("limit", "20")];
        self.get(&endpoint, Some(&params)).await
    }

    pub async fn genres(&self) -> Result<GenreResponse> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::GenreList);
        self.get(&endpoint, None).await
    }

    pub async fn genre_albums(
        &self,
        genre_id: u32,
        featured_type: FeaturedGenreAlbumType,
    ) -> Result<FeaturedAlbumsResponse> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::GenreFeatured);
        let genre_id_str = genre_id.to_string();
        let type_string = featured_type.as_str();

        let params = vec![
            ("type", type_string),
            ("genre_id", genre_id_str.as_str()),
            ("offset", "0"),
            ("limit", "20"),
        ];
        self.get(&endpoint, Some(&params)).await
    }

    pub async fn genre_playlists(&self, genre_id: u32) -> Result<GenreFeaturedPlaylists> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::GenrePlaylists);
        let genre_id = genre_id.to_string();

        let params = vec![
            ("genre_ids", genre_id.as_str()),
            ("offset", "0"),
            ("limit", "20"),
        ];

        self.get(&endpoint, Some(&params)).await
    }

    pub async fn user_playlists(&self) -> Result<UserPlaylistsResult> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::UserPlaylist);
        let params = vec![("limit", "500"), ("extra", "tracks"), ("offset", "0")];

        self.get(&endpoint, Some(&params)).await
    }

    pub async fn playlist(&self, playlist_id: u32) -> Result<Playlist> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::Playlist);
        let id_string = playlist_id.to_string();
        let params = vec![
            ("limit", "500"),
            ("extra", "tracks"),
            ("playlist_id", id_string.as_str()),
            ("offset", "0"),
        ];

        self.get(&endpoint, Some(&params)).await
    }

    pub async fn create_playlist(
        &self,
        name: String,
        is_public: bool,
        description: String,
        is_collaborative: Option<bool>,
    ) -> Result<Playlist> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::PlaylistCreate);

        let mut form_data = HashMap::new();
        form_data.insert("name", name.as_str());

        let is_collaborative = is_collaborative.unwrap_or(false);

        let is_collaborative = if !is_public {
            false.to_string()
        } else {
            is_collaborative.to_string()
        };

        form_data.insert("is_collaborative", is_collaborative.as_str());

        let is_public = is_public.to_string();
        form_data.insert("is_public", is_public.as_str());
        form_data.insert("description", description.as_str());

        self.post(&endpoint, form_data).await
    }

    pub async fn delete_playlist(&self, playlist_id: u32) -> Result<SuccessfulResponse> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::PlaylistDelete);

        let mut form_data = HashMap::new();
        let playlist_id = playlist_id.to_string();
        form_data.insert("playlist_id", playlist_id.as_str());

        self.post(&endpoint, form_data).await
    }

    pub async fn playlist_add_track(
        &self,
        playlist_id: u32,
        playlist_track_ids: &[u32],
    ) -> Result<Playlist> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::PlaylistAddTracks);

        let track_ids = playlist_track_ids
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(",");

        let playlist_id = playlist_id.to_string();

        let mut form_data = HashMap::new();
        form_data.insert("playlist_id", playlist_id.as_str());
        form_data.insert("track_ids", track_ids.as_str());
        // form_data.insert("no_duplicate", "true");

        self.post(&endpoint, form_data).await
    }

    pub async fn playlist_delete_track(
        &self,
        playlist_id: u32,
        playlist_track_ids: &[u64],
    ) -> Result<Playlist> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::PlaylistDeleteTracks);

        let track_ids = playlist_track_ids
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let playlist_id = playlist_id.to_string();

        let mut form_data = HashMap::new();
        form_data.insert("playlist_id", playlist_id.as_str());
        form_data.insert("playlist_track_ids", track_ids.as_str());

        self.post(&endpoint, form_data).await
    }

    pub async fn update_playlist_track_position(
        &self,
        index: usize,
        playlist_id: u32,
        playlist_track_id: u64,
    ) -> Result<Playlist> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::PlaylistUpdatePosition);

        let index = index.to_string();
        let playlist_id = playlist_id.to_string();
        let track_id = playlist_track_id.to_string();

        let mut form_data = HashMap::new();
        form_data.insert("playlist_id", playlist_id.as_str());
        form_data.insert("playlist_track_ids", track_id.as_str());
        form_data.insert("insert_before", index.as_str());

        self.post(&endpoint, form_data).await
    }

    async fn renew_session(&mut self) -> Result<()> {
        tracing::info!("Renewing session");

        let endpoint = format!("{}{}", &self.base_url, Endpoint::SessionStart);
        let now = format!("{}", time::OffsetDateTime::now_utc().unix_timestamp());

        let mut args = BTreeMap::<&str, String>::new();
        args.insert("profile", "qbz-1".to_string());

        let request_sig = get_request_sig("sessionstart", args, &now);

        let mut form_data = HashMap::new();
        form_data.insert("profile", "qbz-1");
        form_data.insert("request_ts", now.as_str());
        form_data.insert("request_sig", request_sig.as_str());

        let result: StartResponse = self.post(&endpoint, form_data).await?;

        tracing::info!("Session renewed: {}", result.session_id);
        if let Some(infos) = &result.infos {
            tracing::debug!("Session infos: {}", infos);
        }

        self.session = Some(result);
        Ok(())
    }

    async fn ensure_valid_session(&mut self) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as u32;

        let need_new_session = match &self.session {
            None => true,
            Some(s) => s.expires_at <= now,
        };

        if need_new_session {
            self.renew_session().await?;
        }

        Ok(())
    }

    pub fn session_infos(&self) -> Option<&str> {
        self.session.as_ref().and_then(|s| s.infos.as_deref())
    }

    pub async fn track_url(&mut self, track_id: u32) -> Result<TrackURL> {
        self.ensure_valid_session().await?;

        let endpoint = format!("{}{}", &self.base_url, Endpoint::TrackURL);
        let now = format!("{}", time::OffsetDateTime::now_utc().unix_timestamp());
        let quality_string = self.max_audio_quality.to_string();
        let track_id_str = track_id.to_string();

        let mut args = BTreeMap::<&str, String>::new();
        args.insert("format_id", quality_string.clone());
        args.insert("intent", "stream".to_string());
        args.insert("track_id", track_id_str.clone());

        let request_sig = get_request_sig("fileurl", args, &now);

        let params = vec![
            ("request_ts", now.as_str()),
            ("request_sig", request_sig.as_str()),
            ("track_id", track_id_str.as_str()),
            ("format_id", quality_string.as_str()),
            ("intent", "stream"),
        ];

        let session_id = self.session.as_ref().unwrap().session_id.clone();

        match make_get_call(
            &endpoint,
            Some(&params),
            &self.http_client,
            &self.app_id,
            Some(&self.user_token),
            Some(&session_id),
        )
        .await
        {
            Ok(response) => match serde_json::from_str::<TrackURL>(response.as_str()) {
                Ok(item) => Ok(item),
                Err(error) => {
                    tracing::debug!("TrackURL deserialize error: {}", error);
                    tracing::debug!("Response was: {}", response);
                    Err(Error::DeserializeJSON {
                        message: error.to_string(),
                    })
                }
            },
            Err(error) => Err(Error::Api {
                message: error.to_string(),
            }),
        }
    }

    pub async fn favorites(&self, limit: i32) -> Result<Favorites> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::Favorites);

        let limit = limit.to_string();
        let params = vec![("limit", limit.as_str())];

        self.get(&endpoint, Some(&params)).await
    }

    pub async fn add_favorite_track(&self, id: u32) -> Result<SuccessfulResponse> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::FavoriteAdd);
        let mut form_data = HashMap::new();
        let id = id.to_string();
        form_data.insert("track_ids", id.as_str());

        self.post(&endpoint, form_data).await
    }

    pub async fn remove_favorite_track(&self, id: u32) -> Result<SuccessfulResponse> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::FavoriteRemove);
        let mut form_data = HashMap::new();
        let id = id.to_string();
        form_data.insert("track_ids", id.as_str());

        self.post(&endpoint, form_data).await
    }

    pub async fn add_favorite_album(&self, id: &str) -> Result<SuccessfulResponse> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::FavoriteAdd);
        let mut form_data = HashMap::new();
        form_data.insert("album_ids", id);

        self.post(&endpoint, form_data).await
    }

    pub async fn remove_favorite_album(&self, id: &str) -> Result<SuccessfulResponse> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::FavoriteRemove);
        let mut form_data = HashMap::new();
        form_data.insert("album_ids", id);

        self.post(&endpoint, form_data).await
    }

    pub async fn add_favorite_artist(&self, id: u32) -> Result<SuccessfulResponse> {
        let id = id.to_string();
        let endpoint = format!("{}{}", self.base_url, Endpoint::FavoriteAdd);
        let mut form_data = HashMap::new();
        form_data.insert("artist_ids", id.as_str());

        self.post(&endpoint, form_data).await
    }

    pub async fn remove_favorite_artist(&self, id: u32) -> Result<SuccessfulResponse> {
        let id = id.to_string();
        let endpoint = format!("{}{}", self.base_url, Endpoint::FavoriteRemove);
        let mut form_data = HashMap::new();
        form_data.insert("artist_ids", id.as_str());

        self.post(&endpoint, form_data).await
    }

    pub async fn add_favorite_playlist(&self, id: u32) -> Result<SuccessfulResponse> {
        let id = id.to_string();
        let endpoint = format!("{}{}", self.base_url, Endpoint::FavoritePlaylistAdd);
        let mut form_data = HashMap::new();
        form_data.insert("playlist_id", id.as_str());

        self.post(&endpoint, form_data).await
    }

    pub async fn remove_favorite_playlist(&self, id: u32) -> Result<SuccessfulResponse> {
        let id = id.to_string();
        let endpoint = format!("{}{}", self.base_url, Endpoint::FavoritePlaylistRemove);
        let mut form_data = HashMap::new();
        form_data.insert("playlist_id", id.as_str());

        self.post(&endpoint, form_data).await
    }

    pub async fn search_all(&self, query: &str, limit: i32) -> Result<SearchAllResults> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::Search);
        let limit = limit.to_string();
        let params = vec![("query", query), ("limit", &limit)];

        self.get(&endpoint, Some(&params)).await
    }

    pub async fn album(&self, album_id: &str) -> Result<Album> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::Album);
        let params = vec![
            ("album_id", album_id),
            ("extra", "track_ids"),
            ("offset", "0"),
            ("limit", "500"),
        ];

        self.get(&endpoint, Some(&params)).await
    }

    pub async fn track(&self, track_id: u32) -> Result<Track> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::Track);
        let track_id_string = track_id.to_string();
        let params = vec![("track_id", track_id_string.as_str())];

        self.get(&endpoint, Some(&params)).await
    }

    pub async fn suggested_albums(&self, album_id: &str) -> Result<AlbumSuggestionResponse> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::AlbumSuggest);
        let params = vec![("album_id", album_id)];

        self.get(&endpoint, Some(&params)).await
    }

    pub async fn artist(&self, artist_id: u32) -> Result<ArtistPage> {
        let app_id = &self.app_id;

        let endpoint = format!("{}{}", self.base_url, Endpoint::ArtistPage);

        let artistid_string = artist_id.to_string();

        let params = vec![
            ("artist_id", artistid_string.as_str()),
            ("app_id", app_id),
            ("sort", "relevant"),
        ];

        self.get(&endpoint, Some(&params)).await
    }

    async fn get<T>(&self, endpoint: &str, params: Option<&[(&str, &str)]>) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let response = self
            .make_get_call(endpoint, params)
            .await
            .map_err(|error| Error::Api {
                message: error.to_string(),
            })?;

        let item = serde_json::from_str::<T>(response.as_str()).map_err(|error| {
            Error::DeserializeJSON {
                message: error.to_string(),
            }
        })?;

        Ok(item)
    }

    async fn post<T>(&self, endpoint: &str, params: HashMap<&str, &str>) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let response = self
            .make_post_call(endpoint, params)
            .await
            .map_err(|error| Error::Api {
                message: error.to_string(),
            })?;

        let item = serde_json::from_str::<T>(response.as_str()).map_err(|error| {
            Error::DeserializeJSON {
                message: error.to_string(),
            }
        })?;

        Ok(item)
    }

    pub async fn similar_artists(
        &self,
        artist_id: u32,
        limit: Option<i32>,
    ) -> Result<ArtistsResponse> {
        let limit = limit.unwrap_or(10).to_string();

        let endpoint = format!("{}{}", self.base_url, Endpoint::SimilarArtists);
        let artistid_string = artist_id.to_string();

        let params = vec![
            ("artist_id", artistid_string.as_str()),
            ("limit", &limit),
            ("offset", "0"),
        ];

        self.get(&endpoint, Some(&params)).await
    }

    pub async fn artist_releases(
        &self,
        artist_id: u32,
        release_type: ReleaseType,
        limit: Option<i32>,
    ) -> Result<ReleaseQuery> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::ArtistReleases);
        let limit = limit.unwrap_or(100).to_string();

        let artistid_string = artist_id.to_string();

        let params = vec![
            ("artist_id", artistid_string.as_str()),
            ("limit", &limit),
            ("release_type", release_type.as_str()),
            ("sort", "release_date"),
            ("offset", "0"),
            ("track_size", "1"),
        ];

        self.get(&endpoint, Some(&params)).await
    }

    async fn make_get_call(
        &self,
        endpoint: &str,
        params: Option<&[(&str, &str)]>,
    ) -> Result<String> {
        make_get_call(
            endpoint,
            params,
            &self.http_client,
            &self.app_id,
            Some(&self.user_token),
            None,
        )
        .await
    }

    async fn make_post_call(&self, endpoint: &str, params: HashMap<&str, &str>) -> Result<String> {
        let headers = client_headers(&self.app_id, Some(&self.user_token));

        tracing::debug!("calling {} endpoint, with params {params:?}", endpoint);
        let response = self
            .http_client
            .request(Method::POST, endpoint)
            .headers(headers)
            .form(&params)
            .send()
            .await?;

        handle_response(response).await
    }
}

fn get_request_sig(method: &str, args: BTreeMap<&str, String>, now_string: &str) -> String {
    let mut n = String::new();
    for (k, v) in args.iter() {
        n.push_str(k);
        n.push_str(v);
    }

    let req_id = format!("{method}{n}{now_string}{RNG_INIT}");
    format!("{:x}", md5::compute(req_id.as_bytes()))
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct StartResponse {
    session_id: String,
    expires_at: u32,
    #[serde(default)]
    infos: Option<String>,
}

async fn handle_response(response: Response) -> Result<String> {
    if response.status() == StatusCode::OK {
        let res = response.text().await.unwrap_or_default();
        Ok(res)
    } else {
        Err(Error::Api {
            message: response.text().await.unwrap_or_default(),
        })
    }
}

async fn make_get_call(
    endpoint: &str,
    params: Option<&[(&str, &str)]>,
    client: &reqwest::Client,
    app_id: &str,
    user_token: Option<&str>,
    session: Option<&str>,
) -> Result<String> {
    let mut headers = client_headers(app_id, user_token);

    if let Some(session_id) = session {
        headers.insert(
            "X-Session-Id",
            HeaderValue::from_str(session_id).expect("infallible"),
        );
    }

    tracing::debug!("calling {} endpoint, with params {params:?}", endpoint);
    let request = client.request(Method::GET, endpoint).headers(headers);

    if let Some(p) = params {
        let response = request.query(&p).send().await?;
        handle_response(response).await
    } else {
        let response = request.send().await?;
        handle_response(response).await
    }
}

fn client_headers(app_id: &str, user_token: Option<&str>) -> HeaderMap {
    let mut headers = HeaderMap::new();

    tracing::debug!("adding app_id to request headers: {}", app_id);
    headers.insert(
        "X-App-Id",
        HeaderValue::from_str(app_id).expect("infallible"),
    );

    if let Some(token) = user_token {
        tracing::debug!("adding token to request headers: {}", token);
        headers.insert(
            "X-User-Auth-Token",
            HeaderValue::from_str(token).expect("infallible"),
        );
    }

    headers.insert(
        "Access-Control-Request-Headers",
        HeaderValue::from_str("x-app-id,x-user-auth-token").expect("infallible"),
    );

    headers.insert(
        "Accept-Language",
        HeaderValue::from_str("en,en-US;q=0.8,ko;q=0.6,zh;q=0.4,zh-CN;q=0.2").expect("infallible"),
    );

    headers
}

const OAUTH_PRIVATE_KEY: &str = "6lz8C03UDIC7";

pub struct OAuthResult {
    pub user_auth_token: String,
    pub user_id: i64,
}

/// Fetch the app_id from the Qobuz web player bundle.
pub async fn get_app_id() -> Result<String> {
    let http_client = reqwest::Client::new();
    let Secrets { app_id } = get_secrets(&http_client).await?;
    Ok(app_id)
}

/// Exchange an OAuth authorization code for a user_auth_token.
pub async fn exchange_oauth_code(code: &str, app_id: &str) -> Result<OAuthResult> {
    let http_client = reqwest::Client::new();
    let base_url = "https://www.qobuz.com/api.json/0.2/";
    let endpoint = format!("{base_url}oauth/callback");
    let params = vec![("code", code), ("private_key", OAUTH_PRIVATE_KEY)];

    let response = make_get_call(&endpoint, Some(&params), &http_client, app_id, None, None).await;

    let response = match response {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("oauth/callback API error: {e}");
            return Err(e);
        }
    };

    tracing::debug!(
        "oauth/callback response: {}",
        &response[..response.len().min(200)]
    );

    let json: Value = serde_json::from_str(response.as_str())
        .or(Err(Error::DeserializeJSON { message: response }))?;

    let user_auth_token = json["token"]
        .as_str()
        .or_else(|| json["user_auth_token"].as_str())
        .ok_or(Error::Login)?
        .to_string();

    let user_id = json["user_id"]
        .as_i64()
        .or_else(|| json["user_id"].as_str().and_then(|s| s.parse().ok()))
        .ok_or(Error::Login)?;

    Ok(OAuthResult {
        user_auth_token,
        user_id,
    })
}

/// Build the OAuth URL that the user should open in their browser.
pub fn build_oauth_url(app_id: &str, redirect_port: u16) -> String {
    let redirect = format!("http://localhost:{redirect_port}");
    format!("https://www.qobuz.com/signin/oauth?ext_app_id={app_id}&redirect_url={redirect}",)
}

struct Secrets {
    app_id: String,
}

async fn get_secrets(client: &reqwest::Client) -> Result<Secrets> {
    let play_url = "https://play.qobuz.com";

    let login_html = client
        .get(format!("{play_url}/login"))
        .send()
        .await?
        .error_for_status()?
        .text()
        .await
        .map_err(|_| Error::Login)?;

    let bundle_regex = Regex::new(
        r#"<script src="(/resources/\d+\.\d+\.\d+-[a-z0-9]\d{3}/bundle\.js)"></script>"#,
    )
    .map_err(|_| Error::Login)?;

    let app_id_regex = Regex::new(
        r#"production:\{api:\{appId:"(?P<app_id>\d{9})",appSecret:"(?P<app_secret>\w{32})""#,
    )
    .map_err(|_| Error::AppID)?;

    let bundle_path = bundle_regex
        .captures(&login_html)
        .and_then(|c| c.get(1))
        .ok_or(Error::AppID)?
        .as_str();

    let bundle_html = client
        .get(format!("{play_url}{bundle_path}"))
        .send()
        .await?
        .error_for_status()?
        .text()
        .await
        .map_err(|_| Error::AppID)?;

    let app_captures = app_id_regex.captures(&bundle_html).ok_or(Error::AppID)?;
    let app_id = app_captures
        .name("app_id")
        .ok_or(Error::AppID)?
        .as_str()
        .to_owned();

    Ok(Secrets { app_id })
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct SuccessfulResponse {
    status: String,
}
