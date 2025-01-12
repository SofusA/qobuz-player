use axum::{
    extract::Path,
    response::IntoResponse,
    routing::{get, put},
    Router,
};
use leptos::prelude::*;
use qobuz_player_controls::service::{Album, Artist};
use std::sync::Arc;
use tokio::join;

use crate::{
    components::{
        list::{ListAlbums, ListArtistsVertical},
        ToggleFavorite,
    },
    html,
    page::Page,
    view::render,
    AppState,
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/artist/{id}", get(index))
        .route("/artist/{id}/set-favorite", put(set_favorite))
        .route("/artist/{id}/unset-favorite", put(unset_favorite))
}

async fn set_favorite(Path(id): Path<String>) -> impl IntoResponse {
    qobuz_player_controls::add_favorite_artist(&id).await;
}

async fn unset_favorite(Path(id): Path<String>) -> impl IntoResponse {
    qobuz_player_controls::remove_favorite_artist(&id).await;
}

async fn index(Path(id): Path<i32>) -> impl IntoResponse {
    let (artist, albums, similar_artists, favorites) = join!(
        qobuz_player_controls::artist(id),
        qobuz_player_controls::artist_albums(id),
        qobuz_player_controls::similar_artists(id),
        qobuz_player_controls::favorites()
    );

    let is_favorite = favorites
        .artists
        .iter()
        .any(|artist| artist.id == id as u32);

    render(html! {
        <Page active_page=Page::Search>
            <Artist
                artist=artist
                albums=albums
                is_favorite=is_favorite
                similar_artists=similar_artists
            />
        </Page>
    })
}

#[component]
fn artist(
    artist: Artist,
    albums: Vec<Album>,
    similar_artists: Vec<Artist>,
    is_favorite: bool,
) -> impl IntoView {
    html! {
        <div class="flex flex-col h-full">
            <div class="flex gap-4 justify-between items-center p-4">
                <h1 class="text-2xl">{artist.name}</h1>

                <ToggleFavorite id=artist.id.to_string() is_favorite=is_favorite />
            </div>
            <div class="flex flex-col gap-4">
                <ListAlbums albums=albums sort=crate::components::list::AlbumSort::ReleaseYear />
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
            </div>
        </div>
    }
}
