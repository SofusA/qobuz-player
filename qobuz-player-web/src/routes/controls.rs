use std::sync::Arc;

use axum::{Router, extract::State, response::IntoResponse, routing::get};
use serde_json::json;

use crate::{AppState, views::View};

pub(crate) fn routes() -> Router<std::sync::Arc<crate::AppState>> {
    Router::new().route("/controls", get(controls))
}

async fn controls(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    state.render(
        View::Controls,
        &json! ({
            "entity-link": "todo", //TODO: helper
            "title": "todo string",
            "track-title": "todo string",
            "circle": "todo bool", //TODO maybe helper based on entity type?
        }),
    )
}
