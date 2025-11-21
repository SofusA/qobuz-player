use std::sync::Arc;

use axum::{
    Router,
    extract::State,
    response::{Html, IntoResponse},
    routing::get,
};
use serde_json::json;

use crate::{AppState, views::View};

pub(crate) fn routes() -> Router<std::sync::Arc<crate::AppState>> {
    Router::new()
        .route("/", get(index))
        .route("/status", get(status_partial))
        .route("/now-playing", get(now_playing_partial))
}

async fn index(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    now_playing(&state, false)
}

async fn status_partial(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    state.render(View::PlayPause, &())
}

async fn now_playing_partial(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    now_playing(&state, true)
}

fn now_playing(state: &AppState, partial: bool) -> Html<String> {
    let tracklist = state.tracklist_receiver.borrow().clone();
    let current_track = tracklist.current_track().cloned();

    let position_mseconds = state.position_receiver.borrow().as_millis();
    let current_volume = state.volume_receiver.borrow();
    let current_volume = (*current_volume * 100.0) as u32;

    let current_position = tracklist.current_position();

    let (duration_seconds, explicit, hires_available) =
        current_track
            .as_ref()
            .map_or((None, false, false), |track| {
                (
                    Some(track.duration_seconds),
                    track.explicit,
                    track.hires_available,
                )
            });

    let number_of_tracks = tracklist.total();

    state.render(
        View::NowPlaying,
        &json! ({
            "partial": partial,
            "number_of_tracks": number_of_tracks,
            "current_volume": current_volume,
            "duration_seconds": duration_seconds,
            "position_mseconds": position_mseconds,
            "current_position": current_position + 1,
            "explicit": explicit,
            "hires_available": hires_available,
        }),
    )
}
