use crate::{
    Error, Result,
    qobuz_models::{
        TrackURL,
        album_suggestion::{
            AlbumOfTheWeekQuery, AlbumSuggestion, AlbumSuggestionResponse, ReleaseQuery,
        },
        artist::{self, ArtistsResponse},
        artist_page,
        favorites::Favorites,
        featured::{FeaturedAlbumsResponse, FeaturedPlaylistsResponse},
        playlist::{self, UserPlaylistsResult},
        search_results::SearchAllResults,
        track,
    },
};
use base64::{Engine as _, engine::general_purpose};
use reqwest::{
    Method, Response, StatusCode,
    header::{HeaderMap, HeaderValue},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, fmt::Display};
use time::macros::format_description;
use tokio::try_join;

#[derive(Debug)]
pub struct Client {
    active_secret: String,
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

pub async fn new(
    username: &str,
    password: &str,
    max_audio_quality: AudioQuality,
) -> Result<Client> {
    let http_client = reqwest::Client::builder()
        .cookie_store(true)
        .build()
        .expect("infallible");

    let Secrets { secrets, app_id } = get_secrets(&http_client).await?;

    tracing::debug!("Got login secrets");

    let base_url = "https://www.qobuz.com/api.json/0.2/".to_string();

    let login = login(username, password, &app_id, &base_url, &http_client).await?;
    tracing::debug!("Logged in");

    let active_secret =
        find_active_secret(secrets, &base_url, &http_client, &app_id, &login.user_token).await?;

    tracing::debug!("Found active secrets");

    let client = Client {
        http_client,
        active_secret,
        user_token: login.user_token,
        user_id: login.user_id,
        app_id,
        base_url,
        max_audio_quality,
    };

    Ok(client)
}

enum Endpoint {
    Album,
    ArtistPage,
    SimilarArtists,
    ArtistReleases,
    Login,
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
    Favorites,
    FavoriteAdd,
    FavoriteRemove,
    FavoritePlaylistAdd,
    FavoritePlaylistRemove,
    AlbumSuggest,
    AlbumFeatured,
    AlbumOfTheWeek,
    PlaylistFeatured,
}

impl Display for Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let endpoint = match self {
            Endpoint::Album => "album/get",
            Endpoint::ArtistPage => "artist/page",
            Endpoint::ArtistReleases => "artist/getReleasesList",
            Endpoint::SimilarArtists => "artist/getSimilarArtists",
            Endpoint::Login => "user/login",
            Endpoint::Playlist => "playlist/get",
            Endpoint::PlaylistCreate => "playlist/create",
            Endpoint::PlaylistDelete => "playlist/delete",
            Endpoint::PlaylistAddTracks => "playlist/addTracks",
            Endpoint::PlaylistDeleteTracks => "playlist/deleteTracks",
            Endpoint::PlaylistUpdatePosition => "playlist/updateTracksPosition",
            Endpoint::Search => "catalog/search",
            Endpoint::Track => "track/get",
            Endpoint::TrackURL => "track/getFileUrl",
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
        };

        f.write_str(endpoint)
    }
}

macro_rules! get {
    ($self:ident, $endpoint:expr, $params:expr) => {
        match $self.make_get_call($endpoint, $params).await {
            Ok(response) => match serde_json::from_str(response.as_str()) {
                Ok(item) => Ok(item),
                Err(error) => Err(Error::DeserializeJSON {
                    message: error.to_string(),
                }),
            },
            Err(error) => Err(Error::Api {
                message: error.to_string(),
            }),
        }
    };
}

macro_rules! post {
    ($self:ident, $endpoint:expr, $form:expr) => {
        match $self.make_post_call($endpoint, $form).await {
            Ok(response) => match serde_json::from_str(response.as_str()) {
                Ok(item) => Ok(item),
                Err(error) => Err(Error::DeserializeJSON {
                    message: error.to_string(),
                }),
            },
            Err(error) => Err(Error::Api {
                message: error.to_string(),
            }),
        }
    };
}

