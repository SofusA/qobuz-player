use assets::static_handler;
use axum::{
    Router,
    extract::State,
    response::{Html, Sse, sse::Event},
    routing::get,
};
use futures::stream::Stream;
use handlebars::{Handlebars, handlebars_helper};
use qobuz_player_controls::{
    PositionReceiver, Result, Status, StatusReceiver, TracklistReceiver, VolumeReceiver,
    client::Client,
    controls::Controls,
    error::Error,
    notification::{Notification, NotificationBroadcast},
};
use qobuz_player_models::{Album, AlbumSimple, Favorites, Playlist};
use qobuz_player_rfid::RfidState;
use std::{convert::Infallible, env, path::Path, sync::Arc};
use tokio::{
    sync::broadcast::{self, Receiver, Sender},
    try_join,
};
use tokio_stream::StreamExt as _;
use tokio_stream::wrappers::BroadcastStream;

use crate::routes::now_playing;

mod assets;
// mod components;
// mod icons;
// mod page;
mod routes;
// mod view;

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

macro_rules! views {
    ( $( $name:ident => $path:expr ),+ $(,)? ) => {
        #[derive(Clone, Copy, Debug)]
        pub(crate) enum View {
            $( $name ),+
        }

        impl View {
            pub fn path(self) -> &'static str {
                match self {
                    $( View::$name => $path ),+
                }
            }

            pub fn iter() -> impl Iterator<Item = View> {
                [ $( View::$name ),+ ].into_iter()
            }
        }
    }
}

views! {
    Page => "page.hbs",
    NowPlaying => "now-playing.hbs",
    NowPlayingPartial => "now-playing-partial.hbs",
    LoadingSpinner => "icons/loading-spinner.hbs",
    VolumeSlider => "volume-slider.hbs",
    PlayPause => "play-pause.hbs",
    Play => "play.hbs",
    Pause => "pause.hbs",
    Next => "next.hbs",
    Previous => "previous.hbs",
    Progress => "progress.hbs",
    PlayerState => "player-state.hbs",
    Info => "info.hbs",
}

impl View {
    pub(crate) fn name(&self) -> String {
        self.path()
            .split("/")
            .last()
            .unwrap()
            .trim_end_matches(".hbs")
            .into()
    }
}

#[cfg(not(debug_assertions))]
#[derive(RustEmbed)]
#[folder = "templates"]
struct Templates;

fn templates(root_dir: &Path) -> Handlebars<'static> {
    let mut reg = Handlebars::new();
    #[cfg(debug_assertions)]
    reg.set_dev_mode(true);

    for file in View::iter() {
        let name = file.name();

        #[cfg(debug_assertions)]
        {
            let mut path = root_dir.to_path_buf();
            path.push(file.path());
            reg.register_template_file(&name, path).unwrap();
        }

        #[cfg(not(debug_assertions))]
        {
            let content = Templates::get(&file.path()).unwrap();
            let content = String::from_utf8_lossy(&content.data);
            reg.register_template_string(&name, content).unwrap();
        }
    }

    reg.register_helper("msec-to-mmss", Box::new(mseconds_to_mm_ss));
    reg.register_helper("multiply", Box::new(multiply));
    reg
}

handlebars_helper!(mseconds_to_mm_ss: |a: i64| {
    let seconds: i64= a / 1000;

    let minutes = seconds / 60;
    let seconds = seconds % 60;
    format!("{minutes:02}:{seconds:02}")
});

handlebars_helper!(multiply: |a: i64, b: i64| a * b);

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

    #[cfg(debug_assertions)]
    {
        let watcher_sender = tx.clone();
        let watcher = filesentry::Watcher::new().unwrap();
        watcher.add_root(&template_path, true, |_| ()).unwrap();

        watcher.add_handler(move |events| {
            for event in &*events {
                if event.ty == filesentry::EventType::Modified {
                    let event = ServerSentEvent {
                        event_name: "reload".into(),
                        event_data: "template changed".into(),
                    };

                    _ = watcher_sender.send(event);
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
        templates,
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
        // .merge(search::routes())
        // .merge(album::routes())
        // .merge(artist::routes())
        // .merge(playlist::routes())
        // .merge(favorites::routes())
        // .merge(queue::routes())
        // .merge(discover::routes())
        // .merge(controls::routes())
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
    pub(crate) templates: Handlebars<'static>,
}

impl AppState {
    pub(crate) fn render<T>(&self, view: View, context: &T) -> Html<String>
    where
        T: serde::Serialize,
    {
        let result = self
            .templates
            .render(&view.name(), context)
            .unwrap_or_else(|e| e.to_string());

        Html(result)
    }

    pub(crate) async fn get_favorites(&self) -> Result<Favorites> {
        self.client.favorites().await
    }

    pub(crate) async fn get_album(&self, id: &str) -> Result<AlbumData> {
        let (album, suggested_albums) =
            try_join!(self.client.album(id), self.client.suggested_albums(id))?;

        Ok(AlbumData {
            album,
            suggested_albums,
        })
    }

    pub(crate) async fn is_album_favorite(&self, id: &str) -> Result<bool> {
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

// #[allow(clippy::result_large_err)]
// fn ok_or_error_component<T>(
//     value: Result<T, qobuz_player_controls::error::Error>,
// ) -> Result<T, axum::response::Response> {
//     match value {
//         Ok(value) => Ok(value),
//         Err(err) => Err(render(html! { <div>{format!("{err}")}</div> })),
//     }
// }

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
