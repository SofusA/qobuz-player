use leptos::{IntoView, component, prelude::*};
use qobuz_player_models::{Album, AlbumSimple, Artist, Playlist, Track};

use crate::{
    components::Info,
    html,
    icons::{Play, User},
};

#[component]
pub(crate) fn list(children: Children) -> impl IntoView {
    html! { <ul class="overflow-y-auto w-full h-full leading-tight align-start">{children()}</ul> }
}

#[component]
pub(crate) fn list_item(children: Children) -> impl IntoView {
    html! { <li class="w-full text-left border-b border-gray-700 *:p-4">{children()}</li> }
}

#[component]
pub(crate) fn list_albums_vertical(albums: Vec<AlbumSimple>) -> impl IntoView {
    html! {
        <div class="flex overflow-scroll gap-4 p-2 w-full">
            {albums
                .into_iter()
                .map(|album| {
                    html! {
                        <a href=format!("/album/{}", album.id) class="h-full shrink-0 size-32">
                            <img class="rounded-lg" alt=album.title.clone() src=album.image />
                            <p class="text-sm truncate">{album.title}</p>
                            <p class="text-sm text-gray-500 truncate">{album.artist.name}</p>
                        </a>
                    }
                        .attr("preload", "mousedown")
                        .attr("preload-images", "true")
                })
                .collect::<Vec<_>>()}
        </div>
    }
}

#[component]
pub(crate) fn list_playlists_vertical(playlists: Vec<Playlist>) -> impl IntoView {
    html! {
        <div class="flex overflow-scroll gap-4 p-2 w-full">
            {playlists
                .into_iter()
                .map(|playlist| {
                    let img_src = playlist
                        .image
                        .map(|image| format!("background-image: url({image});"));
                    html! {
                        <a
                            href=format!("/playlist/{}", playlist.id)
                            class="h-full shrink-0 size-32"
                        >
                            <div
                                class="bg-gray-800 bg-center bg-no-repeat bg-cover rounded-lg aspect-square"
                                style=img_src
                            ></div>
                            <p class="text-sm truncate">{playlist.title}</p>
                        </a>
                    }
                        .attr("preload", "mousedown")
                })
                .collect::<Vec<_>>()}
        </div>
    }
}

#[component]
pub(crate) fn list_artists_vertical(artists: Vec<Artist>) -> impl IntoView {
    html! {
        <div class="flex overflow-scroll gap-4 p-2 w-full">
            {artists
                .into_iter()
                .map(|artist| {
                    let artist_image_style = artist
                        .image
                        .map(|image| format!("background-image: url({image});"));
                    html! {
                        <a href=format!("/artist/{}", artist.id) class="w-32 h-full text-center">
                            {match artist_image_style {
                                Some(img_src) => {
                                    html! {
                                        <div
                                            class="bg-gray-800 bg-center bg-no-repeat bg-cover rounded-full aspect-square size-32"
                                            style=img_src
                                        ></div>
                                    }
                                        .into_any()
                                }
                                None => {
                                    html! {
                                        <div class="flex justify-center items-center bg-gray-500 rounded-full aspect-square size-32">
                                            <div class="w-20">
                                                <User />
                                            </div>
                                        </div>
                                    }
                                        .into_any()
                                }
                            }}

                            <p class="text-sm truncate">{artist.name}</p>
                        </a>
                    }
                        .attr("preload", "mousedown")
                        .attr("preload-images", "true")
                })
                .collect::<Vec<_>>()}
        </div>
    }
}

pub(crate) enum AlbumSort {
    Default,
    Artist,
}

#[component]
pub(crate) fn list_albums(mut albums: Vec<Album>, sort: AlbumSort) -> impl IntoView {
    match sort {
        AlbumSort::Default => (),
        AlbumSort::Artist => albums.sort_by(|a, b| {
            a.artist
                .name
                .cmp(&b.artist.name)
                .then_with(|| b.release_year.cmp(&a.release_year))
        }),
    };

    html! {
        <List>
            {albums
                .into_iter()
                .map(|album| {
                    html! {
                        <ListItem>
                            <Album album=album />
                        </ListItem>
                    }
                })
                .collect::<Vec<_>>()}
        </List>
    }
}

#[component]
fn album(album: Album) -> impl IntoView {
    html! {
        <a
            class="flex gap-4 items-center w-full"
            hx-push-url="true"
            href=format!("/album/{}", album.id)
        >
            <img
                class="inline text-sm text-gray-500 bg-gray-800 rounded-md aspect-square size-12"
                alt=album.title.clone()
                src=album.image_thumbnail
            />

            <div class="overflow-hidden w-full">
                <div class="flex justify-between">
                    <h3 class="text-lg truncate">{album.title}</h3>
                    <Info explicit=album.explicit hires_available=album.hires_available />
                </div>

                <h4 class="flex gap-2 text-left text-gray-400">
                    <span class="truncate">{album.artist.name}</span>
                    <span>"•︎"</span>
                    <span>{album.release_year}</span>
                </h4>
            </div>
        </a>
    }
    .attr("preload", "mousedown")
    .attr("preload-images", "true")
}

pub(crate) enum ArtistSort {
    Default,
    Name,
}

