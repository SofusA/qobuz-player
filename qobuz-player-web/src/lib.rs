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
    PositionReceiver, Result, Status, StatusReceiver, TracklistReceiver, VolumeReceiver,
    client::Client,
    controls::Controls,
    error::Error,
    notification::{Notification, NotificationBroadcast},
};
use qobuz_player_models::{Album, AlbumSimple, Favorites, Playlist};
use qobuz_player_rfid::RfidState;
use routes::{
    album, artist, auth, controls, discover, favorites, now_playing, playlist, queue, search,
};
use std::{convert::Infallible, sync::Arc};
use tokio::{
    sync::broadcast::{self, Receiver, Sender},
    try_join,
};
use tokio_stream::StreamExt as _;
use tokio_stream::wrappers::BroadcastStream;

use crate::view::render;

mod assets;
mod components;
mod icons;
mod page;
mod routes;
mod view;

#[allow(clippy::too_many_arguments)]
pub async fn init(
    controls: Controls,
    position_receiver: PositionReceiver,
    tracklist_receiver: TracklistReceiver,
    volume_receiver: VolumeReceiver,
    status_receiver: StatusReceiver,
    port: u16,
    web_secret: Option<String>,
    rfid_state: Option<RfidState>,
    broadcast: Arc<NotificationBroadcast>,
    client: Arc<Client>,
) -> Result<()> {
    let interface = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&interface)
        .await
        .or(Err(Error::PortInUse { port }))?;

    let router = create_router(
        controls,
        position_receiver,
        tracklist_receiver,
        volume_receiver,
        status_receiver,
        web_secret,
        rfid_state,
        broadcast,
        client,
    )
    .await;

    axum::serve(listener, router).await.expect("infailable");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn create_router(
    controls: Controls,
    position_receiver: PositionReceiver,
    tracklist_receiver: TracklistReceiver,
    volume_receiver: VolumeReceiver,
    status_receiver: StatusReceiver,
    web_secret: Option<String>,
    rfid_state: Option<RfidState>,
    broadcast: Arc<NotificationBroadcast>,
    client: Arc<Client>,
) -> Router {
    let (tx, _rx) = broadcast::channel::<ServerSentEvent>(100);
    let broadcast_subscribe = broadcast.subscribe();
    let shared_state = Arc::new(AppState {
        controls,
        web_secret,
        rfid_state,
        broadcast,
        client,
        tx: tx.clone(),
        position_receiver: position_receiver.clone(),
        tracklist_receiver: tracklist_receiver.clone(),
        volume_receiver: volume_receiver.clone(),
        status_receiver: status_receiver.clone(),
    });
    tokio::spawn(background_task(
        tx,
        broadcast_subscribe,
        position_receiver,
        tracklist_receiver,
        volume_receiver,
        status_receiver,
    ));

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

async fn background_task(
    tx: Sender<ServerSentEvent>,
    mut receiver: Receiver<Notification>,
    mut position: PositionReceiver,
    mut tracklist: TracklistReceiver,
    mut volume: VolumeReceiver,
    mut status: StatusReceiver,
) {
    loop {
        tokio::select! {
            Ok(_) = position.changed() => {
                let position_duration = position.borrow_and_update();
                let event = ServerSentEvent {
                    event_name: "position".into(),
                    event_data: position_duration.as_millis().to_string(),
                };

                _ = tx.send(event);
            },
            Ok(_) = tracklist.changed() => {
                _ = tracklist.borrow_and_update();
                let event = ServerSentEvent {
                    event_name: "tracklist".into(),
                    event_data: Default::default(),
                };
                _ = tx.send(event);
            },
            Ok(_) = volume.changed() => {
                let volume = *volume.borrow_and_update();
                let volume = (volume * 100.0) as u32;
                let event = ServerSentEvent {
                    event_name: "volume".into(),
                    event_data: volume.to_string(),
                };
                _ = tx.send(event);
            }
            Ok(_) = status.changed() => {
                let status = status.borrow_and_update();
                let message_data = match *status {
                    Status::Paused => "pause",
                    Status::Playing => "play",
                    Status::Buffering => "buffering",
                };

                let event = ServerSentEvent {
                    event_name: "status".into(),
                    event_data: message_data.into(),
                };
                _ = tx.send(event);
            }
            notification = receiver.recv() => {
                if let Ok(message) = notification {
                    let toast = components::toast(message.clone()).to_html();

                    let event = match message {
                        qobuz_player_controls::notification::Notification::Error(_) => ServerSentEvent {
                            event_name: "error".into(),
                            event_data: toast,
                        },
                        qobuz_player_controls::notification::Notification::Warning(_) => {
                            ServerSentEvent {
                                event_name: "warn".into(),
                                event_data: toast,
                            }
                        }
                        qobuz_player_controls::notification::Notification::Success(_) => {
                            ServerSentEvent {
                                event_name: "success".into(),
                                event_data: toast,
                            }
                        }
                        qobuz_player_controls::notification::Notification::Info(_) => ServerSentEvent {
                            event_name: "info".into(),
                            event_data: toast,
                        },
                    };
                    _ = tx.send(event);
                }
            }
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
    headers.insert("X-Accel-Buffering", "no".parse().expect("infailable"));

    (headers, Sse::new(stream))
}

pub(crate) struct AppState {
    tx: Sender<ServerSentEvent>,
    pub(crate) web_secret: Option<String>,
    pub(crate) rfid_state: Option<RfidState>,
    pub(crate) broadcast: Arc<NotificationBroadcast>,
    pub(crate) client: Arc<Client>,
    pub(crate) controls: Controls,
    pub(crate) position_receiver: PositionReceiver,
    pub(crate) tracklist_receiver: TracklistReceiver,
    pub(crate) status_receiver: StatusReceiver,
    pub(crate) volume_receiver: VolumeReceiver,
}

impl AppState {
    pub async fn get_favorites(&self) -> Result<Favorites> {
        self.client.favorites().await
    }

    pub async fn get_album(&self, id: &str) -> Result<AlbumData> {
        let (album, suggested_albums) =
            try_join!(self.client.album(id), self.client.suggested_albums(id))?;

        Ok(AlbumData {
            album,
            suggested_albums,
        })
    }

    pub async fn is_album_favorite(&self, id: &str) -> Result<bool> {
        let favorites = self.get_favorites().await?;
        Ok(favorites.albums.iter().any(|album| album.id == id))
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

type ResponseResult = std::result::Result<axum::response::Response, axum::response::Response>;

#[allow(clippy::result_large_err)]
fn ok_or_error_component<T>(
    value: Result<T, qobuz_player_controls::error::Error>,
) -> Result<T, axum::response::Response> {
    match value {
        Ok(value) => Ok(value),
        Err(err) => Err(render(html! { <div>{format!("{err}")}</div> })),
    }
}

#[allow(clippy::result_large_err)]
fn ok_or_broadcast<T>(
    broadcast: &NotificationBroadcast,
    value: Result<T, qobuz_player_controls::error::Error>,
) -> Result<T, axum::response::Response> {
    match value {
        Ok(value) => Ok(value),
        Err(err) => {
            broadcast.send(Notification::Error(format!("{err}")));

            let mut response = render(html! { <div></div> });
            let headers = response.headers_mut();
            headers.insert("HX-Reswap", "none".try_into().expect("infailable"));

            Err(response)
        }
    }
}
