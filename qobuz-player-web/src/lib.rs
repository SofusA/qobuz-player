use assets::static_handler;
use axum::{
    extract::State,
    response::{sse::Event, Sse},
    routing::get,
    Router,
};
use futures::stream::Stream;
use leptos::html::*;
use leptos::*;
use qobuz_player_controls::notification::Notification;
use routes::{album, artist, favorites, now_playing, playlist, queue, search};
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
    println!("Lisening on {address}");
    let router = create_router().await;
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            let mut broadcast_receiver = qobuz_player_controls::notify_receiver();

            loop {
                if let Some(message) = broadcast_receiver.next().await {
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

    let router = axum::Router::new()
        .merge(now_playing::routes())
        .merge(search::routes())
        .merge(album::routes())
        .merge(artist::routes())
        .merge(playlist::routes())
        .merge(favorites::routes())
        .merge(queue::routes())
        .route("/sse", get(sse_handler))
        .route("/assets/{*file}", get(static_handler));

    router.with_state(shared_state)
}

async fn background_task(tx: Sender<ServerSentEvent>) {
    let mut receiver = qobuz_player_controls::notify_receiver();

    loop {
        if let Ok(notification) = receiver.recv().await {
            if let Notification::Status { status } = &notification {
                let event = ServerSentEvent {
                    event_name: "status".into(),
                    event_data: if status == &gstreamer::State::Playing {
                        "playing".into()
                    } else {
                        "paused".into()
                    },
                };
                _ = tx.send(event);
            }

            match notification {
                Notification::Buffering {
                    is_buffering: _,
                    percent: _,
                    target_state: _,
                } => {}
                Notification::Status { status } => {
                    let message_data = match status {
                        gstreamer::State::VoidPending => "pause",
                        gstreamer::State::Null => "pause",
                        gstreamer::State::Ready => "pause",
                        gstreamer::State::Paused => "pause",
                        gstreamer::State::Playing => "play",
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
                Notification::CurrentTrackList { list } => {
                    let serialized = serde_json::to_string(&list).unwrap_or("".into());

                    let event = ServerSentEvent {
                        event_name: "tracklist".into(),
                        event_data: serialized,
                    };
                    _ = tx.send(event);
                }
                Notification::Quit => (),
                Notification::Loading {
                    is_loading: _,
                    target_state: _,
                } => {}
                Notification::Error { error: _ } => (),
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
