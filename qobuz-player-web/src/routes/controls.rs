use std::sync::Arc;

use axum::{Router, extract::State, response::IntoResponse, routing::get};
use leptos::{IntoView, component, prelude::*};
use qobuz_player_controls::{
    Status,
    tracklist::{Tracklist, TracklistType},
};

use crate::{
    AppState, html,
    now_playing::PlayerState,
    routes::now_playing::{Next, Previous},
    view::render,
};

pub(crate) fn routes() -> Router<std::sync::Arc<crate::AppState>> {
    Router::new().route("/controls", get(controls))
}

#[component]
pub(crate) fn controls<'a>(current_status: Status, tracklist: &'a Tracklist) -> impl IntoView {
    html! {
        <div
            hx-get="/controls"
            hx-trigger="tracklist"
            data-sse="tracklist"
            hx-target="this"
            hx-preserve
            id="controls"
        >
            <ControlsPartial current_status=current_status tracklist=tracklist />
        </div>
    }
}

async fn controls(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let current_status = state.status_receiver.borrow();
    let tracklist = state.tracklist_receiver.borrow();

    render(html! { <ControlsPartial current_status=*current_status tracklist=&tracklist /> })
}

#[component]
fn controls_partial<'a>(current_status: Status, tracklist: &'a Tracklist) -> impl IntoView {
    let track_title = tracklist.current_track().map(|track| track.title.clone());

    let (image, title, entity_link) = match tracklist.list_type() {
        TracklistType::Album(tracklist) => (
            image(tracklist.image.clone(), false).into_any(),
            Some(tracklist.title.clone()),
            Some(format!("/album/{}", tracklist.id)),
        ),
        TracklistType::Playlist(tracklist) => (
            image(tracklist.image.clone(), false).into_any(),
            Some(tracklist.title.clone()),
            Some(format!("/playlist/{}", tracklist.id)),
        ),
        TracklistType::TopTracks(tracklist) => (
            image(tracklist.image.clone(), true).into_any(),
            Some(tracklist.artist_name.clone()),
            Some(format!("/artist/{}", tracklist.id)),
        ),
        TracklistType::Track(tracklist) => (
            image(tracklist.image.clone(), false).into_any(),
            Some(tracklist.track_title.clone()),
            tracklist.album_id.as_ref().map(|id| format!("/album/{id}")),
        ),
        TracklistType::None => (image(None, false).into_any(), None, None),
    };

    html! {
        <div class="controls-placeholder"></div>
        <div class="fixed right-0 left-0 bottom-14 controls">
            <div class="flex gap-2 justify-between items-center p-2 rounded-md bg-gray-900/70 backdrop-blur">
                <a
                    class="flex overflow-hidden gap-2 items-center w-full"
                    hx-target="unset"
                    href=entity_link
                >
                    {image}
                    <div class="flex overflow-hidden flex-wrap gap-2 leading-none">
                        <span class="truncate">{title}</span>
                        <span class="text-gray-500 truncate">{track_title}</span>
                    </div>
                </a>
                <div class="flex gap-4 items-center">
                    <span class="hidden w-8 sm:flex">
                        <Previous />
                    </span>
                    <span class="flex w-8">
                        <PlayerState status=current_status />
                    </span>
                    <span class="flex w-8">
                        <Next />
                    </span>
                </div>
            </div>
        </div>
    }
}

fn image(url: Option<String>, cicle: bool) -> impl IntoView {
    let image_style = url.map(|url| format!("background-image: url({url});"));

    html! {
        <div
            class=format!(
                "bg-gray-800 bg-center bg-no-repeat bg-cover shadow aspect-square size-10 {}",
                if cicle { "rounded-full" } else { "rounded-md" },
            )
            style=image_style
        ></div>
    }
}
