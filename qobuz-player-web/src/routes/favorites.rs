use axum::{extract::Path, response::IntoResponse, routing::get, Router};
use leptos::{component, prelude::*, IntoView};
use qobuz_player_controls::service::{Favorites, Playlist};
use std::sync::Arc;
use tokio::join;

use crate::{
    components::{
        list::{ListAlbums, ListArtists, ListPlaylists},
        Tab,
    },
    html,
    page::Page,
    view::render,
    AppState,
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/favorites/{tab}", get(index))
}

async fn index(Path(tab): Path<Tab>) -> impl IntoResponse {
    let (favorites, favorite_playlists) = join!(
        qobuz_player_controls::favorites(),
        qobuz_player_controls::user_playlists()
    );

    render(html! {
        <Page active_page=Page::Favorites>
            <Favorites favorites=favorites playlists=favorite_playlists tab=tab />
        </Page>
    })
}

#[component]
fn favorites(favorites: Favorites, playlists: Vec<Playlist>, tab: Tab) -> impl IntoView {
    html! {
        <div class="flex flex-col h-full">
            <div class="flex flex-col flex-grow gap-4 p-4 max-h-full">
                <h1 class="text-2xl">Favorites</h1>

                <div class="flex justify-between group *:rounded-full *:px-2 *:py-1 *:transition-colors *:hover:bg-blue-600">
                    <a href="albums" class=if tab == Tab::Albums { "bg-blue-800" } else { "" }>
                        Albums
                    </a>
                    <a href="artists" class=if tab == Tab::Artists { "bg-blue-800" } else { "" }>
                        Artists
                    </a>
                    <a
                        href="playlists"
                        class=if tab == Tab::Playlists { "bg-blue-800" } else { "" }
                    >
                        Playlists
                    </a>
                </div>
            </div>

            <div class="overflow-auto h-full">
                {match tab {
                    Tab::Albums => {
                        html! {
                            <ListAlbums
                                albums=favorites.albums
                                sort=crate::components::list::AlbumSort::Artist
                            />
                        }
                            .into_any()
                    }
                    Tab::Artists => {
                        html! {
                            <ListArtists
                                artists=favorites.artists
                                sort=crate::components::list::ArtistSort::Name
                            />
                        }
                            .into_any()
                    }
                    Tab::Playlists => {
                        html! {
                            <ListPlaylists
                                playlists=playlists
                                sort=crate::components::list::PlaylistSort::Title
                            />
                        }
                            .into_any()
                    }
                }}
            </div>
        </div>
    }
}
