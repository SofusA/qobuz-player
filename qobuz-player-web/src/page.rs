use leptos::{IntoView, component, prelude::*};
use qobuz_player_controls::{Status, tracklist::Tracklist};

use crate::{
    html,
    icons::{self, MagnifyingGlass, PlayCircle, QueueList, Star},
    routes::controls::Controls,
};

#[derive(PartialEq)]
pub(crate) enum Page {
    NowPlaying,
    Queue,
    Favorites,
    Search,
    Discover,
    None,
}

#[component]
pub(crate) fn page<'a>(
    children: Children,
    active_page: Page,
    current_status: Status,
    tracklist: &'a Tracklist,
) -> impl IntoView {
    html! {
        <!DOCTYPE html>
        <html lang="en" class="dark">
            <Head load_htmx=true />
            <body
                class="text-gray-50 bg-black touch-pan-y"
                hx-ext="preload, remove-me, morph"
                hx-indicator="#loading-spinner"
            >
                <div
                    id="loading-spinner"
                    hx-preserve
                    class="fixed top-8 right-8 z-10 p-2 rounded-lg pointer-events-none m-safe bg-gray-900/20 my-indicator backdrop-blur size-12"
                >
                    <icons::LoadingSpinner />
                </div>

                <div
                    id="toast-container"
                    class="flex fixed top-8 right-8 z-20 flex-col gap-4"
                ></div>

                {children()}
                {(active_page != Page::NowPlaying)
                    .then(|| {
                        html! { <Controls current_status=current_status tracklist=tracklist /> }
                    })}
                <Navigation active_page=active_page />

            </body>
        </html>
    }
}

#[component]
pub(crate) fn unauthorized_page(children: Children) -> impl IntoView {
    html! {
        <!DOCTYPE html>
        <html lang="en" class="h-full dark">
            <Head load_htmx=false />
            <body class="flex flex-col justify-between h-full text-gray-50 bg-black">
                {children()}
            </body>
        </html>
    }
}

#[component]
fn head(load_htmx: bool) -> impl IntoView {
    let style_url = "/assets/styles.css?version=16";
    html! {
        <head>
            <title>Qobuz Player</title>
            <link rel="shortcut icon" href="/assets/favicon.svg" type="image/svg" />
            <link rel="manifest" href="/assets/manifest.json" />
            <link rel="apple-touch-icon" href="/assets/apple-touch-icon.png" />
            <meta
                name="viewport"
                content="width=device-width, initial-scale=1, maximum-scale=5 viewport-fit=cover"
            />
            <meta name="mobile-web-app-capable" content="yes" />
            <meta name="apple-mobile-web-app-status-bar-style" content="black-translucent" />
            <link rel="stylesheet" href=style_url />
            {load_htmx
                .then_some({
                    html! {
                        <script src="https://unpkg.com/htmx.org@2.0.4"></script>
                        <script src="https://unpkg.com/htmx-ext-preload@2.1.0/preload.js"></script>
                        <script src="https://unpkg.com/htmx-ext-remove-me@2.0.0/remove-me.js"></script>
                        <script src="https://unpkg.com/idiomorph@0.7.3"></script>
                        <script src="/assets/script.js?version=1"></script>
                    }
                })}
        </head>
    }
}

#[component]
fn navigation(active_page: Page) -> impl IntoView {
    html! {
        <nav class="flex fixed bottom-0 justify-evenly w-full backdrop-blur bg-black/80 *:flex *:h-[3.25rem] *:w-20 *:flex-col *:items-center *:overflow-visible *:text-nowrap *:px-4 *:py-1 *:text-[10px] *:font-medium *:transition-colors">
            {html! {
                <a
                    href="/"
                    class=if active_page == Page::NowPlaying {
                        "text-blue-500"
                    } else {
                        "text-gray-500"
                    }
                >
                    <PlayCircle />
                    Now Playing
                </a>
            }
                .attr("preload", "mouseover")
                .attr("preload-images", "true")}
            <a
                href="/queue"
                class=if active_page == Page::Queue { "text-blue-500" } else { "text-gray-500" }
            >
                <QueueList />
                Queue
            </a>
            {html! {
                <a
                    href="/discover"
                    class=if active_page == Page::Discover {
                        "text-blue-500"
                    } else {
                        "text-gray-500"
                    }
                >
                    <icons::Megaphone solid=true />
                    Discover
                </a>
            }
                .attr("preload", "mouseover")
                .attr("preload-images", "true")}
            {html! {
                <a
                    href="/favorites/albums"
                    class=if active_page == Page::Favorites {
                        "text-blue-500"
                    } else {
                        "text-gray-500"
                    }
                >
                    <Star solid=true />
                    Favorites
                </a>
            }
                .attr("preload", "mouseover")
                .attr("preload-images", "true")}
            {if active_page == Page::Search {
                html! {
                    <button class="text-blue-500" onclick="focusSearchInput()">
                        <MagnifyingGlass />
                        Search
                    </button>
                }
                    .into_any()
            } else {
                html! {
                    <a href="/search/albums" class="text-gray-500">
                        <MagnifyingGlass />
                        Search
                    </a>
                }
                    .into_any()
            }}
        </nav>
    }
}
