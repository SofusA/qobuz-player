use axum::{
    Router,
    response::IntoResponse,
    routing::{get, post, put},
};
use leptos::{IntoView, component, prelude::*};
use qobuz_player_controls::{
    models,
    tracklist::{self, Tracklist, TracklistType},
};

use crate::{
    components::Info,
    html,
    icons::{Backward, Forward, Pause, Play},
    page::Page,
    view::render,
};

pub fn routes() -> Router<std::sync::Arc<crate::AppState>> {
    Router::new()
        .route("/", get(index))
        .route("/progress", get(progress_partial))
        .route("/status", get(status_partial))
        .route("/volume-slider", get(volume_slider_partial))
        .route("/now-playing", get(now_playing_partial))
        .route("/play", put(play))
        .route("/pause", put(pause))
        .route("/previous", put(previous))
        .route("/next", put(next))
        .route("/volume", post(set_volume))
}

#[derive(serde::Deserialize, Clone, Copy)]
struct VolumeParameters {
    volume: i32,
}

async fn volume_slider_partial() -> impl IntoResponse {
    let current_volume = (qobuz_player_controls::volume() * 100.0) as u32;
    render(html! { <VolumeSlider current_volume=current_volume /> })
}

#[component]
fn volume_slider(current_volume: u32) -> impl IntoView {
    html! {
        <div
            id="volume-slider"
            hx-target="this"
            hx-trigger="sse:volume delay:5000"
            hx-get="/volume-slider"
        >
            <input
                class="w-full"
                hx-post="volume"
                hx-trigger="input delay:100ms"
                hx-swap="none"
                value=current_volume
                type="range"
                name="volume"
                min="0"
                max="100"
            />
        </div>
    }
}

async fn set_volume(axum::Form(parameters): axum::Form<VolumeParameters>) -> impl IntoResponse {
    let mut volume = parameters.volume;

    if volume < 0 {
        volume = 0;
    };

    if volume > 100 {
        volume = 100;
    };

    let formatted_volume = volume as f64 / 100.0;

    qobuz_player_controls::set_volume(formatted_volume);
}

async fn status_partial() -> impl IntoResponse {
    let current_status = qobuz_player_controls::current_state().await;

    if current_status == tracklist::Status::Playing {
        render(html! { <PlayPause play=true /> })
    } else {
        render(html! { <PlayPause play=false /> })
    }
}

#[component]
fn play_pause(play: bool) -> impl IntoView {
    html! {
        <button
            class="transition-colors cursor-pointer"
            hx-swap="none"
            hx-put=format!("{}", if play { "/pause" } else { "/play" })
        >
            {match play {
                true => html! { <Pause /> }.into_any(),
                false => html! { <Play /> }.into_any(),
            }}
        </button>
    }
}

#[component]
pub fn next() -> impl IntoView {
    html! {
        <button hx-swap="none" hx-put="/next" class="transition-colors cursor-pointer">
            <Forward />
        </button>
    }
}

#[component]
pub fn previous() -> impl IntoView {
    html! {
        <button hx-swap="none" hx-put="/previous" class="transition-colors cursor-pointer">
            <Backward />
        </button>
    }
}

async fn play() -> impl IntoResponse {
    match qobuz_player_controls::play().await {
        Ok(_) => render(html! { <PlayPause play=true /> }),
        Err(_) => render(html! { <PlayPause play=false /> }),
    }
}

async fn pause() -> impl IntoResponse {
    match qobuz_player_controls::pause().await {
        Ok(_) => render(html! { <PlayPause play=false /> }),
        Err(_) => render(html! { <PlayPause play=true /> }),
    }
}

async fn previous() -> impl IntoResponse {
    qobuz_player_controls::previous().await.unwrap();
}

async fn next() -> impl IntoResponse {
    qobuz_player_controls::next().await.unwrap();
}

async fn index() -> impl IntoResponse {
    let current_tracklist = qobuz_player_controls::current_tracklist().await;
    let current_track = current_tracklist.current_track().cloned();

    let position_mseconds = qobuz_player_controls::position().map(|position| position.mseconds());
    let current_status = qobuz_player_controls::current_state().await;
    let current_volume = (qobuz_player_controls::volume() * 100.0) as u32;

    render(html! {
        <Page
            active_page=Page::NowPlaying
            current_status=current_status
            current_tracklist=current_tracklist.clone()
        >
            <NowPlaying
                current_tracklist=current_tracklist
                current_track=current_track
                position_mseconds=position_mseconds
                current_status=current_status
                current_volume=current_volume
            />
        </Page>
    })
}

