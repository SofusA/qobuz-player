use std::sync::Arc;

use axum::{Router, extract::State, routing::get};
use serde_json::json;
use tokio::try_join;

use crate::{AppState, Discover, ResponseResult, ok_or_error_page};

pub(crate) fn routes() -> Router<std::sync::Arc<crate::AppState>> {
    Router::new().route("/discover", get(index))
}

async fn index(State(state): State<Arc<AppState>>) -> ResponseResult {
    let (albums, playlists) = ok_or_error_page(
        &state,
        try_join!(
            state.client.featured_albums(),
            state.client.featured_playlists(),
        ),
    )?;

    let discover = Discover { albums, playlists };

    Ok(state.render(
        "discover.html",
        &json! ({
            "discover": discover,
        }),
    ))
}
