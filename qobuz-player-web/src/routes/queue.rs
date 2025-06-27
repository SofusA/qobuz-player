use axum::{
    Router,
    extract::Path,
    response::IntoResponse,
    routing::{get, put},
};
use leptos::{IntoView, component, prelude::*};
use qobuz_player_controls::tracklist::{Tracklist, TracklistType};
use tokio::join;

use crate::{
    components::list::{List, ListTracks, TrackNumberDisplay},
    html,
    page::Page,
    view::render,
};

pub fn routes() -> Router<std::sync::Arc<crate::AppState>> {
    Router::new()
        .route("/queue", get(index))
        .route("/queue/list", get(queue_partial))
        .route("/queue/skip-to/{track_number}", put(skip_to))
}

async fn skip_to(Path(track_number): Path<u32>) -> impl IntoResponse {
    qobuz_player_controls::skip_to_position(track_number, true)
        .await
        .unwrap();
}

async fn index() -> impl IntoResponse {
    let (current_tracklist, current_status) = join!(
        qobuz_player_controls::current_tracklist(),
        qobuz_player_controls::current_state()
    );

    render(html! {
        <Page
            active_page=Page::Queue
            current_status=current_status
            current_tracklist=current_tracklist.clone()
        >
            <Queue current_tracklist=current_tracklist />
        </Page>
    })
}

#[component]
fn queue(current_tracklist: Tracklist) -> impl IntoView {
    let (entity_title, entity_link) = match current_tracklist.list_type() {
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
        <div hx-get="/queue/list" hx-trigger="sse:tracklist" hx-target="#queue-list">
            <div class="flex flex-col gap-4 p-4">
                <div class="sticky top-0 pb-2 pt-safe bg-black/20 backdrop-blur">
                    <a hx-target="unset" href=entity_link class="text-2xl">
                        {entity_title}
                    </a>
                </div>

                <div id="queue-list">
                    <QueueList current_tracklist=current_tracklist />
                </div>
            </div>
        </div>
    }
}

async fn queue_partial() -> impl IntoResponse {
    let current_tracklist = qobuz_player_controls::current_tracklist().await;

    render(html! { <QueueList current_tracklist=current_tracklist /> })
}

#[component]
fn queue_list(current_tracklist: Tracklist) -> impl IntoView {
    let now_playing_id = current_tracklist.currently_playing();
    let tracks = current_tracklist.queue;

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
