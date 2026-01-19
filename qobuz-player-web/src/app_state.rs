use axum::response::{Html, IntoResponse, Response};
use futures::try_join;
use qobuz_player_controls::{
    PositionReceiver, Result, Status, StatusReceiver, TracklistReceiver, VolumeReceiver,
    client::Client,
    controls::Controls,
    notification::{Notification, NotificationBroadcast},
};
use qobuz_player_models::Favorites;
use qobuz_player_rfid::RfidState;
use serde_json::json;
use skabelon::Templates;
use std::sync::Arc;
use tokio::sync::{broadcast::Sender, watch};

use crate::{AlbumData, ServerSentEvent};

pub struct AppState {
    pub tx: Sender<ServerSentEvent>,
    pub web_secret: Option<String>,
    pub rfid_state: Option<RfidState>,
    pub broadcast: Arc<NotificationBroadcast>,
    pub client: Arc<Client>,
    pub controls: Controls,
    pub position_receiver: PositionReceiver,
    pub tracklist_receiver: TracklistReceiver,
    pub status_receiver: StatusReceiver,
    pub volume_receiver: VolumeReceiver,
    pub templates: watch::Receiver<Templates>,
}

impl AppState {
    pub fn render<T>(&self, view: &str, context: &T) -> Response
    where
        T: serde::Serialize,
    {
        let tracklist = self.tracklist_receiver.borrow().clone();
        let current_track = tracklist.current_track().cloned();
        let status = *self.status_receiver.borrow();
        let artist_name = current_track
            .as_ref()
            .and_then(|track| track.artist_name.clone());
        let artist_id = current_track.as_ref().and_then(|track| track.artist_id);

        let (title, artist_link) =
            current_track
                .as_ref()
                .map_or((String::default(), None), |track| {
                    (
                        track.title.clone(),
                        artist_id.map(|id| format!("/artist/{id}")),
                    )
                });

        let entity = tracklist.entity_playing();
        let now_playing_id = tracklist.currently_playing();

        let playing_info = PlayingInfo {
            title,
            now_playing_id,
            artist_link,
            artist_name,
            entity_title: entity.title,
            entity_link: entity.link,
            status,
            cover_image: entity.cover_link,
        };

        let playing_info = serde_json::json!({"playing_info": playing_info});

        let context = merge_serialized(&playing_info, context).unwrap();
        let templates = self.templates.borrow();
        let render = templates.render(view, &context);

        Html(render).into_response()
    }

    pub fn send_toast(&self, message: Notification) -> Response {
        let (message_string, severity) = match &message {
            Notification::Error(message) => (message, 1),
            Notification::Warning(message) => (message, 2),
            Notification::Success(message) => (message, 3),
            Notification::Info(message) => (message, 4),
        };

        self.render(
            "send-toast.html",
            &json!({"message": message_string, "severity": severity}),
        )
    }

    pub fn send_sse(&self, event: String, data: String) {
        let event = ServerSentEvent {
            event_name: event,
            event_data: data,
        };

        _ = self.tx.send(event);
    }

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

#[derive(serde::Serialize, serde::Deserialize)]
struct PlayingInfo {
    title: String,
    now_playing_id: Option<u32>,
    artist_link: Option<String>,
    artist_name: Option<String>,
    entity_title: Option<String>,
    entity_link: Option<String>,
    status: Status,
    cover_image: Option<String>,
}

fn merge_serialized<T: serde::Serialize, Y: serde::Serialize>(
    info: &T,
    extra: &Y,
) -> serde_json::Result<serde_json::Value> {
    let mut info_value = serde_json::to_value(info)?;
    let extra_value = serde_json::to_value(extra)?;

    if let (serde_json::Value::Object(info_map), serde_json::Value::Object(extra_map)) =
        (&mut info_value, extra_value)
    {
        for (k, v) in extra_map {
            info_map.insert(k, v);
        }
    }

    Ok(info_value)
}
