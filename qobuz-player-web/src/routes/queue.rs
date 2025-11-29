use std::sync::Arc;

use axum::{
    Router,
    extract::State,
    response::{IntoResponse, Response},
    routing::get,
};
use serde_json::json;

use crate::app_state::AppState;

pub(crate) fn routes() -> Router<std::sync::Arc<crate::AppState>> {
    Router::new()
        .route("/queue", get(index))
        .route("/queue/partial", get(queue_partial))
}

async fn index(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    queue(&state, false)
}

async fn queue_partial(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    queue(&state, true)
}

fn queue(state: &AppState, partial: bool) -> Response {
    let tracks = state.tracklist_receiver.borrow().queue().to_vec();

    state.render(
        "queue.html",
        &json! ({
            "partial": partial,
            "tracks": tracks,
        }),
    )
}
