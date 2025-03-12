use assets::static_handler;
use axum::{
    extract::State,
    response::{sse::Event, Sse},
    routing::get,
    Router,
};
use futures::stream::Stream;
use leptos::*;
use leptos::{html::*, prelude::RenderHtml};
use qobuz_player_controls::{notification::Notification, tracklist};
use routes::{album, artist, controls, discover, favorites, now_playing, playlist, queue, search};
use std::{convert::Infallible, sync::Arc};
use tokio::sync::broadcast::{self, Sender};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt as _;

mod assets;
mod components;
mod icons;
mod page;
mod routes;
mod view;

pub fn is_htmx_request(headers: &axum::http::HeaderMap) -> bool {
    headers.get("HX-Request").is_some() && headers.get("HX-Boosted").is_none()
}

pub async fn init(address: String) {
    tracing::info!("Listening on {address}");
    let router = create_router().await;
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            let mut broadcast_receiver = qobuz_player_controls::notify_receiver();

            loop {
                if let Ok(message) = broadcast_receiver.recv().await {
                    if message == Notification::Quit {
                        break;
                    }
                }
            }
        })
        .await
        .unwrap();
}

async fn create_router() -> Router {
    let (tx, _rx) = broadcast::channel::<ServerSentEvent>(100);
    let shared_state = Arc::new(AppState { tx: tx.clone() });
    tokio::spawn(background_task(tx));

    axum::Router::new()
        .route("/sse", get(sse_handler))
        .with_state(shared_state)
        .merge(now_playing::routes())
        .merge(search::routes())
        .merge(album::routes())
        .merge(artist::routes())
        .merge(playlist::routes())
        .merge(favorites::routes())
        .merge(queue::routes())
        .merge(discover::routes())
        .merge(controls::routes())
        .route("/assets/{*file}", get(static_handler))
}

async fn background_task(tx: Sender<ServerSentEvent>) {
    let mut receiver = qobuz_player_controls::notify_receiver();

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
                Notification::Position { clock } => {
                    let event = ServerSentEvent {
                        event_name: "position".into(),
                        event_data: clock.seconds().to_string(),
                    };
                    _ = tx.send(event);
                }
                Notification::CurrentTrackList { list: _ } => {
                    let event = ServerSentEvent {
                        event_name: "tracklist".into(),
                        event_data: Default::default(),
                    };
                    _ = tx.send(event);
                }
                Notification::Quit => (),
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
                    let event = ServerSentEvent {
                        event_name: "volume".into(),
                        event_data: volume.to_string(),
                    };
                    _ = tx.send(event);
                }
            };
        }
    }
}

async fn sse_handler(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(event) => Some(Ok(Event::default()
            .event(event.event_name)
            .data(event.event_data))),
        Err(_) => None,
    });

    Sse::new(stream)
}

pub struct AppState {
    pub tx: Sender<ServerSentEvent>,
}

#[derive(Clone)]
pub struct ServerSentEvent {
    event_name: String,
    event_data: String,
}
