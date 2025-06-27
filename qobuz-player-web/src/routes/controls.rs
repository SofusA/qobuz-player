use axum::{Router, response::IntoResponse, routing::get};
use leptos::{IntoView, component, prelude::*};
use qobuz_player_controls::tracklist::{self, Status, Tracklist, TracklistType};

use crate::{
    html,
    routes::now_playing::{Next, Previous, State},
    view::render,
};

pub fn routes() -> Router<std::sync::Arc<crate::AppState>> {
    Router::new().route("/controls", get(controls))
}

#[component]
pub fn controls(current_status: Status, current_tracklist: Tracklist) -> impl IntoView {
    html! {
        <div
            hx-get="/controls"
            hx-trigger="sse:tracklist"
            hx-target="this"
            hx-preserve
            id="controls"
        >
            <ControlsPartial current_status=current_status current_tracklist=current_tracklist />
        </div>
    }
}

async fn controls() -> impl IntoResponse {
    let current_status = qobuz_player_controls::current_state().await;
    let current_tracklist = qobuz_player_controls::current_tracklist().await;
    render(
        html! { <ControlsPartial current_status=current_status current_tracklist=current_tracklist /> },
    )
}

#[component]
fn controls_partial(current_status: Status, current_tracklist: Tracklist) -> impl IntoView {
    let track_title = current_tracklist
        .current_track()
        .map(|track| track.title.clone());

    let (playing, show) = match current_status {
        tracklist::Status::Stopped => (false, false),
        tracklist::Status::Paused => (false, true),
        tracklist::Status::Playing => (true, true),
    };

    let (image, title, entity_link) = match current_tracklist.list_type {
        TracklistType::Album(tracklist) => (
            image(tracklist.image, false).into_any(),
            Some(tracklist.title),
            Some(format!("/album/{}", tracklist.id)),
        ),
        TracklistType::Playlist(tracklist) => (
            image(tracklist.image, false).into_any(),
            Some(tracklist.title),
            Some(format!("/playlist/{}", tracklist.id)),
        ),
        TracklistType::TopTracks(tracklist) => (
            image(tracklist.image, true).into_any(),
            Some(tracklist.artist_name),
            Some(format!("/artist/{}", tracklist.id)),
        ),
        TracklistType::Track(tracklist) => (
            image(tracklist.image, false).into_any(),
            Some(tracklist.track_title),
            tracklist.album_id.map(|id| format!("/album/{id}")),
        ),
        TracklistType::None => (image(None, false).into_any(), None, None),
    };

    html! {
        {show
            .then(|| {
                html! {
                    <div class="h-16"></div>
                    <div class="fixed right-0 left-0 bottom-14 px-safe-offset-2 py-safe">
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
                                    <State playing=playing />
                                </span>
                                <span class="flex w-8">
                                    <Next />
                                </span>
                            </div>
                        </div>
                    </div>
                }
            })}
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
