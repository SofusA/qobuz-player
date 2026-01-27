use std::{sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post, put},
};
use axum_extra::extract::Form;
use qobuz_player_controls::{Result, client::Client, notification::Notification};
use qobuz_player_models::{AlbumSimple, Artist, Playlist, Track};
use serde::Deserialize;

use crate::{AppState, ResponseResult, hx_redirect, ok_or_send_error_toast};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/play", put(play))
        .route("/api/pause", put(pause))
        .route("/api/previous", put(previous))
        .route("/api/next", put(next))
        .route("/api/volume", post(set_volume))
        .route("/api/position", post(set_position))
        .route("/api/skip-to/{track_number}", put(skip_to))
        .route(
            "/api/remove-queue-item/{index}",
            put(remove_index_from_queue),
        )
        .route("/api/track/play/{track_id}", put(play_track))
        .route("/api/track/action", put(track_action))
        .route("/api/queue/reorder", put(reorder_queue))
        .route("/api/favorites/albums", get(favorite_albums))
        .route("/api/favorites/artists", get(favorite_artists))
        .route("/api/favorites/playlists", get(favorite_playlists))
        .route("/api/favorites/tracks", get(favorite_tracks))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TrackAction {
    AddFavorite,
    RemoveFavorite,
    AddToQueue,
    PlayNext,
    AddToPlaylist,
}
#[derive(Deserialize)]
struct TrackActionParammeters {
    track_id: u32,
    action: TrackAction,
}
async fn track_action(
    State(state): State<Arc<AppState>>,
    Form(req): Form<TrackActionParammeters>,
) -> ResponseResult {
    match req.action {
        TrackAction::AddFavorite => {
            ok_or_send_error_toast(&state, state.client.add_favorite_track(req.track_id).await)?;
            state.send_sse("tracklist".into(), "New favorite track".into());
            Ok(state.send_toast(Notification::Info("Track added to favorites".into())))
        }
        TrackAction::RemoveFavorite => {
            ok_or_send_error_toast(
                &state,
                state.client.remove_favorite_track(req.track_id).await,
            )?;
            state.send_sse("tracklist".into(), "Removed favorite track".into());
            Ok(state.send_toast(Notification::Info("Track removed from favorites".into())))
        }
        TrackAction::AddToQueue => {
            state.controls.add_track_to_queue(req.track_id);
            state.send_sse("tracklist".into(), "Track added to queue".into());
            Ok(state.send_toast(Notification::Info("Track added to queue".into())))
        }
        TrackAction::PlayNext => {
            state.controls.play_track_next(req.track_id);
            state.send_sse("tracklist".into(), "Track queued next".into());
            Ok(state.send_toast(Notification::Info("Track queued next".into())))
        }
        TrackAction::AddToPlaylist => Ok(hx_redirect(&format!(
            "/playlist/add-track/{}",
            req.track_id
        ))),
    }
}

#[derive(Deserialize)]
struct ReorderQueueParameters {
    new_order: Vec<usize>,
}

async fn reorder_queue(
    State(state): State<Arc<AppState>>,
    Form(req): Form<ReorderQueueParameters>,
) -> impl IntoResponse {
    state.controls.reorder_queue(req.new_order);
}

async fn remove_index_from_queue(
    State(state): State<Arc<AppState>>,
    Path(index): Path<usize>,
) -> impl IntoResponse {
    state.controls.remove_index_from_queue(index);
}

async fn play_track(
    State(state): State<Arc<AppState>>,
    Path(track_id): Path<u32>,
) -> impl IntoResponse {
    state.controls.play_track(track_id);
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

async fn skip_to(
    State(state): State<Arc<AppState>>,
    Path(track_number): Path<usize>,
) -> impl IntoResponse {
    state.controls.skip_to_position(track_number, true);
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

async fn favorite_albums(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match get_favorite_albums(&state.client).await {
        Ok(albums) => Json(albums).into_response(),
        Err(err) => err.to_string().into_response(),
    }
}

async fn get_favorite_albums(client: &Client) -> Result<Vec<AlbumSimple>> {
    let favorites = client.favorites().await?;
    Ok(favorites.albums)
}

async fn favorite_artists(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match get_favorite_artists(&state.client).await {
        Ok(artists) => Json(artists).into_response(),
        Err(err) => err.to_string().into_response(),
    }
}

async fn get_favorite_artists(client: &Client) -> Result<Vec<Artist>> {
    let favorites = client.favorites().await?;
    Ok(favorites.artists)
}

async fn favorite_playlists(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match get_favorite_playlists(&state.client).await {
        Ok(playlists) => Json(playlists).into_response(),
        Err(err) => err.to_string().into_response(),
    }
}

async fn get_favorite_playlists(client: &Client) -> Result<Vec<Playlist>> {
    let favorites = client.favorites().await?;
    Ok(favorites.playlists)
}

async fn favorite_tracks(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match get_favorite_tracks(&state.client).await {
        Ok(tracks) => Json(tracks).into_response(),
        Err(err) => err.to_string().into_response(),
    }
}

async fn get_favorite_tracks(client: &Client) -> Result<Vec<Track>> {
    let favorites = client.favorites().await?;
    Ok(favorites.tracks)
}
