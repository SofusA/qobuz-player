use std::sync::{Arc, OnceLock};

use cursive::{
    align::HAlign,
    direction::Orientation,
    event::Event,
    reexports::crossbeam_channel::Sender,
    theme::{BorderStyle, Effect, Palette, Style},
    utils::{markup::StyledString, Counter},
    view::{Nameable, Resizable, Scrollable, SizeConstraint},
    views::{
        Dialog, EditView, HideableView, LinearLayout, MenuPopup, PaddedView, Panel, ProgressBar,
        ResizedView, ScreensView, ScrollView, SelectView, TextView,
    },
    Cursive, With,
};
use futures::executor::block_on;
use gstreamer::{ClockTime, State as GstState};
use qobuz_player_controls::{
    notification::Notification,
    service::{Album, Artist, Favorites, Playlist, SearchResults, Track, TrackStatus},
    tracklist::TrackListType,
};
use tracing::debug;

type CursiveSender = Sender<Box<dyn FnOnce(&mut Cursive) + Send>>;

static SINK: OnceLock<CursiveSender> = OnceLock::new();

static UNSTREAMABLE: &str = "UNSTREAMABLE";

pub async fn init() {
    let mut siv = cursive::default();

    SINK.set(siv.cb_sink().clone()).expect("error setting sink");
    tokio::spawn(async { receive_notifications().await });

    siv.set_theme(cursive::theme::Theme {
        shadow: false,
        borders: BorderStyle::Simple,
        palette: Palette::terminal_default().with(|palette| {
            use cursive::theme::BaseColor::*;

            {
                use cursive::theme::Color::TerminalDefault;
                use cursive::theme::PaletteColor::*;

                palette[Background] = TerminalDefault;
                palette[View] = TerminalDefault;
                palette[Primary] = White.dark();
                palette[Highlight] = Cyan.dark();
                palette[HighlightInactive] = Black.dark();
                palette[HighlightText] = Black.dark();
            }

            {
                use cursive::theme::Color::TerminalDefault;
                use cursive::theme::Effect::*;
                use cursive::theme::PaletteStyle::*;

                palette[Highlight] = Style::from(Cyan.dark())
                    .combine(Underline)
                    .combine(Reverse)
                    .combine(Bold);
                palette[HighlightInactive] = Style::from(TerminalDefault).combine(Reverse);
                palette[TitlePrimary] = Style::from(Cyan.dark()).combine(Bold);
            }
        }),
    });

    let player = player();
    let search = search();

    let favorites = qobuz_player_controls::favorites().await;

    let Favorites {
        albums,
        artists,
        playlists,
    } = favorites;

    let favorite_albums = favorite_albums(albums);
    let favorite_artists = favorite_artists(artists);
    let favorite_playlists = favorite_playlists(playlists);

    siv.screen_mut().add_fullscreen_layer(PaddedView::lrtb(
        0,
        0,
        1,
        0,
        player.resized(SizeConstraint::Full, SizeConstraint::Free),
    ));

    siv.add_active_screen();
    siv.screen_mut().add_fullscreen_layer(PaddedView::lrtb(
        0,
        0,
        1,
        0,
        favorite_albums.resized(SizeConstraint::Full, SizeConstraint::Free),
    ));

    siv.add_active_screen();
    siv.screen_mut().add_fullscreen_layer(PaddedView::lrtb(
        0,
        0,
        1,
        0,
        favorite_artists.resized(SizeConstraint::Full, SizeConstraint::Free),
    ));

    siv.add_active_screen();
    siv.screen_mut().add_fullscreen_layer(PaddedView::lrtb(
        0,
        0,
        1,
        0,
        favorite_playlists.resized(SizeConstraint::Full, SizeConstraint::Free),
    ));

    siv.add_active_screen();
    siv.screen_mut().add_fullscreen_layer(PaddedView::lrtb(
        0,
        0,
        1,
        0,
        search.resized(SizeConstraint::Full, SizeConstraint::Free),
    ));

    siv.set_screen(0);

    global_events(&mut siv);
    menubar(&mut siv);
    siv.run();
}