async fn now_playing_partial() -> impl IntoResponse {
    let current_tracklist = qobuz_player_controls::current_tracklist().await;
    let current_track = current_tracklist.current_track().cloned();
    let position_mseconds = qobuz_player_controls::position().map(|position| position.mseconds());
    let current_status = qobuz_player_controls::current_state().await;
    let current_volume = (qobuz_player_controls::volume() * 100.0) as u32;

    render(html! {
        <NowPlaying
            current_tracklist=current_tracklist
            current_track=current_track
            position_mseconds=position_mseconds
            current_status=current_status
            current_volume=current_volume
        />
    })
}

async fn progress_partial() -> impl IntoResponse {
    let position_mseconds = qobuz_player_controls::position().map(|position| position.mseconds());
    let current_tracklist = qobuz_player_controls::current_tracklist().await;
    let current_track = current_tracklist.current_track().cloned();
    let duration_seconds = current_track.map(|track| track.duration_seconds);

    render(
        html! { <Progress position_seconds=position_mseconds duration_seconds=duration_seconds /> },
    )
}

#[component]
fn progress(position_seconds: Option<u64>, duration_seconds: Option<u32>) -> impl IntoView {
    let position = position_seconds.map_or("00:00".to_string(), mseconds_to_mm_ss);
    let duration = duration_seconds.map_or("00:00".to_string(), seconds_to_mm_ss);

    let progress = position_seconds
        .and_then(|position| duration_seconds.map(|duration| position as u32 * 100 / duration))
        .unwrap_or(0);

    html! {
        <div class="grid h-2 rounded-full overflow-clip">
            <div style="grid-column: 1; grid-row: 1;" class="w-full bg-gray-800"></div>
            <div
                id="progress-bar"
                class="bg-gray-500 transition-all"
                style=format!("grid-column: 1; grid-row: 1; width: calc({progress}%/1000)")
            ></div>
        </div>
        <div class="flex justify-between text-sm text-gray-500">
            <span>{position}</span>
            <span>{duration}</span>
        </div>
    }
}

#[component]
pub fn state(playing: bool) -> impl IntoView {
    html! {
        <div
            hx-trigger="sse:status"
            hx-get="/status"
            hx-swap="innerHTML"
            hx-target="this"
            class="flex justify-center"
        >
            <PlayPause play=playing />
        </div>
    }
}

#[component]
fn now_playing(
    current_tracklist: Tracklist,
    current_track: Option<models::Track>,
    position_mseconds: Option<u64>,
    current_status: tracklist::Status,
    current_volume: u32,
) -> impl IntoView {
    let cover_image = current_track.as_ref().and_then(|track| track.image.clone());
    let artist_name = current_track
        .as_ref()
        .and_then(|track| track.artist_name.clone());
    let artist_id = current_track.as_ref().and_then(|track| track.artist_id);

    let current_position = current_tracklist.current_position();

    let (entity_title, entity_link) = match current_tracklist.list_type() {
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

    let playing = match current_status {
        tracklist::Status::Stopped => false,
        tracklist::Status::Paused => false,
        tracklist::Status::Playing => true,
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

    let number_of_tracks = current_tracklist.total();

    html! {
        <div
            class="flex flex-col gap-4 p-4 mx-auto touch-none"
            style="max-width: calc(600px + 2rem); height: calc(100% - 4rem - env(safe-area-inset-bottom))"
            hx-get="/now-playing"
            hx-trigger="sse:tracklist"
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

                    <div hx-get="progress" hx-trigger="sse:position" hx-swap="innerHTML">
                        <Progress
                            position_seconds=position_mseconds
                            duration_seconds=duration_seconds
                        />
                    </div>
                </div>

                <div class="flex flex-col gap-4">
                    <div class="flex flex-row gap-2 justify-center h-10">
                        <Previous />
                        <State playing=playing />
                        <Next />
                    </div>
                    <VolumeSlider current_volume=current_volume />
                </div>
            </div>
        </div>
    }
}

fn seconds_to_mm_ss<T: Into<u64>>(seconds: T) -> String {
    let seconds = seconds.into();
    let minutes = seconds / 60;
    let seconds = seconds % 60;
    format!("{minutes:02}:{seconds:02}")
}

fn mseconds_to_mm_ss<T: Into<u64>>(seconds: T) -> String {
    let seconds = seconds.into() / 1000;
    let minutes = seconds / 60;
    let seconds = seconds % 60;
    format!("{minutes:02}:{seconds:02}")
}
