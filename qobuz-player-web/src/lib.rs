use assets::static_handler;
use axum::{
    Router,
    extract::State,
    response::{Sse, sse::Event},
    routing::get,
};
use futures::stream::Stream;
use leptos::*;
use leptos::{html::*, prelude::RenderHtml};
use qobuz_player_controls::{
    Broadcast,
    models::{Album, AlbumSimple, Favorites, Playlist},
    notification::Notification,
    tracklist,
};
use routes::{
    album, artist, auth, controls, discover, favorites, now_playing, playlist, queue, search,
};
use std::{convert::Infallible, sync::Arc};
use time::Duration;
use tokio::{
    join,
    sync::{
        RwLock,
        broadcast::{self, Sender},
    },
    time::Instant,
};
use tokio_stream::StreamExt as _;
use tokio_stream::wrappers::BroadcastStream;

mod assets;
mod components;
mod icons;
mod page;
mod routes;
mod view;

pub async fn init(state: Arc<qobuz_player_state::State>) {
    let listener = tokio::net::TcpListener::bind(&state.web_interface)
        .await
        .unwrap();

    let router = create_router(state.clone()).await;

    axum::serve(listener, router).await.unwrap();
}

async fn create_router(state: Arc<qobuz_player_state::State>) -> Router {
    let (tx, _rx) = broadcast::channel::<ServerSentEvent>(100);

    let album_cache = moka::future::CacheBuilder::new(1000)
        .time_to_live(std::time::Duration::from_secs(60 * 60 * 24 * 7))
        .build();

    let shared_state = Arc::new(AppState {
        tx: tx.clone(),
        player_state: state.clone(),
        favorites_cache: Cache::new(Duration::weeks(1)),
        discover_cache: Cache::new(Duration::days(1)),
        album_cache,
    });
    tokio::spawn(background_task(tx, state.broadcast.clone()));

    axum::Router::new()
        .route("/sse", get(sse_handler))
        .merge(now_playing::routes())
        .merge(search::routes())
        .merge(album::routes())
        .merge(artist::routes())
        .merge(playlist::routes())
        .merge(favorites::routes())
        .merge(queue::routes())
        .merge(discover::routes())
        .merge(controls::routes())
        .layer(axum::middleware::from_fn_with_state(
            shared_state.clone(),
            auth::auth_middleware,
        ))
        .route("/assets/{*file}", get(static_handler))
        .merge(auth::routes())
        .with_state(shared_state.clone())
}

async fn background_task(tx: Sender<ServerSentEvent>, receiver: Arc<Broadcast>) {
    let mut receiver = receiver.notify_receiver();

    loop {
        if let Ok(notification) = receiver.recv().await {
            match notification {
                Notification::Status { status } => {
                    let message_data = match status {
                        tracklist::Status::Stopped => "pause",
                        tracklist::Status::Paused => "pause",
                        tracklist::Status::Playing => "play",
                    };

                    let event = ServerSentEvent {
                        event_name: "status".into(),
                        event_data: message_data.into(),
                    };
                    _ = tx.send(event);
                }
                Notification::Position { position } => {
                    let event = ServerSentEvent {
                        event_name: "position".into(),
                        event_data: position.mseconds().to_string(),
                    };

                    _ = tx.send(event);
                }
                Notification::CurrentTrackList { tracklist: _ } => {
                    let event = ServerSentEvent {
                        event_name: "tracklist".into(),
                        event_data: Default::default(),
                    };
                    _ = tx.send(event);
                }
                Notification::Quit => break,
                Notification::Message { message } => {
                    let toast = components::toast(message.clone()).to_html();

                    let event = match message {
                        qobuz_player_controls::notification::Message::Error(_) => ServerSentEvent {
                            event_name: "error".into(),
                            event_data: toast,
                        },
                        qobuz_player_controls::notification::Message::Warning(_) => {
                            ServerSentEvent {
                                event_name: "warn".into(),
                                event_data: toast,
                            }
                        }
                        qobuz_player_controls::notification::Message::Success(_) => {
                            ServerSentEvent {
                                event_name: "success".into(),
                                event_data: toast,
                            }
                        }
                        qobuz_player_controls::notification::Message::Info(_) => ServerSentEvent {
                            event_name: "info".into(),
                            event_data: toast,
                        },
                    };
                    _ = tx.send(event);
                }
                Notification::Volume { volume } => {
                    let volume = (volume * 100.0) as u32;
                    let event = ServerSentEvent {
                        event_name: "volume".into(),
                        event_data: volume.to_string(),
                    };
                    _ = tx.send(event);
                }
                Notification::Play(_) => (),
            };
        }
    }
}