fn player() -> LinearLayout {
    let mut container = LinearLayout::new(Orientation::Vertical);
    let mut track_info = LinearLayout::new(Orientation::Horizontal);

    let meta = PaddedView::lrtb(
        1,
        1,
        0,
        0,
        LinearLayout::new(Orientation::Vertical)
            .child(
                TextView::new("")
                    .style(Style::highlight().combine(Effect::Bold))
                    .with_name("current_track_title")
                    .scrollable()
                    .show_scrollbars(false)
                    .scroll_x(true),
            )
            .child(TextView::new("").with_name("artist_name"))
            .child(
                TextView::new("")
                    .with_name("entity_title")
                    .scrollable()
                    .show_scrollbars(false)
                    .scroll_x(true),
            ),
    )
    .resized(SizeConstraint::Full, SizeConstraint::Free);

    let track_num = LinearLayout::new(Orientation::Vertical)
        .child(
            TextView::new("000")
                .h_align(HAlign::Left)
                .with_name("current_track_number"),
        )
        .child(TextView::new("of").h_align(HAlign::Center))
        .child(
            TextView::new("000")
                .h_align(HAlign::Left)
                .with_name("total_tracks"),
        )
        .fixed_width(3);

    let player_status = LinearLayout::new(Orientation::Vertical)
        .child(
            TextView::new(format!(" {}", '\u{23f9}'))
                .h_align(HAlign::Center)
                .with_name("player_status"),
        )
        .fixed_width(8);

    let counter = Counter::new(0);
    let progress = ProgressBar::new()
        .with_value(counter)
        .with_label(|value, (_, max)| {
            let position =
                ClockTime::from_seconds(value as u64).to_string().as_str()[2..7].to_string();
            let duration =
                ClockTime::from_seconds(max as u64).to_string().as_str()[2..7].to_string();

            format!("{position} / {duration}")
        })
        .with_name("progress");

    track_info.add_child(track_num);
    track_info.add_child(meta);
    track_info.add_child(player_status);

    container.add_child(track_info);
    container.add_child(progress);

    let mut track_list: SelectView<usize> = SelectView::new();

    track_list.set_on_submit(move |_s, item| {
        let i = item.to_owned();
        tokio::spawn(async move { qobuz_player_controls::skip_to_position(i as u32, true).await });
    });

    let mut layout = LinearLayout::new(Orientation::Vertical).child(
        Panel::new(container)
            .title("player")
            .with_name("player_panel"),
    );

    layout.add_child(Panel::new(
        HideableView::new(
            track_list
                .scrollable()
                .scroll_y(true)
                .scroll_x(true)
                .with_name("current_track_list"),
        )
        .visible(true),
    ));

    layout
}

fn global_events(s: &mut Cursive) {
    s.clear_global_callbacks(Event::CtrlChar('c'));

    s.set_on_pre_event(Event::CtrlChar('c'), move |s| {
        let dialog = Dialog::text("Do you want to quit?")
            .button("Yes", move |s: &mut Cursive| {
                s.quit();
            })
            .dismiss_button("No");

        s.add_layer(dialog);
    });

    s.add_global_callback('1', move |s| {
        s.set_screen(0);
    });

    s.add_global_callback('2', move |s| {
        s.set_screen(1);
    });

    s.add_global_callback('3', move |s| {
        s.set_screen(2);
    });

    s.add_global_callback('4', move |s| {
        s.set_screen(3);
    });

    s.add_global_callback('5', move |s| {
        s.set_screen(4);
    });

    s.add_global_callback(' ', move |_| {
        block_on(async { qobuz_player_controls::play_pause().await.expect("") });
    });

    s.add_global_callback('N', move |_| {
        block_on(async { qobuz_player_controls::next().await.expect("") });
    });

    s.add_global_callback('P', move |_| {
        block_on(async { qobuz_player_controls::previous().await.expect("") });
    });

    s.add_global_callback('l', move |_| {
        block_on(async { qobuz_player_controls::jump_forward().await.expect("") });
    });

    s.add_global_callback('h', move |_| {
        block_on(async { qobuz_player_controls::jump_backward().await.expect("") });
    });
}

