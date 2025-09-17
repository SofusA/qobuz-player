use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
};
use leptos::{IntoView, component, prelude::*};
use qobuz_player_controls::models::Favorites;

use crate::{
    AppState,
    components::{
        Tab,
        list::{ListAlbums, ListArtists, ListPlaylists},
    },
    html,
    page::Page,
    view::render,
};

pub(crate) fn routes() -> Router<std::sync::Arc<crate::AppState>> {
    Router::new().route("/favorites/{tab}", get(index))
}

async fn index(State(state): State<Arc<AppState>>, Path(tab): Path<Tab>) -> impl IntoResponse {
    let favorites = state.get_favorites().await;

    let tracklist = state.tracklist_receiver.borrow().clone();
    let current_status = state.status_receiver.borrow();

    render(html! {
        <Page active_page=Page::Favorites current_status=*current_status tracklist=&tracklist>
            <Favorites favorites=favorites tab=tab />
        </Page>
    })
}

#[component]
fn favorites(favorites: Favorites, tab: Tab) -> impl IntoView {
    html! {
        <div class="flex flex-col px-4">
            <div class="flex sticky top-0 flex-col flex-grow gap-4 pb-2 max-h-full pt-safe-or-2 bg-black/80 backdrop-blur">
                <h1 class="text-2xl">Favorites</h1>

                <div class="flex justify-between group *:rounded-full *:px-2 *:py-1 *:transition-colors">
                    {html! {
                        <a href="albums" class=(tab == Tab::Albums).then_some("bg-blue-800")>
                            Albums
                        </a>
                    }
                        .attr("preload", "mouseover")
                        .attr("preload-images", "true")}
                    {html! {
                        <a href="artists" class=(tab == Tab::Artists).then_some("bg-blue-800")>
                            Artists
                        </a>
                    }
                        .attr("preload", "mouseover")
                        .attr("preload-images", "true")}
                    {html! {
                        <a href="playlists" class=(tab == Tab::Playlists).then_some("bg-blue-800")>
                            Playlists
                        </a>
                    }
                        .attr("preload", "mouseover")
                        .attr("preload-images", "true")}
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
                                playlists=favorites.playlists
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
