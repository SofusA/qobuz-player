use std::{sync::Arc, time::Duration};

use axum::{
    Router,
    extract::State,
    response::IntoResponse,
    routing::{get, post, put},
};
use leptos::{IntoView, component, prelude::*};
use qobuz_player_controls::{
    Status,
    tracklist::{Tracklist, TracklistType},
};

use crate::{
    AppState,
    components::Info,
    html,
    icons::{Backward, Forward, LoadingSpinner, Pause, Play},
    page::Page,
    view::render,
};

pub(crate) fn routes() -> Router<std::sync::Arc<crate::AppState>> {
    Router::new()
        .route("/", get(index))
        .route("/status", get(status_partial))
        .route("/now-playing", get(now_playing_partial))
        .route("/play", put(play))
        .route("/pause", put(pause))
        .route("/previous", put(previous))
        .route("/next", put(next))
        .route("/volume", post(set_volume))
        .route("/position", post(set_position))
}

#[derive(serde::Deserialize, Clone, Copy)]
struct SliderParameters {
    value: i32,
}

#[component]
fn volume_slider(current_volume: u32) -> impl IntoView {
    html! {
        <input
            id="volume-slider"
            class="w-full accent-blue-500"
            autocomplete="off"
            hx-post="volume"
            hx-trigger="input delay:100ms"
            hx-swap="none"
            value=current_volume
            type="range"
            name="value"
            min="0"
            max="100"
        />
    }
}

