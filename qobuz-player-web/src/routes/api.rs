use std::{sync::Arc, time::Duration};

use axum::{
    Router,
    extract::State,
    response::IntoResponse,
    routing::{post, put},
};

use crate::AppState;

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/play", put(play))
        .route("/api/pause", put(pause))
        .route("/api/previous", put(previous))
        .route("/api/next", put(next))
        .route("/api/volume", post(set_volume))
        .route("/api/position", post(set_position))
}

async fn play(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    state.controls.play();
}

async fn pause(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    state.controls.pause();
}

async fn previous(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    state.controls.previous();
}

async fn next(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    state.controls.next();
}

#[derive(serde::Deserialize, Clone, Copy)]
struct SliderParameters {
    value: i32,
}
async fn set_volume(
    State(state): State<Arc<AppState>>,
    axum::Form(parameters): axum::Form<SliderParameters>,
) -> impl IntoResponse {
    let mut volume = parameters.value;

    if volume < 0 {
        volume = 0;
    };

    if volume > 100 {
        volume = 100;
    };

    let formatted_volume = volume as f32 / 100.0;

    state.controls.set_volume(formatted_volume);
}

async fn set_position(
    State(state): State<Arc<AppState>>,
    axum::Form(parameters): axum::Form<SliderParameters>,
) -> impl IntoResponse {
    let time = Duration::from_millis(parameters.value as u64);
    state.controls.seek(time);
}
