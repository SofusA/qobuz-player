use assets::static_handler;
use axum::{
    Router,
    extract::State,
    response::{Html, Sse, sse::Event},
    routing::get,
};
use futures::stream::Stream;
use qobuz_player_controls::{
    PositionReceiver, Result, Status, StatusReceiver, TracklistReceiver, VolumeReceiver,
    client::Client,
    controls::Controls,
    error::Error,
    notification::{Notification, NotificationBroadcast},
};
use qobuz_player_models::{Album, AlbumSimple, Playlist};
use qobuz_player_rfid::RfidState;
use serde_json::json;
use std::{
    convert::Infallible,
    env,
    sync::{Arc, RwLock},
};
use tera::Context;
use tokio::sync::{
    broadcast::{self, Receiver, Sender},
    watch,
};
use tokio_stream::StreamExt as _;
use tokio_stream::wrappers::BroadcastStream;

use crate::{
    app_state::AppState,
    routes::{api, controls, favorites, now_playing, queue},
    views::templates,
};

mod app_state;
mod assets;
mod routes;
mod views;

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

    axum::serve(listener, router).await.expect("infallible");
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

    let template_path = {
        let current_dir = env::current_dir().expect("Failed to get current directory");
        current_dir.join("qobuz-player-web/templates")
    };

    let templates = templates(&template_path);

    let (templates_tx, templates_rx) = watch::channel(templates);

    let templates_clone = templates_rx.clone();
    let broadcast_clone = broadcast.clone();

    #[cfg(all(debug_assertions, target_os = "linux"))]
    {
        let watcher_sender = tx.clone();
        let watcher = filesentry::Watcher::new().unwrap();
        watcher.add_root(&template_path, true, |_| ()).unwrap();

        watcher.add_handler(move |events| {
            for event in &*events {
                match event.ty {
                    filesentry::EventType::Modified | filesentry::EventType::Create => {
                        let mut templates = templates_clone.borrow().clone();

                        match templates.full_reload().is_ok() {
                            true => {
                                templates_tx.send(templates).unwrap();
                            }
                            false => {
                                broadcast_clone.send_error("Error parsing templates".into());
                            }
                        };

                        let event = ServerSentEvent {
                            event_name: "reload".into(),
                            event_data: "template changed".into(),
                        };

                        _ = watcher_sender.send(event);
                    }
                    _ => (),
                }
            }
            true
        });
        watcher.start();
    }

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
        templates: templates_rx,
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
        .merge(queue::routes())
        .merge(api::routes())
        // .merge(search::routes())
        // .merge(album::routes())
        // .merge(artist::routes())
        // .merge(playlist::routes())
        .merge(favorites::routes())
        // .merge(queue::routes())
        // .merge(discover::routes())
        .merge(controls::routes())
        // .layer(axum::middleware::from_fn_with_state(
        //     shared_state.clone(),
        //     auth::auth_middleware,
        // ))
        .route("/assets/{*file}", get(static_handler))
        // .merge(auth::routes())
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
                    event_data: "new tracklist".into(),
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
                tracing::info!("notification: {:?}", notification);
                // if let Ok(message) = notification {
                //     let toast = components::toast(message.clone()).to_html();

                //     let event = match message {
                //         qobuz_player_controls::notification::Notification::Error(_) => ServerSentEvent {
                //             event_name: "error".into(),
                //             event_data: toast,
                //         },
                //         qobuz_player_controls::notification::Notification::Warning(_) => {
                //             ServerSentEvent {
                //                 event_name: "warn".into(),
                //                 event_data: toast,
                //             }
                //         }
                //         qobuz_player_controls::notification::Notification::Success(_) => {
                //             ServerSentEvent {
                //                 event_name: "success".into(),
                //                 event_data: toast,
                //             }
                //         }
                //         qobuz_player_controls::notification::Notification::Info(_) => ServerSentEvent {
                //             event_name: "info".into(),
                //             event_data: toast,
                //         },
                //     };
                //     _ = tx.send(event);
                // }
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
    headers.insert("X-Accel-Buffering", "no".parse().expect("infallible"));

    (headers, Sse::new(stream))
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

type ResponseResult =
    std::result::Result<axum::response::Html<String>, axum::response::Html<String>>;

fn ok_or_error_component<T>(
    state: &AppState,
    value: Result<T, qobuz_player_controls::error::Error>,
) -> Result<T, axum::response::Html<String>> {
    match value {
        Ok(value) => Ok(value),
        Err(err) => {
            let err = format!("{err}");
            Err(Html(
                state
                    .templates
                    .borrow()
                    .render(
                        "error.html",
                        &Context::from_serialize(json!({"error": err})).unwrap(),
                    )
                    .unwrap(),
            ))
        }
    }
}
// Err(Html(
//             state
//                 .templates
//                 .render(View::Error.name(), &json!({error: err})),
//         ))
// #[allow(clippy::result_large_err)]
// fn ok_or_broadcast<T>(
//     broadcast: &NotificationBroadcast,
//     value: Result<T, qobuz_player_controls::error::Error>,
// ) -> Result<T, axum::response::Response> {
//     match value {
//         Ok(value) => Ok(value),
//         Err(err) => {
//             broadcast.send(Notification::Error(format!("{err}")));

//             let mut response = render(html! { <div></div> });
//             let headers = response.headers_mut();
//             headers.insert("HX-Reswap", "none".try_into().expect("infallible"));

//             Err(response)
//         }
//     }
// }
