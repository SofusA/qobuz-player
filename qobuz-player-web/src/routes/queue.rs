use axum::{
    extract::Path,
    response::IntoResponse,
    routing::{get, put},
    Router,
};
use leptos::{component, prelude::*, IntoView};
use qobuz_player_controls::{
    queue::{TrackListType, TrackListValue},
    service::TrackStatus,
};
use std::sync::Arc;

use crate::{
    components::{
        list::{List, ListItem},
        Info,
    },
    html,
    page::Page,
    view::render,
    AppState,
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/queue", get(index))
        .route("/queue/list", get(queue_partial))
        .route("/queue/skip-to/{track_number}", put(skip_to))
}

async fn skip_to(Path(track_number): Path<u32>) -> impl IntoResponse {
    _ = qobuz_player_controls::skip(track_number, true).await;
}

async fn index() -> impl IntoResponse {
    let current_tracklist = qobuz_player_controls::current_tracklist().await;

    render(html! {
        <Page active_page=Page::Queue>
            <Queue current_tracklist=current_tracklist />
        </Page>
    })
}

#[component]
pub fn queue(current_tracklist: TrackListValue) -> impl IntoView {
    let album = current_tracklist.get_album();
    let entity_title = match current_tracklist.list_type() {
        TrackListType::Album => album.map(|album| album.title.clone()),
        TrackListType::Playlist => current_tracklist
            .clone()
            .playlist
            .map(|playlist| playlist.title),
        TrackListType::Track => album.map(|album| album.title.clone()),
        TrackListType::Unknown => None,
    };

    html! {
        <div
            hx-get="/queue/list"
            hx-trigger="sse:tracklist"
            hx-swap="outerHTML"
            class="flex flex-col flex-grow gap-4 max-h-full"
        >
            <div class="p-4 text-center">
                <p class="text-lg">{entity_title}</p>
            </div>

            <QueueList current_tracklist=current_tracklist />
        </div>
    }
}

async fn queue_partial() -> impl IntoResponse {
    let current_tracklist = qobuz_player_controls::current_tracklist().await;

    render(html! { <QueueList current_tracklist=current_tracklist /> })
}

#[component]
pub fn queue_list(current_tracklist: TrackListValue) -> impl IntoView {
    html! {
        <List>
            {current_tracklist
                .queue
                .into_iter()
                .map(|x| x.1)
                .map(|track| {
                    let list_type = current_tracklist.list_type.clone();
                    html! {
                        <ListItem>
                            <button
                                hx-swap="none"
                                hx-put=format!("/queue/skip-to/{}", track.position)
                                class=format!(
                                    "flex w-full items-center flex-row gap-4 text-left {}",
                                    if track.status == TrackStatus::Playing {
                                        "bg-blue-800"
                                    } else if track.status == TrackStatus::Played {
                                        "text-gray-500"
                                    } else {
                                        ""
                                    },
                                )
                            >
                                <span class="w-5 text-center">

                                    {if list_type == TrackListType::Album
                                        || list_type == TrackListType::Track
                                    {
                                        Some(
                                            html! { <span class="text-gray-400">{track.number}</span> },
                                        )
                                    } else if list_type == TrackListType::Playlist {
                                        Some(
                                            html! {
                                                <span class="text-gray-400">{track.position}</span>
                                            },
                                        )
                                    } else {
                                        None
                                    }}

                                </span>

                                <div class="flex overflow-hidden flex-grow justify-between items-center">
                                    <span class="truncate">{track.title.clone()}</span>
                                    <Info
                                        explicit=track.explicit
                                        hires_available=track.hires_available
                                    />
                                </div>
                            </button>
                        </ListItem>
                    }
                })
                .collect::<Vec<_>>()}
        </List>
    }
}
