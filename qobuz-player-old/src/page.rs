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
            <meta
                name="viewport"
                content="width=device-width, initial-scale=1, maximum-scale=5 viewport-fit=cover"
            />
            <link rel="stylesheet" href=style_url />
            <AppleHead />

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
        <div class="pb-safe">
            <div class="h-12"></div>
        </div>
        <nav class="flex fixed bottom-0 justify-evenly w-full pb-safe px-safe backdrop-blur bg-black/80 *:flex *:h-[3.25rem] *:w-20 *:flex-col *:items-center *:overflow-visible *:text-nowrap *:px-4 *:py-1 *:text-[10px] *:font-medium *:transition-colors">
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

#[component]
fn apple_head() -> impl IntoView {
    html! {
        <link rel="apple-touch-icon" href="/assets/pwa/apple-icon-180.png" />
        <meta name="apple-mobile-web-app-capable" content="yes" />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-2048-2732.jpg"
            media="(device-width: 1024px) and (device-height: 1366px) and (-webkit-device-pixel-ratio: 2) and (orientation: portrait)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-2732-2048.jpg"
            media="(device-width: 1024px) and (device-height: 1366px) and (-webkit-device-pixel-ratio: 2) and (orientation: landscape)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-1668-2388.jpg"
            media="(device-width: 834px) and (device-height: 1194px) and (-webkit-device-pixel-ratio: 2) and (orientation: portrait)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-2388-1668.jpg"
            media="(device-width: 834px) and (device-height: 1194px) and (-webkit-device-pixel-ratio: 2) and (orientation: landscape)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-1536-2048.jpg"
            media="(device-width: 768px) and (device-height: 1024px) and (-webkit-device-pixel-ratio: 2) and (orientation: portrait)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-2048-1536.jpg"
            media="(device-width: 768px) and (device-height: 1024px) and (-webkit-device-pixel-ratio: 2) and (orientation: landscape)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-1640-2360.jpg"
            media="(device-width: 820px) and (device-height: 1180px) and (-webkit-device-pixel-ratio: 2) and (orientation: portrait)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-2360-1640.jpg"
            media="(device-width: 820px) and (device-height: 1180px) and (-webkit-device-pixel-ratio: 2) and (orientation: landscape)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-1668-2224.jpg"
            media="(device-width: 834px) and (device-height: 1112px) and (-webkit-device-pixel-ratio: 2) and (orientation: portrait)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-2224-1668.jpg"
            media="(device-width: 834px) and (device-height: 1112px) and (-webkit-device-pixel-ratio: 2) and (orientation: landscape)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-1620-2160.jpg"
            media="(device-width: 810px) and (device-height: 1080px) and (-webkit-device-pixel-ratio: 2) and (orientation: portrait)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-2160-1620.jpg"
            media="(device-width: 810px) and (device-height: 1080px) and (-webkit-device-pixel-ratio: 2) and (orientation: landscape)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-1488-2266.jpg"
            media="(device-width: 744px) and (device-height: 1133px) and (-webkit-device-pixel-ratio: 2) and (orientation: portrait)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-2266-1488.jpg"
            media="(device-width: 744px) and (device-height: 1133px) and (-webkit-device-pixel-ratio: 2) and (orientation: landscape)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-1320-2868.jpg"
            media="(device-width: 440px) and (device-height: 956px) and (-webkit-device-pixel-ratio: 3) and (orientation: portrait)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-2868-1320.jpg"
            media="(device-width: 440px) and (device-height: 956px) and (-webkit-device-pixel-ratio: 3) and (orientation: landscape)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-1206-2622.jpg"
            media="(device-width: 402px) and (device-height: 874px) and (-webkit-device-pixel-ratio: 3) and (orientation: portrait)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-2622-1206.jpg"
            media="(device-width: 402px) and (device-height: 874px) and (-webkit-device-pixel-ratio: 3) and (orientation: landscape)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-1260-2736.jpg"
            media="(device-width: 420px) and (device-height: 912px) and (-webkit-device-pixel-ratio: 3) and (orientation: portrait)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-2736-1260.jpg"
            media="(device-width: 420px) and (device-height: 912px) and (-webkit-device-pixel-ratio: 3) and (orientation: landscape)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-1290-2796.jpg"
            media="(device-width: 430px) and (device-height: 932px) and (-webkit-device-pixel-ratio: 3) and (orientation: portrait)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-2796-1290.jpg"
            media="(device-width: 430px) and (device-height: 932px) and (-webkit-device-pixel-ratio: 3) and (orientation: landscape)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-1179-2556.jpg"
            media="(device-width: 393px) and (device-height: 852px) and (-webkit-device-pixel-ratio: 3) and (orientation: portrait)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-2556-1179.jpg"
            media="(device-width: 393px) and (device-height: 852px) and (-webkit-device-pixel-ratio: 3) and (orientation: landscape)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-1170-2532.jpg"
            media="(device-width: 390px) and (device-height: 844px) and (-webkit-device-pixel-ratio: 3) and (orientation: portrait)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-2532-1170.jpg"
            media="(device-width: 390px) and (device-height: 844px) and (-webkit-device-pixel-ratio: 3) and (orientation: landscape)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-1284-2778.jpg"
            media="(device-width: 428px) and (device-height: 926px) and (-webkit-device-pixel-ratio: 3) and (orientation: portrait)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-2778-1284.jpg"
            media="(device-width: 428px) and (device-height: 926px) and (-webkit-device-pixel-ratio: 3) and (orientation: landscape)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-1125-2436.jpg"
            media="(device-width: 375px) and (device-height: 812px) and (-webkit-device-pixel-ratio: 3) and (orientation: portrait)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-2436-1125.jpg"
            media="(device-width: 375px) and (device-height: 812px) and (-webkit-device-pixel-ratio: 3) and (orientation: landscape)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-1242-2688.jpg"
            media="(device-width: 414px) and (device-height: 896px) and (-webkit-device-pixel-ratio: 3) and (orientation: portrait)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-2688-1242.jpg"
            media="(device-width: 414px) and (device-height: 896px) and (-webkit-device-pixel-ratio: 3) and (orientation: landscape)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-828-1792.jpg"
            media="(device-width: 414px) and (device-height: 896px) and (-webkit-device-pixel-ratio: 2) and (orientation: portrait)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-1792-828.jpg"
            media="(device-width: 414px) and (device-height: 896px) and (-webkit-device-pixel-ratio: 2) and (orientation: landscape)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-1242-2208.jpg"
            media="(device-width: 414px) and (device-height: 736px) and (-webkit-device-pixel-ratio: 3) and (orientation: portrait)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-2208-1242.jpg"
            media="(device-width: 414px) and (device-height: 736px) and (-webkit-device-pixel-ratio: 3) and (orientation: landscape)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-750-1334.jpg"
            media="(device-width: 375px) and (device-height: 667px) and (-webkit-device-pixel-ratio: 2) and (orientation: portrait)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-1334-750.jpg"
            media="(device-width: 375px) and (device-height: 667px) and (-webkit-device-pixel-ratio: 2) and (orientation: landscape)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-640-1136.jpg"
            media="(device-width: 320px) and (device-height: 568px) and (-webkit-device-pixel-ratio: 2) and (orientation: portrait)"
        />
        <link
            rel="apple-touch-startup-image"
            href="/assets/pwa/apple-splash-1136-640.jpg"
            media="(device-width: 320px) and (device-height: 568px) and (-webkit-device-pixel-ratio: 2) and (orientation: landscape)"
        />
    }
}