async fn sse_handler(
    State(state): State<Arc<AppState>>,
) -> (
    axum::http::HeaderMap,
    Sse<impl Stream<Item = Result<Event, Infallible>>>,
) {
    let rx = state.tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(event) => Some(Ok(Event::default()
            .event(event.event_name)
            .data(event.event_data))),
        Err(_) => None,
    });

    let mut headers = axum::http::HeaderMap::new();
    headers.insert("X-Accel-Buffering", "no".parse().unwrap());

    (headers, Sse::new(stream))
}

pub(crate) struct AppState {
    tx: Sender<ServerSentEvent>,
    pub player_state: Arc<qobuz_player_state::State>,
    pub favorites_cache: Cache<Favorites>,
    pub discover_cache: Cache<Discover>,
    pub album_cache: moka::future::Cache<String, AlbumData>,
}

impl AppState {
    pub async fn get_favorites(&self) -> Favorites {
        if let Some(cached) = self.favorites_cache.get().await {
            return cached;
        }

        let favorites = self.player_state.client.favorites().await.unwrap();

        self.favorites_cache.set(favorites.clone()).await;

        favorites
    }

    pub async fn get_album(&self, id: &str) -> AlbumData {
        if let Some(cached) = self.album_cache.get(&id.to_string()) {
            return cached;
        };

        let (album, suggested_albums) = join!(
            self.player_state.client.album(id),
            self.player_state.client.suggested_albums(id),
        );

        let album = album.unwrap();
        let suggested_albums = suggested_albums.unwrap();

        let data = AlbumData {
            album,
            suggested_albums,
        };

        self.album_cache.insert(id.to_string(), data.clone()).await;

        data
    }

    pub async fn is_album_favorite(&self, id: &str) -> bool {
        let favorites = self.get_favorites().await;
        favorites.albums.iter().any(|album| album.id == id)
    }
}

#[derive(Clone)]
pub(crate) struct AlbumData {
    pub album: Album,
    pub suggested_albums: Vec<AlbumSimple>,
}

#[derive(Clone)]
pub(crate) struct ServerSentEvent {
    event_name: String,
    event_data: String,
}

#[derive(Clone)]
pub(crate) struct Discover {
    pub albums: Vec<(String, Vec<AlbumSimple>)>,
    pub playlists: Vec<(String, Vec<Playlist>)>,
}

pub(crate) struct Cache<T> {
    value: RwLock<Option<T>>,
    ttl: Duration,
    created: RwLock<Option<Instant>>,
}

impl<T> Cache<T> {
    pub fn new(ttl: Duration) -> Self {
        Self {
            value: RwLock::new(None),
            ttl,
            created: RwLock::new(None),
        }
    }

    pub async fn get(&self) -> Option<T>
    where
        T: Clone,
    {
        if self.valid().await {
            self.value.read().await.clone()
        } else {
            None
        }
    }

    pub async fn set(&self, value: T) {
        let mut val_lock = self.value.write().await;
        let mut time_lock = self.created.write().await;
        *val_lock = Some(value);
        *time_lock = Some(Instant::now());
    }

    pub async fn clear(&self) {
        let mut val_lock = self.value.write().await;
        let mut time_lock = self.created.write().await;
        *val_lock = None;
        *time_lock = None;
    }

    async fn valid(&self) -> bool {
        let time_lock = self.created.read().await;
        match *time_lock {
            Some(created) => created.elapsed() < self.ttl,
            None => false,
        }
    }
}
