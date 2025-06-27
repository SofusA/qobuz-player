use axum::{Router, response::IntoResponse, routing::get};
use leptos::prelude::*;
use qobuz_player_controls::models::{AlbumSimple, Playlist};
use tokio::{join, try_join};

use crate::{
    components::list::{ListAlbumsVertical, ListPlaylistsVertical},
    html,
    page::Page,
    view::render,
};

pub fn routes() -> Router<std::sync::Arc<crate::AppState>> {
    Router::new().route("/discover", get(index))
}

async fn index() -> impl IntoResponse {
    let (press_awards, new_releases, qobuzissims, ideal_discography, editor_picks) = try_join!(
        qobuz_player_controls::featured_albums(
            qobuz_player_controls::AlbumFeaturedType::PressAwards
        ),
        qobuz_player_controls::featured_albums(
            qobuz_player_controls::AlbumFeaturedType::NewReleasesFull
        ),
        qobuz_player_controls::featured_albums(
            qobuz_player_controls::AlbumFeaturedType::Qobuzissims
        ),
        qobuz_player_controls::featured_albums(
            qobuz_player_controls::AlbumFeaturedType::IdealDiscography
        ),
        qobuz_player_controls::featured_playlists(
            qobuz_player_controls::PlaylistFeaturedType::EditorPicks
        ),
    )
    .unwrap();

    let (current_tracklist, current_status) = join!(
        qobuz_player_controls::current_tracklist(),
        qobuz_player_controls::current_state()
    );

    render(html! {
        <Page
            active_page=Page::Discover
            current_status=current_status
            current_tracklist=current_tracklist
        >
            <div class="flex flex-col gap-8 p-4">
                <div class="flex sticky top-0 flex-col flex-grow gap-4 pb-2 max-h-full pt-safe bg-black/80 backdrop-blur">
                    <h1 class="text-2xl">Discover</h1>
                </div>
                <AlbumFeature albums=press_awards name="Press awards".to_string() />
                <AlbumFeature albums=new_releases name="New releases".to_string() />
                <AlbumFeature albums=qobuzissims name="Qobuzissims".to_string() />
                <AlbumFeature albums=ideal_discography name="Ideal discography".to_string() />
                <PlaylistFeature playlists=editor_picks name="Featured playlists".to_string() />
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
