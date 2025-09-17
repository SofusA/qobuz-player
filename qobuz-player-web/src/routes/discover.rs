use std::sync::Arc;

use axum::{Router, extract::State, response::IntoResponse, routing::get};
use leptos::prelude::*;
use qobuz_player_controls::models::{AlbumSimple, Playlist};
use tokio::try_join;

use crate::{
    AppState, Discover,
    components::list::{ListAlbumsVertical, ListPlaylistsVertical},
    html,
    page::Page,
    view::render,
};

pub(crate) fn routes() -> Router<std::sync::Arc<crate::AppState>> {
    Router::new().route("/discover", get(index))
}

async fn get_discover(state: &AppState) -> Discover {
    let (albums, playlists) = try_join!(
        state.client.featured_albums(),
        state.client.featured_playlists(),
    )
    .unwrap();

    Discover { albums, playlists }
}

async fn index(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let discover = get_discover(&state).await;

    let tracklist = state.tracklist_receiver.borrow().clone();
    let current_status = state.status_receiver.borrow();

    let album_features = discover
        .albums
        .into_iter()
        .map(|x| html! { <AlbumFeature albums=x.1 name=x.0 /> })
        .collect::<Vec<_>>();

    let playlist_features = discover
        .playlists
        .into_iter()
        .map(|x| html! { <PlaylistFeature playlists=x.1 name=x.0 /> })
        .collect::<Vec<_>>();

    render(html! {
        <Page active_page=Page::Discover current_status=*current_status tracklist=&tracklist>
            <div class="flex flex-col gap-8 px-4 inner-page">
                <div class="flex sticky top-0 flex-col flex-grow gap-4 pt-4 pb-2 max-h-full bg-black/80 backdrop-blur top-bar">
                    <h1 class="text-2xl">Discover</h1>
                </div>
                {album_features}
                {playlist_features}
            </div>
        </Page>
    })
}

#[component]
fn album_feature(albums: Vec<AlbumSimple>, name: String) -> impl IntoView {
    html! {
        <div class="flex flex-col gap-2">
            <h3 class="text-lg">{name}</h3>
            <ListAlbumsVertical albums=albums />
        </div>
    }
}

#[component]
fn playlist_feature(playlists: Vec<Playlist>, name: String) -> impl IntoView {
    html! {
        <div class="flex flex-col gap-2">
            <h3 class="text-lg">{name}</h3>
            <ListPlaylistsVertical playlists=playlists />
        </div>
    }
}
