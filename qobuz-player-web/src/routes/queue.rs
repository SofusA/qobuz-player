use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, put},
};
use leptos::{IntoView, component, prelude::*};
use qobuz_player_controls::tracklist::{Tracklist, TracklistType};

use crate::{
    AppState,
    components::list::{List, ListTracks, TrackNumberDisplay},
    html,
    page::Page,
    view::render,
};

pub(crate) fn routes() -> Router<std::sync::Arc<crate::AppState>> {
    Router::new()
        .route("/queue", get(index))
        .route("/queue/list", get(queue_partial))
        .route("/queue/skip-to/{track_number}", put(skip_to))
}

async fn skip_to(Path(track_number): Path<u32>) -> impl IntoResponse {
    qobuz_player_controls::skip_to_position(track_number, true);
}

async fn index(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let current_status = state.player_state.target_status.read().await;
    let tracklist = state.player_state.tracklist.read().await;
    let tracklist_clone = tracklist.clone();

    render(html! {
        <Page active_page=Page::Queue current_status=&current_status tracklist=&tracklist>
            <Queue tracklist=tracklist_clone />
        </Page>
    })
}

#[component]
fn queue(tracklist: Tracklist) -> impl IntoView {
    let (entity_title, entity_link) = match tracklist.list_type() {
        TracklistType::Album(tracklist) => (
            tracklist.title.clone(),
            Some(format!("/album/{}", tracklist.id)),
        ),
        TracklistType::Playlist(tracklist) => (
            tracklist.title.clone(),
            Some(format!("/playlist/{}", tracklist.id)),
        ),
        TracklistType::TopTracks(tracklist) => (
            tracklist.artist_name.clone(),
            Some(format!("/artist/{}", tracklist.id)),
        ),
        TracklistType::Track(tracklist) => (
            tracklist.track_title.clone(),
            tracklist.album_id.as_ref().map(|id| format!("/album/{id}")),
        ),
        TracklistType::None => ("Empty queue".to_string(), None),
    };

    html! {
        <div
            hx-get="/queue/list"
            hx-trigger="tracklist"
            data-sse="tracklist"
            hx-target="#queue-list"
        >
            <div class="flex flex-col gap-4 p-4">
                <div class="sticky top-0 pb-2 pt-safe bg-black/20 backdrop-blur">
                    <a hx-target="unset" href=entity_link class="text-2xl">
                        {entity_title}
                    </a>
                </div>

                <div id="queue-list">
                    <QueueList tracklist=tracklist />
                </div>
            </div>
        </div>
    }
}

async fn queue_partial(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let tracklist = state.player_state.tracklist.read().await;
    render(html! { <QueueList tracklist=tracklist.clone() /> })
}

#[component]
fn queue_list(tracklist: Tracklist) -> impl IntoView {
    let now_playing_id = tracklist.currently_playing();
    let tracks = tracklist.queue().to_vec();

    html! {
        <List>
            <ListTracks
                track_number_display=TrackNumberDisplay::Cover
                tracks=tracks
                show_artist=true
                dim_played=true
                api_call=|index: usize| format!("/queue/skip-to/{index}")
                now_playing_id=now_playing_id
            />
        </List>
    }
}