fn menubar(s: &mut Cursive) {
    s.set_autohide_menu(false);

    s.menubar()
        .add_leaf("Now Playing [1]", move |s| {
            s.set_screen(0);
        })
        .add_delimiter()
        .add_leaf("Favorite albums [2]", move |s| {
            s.set_screen(1);
        })
        .add_delimiter()
        .add_leaf("Favorite artists [3]", move |s| {
            s.set_screen(2);
        })
        .add_delimiter()
        .add_leaf("Playlists [4]", move |s| {
            s.set_screen(3);
        })
        .add_delimiter()
        .add_leaf("Search [5]", move |s| {
            s.set_screen(4);
        })
        .add_delimiter();

    s.add_global_callback('1', move |s| {
        s.set_screen(0);
    });
    s.add_global_callback('2', move |s| {
        s.set_screen(1);
    });
    s.add_global_callback('3', move |s| {
        s.set_screen(2);
    });
}

fn favorite_albums(favorite_albums: Vec<Album>) -> LinearLayout {
    let mut list_layout = LinearLayout::new(Orientation::Vertical);

    let mut album_list = SelectView::new();
    favorite_albums.iter().for_each(|p| {
        album_list.add_item(p.title.clone(), p.id.clone());
    });

    album_list.set_on_submit(move |_s: &mut Cursive, item: &String| {
        let item = item.clone();
        tokio::spawn(async move { qobuz_player_controls::play_album(&item).await });
    });

    list_layout.add_child(
        Panel::new(
            album_list
                .scrollable()
                .scroll_y(true)
                .resized(SizeConstraint::Full, SizeConstraint::Free),
        )
        .title("albums")
        .with_name("albums"),
    );

    list_layout
}

fn favorite_artists(favorite_artists: Vec<Artist>) -> LinearLayout {
    let mut list_layout = LinearLayout::new(Orientation::Vertical);

    let mut artist_list = SelectView::new();
    favorite_artists.iter().for_each(|p| {
        artist_list.add_item(p.name.clone(), p.id);
    });

    artist_list.set_on_submit(move |s: &mut Cursive, item: &u32| {
        submit_artist(s, *item as i32);
    });

    list_layout.add_child(
        Panel::new(
            artist_list
                .scrollable()
                .scroll_y(true)
                .resized(SizeConstraint::Full, SizeConstraint::Free),
        )
        .title("artists")
        .with_name("artists"),
    );

    list_layout
}

fn favorite_playlists(favorite_playlists: Vec<Playlist>) -> LinearLayout {
    let mut list_layout = LinearLayout::new(Orientation::Vertical);

    let mut playlist_list = SelectView::new();
    favorite_playlists.iter().for_each(|p| {
        playlist_list.add_item(p.title.clone(), p.id);
    });

    playlist_list.set_on_submit(move |_s: &mut Cursive, item: &u32| {
        let item = *item;
        tokio::spawn(async move { qobuz_player_controls::play_playlist(item as i64).await });
    });

    list_layout.add_child(
        Panel::new(
            playlist_list
                .scrollable()
                .scroll_y(true)
                .resized(SizeConstraint::Full, SizeConstraint::Free),
        )
        .title("playlists")
        .with_name("playlists"),
    );

    list_layout
}

fn search() -> LinearLayout {
    let mut layout = LinearLayout::new(Orientation::Vertical);

    let on_submit = move |s: &mut Cursive, item: &String| {
        load_search_results(item, s);
    };

    let search_type = SelectView::new()
        .item_str("Albums")
        .item_str("Artists")
        .item_str("Playlists")
        .on_submit(on_submit)
        .popup()
        .with_name("search_type")
        .wrap_with(Panel::new);

    let search_form = EditView::new()
        .on_submit_mut(move |_, item| {
            let item = item.to_string();

            tokio::spawn(async move {
                let results = qobuz_player_controls::search(&item).await;

                SINK.get()
                    .unwrap()
                    .send(Box::new(move |s| {
                        s.set_user_data(results);

                        if let Some(view) = s.find_name::<SelectView>("search_type") {
                            if let Some(value) = view.selection() {
                                load_search_results(&value, s);
                            }
                        }
                    }))
                    .expect("failed to send update");
            });
        })
        .wrap_with(Panel::new);

    let search_results: SelectView<String> = SelectView::new();

    layout.add_child(search_form.title("search"));
    layout.add_child(search_type);

    layout.add_child(
        Panel::new(
            search_results
                .with_name("search_results")
                .scrollable()
                .scroll_y(true)
                .scroll_x(true)
                .resized(SizeConstraint::Free, SizeConstraint::Full),
        )
        .title("results"),
    );

    layout
}

