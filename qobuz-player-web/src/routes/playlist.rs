use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, put},
};
use leptos::prelude::*;
use qobuz_player_controls::models::{Playlist, Track};

use crate::{
    AppState,
    components::{
        ButtonGroup, ToggleFavorite, button_class,
        list::{ListTracks, TrackNumberDisplay},
        parse_duration,
    },
    html,
    icons::{Link, Play},
    page::Page,
    view::{LazyLoadComponent, render},
};

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
        qobuz_player_database::LinkRequest::Playlist(id),
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
) -> impl IntoResponse {
    state.client.add_favorite_playlist(&id).await.unwrap();
    render(html! { <ToggleFavorite id=id is_favorite=true /> })
}

async fn unset_favorite(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    state.client.remove_favorite_playlist(&id).await.unwrap();
    render(html! { <ToggleFavorite id=id is_favorite=false /> })
}

async fn index(State(state): State<Arc<AppState>>, Path(id): Path<u32>) -> impl IntoResponse {
    let url = format!("/playlist/{id}/content");

    let current_status = state.status_receiver.borrow();
    let tracklist = state.tracklist_receiver.borrow();

    render(html! {
        <Page active_page=Page::None current_status=*current_status tracklist=&tracklist>
            <LazyLoadComponent url=url />
        </Page>
    })
}

async fn content(State(state): State<Arc<AppState>>, Path(id): Path<u32>) -> impl IntoResponse {
    let playlist = state.client.playlist(id).await.unwrap();
    let favorites = state.get_favorites().await;
    let is_favorite = favorites.playlists.iter().any(|playlist| playlist.id == id);
    let currently_playing = state.tracklist_receiver.borrow().currently_playing();

    render(html! {
        <Playlist
            now_playing_id=currently_playing
            playlist=playlist
            is_favorite=is_favorite
            rfid=state.rfid_state.is_some()
        />
    })
}

async fn tracks_partial(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> impl IntoResponse {
    let playlist = state.client.playlist(id).await.unwrap();
    let currently_playing = state.tracklist_receiver.borrow().currently_playing();

    render(
        html! { <Tracks tracks=playlist.tracks playlist_id=playlist.id now_playing_id=currently_playing /> },
    )
}

#[component]
fn tracks(now_playing_id: Option<u32>, tracks: Vec<Track>, playlist_id: u32) -> impl IntoView {
    html! {
        <div
            class="w-full"
            hx-trigger="tracklist"
            hx-target="this"
            data-sse="tracklist"
            hx-swap="morph:outerHTML"
            hx-get=format!("/playlist/{}/tracks", playlist_id)
        >
            <ListTracks
                track_number_display=TrackNumberDisplay::Cover
                tracks=tracks
                show_artist=true
                dim_played=false
                api_call=move |index: usize| format!("/playlist/{playlist_id}/play/{index}")
                now_playing_id=now_playing_id
            />
        </div>
    }
}

#[component]
fn playlist(
    now_playing_id: Option<u32>,
    playlist: Playlist,
    is_favorite: bool,
    rfid: bool,
) -> impl IntoView {
    let duration = parse_duration(playlist.duration_seconds);

    html! {
        <div class="flex flex-wrap gap-4 justify-center items-end p-4 w-full *:max-w-sm inner-page">
            <img
                src=playlist.image
                alt=playlist.title.clone()
                class="object-contain rounded-lg size-full"
            />

            <div class="flex flex-col flex-grow gap-4 items-center w-full">
                <div class="flex flex-col gap-2 justify-center items-center w-full text-center">
                    <span class="text-lg sm:text-xl">{playlist.title}</span>
                    <span class="text-gray-400 sm:text-lg">
                        {format!("{} minutes", duration.minutes)}
                    </span>
                </div>

                {
                    html! {
                        <ButtonGroup>
                            <button
                                class=button_class()
                                hx-swap="none"
                                hx-put=format!("{}/play", playlist.id.clone())
                            >
                                <span class="size-6">
                                    <Play />
                                </span>
                                <span>Play</span>
                            </button>

                            <button
                                class=button_class()
                                hx-swap="none"
                                hx-put=format!("{}/play/shuffle", playlist.id.clone())
                            >
                                <span class="size-6">
                                    <Play />
                                </span>
                                <span>Shuffle</span>
                            </button>

                            {(!playlist.is_owned)
                                .then_some({
                                    html! {
                                        <ToggleFavorite
                                            id=playlist.id.to_string()
                                            is_favorite=is_favorite
                                        />
                                    }
                                })}

                            {rfid
                                .then_some(
                                    html! {
                                        <button
                                            class=button_class()
                                            hx-swap="none"
                                            hx-put=format!("{}/link", playlist.id.clone())
                                        >
                                            <span class="size-6">
                                                <Link />
                                            </span>
                                            <span>Link RFID</span>
                                        </button>
                                    },
                                )}

                        </ButtonGroup>
                    }
                }
            </div>
        </div>
        <div class="flex flex-col gap-4 w-full">
            <div class="sm:p-4">
                <Tracks
                    now_playing_id=now_playing_id
                    tracks=playlist.tracks
                    playlist_id=playlist.id
                />
            </div>
        </div>
    }
}
