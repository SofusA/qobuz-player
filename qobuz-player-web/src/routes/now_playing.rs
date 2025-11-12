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
    // let context = Context::new();
    // let template = state.tera.render("now-playing.html", &context).unwrap();
    // Html(template)

    let result = state
        .templates
        .render("now-playing", &json!({}))
        .unwrap_or_else(|e| e.to_string());

    Html(result)
}