#[component]
pub(crate) fn list_artists(mut artists: Vec<Artist>, sort: ArtistSort) -> impl IntoView {
    match sort {
        ArtistSort::Default => (),
        ArtistSort::Name => artists.sort_by_key(|artist| artist.name.clone()),
    };

    html! {
        <List>
            {artists
                .into_iter()
                .map(|artist| {
                    let artist_image_style = artist
                        .image
                        .map(|image| format!("background-image: url({image});"));
                    html! {
                        <ListItem>
                            {html! {
                                <a
                                    class="flex gap-4 items-center"
                                    hx-push-url="true"
                                    href=format!("/artist/{}", artist.id)
                                >
                                    {match artist_image_style {
                                        Some(img_src) => {
                                            html! {
                                                <div
                                                    class="bg-gray-800 bg-center bg-no-repeat bg-cover rounded-full aspect-square size-12"
                                                    style=img_src
                                                ></div>
                                            }
                                                .into_any()
                                        }
                                        None => {
                                            html! {
                                                <div class="flex justify-center items-center bg-gray-500 rounded-full aspect-square size-12">
                                                    <div class="w-8">
                                                        <User />
                                                    </div>
                                                </div>
                                            }
                                                .into_any()
                                        }
                                    }}
                                    <p class="w-full text-lg truncate">{artist.name}</p>
                                </a>
                            }
                                .attr("preload", "mousedown")
                                .attr("preload-images", "true")}
                        </ListItem>
                    }
                })
                .collect::<Vec<_>>()}
        </List>
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub(crate) enum TrackNumberDisplay {
    Number,
    Cover,
}

#[component]
pub(crate) fn list_tracks(
    tracks: Vec<Track>,
    track_number_display: TrackNumberDisplay,
    show_artist: bool,
    dim_played: bool,
    now_playing_id: Option<u32>,
    #[prop(into)] api_call: Callback<(usize,), String>,
) -> impl IntoView {
    html! {
        <List>
            {tracks
                .into_iter()
                .enumerate()
                .map(|(index, track)| {
                    let is_playing = now_playing_id.is_some_and(|id| id == track.id);
                    html! {
                        <ListItem>
                            <button
                                hx-swap="none"
                                hx-put=api_call.run((index,))
                                class=format!(
                                    "flex justify-between items-center w-full text-left cursor-pointer disabled:text-gray-500 disabled:cursor-default {}",
                                    if dim_played { "disabled:text-gray-500" } else { "" },
                                )
                                disabled=!track.available
                            >
                                <div class="flex overflow-hidden gap-4 items-center w-full">
                                    <div class=format!(
                                        "flex justify-center items-center {}",
                                        if track_number_display == TrackNumberDisplay::Cover {
                                            "size-12 aspect-square"
                                        } else {
                                            ""
                                        },
                                    )>
                                        {is_playing
                                            .then_some({
                                                html! {
                                                    <div class="text-blue-500 size-5">
                                                        <Play />
                                                    </div>
                                                }
                                            })}
                                        {(!is_playing)
                                            .then_some({
                                                html! {
                                                    {match track_number_display {
                                                        TrackNumberDisplay::Number => {
                                                            html! {
                                                                <span class="w-5 text-center text-gray-400">
                                                                    {track.number}
                                                                </span>
                                                            }
                                                                .into_any()
                                                        }
                                                        TrackNumberDisplay::Cover => {
                                                            html! {
                                                                <div
                                                                    class="bg-gray-800 bg-center bg-no-repeat bg-cover rounded-md aspect-square size-12"
                                                                    style=track
                                                                        .image
                                                                        .map(|image| { format!("background-image: url({image});") })
                                                                ></div>
                                                            }
                                                                .into_any()
                                                        }
                                                    }}
                                                }
                                            })}
                                    </div>

                                    {match show_artist && track.artist_name.is_some() {
                                        true => {
                                            html! {
                                                <div class="flex overflow-hidden flex-col">
                                                    <h2 class="truncate">{track.title}</h2>
                                                    <h3 class="text-sm text-gray-400 truncate">
                                                        {track.artist_name.unwrap_or_default()}
                                                    </h3>
                                                </div>
                                            }
                                                .into_any()
                                        }
                                        false => {
                                            html! { <h2 class="w-full truncate">{track.title}</h2> }
                                                .into_any()
                                        }
                                    }}

                                </div>
                                <Info
                                    explicit=track.explicit
                                    hires_available=track.hires_available
                                />
                            </button>
                        </ListItem>
                    }
                })
                .collect::<Vec<_>>()}
        </List>
    }
}

pub(crate) enum PlaylistSort {
    Default,
    Title,
}

#[component]
pub(crate) fn list_playlists(mut playlists: Vec<Playlist>, sort: PlaylistSort) -> impl IntoView {
    match sort {
        PlaylistSort::Default => (),
        PlaylistSort::Title => playlists.sort_by_key(|playlist| playlist.title.clone()),
    };

    html! {
        <List>
            {playlists
                .into_iter()
                .map(|playlist| {
                    let img_src = playlist
                        .image
                        .map(|image| format!("background-image: url({image});"));
                    html! {
                        <ListItem>
                            {html! {
                                <a
                                    class="flex gap-4 items-center w-full text-lg text-left"
                                    href=format!("/playlist/{}", playlist.id)
                                >
                                    <div
                                        class="bg-gray-800 bg-center bg-no-repeat bg-cover rounded-md aspect-square size-12"
                                        style=img_src
                                    ></div>

                                    <p class="w-full text-lg truncate">{playlist.title}</p>
                                </a>
                            }
                                .attr("preload", "mousedown")}
                        </ListItem>
                    }
                })
                .collect::<Vec<_>>()}
        </List>
    }
}