fn load_search_results(item: &str, s: &mut Cursive) {
    if let Some(mut search_results) = s.find_name::<SelectView>("search_results") {
        search_results.clear();

        if let Some(data) = s.user_data::<SearchResults>() {
            match item {
                "Albums" => {
                    for a in &data.albums {
                        let id = if a.available {
                            a.id.clone()
                        } else {
                            UNSTREAMABLE.to_string()
                        };

                        search_results.add_item(a.list_item(), id);
                    }

                    search_results.set_on_submit(move |_s: &mut Cursive, item: &String| {
                        if item != UNSTREAMABLE {
                            let item = item.clone();
                            tokio::spawn(
                                async move { qobuz_player_controls::play_album(&item).await },
                            );
                        }
                    });
                }
                "Artists" => {
                    for a in &data.artists {
                        search_results.add_item(a.name.clone(), a.id.to_string());
                    }

                    search_results.set_on_submit(move |s: &mut Cursive, item: &String| {
                        submit_artist(s, item.parse::<i32>().expect("failed to parse string"));
                    });
                }
                "Playlists" => {
                    for p in &data.playlists {
                        search_results.add_item(p.title.clone(), p.id.to_string())
                    }

                    search_results.set_on_submit(move |_s: &mut Cursive, item: &String| {
                        let item = item.parse::<i64>().expect("failed to parse string");
                        tokio::spawn(
                            async move { qobuz_player_controls::play_playlist(item).await },
                        );
                    });
                }
                _ => {}
            }
        }
    }
}

fn submit_artist(s: &mut Cursive, item: i32) {
    let artist_albums = block_on(async { qobuz_player_controls::artist_albums(item).await });

    if !artist_albums.is_empty() {
        let mut tree = cursive::menu::Tree::new();

        for a in artist_albums {
            if !a.available {
                continue;
            }

            tree.add_leaf(a.list_item(), move |s: &mut Cursive| {
                let id = a.id.clone();
                tokio::spawn(async move { qobuz_player_controls::play_album(&id).await });

                s.call_on_name(
                    "screens",
                    |screens: &mut ScreensView<ResizedView<LinearLayout>>| {
                        screens.set_active_screen(0);
                    },
                );
            });
        }

        let album_list = MenuPopup::new(Arc::new(tree));

        let events = album_list
            .scrollable()
            .resized(SizeConstraint::Full, SizeConstraint::Free);

        s.screen_mut().add_layer(events);
    }
}

fn set_current_track(s: &mut Cursive, track: &Track, lt: &TrackListType) {
    if let (Some(mut track_num), Some(mut track_title), Some(mut progress)) = (
        s.find_name::<TextView>("current_track_number"),
        s.find_name::<TextView>("current_track_title"),
        s.find_name::<ProgressBar>("progress"),
    ) {
        match lt {
            TrackListType::Album => {
                track_num.set_content(format!("{:03}", track.number));
            }
            TrackListType::Playlist => {
                track_num.set_content(format!("{:03}", track.position));
            }
            TrackListType::Track => {
                track_num.set_content(format!("{:03}", track.number));
            }
            TrackListType::Unknown => {
                track_num.set_content(format!("{:03}", track.position));
            }
        };

        track_title.set_content(track.title.trim());
        progress.set_max(track.duration_seconds as usize);
    }

    if let Some(artist) = &track.artist {
        s.call_on_name("artist_name", |view: &mut TextView| {
            view.set_content(artist.name.clone());
        });
    }
}

fn get_state_icon(state: GstState) -> String {
    match state {
        GstState::Playing => {
            format!(" {}", '\u{23f5}')
        }
        GstState::Paused => {
            format!(" {}", '\u{23f8}')
        }
        GstState::Ready => {
            format!(" {}", '\u{23f9}')
        }
        GstState::Null => {
            format!(" {}", '\u{23f9}')
        }
        _ => format!(" {}", '\u{23f9}'),
    }
}

