use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post, put},
};
use axum_extra::extract::Form;
use qobuz_player_controls::notification::Notification;
use serde::Deserialize;
use serde_json::json;

use crate::{AppState, ResponseResult, hx_redirect, ok_or_send_error_toast};

pub(crate) fn routes() -> Router<std::sync::Arc<crate::AppState>> {
    Router::new()
        .route("/playlist/create", get(create).post(create_form))
        .route("/playlist/{id}", get(index).delete(delete))
        .route("/playlist/{id}/content", get(content))
        .route("/playlist/{id}/tracks", get(tracks_partial))
        .route("/playlist/{id}/tracks/edit", get(edit_tracks_partial))
        .route("/playlist/{id}/set-favorite", put(set_favorite))
        .route("/playlist/{id}/unset-favorite", put(unset_favorite))
        .route("/playlist/{id}/play", put(play))
        .route("/playlist/{id}/play/shuffle", put(shuffle))
        .route("/playlist/{id}/play/{track_position}", put(play_track))
        .route("/playlist/{id}/link", put(link))
        .route("/playlist/add-track/{id}", get(add_track_to_playlist_page))
        .route(
            "/playlist/remove-track",
            post(remove_track_from_playlist_action),
        )
        .route("/playlist/add-track", post(add_track_to_playlist_action))
}

#[derive(Deserialize)]
struct ModifyTrackParameters {
    track_id: u64,
    playlist_id: u32,
}

async fn add_track_to_playlist_action(
    State(state): State<Arc<AppState>>,
    Form(req): Form<ModifyTrackParameters>,
) -> ResponseResult {
    let res = state
        .client
        .playlist_add_track(req.playlist_id, &[req.track_id as u32])
        .await;
    let res = ok_or_send_error_toast(&state, res)?;

    Ok(state.send_toast(Notification::Success(format!("Added to {}", res.title))))
}

async fn remove_track_from_playlist_action(
    State(state): State<Arc<AppState>>,
    Form(req): Form<ModifyTrackParameters>,
) -> ResponseResult {
    let res = state
        .client
        .playlist_delete_track(req.playlist_id, &[req.track_id])
        .await;
    let res = ok_or_send_error_toast(&state, res)?;
    let res = state.client.playlist(res.id).await;
    let res = ok_or_send_error_toast(&state, res)?;

    Ok(state.render("playlist-edit-tracks.html", &json!({"playlist": res,})))
}

async fn add_track_to_playlist_page(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> ResponseResult {
    let track = state.client.track(id).await;
    let track = ok_or_send_error_toast(&state, track)?;

    let playlists = state.get_favorites().await;
    let playlists = ok_or_send_error_toast(&state, playlists)?;
    let playlists: Vec<_> = playlists
        .playlists
        .into_iter()
        .filter(|x| x.is_owned)
        .collect();

    Ok(state.render(
        "add-track-to-playlist.html",
        &json!({"track": track, "playlists": playlists}),
    ))
}

async fn create(State(state): State<Arc<AppState>>) -> ResponseResult {
    Ok(state.render("create-playlist.html", &json!({})))
}

#[derive(Deserialize)]
struct CreatePlaylist {
    name: String,
    description: String,
    is_public: Option<bool>,
    is_collaborative: Option<bool>,
}

async fn create_form(
    State(state): State<Arc<AppState>>,
    Form(req): Form<CreatePlaylist>,
) -> ResponseResult {
    let is_public = req.is_public.unwrap_or(false);

    let res = state
        .client
        .create_playlist(req.name, is_public, req.description, req.is_collaborative)
        .await;
    let res = ok_or_send_error_toast(&state, res)?;

    Ok(hx_redirect(&format!("/playlist/{}", res.id)))
}

async fn delete(State(state): State<Arc<AppState>>, Path(id): Path<u32>) -> ResponseResult {
    let res = state.client.delete_playlist(id).await;
    ok_or_send_error_toast(&state, res)?;

    Ok(hx_redirect("/favorites/playlists"))
}

async fn play_track(
    State(state): State<Arc<AppState>>,
    Path((id, track_position)): Path<(u32, usize)>,
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

async fn set_favorite(State(state): State<Arc<AppState>>, Path(id): Path<u32>) -> ResponseResult {
    ok_or_send_error_toast(&state, state.client.add_favorite_playlist(id).await)?;

    Ok(state.render(
        "toggle-favorite.html",
        &json!({"api": "/playlist", "id": id, "is_favorite": true}),
    ))
}

async fn unset_favorite(State(state): State<Arc<AppState>>, Path(id): Path<u32>) -> ResponseResult {
    ok_or_send_error_toast(&state, state.client.remove_favorite_playlist(id).await)?;

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
    let playlist = ok_or_send_error_toast(&state, state.client.playlist(id).await)?;
    let favorites = ok_or_send_error_toast(&state, state.get_favorites().await)?;
    let is_favorite = favorites.playlists.iter().any(|playlist| playlist.id == id);
    let duration = playlist.duration_seconds / 60;
    let click_string = format!("/playlist/{}/play/", playlist.id);

    Ok(state.render(
        "playlist.html",
        &json!({
            "playlist": playlist,
            "duration": duration,
            "is_favorite": is_favorite,
            "rfid": state.rfid_state.is_some(),
            "click": click_string
        }),
    ))
}

async fn tracks_partial(State(state): State<Arc<AppState>>, Path(id): Path<u32>) -> ResponseResult {
    let playlist = ok_or_send_error_toast(&state, state.client.playlist(id).await)?;
    let click_string = format!("/playlist/{}/play/", playlist.id);

    Ok(state.render(
        "playlist-tracks.html",
        &json!({
            "playlist": playlist,
            "click": click_string
        }),
    ))
}

async fn edit_tracks_partial(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> ResponseResult {
    let playlist = ok_or_send_error_toast(&state, state.client.playlist(id).await)?;

    Ok(state.render(
        "playlist-edit-tracks.html",
        &json!({
            "playlist": playlist,
        }),
    ))
}
