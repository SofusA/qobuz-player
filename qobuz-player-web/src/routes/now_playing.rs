use std::sync::Arc;

use axum::{Router, extract::State, response::IntoResponse, routing::get};

use crate::{AppState, View};

pub(crate) fn routes() -> Router<std::sync::Arc<crate::AppState>> {
    Router::new().route("/", get(index))
    // .route("/status", get(status_partial))
    // .route("/now-playing", get(now_playing_partial))
    // .route("/play", put(play))
    // .route("/pause", put(pause))
    // .route("/previous", put(previous))
    // .route("/next", put(next))
    // .route("/volume", post(set_volume))
    // .route("/position", post(set_position))
}

async fn index(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    state.render(View::NowPlaying, &())
}