async fn receive_notifications() {
    let mut broadcast_receiver = qobuz_player_controls::notify_receiver();

    loop {
        if let Ok(notification) = broadcast_receiver.recv().await {
            match notification {
                Notification::Quit => {
                    debug!("exiting tui notification thread");
                    return;
                }
                Notification::Status { status } => {
                    if SINK
                        .get()
                        .unwrap()
                        .send(Box::new(move |s| {
                            if let Some(mut view) = s.find_name::<TextView>("player_status") {
                                view.set_content(get_state_icon(status));
                                match status {
                                    GstState::Ready => {
                                        s.call_on_name("progress", |progress: &mut ProgressBar| {
                                            progress.set_value(0);
                                        });
                                    }
                                    GstState::Null => {
                                        s.call_on_name("progress", |progress: &mut ProgressBar| {
                                            progress.set_value(0);
                                        });
                                    }
                                    _ => {}
                                }
                            }
                        }))
                        .is_ok()
                    {}
                }
                Notification::Position { clock } => {
                    if SINK
                        .get()
                        .unwrap()
                        .send(Box::new(move |s| {
                            if let Some(mut progress) = s.find_name::<ProgressBar>("progress") {
                                progress.set_value(clock.seconds() as usize);
                            }
                        }))
                        .is_ok()
                    {}
                }
                Notification::CurrentTrackList { list } => match list.list_type() {
                    TrackListType::Album => {
                        if SINK
                            .get()
                            .unwrap()
                            .send(Box::new(move |s| {
                                if let Some(mut list_view) = s
                                    .find_name::<ScrollView<SelectView<usize>>>(
                                        "current_track_list",
                                    )
                                {
                                    list_view.get_inner_mut().clear();

                                    list.queue
                                        .iter()
                                        .filter(|t| t.1.status == TrackStatus::Unplayed)
                                        .map(|t| t.1)
                                        .for_each(|i| {
                                            list_view.get_inner_mut().add_item(
                                                i.track_list_item(list.list_type(), false),
                                                i.position as usize,
                                            );
                                        });

                                    list.queue
                                        .iter()
                                        .filter(|t| t.1.status == TrackStatus::Played)
                                        .map(|t| t.1)
                                        .for_each(|i| {
                                            list_view.get_inner_mut().add_item(
                                                i.track_list_item(list.list_type(), true),
                                                i.position as usize,
                                            );
                                        });
                                }
                                if let (
                                    Some(album),
                                    Some(mut entity_title),
                                    Some(mut total_tracks),
                                ) = (
                                    list.get_album(),
                                    s.find_name::<TextView>("entity_title"),
                                    s.find_name::<TextView>("total_tracks"),
                                ) {
                                    let mut title = StyledString::plain(album.title.clone());
                                    title.append_plain(" ");
                                    title.append_styled(
                                        format!("({})", album.release_year),
                                        Effect::Dim,
                                    );

                                    entity_title.set_content(title);
                                    total_tracks.set_content(format!("{:03}", album.total_tracks));
                                }

                                for t in list.queue.values() {
                                    if t.status == TrackStatus::Playing {
                                        set_current_track(s, t, list.list_type());
                                        break;
                                    }
                                }
                            }))
                            .is_ok()
                        {}
                    }
                    TrackListType::Playlist => {
                        if SINK
                            .get()
                            .unwrap()
                            .send(Box::new(move |s| {
                                if let Some(mut list_view) = s
                                    .find_name::<ScrollView<SelectView<usize>>>(
                                        "current_track_list",
                                    )
                                {
                                    list_view.get_inner_mut().clear();

                                    list.queue
                                        .iter()
                                        .filter(|t| t.1.status == TrackStatus::Unplayed)
                                        .map(|t| t.1)
                                        .for_each(|i| {
                                            list_view.get_inner_mut().add_item(
                                                i.track_list_item(list.list_type(), false),
                                                i.position as usize,
                                            );
                                        });

                                    list.queue
                                        .iter()
                                        .filter(|t| t.1.status == TrackStatus::Played)
                                        .map(|t| t.1)
                                        .for_each(|i| {
                                            list_view.get_inner_mut().add_item(
                                                i.track_list_item(list.list_type(), true),
                                                i.position as usize,
                                            );
                                        });
                                }

                                if let (
                                    Some(playlist),
                                    Some(mut entity_title),
                                    Some(mut total_tracks),
                                ) = (
                                    list.playlist.as_ref(),
                                    s.find_name::<TextView>("entity_title"),
                                    s.find_name::<TextView>("total_tracks"),
                                ) {
                                    if let Some(first) = playlist.tracks.first_key_value() {
                                        set_current_track(s, first.1, list.list_type());
                                    }

                                    entity_title.set_content(&playlist.title);
                                    total_tracks.set_content(format!("{:03}", list.total()));
                                }

                                for t in list.queue.values() {
                                    if t.status == TrackStatus::Playing {
                                        set_current_track(s, t, list.list_type());
                                        break;
                                    }
                                }
                            }))
                            .is_ok()
                        {}
                    }
                    TrackListType::Track => {
                        if SINK
                            .get()
                            .unwrap()
                            .send(Box::new(move |s| {
                                if let Some(mut list_view) = s
                                    .find_name::<ScrollView<SelectView<usize>>>(
                                        "current_track_list",
                                    )
                                {
                                    list_view.get_inner_mut().clear();
                                }

                                if let (Some(album), Some(mut entity_title)) =
                                    (list.get_album(), s.find_name::<TextView>("entity_title"))
                                {
                                    entity_title.set_content(album.title.trim());
                                }
                                if let Some(mut total_tracks) =
                                    s.find_name::<TextView>("total_tracks")
                                {
                                    total_tracks.set_content("001");
                                }

                                for t in list.queue.values() {
                                    if t.status == TrackStatus::Playing {
                                        set_current_track(s, t, list.list_type());
                                        break;
                                    }
                                }
                            }))
                            .is_ok()
                        {}
                    }
                    _ => {}
                },
                Notification::Error { error: _ } => {}
                Notification::Volume { volume: _ } => {}
            }
        }
    }
}

