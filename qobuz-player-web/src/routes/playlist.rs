use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, put},
};
use serde_json::json;

use crate::{AppState, ResponseResult, ok_or_broadcast, ok_or_error_component};

pub(crate) fn routes() -> Router<std::sync::Arc<crate::AppState>> {
    Router::new()
        .route("/playlist/{id}", get(index))
        .route("/playlist/{id}/content", get(content))
        .route("/playlist/{id}/tracks", get(tracks_partial))
        .route("/playlist/{id}/set-favorite", put(set_favorite))
        .route("/playlist/{id}/unset-favorite", put(unset_favorite))
        .route("/playlist/{id}/play", put(play))
        .route("/playlist/{id}/play/shuffle", put(shuffle))
        .route("/playlist/{id}/play/{track_position}", put(play_track))
        .route("/playlist/{id}/link", put(link))
}

async fn play_track(
    State(state): State<Arc<AppState>>,
    Path((id, track_position)): Path<(u32, u32)>,
) -> impl IntoResponse {
    state.controls.play_playlist(id, track_position, false);
}

async fn play(State(state): State<Arc<AppState>>, Path(id): Path<u32>) -> impl IntoResponse {
    state.controls.play_playlist(id, 0, false);
}

async fn link(State(state): State<Arc<AppState>>, Path(id): Path<u32>) -> impl IntoResponse {
    let Some(rfid_state) = state.rfid_state.clone() else {
        return;
    };
    qobuz_player_rfid::link(
        rfid_state,
        qobuz_player_controls::database::LinkRequest::Playlist(id),
        state.broadcast.clone(),
    )
    .await;
}

async fn shuffle(State(state): State<Arc<AppState>>, Path(id): Path<u32>) -> impl IntoResponse {
    state.controls.play_playlist(id, 0, true);
}

async fn set_favorite(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ResponseResult {
    ok_or_broadcast(
        &state.broadcast,
        state.client.add_favorite_artist(&id).await,
    )?;

    Ok(state.render(
        "toggle-favorite.html",
        &json!({"api": "/playlist", "id": id, "is_favorite": true}),
    ))
}

async fn unset_favorite(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ResponseResult {
    ok_or_broadcast(
        &state.broadcast,
        state.client.remove_favorite_artist(&id).await,
    )?;

    Ok(state.render(
        "toggle-favorite.html",
        &json!({"api": "/playlist", "id": id, "is_favorite": false}),
    ))
}

async fn index(State(state): State<Arc<AppState>>, Path(id): Path<u32>) -> impl IntoResponse {
    let url = format!("/playlist/{id}/content");
    state.render("lazy-load-component.html", &json!({"url": url}))
}

async fn content(State(state): State<Arc<AppState>>, Path(id): Path<u32>) -> ResponseResult {
    let playlist = ok_or_error_component(&state, state.client.playlist(id).await)?;
    let favorites = ok_or_error_component(&state, state.get_favorites().await)?;
    let is_favorite = favorites.playlists.iter().any(|playlist| playlist.id == id);
    let duration = playlist.duration_seconds / 60;

    Ok(state.render(
        "playlist.html",
        &json!({
            "playlist": playlist,
            "duration": duration,
            "is_favorite": is_favorite,
            "rfid": state.rfid_state.is_some(),
        }),
    ))
}

async fn tracks_partial(State(state): State<Arc<AppState>>, Path(id): Path<u32>) -> ResponseResult {
    let playlist = ok_or_broadcast(&state.broadcast, state.client.playlist(id).await)?;

    Ok(state.render(
        "playlist-tracks.html",
        &json!({
            "playlist": playlist
        }),
    ))
}
