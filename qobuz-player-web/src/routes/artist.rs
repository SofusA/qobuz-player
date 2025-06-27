use axum::{
    Router,
    extract::Path,
    response::IntoResponse,
    routing::{get, put},
};
use leptos::prelude::*;
use qobuz_player_controls::models::{self, AlbumSimple, Artist, ArtistPage};
use tokio::join;

use crate::{
    components::{
        ButtonGroup, Description, Info, ToggleFavorite, button_class,
        list::{ListAlbumsVertical, ListArtistsVertical},
    },
    html,
    icons::Play,
    page::Page,
    view::render,
};

pub fn routes() -> Router<std::sync::Arc<crate::AppState>> {
    Router::new()
        .route("/artist/{id}", get(index))
        .route("/artist/{id}/top-tracks", get(top_tracks_partial))
        .route("/artist/{id}/set-favorite", put(set_favorite))
        .route("/artist/{id}/unset-favorite", put(unset_favorite))
        .route(
            "/artist/{artist_id}/play-top-track/{track_index}",
            put(play_top_track),
        )
}

async fn top_tracks_partial(Path(id): Path<u32>) -> impl IntoResponse {
    let (artist, tracklist) = join!(
        qobuz_player_controls::artist_page(id),
        qobuz_player_controls::current_tracklist(),
    );

    let now_playing_id = tracklist.currently_playing();
    let artist = artist.unwrap();

    render(
        html! { <ListTracks artist_id=artist.id tracks=artist.top_tracks now_playing_id=now_playing_id /> },
    )
}

async fn play_top_track(Path((artist_id, track_index)): Path<(u32, u32)>) -> impl IntoResponse {
    qobuz_player_controls::play_top_tracks(artist_id, track_index)
        .await
        .unwrap();
}

async fn set_favorite(Path(id): Path<String>) -> impl IntoResponse {
    qobuz_player_controls::add_favorite_artist(&id)
        .await
        .unwrap();

    render(html! { <ToggleFavorite id=id is_favorite=true /> })
}

async fn unset_favorite(Path(id): Path<String>) -> impl IntoResponse {
    qobuz_player_controls::remove_favorite_artist(&id)
        .await
        .unwrap();
    render(html! { <ToggleFavorite id=id is_favorite=false /> })
}

async fn index(Path(id): Path<u32>) -> impl IntoResponse {
    let (artist, albums, similar_artists, favorites, current_tracklist, current_status) = join!(
        qobuz_player_controls::artist_page(id),
        qobuz_player_controls::artist_albums(id),
        qobuz_player_controls::similar_artists(id),
        qobuz_player_controls::favorites(),
        qobuz_player_controls::current_tracklist(),
        qobuz_player_controls::current_state()
    );

    let now_playing_id = current_tracklist.currently_playing();

    let artist = artist.unwrap();
    let similar_artists = similar_artists.unwrap();
    let albums = albums.unwrap();
    let favorites = favorites.unwrap();

    let is_favorite = favorites.artists.iter().any(|artist| artist.id == id);

    render(html! {
        <Page
            active_page=Page::None
            current_status=current_status
            current_tracklist=current_tracklist
        >
            <Artist
                artist=artist
                albums=albums
                is_favorite=is_favorite
                similar_artists=similar_artists
                now_playing_id=now_playing_id
            />
        </Page>
    })
}

#[component]
fn artist_image(artist_image: Option<String>, artist_name: String) -> impl IntoView {
    artist_image.map(|artist_image| {
        html! { <img src=artist_image alt=artist_name class="object-contain rounded-lg size-full" /> }
    })
}

#[component]
fn artist(
    artist: ArtistPage,
    albums: Vec<AlbumSimple>,
    similar_artists: Vec<Artist>,
    is_favorite: bool,
    now_playing_id: Option<u32>,
) -> impl IntoView {
    html! {
        <div class="flex flex-col">
            <div class="self-center p-4 max-w-md">
                <ArtistImage artist_image=artist.image artist_name=artist.name.clone() />
            </div>

            <div class="flex flex-col flex-grow gap-4 items-center p-4 w-full">
                <h1 class="text-2xl">{artist.name.clone()}</h1>
                <ButtonGroup>
                    <button
                        class=button_class()
                        hx-swap="none"
                        hx-put=format!("{}/play-top-track/0", artist.id)
                    >
                        <span class="size-6">
                            <Play />
                        </span>
                        <span>Play</span>
                    </button>

                    <ToggleFavorite id=artist.id.to_string() is_favorite=is_favorite />
                </ButtonGroup>
            </div>

            <div class="flex flex-col gap-4">
                <div
                    hx-get=format!("{}/top-tracks", artist.id)
                    hx-trigger="sse:tracklist"
                    hx-target="#top-tracks"
                    class="flex flex-col gap-2"
                >
                    <h3 class="px-4 text-lg">Top tracks</h3>
                    <div
                        id="top-tracks"
                        class="flex overflow-x-auto flex-col flex-wrap gap-4 p-4 max-h-92"
                    >
                        <ListTracks
                            artist_id=artist.id
                            tracks=artist.top_tracks
                            now_playing_id=now_playing_id
                        />
                    </div>
                </div>

                <div class="flex flex-col gap-2">
                    <h3 class="px-4 text-lg">Albums</h3>
                    <ListAlbumsVertical albums=albums />
                </div>
                {if !similar_artists.is_empty() {
                    Some(
                        html! {
                            <div class="flex flex-col gap-2">
                                <h3 class="px-4 text-lg">Similar artists</h3>
                                <ListArtistsVertical artists=similar_artists />
                            </div>
                        },
                    )
                } else {
                    None
                }}
                <Description description=artist.description entity_title=artist.name />
            </div>
        </div>
    }
}

#[component]
fn list_tracks(
    artist_id: u32,
    tracks: Vec<models::Track>,
    now_playing_id: Option<u32>,
) -> impl IntoView {
    tracks
        .into_iter()
        .enumerate()
        .map(|(index, track)| {
            let is_playing = now_playing_id.is_some_and(|id| id == track.id);
            html! { <Track artist_id=artist_id track=track index=index is_playing=is_playing /> }
        })
        .collect::<Vec<_>>()
}

#[component]
fn track(artist_id: u32, track: models::Track, index: usize, is_playing: bool) -> impl IntoView {
    html! {
        <button
            class="flex gap-4 items-center w-5/6 max-w-lg rounded cursor-pointer"
            hx-put=format!("{}/play-top-track/{}", artist_id, index)
            hx-swap="none"
        >
            <img
                class="inline text-sm text-gray-500 bg-gray-800 rounded-md aspect-square size-12"
                alt=track.title.clone()
                src=track.image
            />

            <div class="overflow-hidden w-full">
                <div class="flex gap-2 items-center">
                    <h3 class="text-lg truncate">{track.title}</h3>
                    <Info explicit=track.explicit hires_available=track.hires_available />
                    {is_playing
                        .then_some({
                            html! {
                                <div class="text-blue-500 size-6">
                                    <Play />
                                </div>
                            }
                        })}
                </div>

                <h4 class="flex gap-2 text-left text-gray-400">
                    {track
                        .album_title
                        .map(|album_title| {
                            html! { <span class="truncate">{album_title}</span> }
                        })}
                </h4>
            </div>
        </button>
    }
}