impl Client {
    pub async fn featured_albums(
        &self,
    ) -> Result<Vec<(String, Vec<qobuz_player_models::AlbumSimple>)>> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::AlbumFeatured);

        let make_call = |type_string| {
            let params = vec![("type", type_string), ("offset", "0"), ("limit", "20")];
            let endpoint = endpoint.clone();
            async move { get!(self, &endpoint, Some(&params)) }
        };

        let album_of_the_week: AlbumOfTheWeekQuery = get!(
            self,
            &format!("{}{}", self.base_url, Endpoint::AlbumOfTheWeek),
            None
        )?;

        let album_of_the_week = album_of_the_week
            .items
            .into_iter()
            .map(|a| parse_album_simple(a, &self.max_audio_quality))
            .collect();

        let mut albums = vec![("Album of the week".to_string(), album_of_the_week)];

        let (a, b, c, d) = try_join!(
            make_call("press-awards"),
            make_call("new-releases-full"),
            make_call("qobuzissims"),
            make_call("ideal-discography"),
        )?;

        let mut other = parse_featured_albums(vec![
            ("Press awards".to_string(), a),
            ("New releases".to_string(), b),
            ("Qobuzissims".to_string(), c),
            ("Ideal discography".to_string(), d),
        ]);

        albums.append(&mut other);

        Ok(albums)
    }

    pub async fn featured_playlists(
        &self,
    ) -> Result<Vec<(String, Vec<qobuz_player_models::Playlist>)>> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::PlaylistFeatured);

        let type_string = "editor-picks";

        let params = vec![("type", type_string), ("offset", "0"), ("limit", "20")];

        let response =
            get!(self, &endpoint, Some(&params)).map(|x| vec![("Editor picks".to_string(), x)])?;

        Ok(parse_featured_playlists(
            response,
            self.user_id,
            &self.max_audio_quality,
        ))
    }

    pub async fn user_playlists(&self) -> Result<Vec<qobuz_player_models::Playlist>> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::UserPlaylist);
        let params = vec![("limit", "500"), ("extra", "tracks"), ("offset", "0")];

        let response: UserPlaylistsResult = get!(self, &endpoint, Some(&params))?;

        Ok(response
            .playlists
            .items
            .into_iter()
            .map(|playlist| parse_playlist(playlist, self.user_id, &self.max_audio_quality))
            .collect())
    }

    pub async fn playlist(&self, playlist_id: u32) -> Result<qobuz_player_models::Playlist> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::Playlist);
        let id_string = playlist_id.to_string();
        let params = vec![
            ("limit", "500"),
            ("extra", "tracks"),
            ("playlist_id", id_string.as_str()),
            ("offset", "0"),
        ];
        let response = get!(self, &endpoint, Some(&params))?;

        Ok(parse_playlist(
            response,
            self.user_id,
            &self.max_audio_quality,
        ))
    }

    pub async fn create_playlist(
        &self,
        name: String,
        is_public: bool,
        description: String,
        is_collaborative: Option<bool>,
    ) -> Result<qobuz_player_models::Playlist> {
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

        let response = post!(self, &endpoint, form_data)?;
        Ok(parse_playlist(
            response,
            self.user_id,
            &self.max_audio_quality,
        ))
    }

    pub async fn delete_playlist(&self, playlist_id: u32) -> Result<SuccessfulResponse> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::PlaylistDelete);

        let mut form_data = HashMap::new();
        let playlist_id = playlist_id.to_string();
        form_data.insert("playlist_id", playlist_id.as_str());

        post!(self, &endpoint, form_data)
    }

    pub async fn playlist_add_track(
        &self,
        playlist_id: u32,
        playlist_track_ids: &[u32],
    ) -> Result<qobuz_player_models::Playlist> {
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

        let response = post!(self, &endpoint, form_data)?;
        Ok(parse_playlist(
            response,
            self.user_id,
            &self.max_audio_quality,
        ))
    }

    pub async fn playlist_delete_track(
        &self,
        playlist_id: u32,
        playlist_track_ids: &[u64],
    ) -> Result<qobuz_player_models::Playlist> {
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

        let response = post!(self, &endpoint, form_data)?;
        Ok(parse_playlist(
            response,
            self.user_id,
            &self.max_audio_quality,
        ))
    }

    pub async fn update_playlist_track_position(
        &self,
        index: usize,
        playlist_id: u32,
        playlist_track_id: u64,
    ) -> Result<qobuz_player_models::Playlist> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::PlaylistUpdatePosition);

        let index = index.to_string();
        let playlist_id = playlist_id.to_string();
        let track_id = playlist_track_id.to_string();

        let mut form_data = HashMap::new();
        form_data.insert("playlist_id", playlist_id.as_str());
        form_data.insert("playlist_track_ids", track_id.as_str());
        form_data.insert("insert_before", index.as_str());

        let response = post!(self, &endpoint, form_data)?;
        Ok(parse_playlist(
            response,
            self.user_id,
            &self.max_audio_quality,
        ))
    }

    pub async fn track_url(&self, track_id: u32) -> Result<TrackURL> {
        track_url(
            track_id,
            &self.active_secret,
            &self.base_url,
            &self.http_client,
            &self.app_id,
            &self.user_token,
            &self.max_audio_quality,
        )
        .await
    }

    pub async fn favorites(&self, limit: i32) -> Result<qobuz_player_models::Favorites> {
        let mut favorite_playlists = self.user_playlists().await?;

        let endpoint = format!("{}{}", self.base_url, Endpoint::Favorites);

        let limit = limit.to_string();
        let params = vec![("limit", limit.as_str())];

        let response: Favorites = get!(self, &endpoint, Some(&params))?;

        let Favorites {
            albums,
            tracks,
            artists,
        } = response;

        let mut albums = albums.items;
        albums.sort_by(|a, b| a.artist.name.cmp(&b.artist.name));

        let mut artists = artists.items;
        artists.sort_by(|a, b| a.name.cmp(&b.name));

        favorite_playlists.sort_by(|a, b| a.title.cmp(&b.title));

        Ok(qobuz_player_models::Favorites {
            albums: albums
                .into_iter()
                .map(|x| parse_album(x, &self.max_audio_quality).into())
                .collect(),
            artists: artists.into_iter().map(parse_artist).collect(),
            playlists: favorite_playlists,
            tracks: tracks
                .items
                .into_iter()
                .map(|track| parse_track(track, &self.max_audio_quality))
                .collect(),
        })
    }

    pub async fn add_favorite_track(&self, id: u32) -> Result<SuccessfulResponse> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::FavoriteAdd);
        let mut form_data = HashMap::new();
        let id = id.to_string();
        form_data.insert("track_ids", id.as_str());

        post!(self, &endpoint, form_data)
    }

    pub async fn remove_favorite_track(&self, id: u32) -> Result<SuccessfulResponse> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::FavoriteRemove);
        let mut form_data = HashMap::new();
        let id = id.to_string();
        form_data.insert("track_ids", id.as_str());

        post!(self, &endpoint, form_data)
    }

    pub async fn add_favorite_album(&self, id: &str) -> Result<SuccessfulResponse> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::FavoriteAdd);
        let mut form_data = HashMap::new();
        form_data.insert("album_ids", id);

        post!(self, &endpoint, form_data)
    }

    pub async fn remove_favorite_album(&self, id: &str) -> Result<SuccessfulResponse> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::FavoriteRemove);
        let mut form_data = HashMap::new();
        form_data.insert("album_ids", id);

        post!(self, &endpoint, form_data)
    }

    pub async fn add_favorite_artist(&self, id: u32) -> Result<SuccessfulResponse> {
        let id = id.to_string();
        let endpoint = format!("{}{}", self.base_url, Endpoint::FavoriteAdd);
        let mut form_data = HashMap::new();
        form_data.insert("artist_ids", id.as_str());

        post!(self, &endpoint, form_data)
    }

    pub async fn remove_favorite_artist(&self, id: u32) -> Result<SuccessfulResponse> {
        let id = id.to_string();
        let endpoint = format!("{}{}", self.base_url, Endpoint::FavoriteRemove);
        let mut form_data = HashMap::new();
        form_data.insert("artist_ids", id.as_str());

        post!(self, &endpoint, form_data)
    }

    pub async fn add_favorite_playlist(&self, id: u32) -> Result<SuccessfulResponse> {
        let id = id.to_string();
        let endpoint = format!("{}{}", self.base_url, Endpoint::FavoritePlaylistAdd);
        let mut form_data = HashMap::new();
        form_data.insert("playlist_id", id.as_str());

        post!(self, &endpoint, form_data)
    }

    pub async fn remove_favorite_playlist(&self, id: u32) -> Result<SuccessfulResponse> {
        let id = id.to_string();
        let endpoint = format!("{}{}", self.base_url, Endpoint::FavoritePlaylistRemove);
        let mut form_data = HashMap::new();
        form_data.insert("playlist_id", id.as_str());

        post!(self, &endpoint, form_data)
    }

    pub async fn search_all(
        &self,
        query: &str,
        limit: i32,
    ) -> Result<qobuz_player_models::SearchResults> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::Search);
        let limit = limit.to_string();
        let params = vec![("query", query), ("limit", &limit)];

        let response = get!(self, &endpoint, Some(&params))?;

        Ok(parse_search_results(
            response,
            self.user_id,
            &self.max_audio_quality,
        ))
    }

    pub async fn album(&self, album_id: &str) -> Result<qobuz_player_models::Album> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::Album);
        let params = vec![
            ("album_id", album_id),
            ("extra", "track_ids"),
            ("offset", "0"),
            ("limit", "500"),
        ];

        let response = get!(self, &endpoint, Some(&params))?;

        Ok(parse_album(response, &self.max_audio_quality))
    }

    pub async fn track(&self, track_id: u32) -> Result<qobuz_player_models::Track> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::Track);
        let track_id_string = track_id.to_string();
        let params = vec![("track_id", track_id_string.as_str())];

        let response = get!(self, &endpoint, Some(&params))?;
        Ok(parse_track(response, &self.max_audio_quality))
    }

    pub async fn suggested_albums(
        &self,
        album_id: &str,
    ) -> Result<Vec<qobuz_player_models::AlbumSimple>> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::AlbumSuggest);
        let params = vec![("album_id", album_id)];

        let response: AlbumSuggestionResponse = get!(self, &endpoint, Some(&params))?;

        Ok(response
            .albums
            .items
            .into_iter()
            .map(|x| parse_album_simple(x, &self.max_audio_quality))
            .collect())
    }

    pub async fn artist(&self, artist_id: u32) -> Result<qobuz_player_models::ArtistPage> {
        let app_id = &self.app_id;

        let endpoint = format!("{}{}", self.base_url, Endpoint::ArtistPage);

        let artistid_string = artist_id.to_string();

        let params = vec![
            ("artist_id", artistid_string.as_str()),
            ("app_id", app_id),
            ("sort", "relevant"),
        ];

        let response = get!(self, &endpoint, Some(&params))?;

        Ok(parse_artist_page(response))
    }

    pub async fn similar_artists(
        &self,
        artist_id: u32,
        limit: Option<i32>,
    ) -> Result<Vec<qobuz_player_models::Artist>> {
        let limit = limit.unwrap_or(10).to_string();

        let endpoint = format!("{}{}", self.base_url, Endpoint::SimilarArtists);
        let artistid_string = artist_id.to_string();

        let params = vec![
            ("artist_id", artistid_string.as_str()),
            ("limit", &limit),
            ("offset", "0"),
        ];

        let response: Result<ArtistsResponse> = get!(self, &endpoint, Some(&params));

        Ok(response
            .map(|res| res.artists)?
            .items
            .into_iter()
            .map(parse_artist)
            .collect())
    }

    pub async fn artist_releases(
        &self,
        artist_id: u32,
        limit: Option<i32>,
    ) -> Result<Vec<qobuz_player_models::AlbumSimple>> {
        let endpoint = format!("{}{}", self.base_url, Endpoint::ArtistReleases);
        let limit = limit.unwrap_or(100).to_string();

        let artistid_string = artist_id.to_string();

        let params = vec![
            ("artist_id", artistid_string.as_str()),
            ("limit", &limit),
            ("release_type", "album"),
            ("sort", "release_date"),
            ("offset", "0"),
            ("track_size", "1"),
        ];

        let response: Result<ReleaseQuery> = get!(self, &endpoint, Some(&params));
        let response = response?.items;

        Ok(response
            .into_iter()
            .map(|s| parse_album_simple(s, &self.max_audio_quality))
            .collect())
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

// Check the retrieved secrets to see which one works.
async fn find_active_secret(
    secrets: HashMap<String, String>,
    base_url: &str,
    client: &reqwest::Client,
    app_id: &str,
    user_token: &str,
) -> Result<String> {
    tracing::debug!("testing secrets: {secrets:?}");

    for (timezone, secret) in secrets.into_iter() {
        let response = track_url(
            64868955,
            &secret,
            base_url,
            client,
            app_id,
            user_token,
            &AudioQuality::Mp3,
        )
        .await;

        if response.is_ok() {
            tracing::debug!("found good secret: {}\t{}", timezone, secret);
            let secret_string = secret;

            return Ok(secret_string);
        }
    }

    Err(Error::ActiveSecret)
}

async fn track_url(
    track_id: u32,
    secret: &str,
    base_url: &str,
    client: &reqwest::Client,
    app_id: &str,
    user_token: &str,
    max_audio_quality: &AudioQuality,
) -> Result<TrackURL> {
    let endpoint = format!("{}{}", base_url, Endpoint::TrackURL);
    let now = format!("{}", time::OffsetDateTime::now_utc().unix_timestamp());

    let sig = format!(
        "trackgetFileUrlformat_id{max_audio_quality}intentstreamtrack_id{track_id}{now}{secret}"
    );

    let hashed_sig = format!("{:x}", md5::compute(sig.as_str()));

    let track_id = track_id.to_string();

    let quality_string = max_audio_quality.to_string();

    let params = vec![
        ("request_ts", now.as_str()),
        ("request_sig", hashed_sig.as_str()),
        ("track_id", track_id.as_str()),
        ("format_id", &quality_string),
        ("intent", "stream"),
    ];

    match make_get_call(&endpoint, Some(&params), client, app_id, Some(user_token)).await {
        Ok(response) => match serde_json::from_str(response.as_str()) {
            Ok(item) => Ok(item),
            Err(error) => Err(Error::DeserializeJSON {
                message: error.to_string(),
            }),
        },
        Err(error) => Err(Error::Api {
            message: error.to_string(),
        }),
    }
}

async fn handle_response(response: Response) -> Result<String> {
    if response.status() == StatusCode::OK {
        let res = response.text().await.unwrap_or_default();
        Ok(res)
    } else {
        Err(Error::Api {
            message: response.status().to_string(),
        })
    }
}

async fn make_get_call(
    endpoint: &str,
    params: Option<&[(&str, &str)]>,
    client: &reqwest::Client,
    app_id: &str,
    user_token: Option<&str>,
) -> Result<String> {
    let headers = client_headers(app_id, user_token);

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

struct LoginResult {
    user_token: String,
    user_id: i64,
}

async fn login(
    username: &str,
    password: &str,
    app_id: &str,
    base_url: &str,
    client: &reqwest::Client,
) -> Result<LoginResult> {
    let endpoint = format!("{}{}", base_url, Endpoint::Login);

    tracing::debug!(
        "logging in with email ({}) and password **HIDDEN** for app_id {}",
        username,
        app_id
    );

    let params = vec![
        ("email", username),
        ("password", password),
        ("app_id", app_id),
    ];

    match make_get_call(&endpoint, Some(&params), client, app_id, None).await {
        Ok(response) => {
            let json: Value = serde_json::from_str(response.as_str())
                .or(Err(Error::DeserializeJSON { message: response }))?;
            tracing::info!("Successfully logged in");
            tracing::debug!("{}", json);
            let mut user_token = json["user_auth_token"].to_string();
            user_token = user_token[1..user_token.len() - 1].to_string();

            let user_id =
                json["user"]["id"]
                    .to_string()
                    .parse::<i64>()
                    .or(Err(Error::DeserializeJSON {
                        message: json["user"].to_string(),
                    }))?;

            Ok(LoginResult {
                user_token,
                user_id,
            })
        }
        Err(err) => {
            tracing::error!("error logging into qobuz: {}", err);
            Err(Error::Login)
        }
    }
}

struct Secrets {
    secrets: HashMap<String, String>,
    app_id: String,
}

// ported from https://github.com/vitiko98/qobuz-dl/blob/master/qobuz_dl/bundle.py
// Retrieve the app_id and generate the secrets needed to authenticate
async fn get_secrets(client: &reqwest::Client) -> Result<Secrets> {
    tracing::debug!("fetching login page");
    let play_url = "https://play.qobuz.com";
    let login_page = client.get(format!("{play_url}/login")).send().await?;

    let contents = login_page.text().await.or(Err(Error::Login))?;

    let bundle_regex = regex::Regex::new(
        r#"<script src="(/resources/\d+\.\d+\.\d+-[a-z0-9]\d{3}/bundle\.js)"></script>"#,
    )
    .or(Err(Error::Login))?;

    let app_id_regex = regex::Regex::new(
        r#"production:\{api:\{appId:"(?P<app_id>\d{9})",appSecret:"(?P<app_secret>\w{32})""#,
    )
    .or(Err(Error::Login))?;
    let seed_regex = regex::Regex::new(
        r#"[a-z]\.initialSeed\("(?P<seed>[\w=]+)",window\.utimezone\.(?P<timezone>[a-z]+)\)"#,
    )
    .or(Err(Error::Login))?;

    let mut secrets = HashMap::new();

    let app_id = if let Some(captures) = bundle_regex.captures(contents.as_str()) {
        let bundle_path = captures.get(1).map_or("", |m| m.as_str());
        let bundle_url = format!("{play_url}{bundle_path}");
        if let Ok(bundle_page) = client.get(bundle_url).send().await {
            if let Ok(bundle_contents) = bundle_page.text().await {
                if let Some(captures) = app_id_regex.captures(bundle_contents.as_str()) {
                    let found_app_id = captures
                        .name("app_id")
                        .map_or("".to_string(), |m| m.as_str().to_string());

                    let seed_data = seed_regex.captures_iter(bundle_contents.as_str());

                    seed_data.for_each(|s| {
                            let seed = s.name("seed").map_or("", |m| m.as_str()).to_string();
                            let mut timezone =
                                s.name("timezone").map_or("", |m| m.as_str()).to_string();
                            capitalize(timezone.as_mut_str());

                            let info_regex = format!(r#"name:"\w+/(?P<timezone>{}([a-z]?))",info:"(?P<info>[\w=]+)",extras:"(?P<extras>[\w=]+)""#, &timezone);
                            regex::Regex::new(info_regex.as_str())
                                .expect("Unable to create regex")
                                .captures_iter(bundle_contents.as_str())
                                .for_each(|c| {
                                    let timezone =
                                        c.name("timezone").map_or("", |m| m.as_str()).to_string();
                                    let info =
                                        c.name("info").map_or("", |m| m.as_str()).to_string();
                                    let extras =
                                        c.name("extras").map_or("", |m| m.as_str()).to_string();

                                    let chars = format!("{seed}{info}{extras}");

                                    let encoded_secret = chars[..chars.len() - 44].to_string();
                                    let decoded_secret = general_purpose::URL_SAFE
                                        .decode(encoded_secret)
                                        .expect("failed to decode base64 secret");
                                    let secret_utf8 = std::str::from_utf8(&decoded_secret)
                                        .expect("failed to convert base64 to string")
                                        .to_string();

                                    secrets.insert(timezone, secret_utf8);
                                });
                        });
                    found_app_id
                } else {
                    return Err(Error::AppID);
                }
            } else {
                return Err(Error::AppID);
            }
        } else {
            return Err(Error::AppID);
        }
    } else {
        return Err(Error::AppID);
    };

    Ok(Secrets { secrets, app_id })
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct SuccessfulResponse {
    status: String,
}

fn capitalize(s: &mut str) {
    if let Some(r) = s.get_mut(0..1) {
        r.make_ascii_uppercase();
    }
}

fn parse_featured_albums(
    response: Vec<(String, FeaturedAlbumsResponse)>,
) -> Vec<(String, Vec<qobuz_player_models::AlbumSimple>)> {
    response
        .into_iter()
        .map(|featured| {
            let featured_type = featured.0;

            let albums = featured
                .1
                .albums
                .items
                .into_iter()
                .map(|value| qobuz_player_models::AlbumSimple {
                    id: value.id,
                    title: value.title,
                    artist: parse_artist(value.artist),
                    hires_available: value.hires_streamable,
                    explicit: value.parental_warning,
                    available: value.streamable,
                    image: value.image.large,
                    duration_seconds: value.duration,
                    release_year: extract_year(&value.release_date_original),
                })
                .collect::<Vec<_>>();

            (featured_type, albums)
        })
        .collect()
}

pub fn parse_featured_playlists(
    response: Vec<(String, FeaturedPlaylistsResponse)>,
    user_id: i64,
    max_audio_quality: &AudioQuality,
) -> Vec<(String, Vec<qobuz_player_models::Playlist>)> {
    response
        .into_iter()
        .map(|featured| {
            let featured_type = featured.0;
            let playlists = featured
                .1
                .playlists
                .items
                .into_iter()
                .map(|playlist| parse_playlist(playlist, user_id, max_audio_quality))
                .collect();

            (featured_type, playlists)
        })
        .collect()
}

fn parse_search_results(
    search_results: SearchAllResults,
    user_id: i64,
    max_audio_quality: &AudioQuality,
) -> qobuz_player_models::SearchResults {
    qobuz_player_models::SearchResults {
        query: search_results.query,
        albums: search_results
            .albums
            .items
            .into_iter()
            .map(|a| parse_album(a, max_audio_quality))
            .collect(),
        artists: search_results
            .artists
            .items
            .into_iter()
            .map(parse_artist)
            .collect(),
        playlists: search_results
            .playlists
            .items
            .into_iter()
            .map(|p| parse_playlist(p, user_id, max_audio_quality))
            .collect(),
        tracks: search_results
            .tracks
            .items
            .into_iter()
            .map(|t| parse_track(t, max_audio_quality))
            .collect(),
    }
}

fn parse_album_simple(
    s: AlbumSuggestion,
    max_audio_quality: &AudioQuality,
) -> qobuz_player_models::AlbumSimple {
    let artist = s.artists.and_then(|vec| vec.into_iter().next());
    let (artist_id, artist_name) = artist.map_or((0, "Unknown".into()), |artist| {
        (artist.id as u32, artist.name)
    });

    qobuz_player_models::AlbumSimple {
        id: s.id,
        title: s.title,
        artist: qobuz_player_models::Artist {
            id: artist_id,
            name: artist_name,
            ..Default::default()
        },
        hires_available: hifi_available(s.rights.hires_streamable, max_audio_quality),
        explicit: s.parental_warning,
        available: s.rights.streamable,
        image: s.image.large,
        duration_seconds: s.duration,
        release_year: extract_year(&s.dates.original),
    }
}

fn extract_year(date_str: &str) -> u32 {
    let format = format_description!("[year]-[month]-[day]");
    let date = time::Date::parse(date_str, &format).expect("failed to parse date");
    date.year() as u32
}

fn parse_album(
    value: crate::qobuz_models::album::Album,
    max_audio_quality: &AudioQuality,
) -> qobuz_player_models::Album {
    let year = extract_year(&value.release_date_original);

    let tracks = value.tracks.map_or(Default::default(), |tracks| {
        tracks
            .items
            .into_iter()
            .map(|t| qobuz_player_models::Track {
                id: t.id,
                title: t.title,
                number: t.track_number,
                explicit: t.parental_warning,
                hires_available: t.hires_streamable,
                available: t.streamable,
                status: Default::default(),
                image: Some(value.image.large.clone()),
                image_thumbnail: Some(value.image.small.clone()),
                duration_seconds: t.duration,
                artist_name: Some(value.artist.name.clone()),
                artist_id: Some(value.artist.id),
                album_title: Some(value.title.clone()),
                album_id: Some(value.id.clone()),
                playlist_track_id: None,
            })
            .collect()
    });

    qobuz_player_models::Album {
        id: value.id,
        title: value.title,
        artist: parse_artist(value.artist),
        total_tracks: value.tracks_count as u32,
        release_year: year
            .to_string()
            .parse::<u32>()
            .expect("error converting year"),
        hires_available: hifi_available(value.hires_streamable, max_audio_quality),
        explicit: value.parental_warning,
        available: value.streamable,
        tracks,
        image: value.image.large,
        image_thumbnail: value.image.small,
        duration_seconds: value.duration.map_or(0, |duration| duration as u32),
        description: sanitize_html(value.description),
    }
}

fn sanitize_html(source: Option<String>) -> Option<String> {
    let source = source?;
    if source.trim() == "" {
        return None;
    }

    let mut data = String::new();
    let mut inside = false;

    for c in source.chars() {
        if c == '<' {
            inside = true;
            continue;
        }
        if c == '>' {
            inside = false;
            continue;
        }

        if !inside {
            data.push(c);
        }
    }

    Some(data.replace("&copy", "©"))
}

fn image_to_string(value: artist_page::Image) -> String {
    format!(
        "https://static.qobuz.com/images/artists/covers/large/{}.{}",
        value.hash, value.format
    )
}

fn parse_artist_page(value: artist_page::ArtistPage) -> qobuz_player_models::ArtistPage {
    let artist_image_url = value.images.portrait.map(image_to_string);

    qobuz_player_models::ArtistPage {
        id: value.id,
        name: value.name.display.clone(),
        image: artist_image_url.clone(),
        top_tracks: value
            .top_tracks
            .into_iter()
            .map(|t| {
                let album_image_url = t.album.image.large;
                let album_image_url_small = t.album.image.small;
                qobuz_player_models::Track {
                    id: t.id,
                    number: t.physical_support.track_number,
                    title: t.title,
                    explicit: t.parental_warning,
                    hires_available: t.rights.hires_streamable,
                    available: t.rights.streamable,
                    status: Default::default(),
                    image: Some(album_image_url),
                    image_thumbnail: Some(album_image_url_small),
                    duration_seconds: t.duration,
                    artist_name: Some(value.name.display.clone()),
                    artist_id: Some(value.id),
                    album_title: Some(t.album.title),
                    album_id: Some(t.album.id),
                    playlist_track_id: None,
                }
            })
            .collect(),
        description: sanitize_html(value.biography.map(|bio| bio.content)),
    }
}

fn parse_artist(value: artist::Artist) -> qobuz_player_models::Artist {
    qobuz_player_models::Artist {
        id: value.id,
        name: value.name,
        image: value.image.map(|i| i.large),
    }
}

fn parse_playlist(
    playlist: playlist::Playlist,
    user_id: i64,
    max_audio_quality: &AudioQuality,
) -> qobuz_player_models::Playlist {
    let tracks = playlist.tracks.map_or(Default::default(), |tracks| {
        tracks
            .items
            .into_iter()
            .map(|t| parse_track(t, max_audio_quality))
            .collect()
    });

    let image = if let Some(image) = playlist.image_rectangle.first() {
        Some(image.clone())
    } else if let Some(images) = playlist.images300 {
        images.first().cloned()
    } else {
        None
    };

    qobuz_player_models::Playlist {
        id: playlist.id as u32,
        is_owned: user_id == playlist.owner.id,
        title: playlist.name,
        duration_seconds: playlist.duration as u32,
        tracks_count: playlist.tracks_count as u32,
        image,
        tracks,
    }
}

fn parse_track(
    value: track::Track,
    max_audio_quality: &AudioQuality,
) -> qobuz_player_models::Track {
    let artist = if let Some(p) = &value.performer {
        Some(qobuz_player_models::Artist {
            id: p.id as u32,
            name: p.name.clone(),
            image: None,
        })
    } else {
        value.album.as_ref().map(|a| parse_artist(a.clone().artist))
    };

    let image = value.album.as_ref().map(|a| a.image.large.clone());
    let image_thumbnail = value.album.as_ref().map(|a| a.image.small.clone());

    qobuz_player_models::Track {
        id: value.id,
        number: value.track_number,
        title: value.title,
        duration_seconds: value.duration,
        explicit: value.parental_warning,
        hires_available: hifi_available(value.hires_streamable, max_audio_quality),
        available: value.streamable,
        status: Default::default(),
        image,
        image_thumbnail,
        artist_name: artist.as_ref().map(move |a| a.name.clone()),
        artist_id: artist.as_ref().map(move |a| a.id),
        album_title: value.album.as_ref().map(|a| a.title.clone()),
        album_id: value.album.as_ref().map(|a| a.id.clone()),
        playlist_track_id: value.playlist_track_id,
    }
}

fn hifi_available(track_has_hires_available: bool, max_audio_quality: &AudioQuality) -> bool {
    if !track_has_hires_available {
        return false;
    }

    match max_audio_quality {
        AudioQuality::Mp3 => false,
        AudioQuality::CD => false,
        AudioQuality::HIFI96 => true,
        AudioQuality::HIFI192 => true,
    }
}
