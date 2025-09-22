use std::sync::Arc;

use axum::{Router, extract::State, routing::get};
use leptos::prelude::*;
use qobuz_player_models::{AlbumSimple, Playlist};
use tokio::try_join;

use crate::{
    AppState, Discover, ResponseResult,
    components::list::{ListAlbumsVertical, ListPlaylistsVertical},
    html, ok_or_error_component,
    page::Page,
    view::render,
};

pub(crate) fn routes() -> Router<std::sync::Arc<crate::AppState>> {
    Router::new().route("/discover", get(index))
}

async fn index(State(state): State<Arc<AppState>>) -> ResponseResult {
    let (albums, playlists) = ok_or_error_component(try_join!(
        state.client.featured_albums(),
        state.client.featured_playlists(),
    ))?;

    let discover = Discover { albums, playlists };

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

    Ok(render(html! {
        <Page active_page=Page::Discover current_status=*current_status tracklist=&tracklist>
            <div class="flex flex-col gap-8 px-4">
                <div class="flex sticky top-0 flex-col flex-grow gap-4 pb-2 max-h-full pt-safe-or-4 bg-black/80 backdrop-blur">
                    <h1 class="text-2xl">Discover</h1>
                </div>
                {album_features}
                {playlist_features}
            </div>
        </Page>
    }))
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