trait CursiveFormat {
    fn list_item(&self) -> StyledString;
    fn track_list_item(&self, _list_type: &TrackListType, _inactive: bool) -> StyledString {
        StyledString::new()
    }
}

impl CursiveFormat for Track {
    fn list_item(&self) -> StyledString {
        let mut style = Style::none();

        if !self.available {
            style = style.combine(Effect::Dim).combine(Effect::Strikethrough);
        }

        let mut title = StyledString::styled(self.title.trim(), style.combine(Effect::Bold));

        if let Some(artist) = &self.artist {
            title.append_styled(" by ", style);
            title.append_styled(&artist.name, style);
        }

        let duration = ClockTime::from_seconds(self.duration_seconds as u64)
            .to_string()
            .as_str()[2..7]
            .to_string();
        title.append_plain(" ");
        title.append_styled(duration, style.combine(Effect::Dim));
        title.append_plain(" ");

        if self.explicit {
            title.append_styled("e", style.combine(Effect::Dim));
        }

        if self.hires_available {
            title.append_styled("*", style.combine(Effect::Dim));
        }

        title
    }
    fn track_list_item(&self, list_type: &TrackListType, inactive: bool) -> StyledString {
        let mut style = Style::none();

        if inactive || !self.available {
            style = style
                .combine(Effect::Dim)
                .combine(Effect::Italic)
                .combine(Effect::Strikethrough);
        }

        let num = match list_type {
            TrackListType::Album => self.number,
            TrackListType::Playlist => self.position,
            TrackListType::Track => self.number,
            TrackListType::Unknown => self.position,
        };

        let mut item = StyledString::styled(format!("{:02} ", num), style);
        item.append_styled(self.title.trim(), style.combine(Effect::Simple));
        item.append_plain(" ");

        let duration = ClockTime::from_seconds(self.duration_seconds as u64)
            .to_string()
            .as_str()[2..7]
            .to_string();

        item.append_styled(duration, style.combine(Effect::Dim));

        item
    }
}

impl CursiveFormat for Album {
    fn list_item(&self) -> StyledString {
        let mut style = Style::none();

        if !self.available {
            style = style.combine(Effect::Dim).combine(Effect::Strikethrough);
        }

        let mut title = StyledString::styled(self.title.as_str(), style.combine(Effect::Bold));

        title.append_styled(" by ", style);
        title.append_styled(self.artist.name.as_str(), style);
        title.append_styled(" ", style);

        title.append_styled(self.release_year.to_string(), style.combine(Effect::Dim));
        title.append_plain(" ");

        if self.explicit {
            title.append_styled("e", style.combine(Effect::Dim));
        }

        if self.hires_available {
            title.append_styled("*", style.combine(Effect::Dim));
        }

        title
    }
}

impl CursiveFormat for Artist {
    fn list_item(&self) -> StyledString {
        StyledString::plain(self.name.as_str())
    }
}
