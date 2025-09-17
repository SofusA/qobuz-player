use std::sync::Arc;

use axum::{
    Form, Router,
    extract::{Path, Query, State},
    response::IntoResponse,
    routing::{get, put},
};
use leptos::{component, prelude::*};
use qobuz_player_controls::models::{self, SearchResults};
use serde::Deserialize;

#[derive(Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Tab {
    Albums,
    Artists,
    Playlists,
    Tracks,
}

use crate::{
    AppState,
    components::{
        Info,
        list::{List, ListAlbums, ListArtists, ListItem, ListPlaylists},
    },
    html,
    icons::MagnifyingGlass,
    page::Page,
    view::render,
};

pub(crate) fn routes() -> Router<std::sync::Arc<crate::AppState>> {
    Router::new()
        .route("/search/{tab}", get(index).post(search))
        .route("/play-track/{track_id}", put(play_track))
}

async fn play_track(
    State(state): State<Arc<AppState>>,
    Path(track_id): Path<u32>,
) -> impl IntoResponse {
    state.controls.play_track(track_id);
}

#[derive(Deserialize, Clone)]
struct SearchParameters {
    query: Option<String>,
}

async fn index(
    State(state): State<Arc<AppState>>,
    Path(tab): Path<Tab>,
    Query(parameters): Query<SearchParameters>,
) -> impl IntoResponse {
    let query = parameters
        .query
        .and_then(|s| if s.is_empty() { None } else { Some(s) });
    let search_results = match query {
        Some(query) => state.client.search(query).await.unwrap(),
        None => SearchResults::default(),
    };

    let current_status = state.status_receiver.borrow();
    let tracklist = state.tracklist_receiver.borrow();

    let html = html! {
        <Page active_page=Page::Search current_status=*current_status tracklist=&tracklist>
            <Search search_results=search_results tab=tab />
        </Page>
    };

    render(html)
}

async fn search(
    State(state): State<Arc<AppState>>,
    Path(tab): Path<Tab>,
    Form(parameters): Form<SearchParameters>,
) -> impl IntoResponse {
    let query = parameters
        .query
        .and_then(|s| if s.is_empty() { None } else { Some(s) });
    let search_results = match query.clone() {
        Some(query) => state.client.search(query).await.unwrap(),
        None => SearchResults::default(),
    };

    let html = html! {
        <SearchPartial search_results=search_results tab=tab.clone() />

        {html! { <TabBar query=query.unwrap_or_default() tab=tab /> }.attr("hx-swap-oob", "true")}
    };

    render(html)
}

#[component]
fn search_partial(search_results: SearchResults, tab: Tab) -> impl IntoView {
    match tab {
        Tab::Albums => html! {
            <ListAlbums
                albums=search_results.albums
                sort=crate::components::list::AlbumSort::Default
            />
        }
        .into_any(),
        Tab::Artists => html! {
            <ListArtists
                artists=search_results.artists
                sort=crate::components::list::ArtistSort::Default
            />
        }
        .into_any(),
        Tab::Playlists => html! {
            <ListPlaylists
                playlists=search_results.playlists
                sort=crate::components::list::PlaylistSort::Default
            />
        }
        .into_any(),
        Tab::Tracks => html! { <ListTracks tracks=search_results.tracks /> }.into_any(),
    }
}

#[component]
fn list_tracks(tracks: Vec<models::Track>) -> impl IntoView {
    html! {
        <List>
            {tracks
                .into_iter()
                .map(|track| {
                    html! {
                        <ListItem>
                            <Track track=track />
                        </ListItem>
                    }
                })
                .collect::<Vec<_>>()}
        </List>
    }
}

#[component]
fn track(track: models::Track) -> impl IntoView {
    html! {
        <button
            class="flex gap-4 items-center w-full cursor-pointer"
            hx-put=format!("/play-track/{}", track.id)
            hx-swap="none"
        >
            <img
                class="inline text-sm text-gray-500 bg-gray-800 rounded-md aspect-square size-12"
                alt=track.title.clone()
                src=track.image_thumbnail
            />

            <div class="overflow-hidden w-full">
                <div class="flex justify-between items-center">
                    <h3 class="text-lg truncate">{track.title}</h3>
                    <Info explicit=track.explicit hires_available=track.hires_available />
                </div>

                <h4 class="flex gap-2 text-left text-gray-400">
                    {track
                        .artist_name
                        .map(|artist_name| {
                            html! { <span class="truncate">{artist_name}</span> }
                        })}
                </h4>
            </div>
        </button>
    }
}

#[component]
fn tab_bar(query: String, tab: Tab) -> impl IntoView {
    html! {
        <div
            id="tabs"
            class="flex justify-between *:rounded-full *:px-2 *:py-1 *:transition-colors"
        >
            {html! {
                <a
                    href=format!("albums?query={}", query)
                    class=(tab == Tab::Albums).then_some("bg-blue-800")
                >

                    Albums
                </a>
            }
                .attr("preload", "mouseover")
                .attr("preload-images", "true")}
            {html! {
                <a
                    href=format!("artists?query={}", query)
                    class=(tab == Tab::Artists).then_some("bg-blue-800")
                >
                    Artists
                </a>
            }
                .attr("preload", "mouseover")
                .attr("preload-images", "true")}
            {html! {
                <a
                    href=format!("playlists?query={}", query)
                    class=(tab == Tab::Playlists).then_some("bg-blue-800")
                >
                    Playlists
                </a>
            }
                .attr("preload", "mouseover")
                .attr("preload-images", "true")}
            {html! {
                <a
                    href=format!("tracks?query={}", query)
                    class=(tab == Tab::Tracks).then_some("bg-blue-800")
                >
                    Tracks
                </a>
            }
                .attr("preload", "mouseover")
                .attr("preload-images", "true")}

        </div>
    }
}

#[component]
fn search(search_results: SearchResults, tab: Tab) -> impl IntoView {
    let query = search_results.query.clone();
    html! {
        <div class="flex flex-col">
            <div class="flex sticky top-0 flex-col flex-grow gap-4 pb-2 max-h-full pt-safe-or-2 bg-black/80 backdrop-blur">
                <div class="flex flex-row gap-4 items-center" id="search-form">
                    <input
                        id="query"
                        name="query"
                        class="p-2 w-full text-black bg-white rounded"
                        autocapitalize="off"
                        autocomplete="off"
                        autocorrect="off"
                        placeholder="Search"
                        spellcheck="false"
                        type="search"
                        oninput="setSearchQuery(this.value)"
                        hx-post=""
                        hx-trigger="input changed delay:500ms, keyup[key=='Enter']"
                        hx-target="#search-results"
                        hx-swap="morph:innerHTML"
                    />
                    <span class="size-8">
                        <MagnifyingGlass />
                    </span>
                    <script>loadSearchInput()</script>
                </div>

                <TabBar query=query tab=tab.clone() />
            </div>

            <div id="search-results" class="overflow-auto h-full">
                <SearchPartial search_results=search_results tab=tab />
            </div>
        </div>
    }
}