async fn set_position(
    State(state): State<Arc<AppState>>,
    axum::Form(parameters): axum::Form<SliderParameters>,
) -> impl IntoResponse {
    let time = Duration::from_millis(parameters.value as u64);
    state.controls.seek(time);
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

async fn status_partial(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let status = state.status_receiver.borrow();
    render(html! { <PlayPause status=*status /> })
}

#[component]
fn play_pause(status: Status) -> impl IntoView {
    let playing = match status {
        Status::Paused | Status::Buffering => false,
        Status::Playing => true,
    };

    let status_icon = match status {
        Status::Playing => html! { <Pause /> }.into_any(),
        Status::Buffering => html! { <LoadingSpinner /> }.into_any(),
        Status::Paused => html! { <Play /> }.into_any(),
    };

    html! {
        <button
            class="transition-colors cursor-pointer"
            hx-swap="none"
            hx-put=format!("{}", if playing { "/pause" } else { "/play" })
        >
            {status_icon}
        </button>
    }
}

#[component]
pub(crate) fn next() -> impl IntoView {
    html! {
        <button hx-swap="none" hx-put="/next" class="transition-colors cursor-pointer">
            <Forward />
        </button>
    }
}

#[component]
pub(crate) fn previous() -> impl IntoView {
    html! {
        <button hx-swap="none" hx-put="/previous" class="transition-colors cursor-pointer">
            <Backward />
        </button>
    }
}

async fn play(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    state.controls.play();
}

async fn pause(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    state.controls.pause();
    render(html! { <PlayPause status=Status::Paused /> })
}

async fn previous(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    state.controls.previous();
}

async fn next(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    state.controls.next();
}

async fn index(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let tracklist = state.tracklist_receiver.borrow().clone();
    let tracklist_clone = tracklist.clone();
    let current_track = tracklist.current_track().cloned();

    let position_mseconds = state.position_receiver.borrow().as_millis();
    let current_status = state.status_receiver.borrow();
    let current_status_copy = *current_status;
    let current_volume = state.volume_receiver.borrow();
    let current_volume = (*current_volume * 100.0) as u32;

    render(html! {
        <Page active_page=Page::NowPlaying current_status=*current_status tracklist=&tracklist>
            <NowPlaying
                tracklist=tracklist_clone
                current_track=current_track
                position_mseconds=position_mseconds
                current_status=current_status_copy
                current_volume=current_volume
            />
        </Page>
    })
}

async fn now_playing_partial(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let tracklist = state.tracklist_receiver.borrow().clone();
    let current_track = tracklist.current_track().cloned();

    let position_mseconds = state.position_receiver.borrow().as_millis();
    let current_status = state.status_receiver.borrow();
    let current_volume = state.volume_receiver.borrow();
    let current_volume = (*current_volume * 100.0) as u32;

    render(html! {
        <NowPlaying
            tracklist=tracklist
            current_track=current_track
            position_mseconds=position_mseconds
            current_status=*current_status
            current_volume=current_volume
        />
    })
}

#[component]
fn progress(position_mseconds: u128, duration_seconds: Option<u32>) -> impl IntoView {
    let duration_mseconds = duration_seconds.map_or(0, |x| x * 1000);

    let position_string = mseconds_to_mm_ss(position_mseconds);
    let duration_string = mseconds_to_mm_ss(duration_mseconds);

    html! {
        <div class="flex flex-col">
            <input
                id="progress-slider"
                class="w-full accent-gray-500"
                autocomplete="off"
                hx-post="position"
                hx-trigger="input delay:100ms"
                hx-swap="none"
                value=position_mseconds
                type="range"
                name="value"
                min="0"
                max=duration_mseconds
            />
            <div class="flex justify-between text-sm text-gray-500">
                <span id="position">{position_string}</span>
                <span>{duration_string}</span>
            </div>
        </div>
    }
}

#[component]
pub(crate) fn player_state(status: Status) -> impl IntoView {
    html! {
        <div
            hx-trigger="status"
            data-sse="status"
            hx-get="/status"
            hx-swap="innerHTML"
            hx-target="this"
            class="flex justify-center"
        >
            <PlayPause status=status />
        </div>
    }
}

#[component]
fn now_playing(
    tracklist: Tracklist,
    current_track: Option<qobuz_player_models::Track>,
    position_mseconds: u128,
    current_status: Status,
    current_volume: u32,
) -> impl IntoView {
    let cover_image = current_track.as_ref().and_then(|track| track.image.clone());
    let artist_name = current_track
        .as_ref()
        .and_then(|track| track.artist_name.clone());
    let artist_id = current_track.as_ref().and_then(|track| track.artist_id);

    let current_position = tracklist.current_position();

    let (entity_title, entity_link) = match tracklist.list_type() {
        TracklistType::Album(tracklist) => (
            Some(tracklist.title.clone()),
            Some(format!("/album/{}", tracklist.id)),
        ),
        TracklistType::Playlist(tracklist) => (
            Some(tracklist.title.clone()),
            Some(format!("/playlist/{}", tracklist.id)),
        ),
        TracklistType::TopTracks(tracklist) => (None, Some(format!("/artist/{}", tracklist.id))),
        TracklistType::Track(tracklist) => (
            current_track
                .as_ref()
                .and_then(|track| track.album_title.clone()),
            tracklist.album_id.as_ref().map(|id| format!("/album/{id}")),
        ),
        TracklistType::None => (None, None),
    };

    let (title, artist_link, duration_seconds, explicit, hires_available) = current_track
        .as_ref()
        .map_or((String::default(), None, None, false, false), |track| {
            (
                track.title.clone(),
                artist_id.map(|id| format!("/artist/{id}")),
                Some(track.duration_seconds),
                track.explicit,
                track.hires_available,
            )
        });

    let number_of_tracks = tracklist.total();

    html! {
        <div
            class="flex flex-col gap-4 p-4 mx-auto touch-none"
            style="max-width: calc(600px + 2rem); height: calc(100% - 4rem - env(safe-area-inset-bottom))"
            hx-get="/now-playing"
            hx-trigger="tracklist"
            data-sse="tracklist"
            hx-swap="outerHTML"
        >
            <div class="flex overflow-hidden justify-center size-full aspect-square max-h-fit">
                {if let Some(cover_image_url) = cover_image {
                    html! { <img src=cover_image_url alt=title.clone() class="rounded-lg" /> }
                        .into_any()
                } else {
                    html! { <div class="h-full bg-gray-900 rounded-lg aspect-square"></div> }
                        .into_any()
                }}
            </div>

            <div class="flex flex-col flex-grow justify-center w-full">
                <div class="flex gap-2 justify-between items-center">
                    <a class="text truncate" href=entity_link>
                        {entity_title}
                    </a>
                    <div class="text-gray-500 whitespace-nowrap">
                        {if current_track.is_some() {
                            format!("{} of {}", current_position + 1, number_of_tracks)
                        } else {
                            String::default()
                        }}
                    </div>
                </div>

                <a href=artist_link class="text-gray-400 truncate w-fit">
                    {artist_name}
                </a>

                <div class="flex flex-col gap-y-4 w-full">
                    <div class="flex gap-2 justify-between items-center">
                        <span class="text-lg truncate">{title}</span>
                        <Info explicit=explicit hires_available=hires_available />
                    </div>

                    <Progress
                        position_mseconds=position_mseconds
                        duration_seconds=duration_seconds
                    />
                </div>

                <div class="flex flex-col gap-4">
                    <div class="flex flex-row gap-2 justify-center h-10">
                        <Previous />
                        <PlayerState status=current_status />
                        <Next />
                    </div>
                    <VolumeSlider current_volume=current_volume />
                </div>
            </div>
        </div>
    }
}

fn mseconds_to_mm_ss<T: Into<u128>>(mseconds: T) -> String {
    let seconds = mseconds.into() / 1000;

    let minutes = seconds / 60;
    let seconds = seconds % 60;
    format!("{minutes:02}:{seconds:02}")
}
