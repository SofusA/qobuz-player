use axum::{
    extract::{Path, Query},
    response::IntoResponse,
    routing::get,
    Form, Router,
};
use leptos::{component, prelude::*, IntoView};
use qobuz_player_controls::service::SearchResults;
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    components::{
        list::{ListAlbums, ListArtists, ListPlaylists},
        Tab,
    },
    html,
    icons::MagnifyingGlass,
    page::Page,
    view::render,
    AppState,
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/search/{tab}", get(index).post(search))
}

#[derive(Deserialize, Clone)]
struct SearchParameters {
    query: Option<String>,
}

async fn index(
    Path(tab): Path<Tab>,
    Query(parameters): Query<SearchParameters>,
) -> impl IntoResponse {
    let query = parameters.query;

    let search_results = match &query {
        Some(query) => qobuz_player_controls::search(query).await,
        None => SearchResults {
            query: query.clone().unwrap_or("".into()),
            albums: vec![],
            tracks: vec![],
            artists: vec![],
            playlists: vec![],
        },
    };

    let html = html! {
        <Page active_page=Page::Search>
            <Search search_results=search_results tab=tab />
        </Page>
    };

    render(html)
}

async fn search(Path(tab): Path<Tab>, Form(query): Form<SearchParameters>) -> impl IntoResponse {
    let query = query.query;

    let search_results = match &query {
        Some(query) => qobuz_player_controls::search(query).await,
        None => SearchResults {
            query: query.clone().unwrap_or("".into()),
            albums: vec![],
            tracks: vec![],
            artists: vec![],
            playlists: vec![],
        },
    };

    let html = html! {
        <SearchPartial search_results=search_results tab=tab.clone() />

        <TabBar query=query.unwrap_or_default() tab=tab />
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
    }
}

#[component]
fn tab_bar(query: String, tab: Tab) -> impl IntoView {
    html! {
        <div
            id="tabs"
            hx-swap-oob="true"
            class="flex justify-between *:rounded-full *:px-2 *:py-1 *:transition-colors"
        >
            <a
                href=format!("albums?query={}", query)
                class=format!(
                    "hover:bg-blue-600 {}",
                    if tab == Tab::Albums { "bg-blue-800" } else { "" },
                )
            >
                Albums
            </a>
            <a
                href=format!("artists?query={}", query)
                class=format!(
                    "hover:bg-blue-600 {}",
                    if tab == Tab::Artists { "bg-blue-800" } else { "" },
                )
            >
                Artists
            </a>
            <a
                href=format!("playlists?query={}", query)
                class=format!(
                    "hover:bg-blue-600 {}",
                    if tab == Tab::Playlists { "bg-blue-800" } else { "" },
                )
            >
                Playlists
            </a>
        </div>
    }
}

#[component]
fn search(search_results: SearchResults, tab: Tab) -> impl IntoView {
    let query = search_results.query.clone();
    html! {
        <div class="flex flex-col h-full">
            <div class="flex flex-col flex-grow gap-4 p-4 max-h-full">
                <form
                    class="flex flex-row gap-4 items-center"
                    id="search-form"
                    hx-preserve
                    hx-post=""
                    hx-trigger="input from:#query delay:500ms, change"
                    hx-target="#search-results"
                    hx-swap="innerHTML"
                >
                    <input
                        id="query"
                        name="query"
                        class="p-2 w-full text-black rounded"
                        autocapitalize="off"
                        autocomplete="off"
                        autocorrect="off"
                        placeholder="Search"
                        spellcheck="false"
                        type="search"
                        autofocus
                    />
                    <span class="size-8">
                        <MagnifyingGlass />
                    </span>
                </form>

                <TabBar query=query tab=tab.clone() />
            </div>

            <div id="search-results" class="overflow-auto h-full">
                <SearchPartial search_results=search_results tab=tab />
            </div>
        </div>
    }
}
