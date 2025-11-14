use std::sync::Arc;

use axum::{
    Router,
    extract::State,
    response::{Html, IntoResponse},
    routing::get,
};
use qobuz_player_controls::{
    Status,
    tracklist::{Tracklist, TracklistType},
};
use serde_json::json;

use crate::{AppState, Page, View};

pub(crate) fn routes() -> Router<std::sync::Arc<crate::AppState>> {
    Router::new()
        .route("/", get(index))
        .route("/status", get(status_partial))
        .route("/now-playing", get(now_playing_partial))
}

async fn index(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let tracklist = state.tracklist_receiver.borrow().clone();
    let current_track = tracklist.current_track().cloned();

    let position_mseconds = state.position_receiver.borrow().as_millis();
    let current_status = state.status_receiver.borrow();
    let current_volume = state.volume_receiver.borrow();
    let current_volume = (*current_volume * 100.0) as u32;

    now_playing(
        &state,
        false,
        tracklist,
        current_track,
        position_mseconds,
        *current_status,
        current_volume,
    )
}

async fn status_partial(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let status = state.status_receiver.borrow();

    state.render(
        View::PlayPause,
        &json! ({
            "status": *status
        }),
    )
}

async fn now_playing_partial(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let tracklist = state.tracklist_receiver.borrow().clone();
    let current_track = tracklist.current_track().cloned();

    let position_mseconds = state.position_receiver.borrow().as_millis();
    let current_status = state.status_receiver.borrow();
    let current_volume = state.volume_receiver.borrow();
    let current_volume = (*current_volume * 100.0) as u32;

    now_playing(
        &state,
        true,
        tracklist,
        current_track,
        position_mseconds,
        *current_status,
        current_volume,
    )
}

fn now_playing(
    state: &AppState,
    partial: bool,
    tracklist: Tracklist,
    current_track: Option<qobuz_player_models::Track>,
    position_mseconds: u128,
    current_status: Status,
    current_volume: u32,
) -> Html<String> {
    let cover_image = current_track.as_ref().and_then(|track| track.image.clone());
    let artist_name = current_track
        .as_ref()
        .and_then(|track| track.artist_name.clone());
    let artist_id = current_track.as_ref().and_then(|track| track.artist_id);

    let current_position = tracklist.current_position();

    let (entity_title, entity_link) = match tracklist.list_type() {
        TracklistType::Album(tracklist) => (
            Some(tracklist.title.clone()),
            Some(format!("/album/{}", tracklist.id)),
        ),
        TracklistType::Playlist(tracklist) => (
            Some(tracklist.title.clone()),
            Some(format!("/playlist/{}", tracklist.id)),
        ),
        TracklistType::TopTracks(tracklist) => (None, Some(format!("/artist/{}", tracklist.id))),
        TracklistType::Track(tracklist) => (
            current_track
                .as_ref()
                .and_then(|track| track.album_title.clone()),
            tracklist.album_id.as_ref().map(|id| format!("/album/{id}")),
        ),
        TracklistType::None => (None, None),
    };

    let (title, artist_link, duration_seconds, explicit, hires_available) = current_track
        .as_ref()
        .map_or((String::default(), None, None, false, false), |track| {
            (
                track.title.clone(),
                artist_id.map(|id| format!("/artist/{id}")),
                Some(track.duration_seconds),
                track.explicit,
                track.hires_available,
            )
        });

    let number_of_tracks = tracklist.total();

    state.render(
        View::NowPlaying,
        &json! ({
            "active-page": Page::NowPlaying,
            "partial": partial,
            "cover-image": cover_image,
            "number-of-tracks": number_of_tracks,
            "current-volume": current_volume,
            "title": title,
            "artist-link": artist_link,
            "artist-name": artist_name,
            "duration-seconds": duration_seconds,
            "position-mseconds": position_mseconds,
            "current-position": current_position + 1,
            "entity-title": entity_title,
            "entity-link": entity_link,
            "explicit": explicit,
            "hires-available": hires_available,
            "status": current_status
        }),
    )
}
