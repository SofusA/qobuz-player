use std::sync::Arc;

use axum::{
    Router,
    extract::State,
    response::IntoResponse,
    routing::get,
};
use serde_json::json;

use crate::app_state::AppState;

pub fn routes() -> Router<std::sync::Arc<crate::AppState>> {
    Router::new()
        .route("/queue", get(index))
        .route("/queue/partial", get(queue_partial))
}

async fn index(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let tracklist = state.tracklist_receiver.borrow();
    let tracks = tracklist.queue().to_vec();
    let currently_playing_position = tracklist.current_position();

    state.render(
        "queue.html",
        &json!({
            "tracks": tracks,
            "currently_playing_position": currently_playing_position
        }),
    )
}

async fn queue_partial(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let tracklist = state.tracklist_receiver.borrow();
    let tracks = tracklist.queue().to_vec();
    let currently_playing_position = tracklist.current_position();

    state.render(
        "queue-content.html",
        &json!({
            "tracks": tracks,
            "currently_playing_position": currently_playing_position
        }),
    )
}
