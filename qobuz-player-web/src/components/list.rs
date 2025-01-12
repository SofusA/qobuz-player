use leptos::{component, prelude::*, IntoView};
use qobuz_player_controls::service::{Album, Artist, Playlist, Track};

use crate::{components::Info, html, icons::Play};

#[component]
pub fn list(children: Children) -> impl IntoView {
    html! { <ul class="overflow-y-auto w-full h-full leading-tight align-start">{children()}</ul> }
}

#[component]
pub fn list_item(children: Children) -> impl IntoView {
    html! {
        <li class="w-full text-left border-b border-gray-700 *:p-4 hover:bg-blue-500/25">
            {children()}
        </li>
    }
}

pub enum AlbumSort {
    Default,
    Artist,
    ReleaseYear,
}

#[component]
pub fn list_albums_vertical(albums: Vec<Album>) -> impl IntoView {
    html! {
        <div class="flex overflow-scroll gap-4 p-2 w-full">
            {albums
                .into_iter()
                .map(|album| {
                    html! {
                        <a href=format!("/album/{}", album.id) class="h-full shrink-0 size-32">
                            <img class="rounded-lg" alt=album.title.clone() src=album.cover_art />
                            <p class="text-sm truncate">{album.title}</p>
                            <p class="text-sm text-gray-500 truncate">{album.artist.name}</p>
                        </a>
                    }
                })
                .collect::<Vec<_>>()}
        </div>
    }
}

#[component]
pub fn list_artists_vertical(artists: Vec<Artist>) -> impl IntoView {
    html! {
        <div class="flex overflow-scroll gap-4 p-2 w-full">
            {artists
                .into_iter()
                .map(|artist| {
                    let artist_image_style = artist
                        .image
                        .map_or(
                            "".into(),
                            |image| format!("background-image: url({});", image.large),
                        );
                    html! {
                        <a href=format!("/artist/{}", artist.id) class="w-32 h-full text-center">
                            <div
                                class="bg-gray-500 bg-center bg-no-repeat bg-cover rounded-full aspect-square size-32"
                                style=artist_image_style
                            ></div>

                            <p class="text-sm truncate">{artist.name}</p>
                        </a>
                    }
                })
                .collect::<Vec<_>>()}
        </div>
    }
}

#[component]
pub fn list_albums(mut albums: Vec<Album>, sort: AlbumSort) -> impl IntoView {
    match sort {
        AlbumSort::Default => (),
        AlbumSort::Artist => albums.sort_by(|a, b| {
            a.artist
                .name
                .cmp(&b.artist.name)
                .then_with(|| b.release_year.cmp(&a.release_year))
        }),
        AlbumSort::ReleaseYear => {
            albums.sort_by_key(|album| album.release_year);
            albums.reverse();
        }
    };

    html! {
        <List>
            {albums
                .into_iter()
                .map(|album| {
                    html! {
                        <ListItem>
                            <a
                                class="flex gap-4 items-center w-full"
                                hx-push-url="true"
                                href=format!("/album/{}", album.id)
                            >
                                <img
                                    class="inline text-sm text-gray-500 bg-gray-800 rounded-md aspect-square size-12"
                                    alt=album.title.clone()
                                    src=album.cover_art_small
                                />

                                <div class="overflow-hidden w-full">
                                    <div class="flex justify-between">
                                        <h3 class="text-lg truncate">{album.title}</h3>
                                        <Info
                                            explicit=album.explicit
                                            hires_available=album.hires_available
                                        />
                                    </div>

                                    <h4 class="flex gap-2 text-left text-gray-400">
                                        <span class="truncate">{album.artist.name}</span>
                                        <span>"•︎"</span>
                                        <span>{album.release_year}</span>
                                    </h4>
                                </div>
                            </a>
                        </ListItem>
                    }
                })
                .collect::<Vec<_>>()}
        </List>
    }
}

pub enum ArtistSort {
    Default,
    Name,
}

#[component]
pub fn list_artists(mut artists: Vec<Artist>, sort: ArtistSort) -> impl IntoView {
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
                        .map_or(
                            "".into(),
                            |image| format!("background-image: url({});", image.large),
                        );
                    html! {
                        <ListItem>
                            <a
                                class="flex gap-4 items-center"
                                hx-push-url="true"
                                href=format!("/artist/{}", artist.id)
                            >
                                <div
                                    class="bg-gray-500 bg-center bg-no-repeat bg-cover rounded-full aspect-square size-12"
                                    style=artist_image_style
                                ></div>

                                <p class="w-full text-lg truncate">{artist.name}</p>
                            </a>
                        </ListItem>
                    }
                })
                .collect::<Vec<_>>()}
        </List>
    }
}

#[component]
pub fn list_tracks(
    tracks: Vec<Track>,
    now_playing_id: Option<u32>,
    show_track_number: bool,
    parent_id: String,
) -> impl IntoView {
    html! {
        <List>
            {tracks
                .into_iter()
                .map(|track| {
                    let now_playing = now_playing_id.is_some_and(|id| id == track.id);
                    let parent_id = parent_id.clone();
                    html! {
                        <ListItem>
                            <button

                                hx-swap="none"
                                hx-put=format!("{}/play/{}", parent_id, track.position)
                                class="flex justify-between items-center w-full text-left"
                            >
                                <span class="flex overflow-hidden gap-4 items-center w-full">
                                    <span class="w-5 text-center">
                                        {now_playing
                                            .then(|| {
                                                html! {
                                                    <span class="text-blue-500 size-4">
                                                        <Play />
                                                    </span>
                                                }
                                            })}
                                        {(!now_playing && show_track_number)
                                            .then(|| {
                                                html! {
                                                    <span class="text-gray-400">{track.position}.</span>
                                                }
                                            })}
                                    </span>

                                    <h2 class="w-full truncate">{track.title}</h2>
                                </span>
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

pub enum PlaylistSort {
    Default,
    Title,
}

#[component]
pub fn list_playlists(mut playlists: Vec<Playlist>, sort: PlaylistSort) -> impl IntoView {
    match sort {
        PlaylistSort::Default => (),
        PlaylistSort::Title => playlists.sort_by_key(|playlist| playlist.title.clone()),
    };

    html! {
        <List>
            {playlists
                .into_iter()
                .map(|playlist| {
                    html! {
                        <ListItem>
                            <a
                                class="flex gap-4 items-center w-full text-lg text-left"
                                href=format!("/playlist/{}", playlist.id)
                            >
                                <img
                                    class="inline text-sm text-gray-500 bg-gray-800 rounded-md aspect-square size-12"
                                    alt=playlist.title.clone()
                                    src=playlist.cover_art
                                />

                                <p class="w-full text-lg truncate">{playlist.title}</p>
                            </a>
                        </ListItem>
                    }
                })
                .collect::<Vec<_>>()}
        </List>
    }
}
