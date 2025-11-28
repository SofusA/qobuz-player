use std::sync::Arc;

use axum::{
    Router,
    extract::State,
    response::{Html, IntoResponse},
    routing::get,
};
use serde_json::json;

use crate::AppState;

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
    state.render("play-pause.html", &())
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

    let current_position = tracklist.current_position() + 1;

    let (duration_mseconds, explicit, hires_available) =
        current_track
            .as_ref()
            .map_or((None, false, false), |track| {
                (
                    Some(track.duration_seconds * 1000),
                    track.explicit,
                    track.hires_available,
                )
            });

    let duration_mseconds = duration_mseconds.unwrap_or_default();

    let number_of_tracks = tracklist.total();

    let position_string = mseconds_to_mm_ss(position_mseconds);
    let duration_string = mseconds_to_mm_ss(duration_mseconds);

    state.render(
        "now-playing.html",
        &json! ({
            "partial": partial,
            "number_of_tracks": number_of_tracks,
            "current_volume": current_volume,
            "duration_mseconds": duration_mseconds,
            "position_mseconds": position_mseconds,
            "position_string": position_string,
            "duration_string": duration_string,
            "current_position": current_position,
            "explicit": explicit,
            "hires_available": hires_available,
        }),
    )
}

fn mseconds_to_mm_ss<T: Into<u128>>(mseconds: T) -> String {
    let seconds = mseconds.into() / 1000;

    let minutes = seconds / 60;
    let seconds = seconds % 60;
    format!("{minutes:02}:{seconds:02}")
}
