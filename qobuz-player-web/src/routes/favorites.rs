use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, State},
    routing::get,
};
use serde_json::json;

use crate::{AppState, ResponseResult, ok_or_error_page};

#[derive(Clone, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
enum Tab {
    Albums,
    Artists,
    Playlists,
    Tracks,
}

pub(crate) fn routes() -> Router<std::sync::Arc<crate::AppState>> {
    Router::new().route("/favorites/{tab}", get(index))
}

async fn index(State(state): State<Arc<AppState>>, Path(tab): Path<Tab>) -> ResponseResult {
    let favorites = ok_or_error_page(&state, state.get_favorites().await)?;

    Ok(state.render(
        "favorites.html",
        &json!({"favorites": favorites, "tab": tab}),
    ))
}
