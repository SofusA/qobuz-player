use axum::response::Html;
use futures::try_join;
use handlebars::Handlebars;
use qobuz_player_controls::{
    PositionReceiver, Result, Status, StatusReceiver, TracklistReceiver, VolumeReceiver,
    client::Client, controls::Controls, notification::NotificationBroadcast,
    tracklist::TracklistType,
};
use qobuz_player_models::Favorites;
use qobuz_player_rfid::RfidState;
use std::sync::Arc;
use tokio::sync::broadcast::Sender;

use crate::{AlbumData, ServerSentEvent, views::View};

pub(crate) struct AppState {
    pub(crate) tx: Sender<ServerSentEvent>,
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
        let tracklist = self.tracklist_receiver.borrow().clone();
        let current_track = tracklist.current_track().cloned();
        let status = *self.status_receiver.borrow();
        let cover_image = current_track.as_ref().and_then(|track| track.image.clone());
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

        let (entity_title, entity_link) = match tracklist.list_type() {
            TracklistType::Album(tracklist) => (
                Some(tracklist.title.clone()),
                Some(format!("/album/{}", tracklist.id)),
            ),
            TracklistType::Playlist(tracklist) => (
                Some(tracklist.title.clone()),
                Some(format!("/playlist/{}", tracklist.id)),
            ),
            TracklistType::TopTracks(tracklist) => {
                (None, Some(format!("/artist/{}", tracklist.id)))
            }
            TracklistType::Track(tracklist) => (
                current_track
                    .as_ref()
                    .and_then(|track| track.album_title.clone()),
                tracklist.album_id.as_ref().map(|id| format!("/album/{id}")),
            ),
            TracklistType::None => (None, None),
        };

        let playing_info = PlayingInfo {
            title,
            artist_link,
            artist_name,
            entity_title,
            entity_link,
            status,
            cover_image,
        };

        let context = merge_serialized(&playing_info, context).unwrap();

        let result = self
            .templates
            .render(&view.name(), &context)
            .or_else(|error| {
                self.templates.render(
                    &View::Error.name(),
                    &serde_json::json!({"error": format!("{error}")}),
                )
            })
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

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
struct PlayingInfo {
    title: String,
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
