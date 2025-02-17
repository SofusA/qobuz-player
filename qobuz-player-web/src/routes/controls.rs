use axum::{response::IntoResponse, routing::get, Router};
use leptos::{component, prelude::*, IntoView};
use qobuz_player_controls::tracklist::TrackListType;

use crate::{
    html,
    routes::now_playing::{Next, Previous, State},
    view::render,
};

pub fn routes() -> Router {
    Router::new().route("/controls", get(controls))
}

async fn controls() -> impl IntoResponse {
    let tracklist = qobuz_player_controls::current_tracklist().await;

    render(html! { <ControlsPartial current_tracklist=tracklist.list_type /> })
}

#[component]
fn controls_partial(current_tracklist: TrackListType) -> impl IntoView {
    let current_status = qobuz_player_controls::current_state();

    let (playing, show) = match current_status {
        qobuz_player_controls::State::VoidPending => (false, false),
        qobuz_player_controls::State::Null => (false, false),
        qobuz_player_controls::State::Ready => (false, true),
        qobuz_player_controls::State::Paused => (false, true),
        qobuz_player_controls::State::Playing => (true, true),
    };

    let (image, title, entity_link) = match current_tracklist {
        TrackListType::Album(album_tracklist) => (
            image(album_tracklist.image, false).into_any(),
            Some(album_tracklist.title),
            Some(format!("/album/{}", album_tracklist.id)),
        ),
        TrackListType::Playlist(playlist_tracklist) => (
            image(playlist_tracklist.image, false).into_any(),
            Some(playlist_tracklist.title),
            Some(format!("/playlist/{}", playlist_tracklist.id)),
        ),
        TrackListType::TopTracks(top_tracklist) => (
            image(top_tracklist.image, true).into_any(),
            Some(top_tracklist.artist_name),
            Some(format!("/artist/{}", top_tracklist.id)),
        ),
        TrackListType::Track(single_tracklist) => (
            image(single_tracklist.image, false).into_any(),
            Some(single_tracklist.track_title),
            single_tracklist.album_id.map(|id| format!("/album/{}", id)),
        ),
        TrackListType::None => (image(None, false).into_any(), None, None),
    };

    html! {
        {show
            .then(|| {
                html! {
                    <div class="flex gap-2 justify-between items-center p-2 bg-gray-900 rounded">
                        <div class="flex overflow-hidden gap-2 items-center">
                            {image} <a hx-target="unset" class="truncate" href=entity_link>
                                {title}
                            </a>
                        </div>
                        <div class="flex gap-2 items-center h-8 shrink-0">
                            <span class="hidden md:contents">
                                <Previous />
                            </span>
                            <State playing=playing />
                            <Next />
                        </div>
                    </div>
                }
            })}
    }
}

#[component]
pub fn controls(current_tracklist: TrackListType) -> impl IntoView {
    html! {
        <div hx-get="/controls" hx-trigger="sse:tracklist" hx-target="this" class="px-2">
            <ControlsPartial current_tracklist=current_tracklist />
        </div>
    }
}

fn image(url: Option<String>, cicle: bool) -> impl IntoView {
    let image_style = url.map(|url| format!("background-image: url({});", url));

    html! {
        <div
            class=format!(
                "bg-gray-800 bg-center bg-no-repeat bg-cover shadow aspect-square size-10 {}",
                if cicle { "rounded-full" } else { "rounded" },
            )
            style=image_style
        ></div>
    }
}
