use std::sync::{Arc, OnceLock};

use cursive::{
    align::HAlign,
    direction::Orientation,
    event::{Event, Key},
    reexports::crossbeam_channel::Sender,
    theme::{BorderStyle, Effect, Palette, Style},
    utils::{markup::StyledString, Counter},
    view::{Nameable, Resizable, Scrollable, SizeConstraint},
    views::{
        Button, Dialog, EditView, HideableView, LinearLayout, MenuPopup, NamedView, OnEventView,
        PaddedView, Panel, ProgressBar, ResizedView, ScreensView, ScrollView, SelectView, TextView,
    },
    Cursive, CursiveRunnable, With,
};
use futures::executor::block_on;
use gstreamer::{ClockTime, State as GstState};
use qobuz_player_controls::{
    notification::Notification,
    service::{Album, Artist, SearchResults, Track, TrackStatus},
    tracklist::TrackListType,
};
use tokio::select;
use tracing::debug;

type CursiveSender = Sender<Box<dyn FnOnce(&mut Cursive) + Send>>;

static SINK: OnceLock<CursiveSender> = OnceLock::new();

static UNSTREAMABLE: &str = "UNSTREAMABLE";

pub struct CursiveUI {
    root: CursiveRunnable,
}

impl CursiveUI {
    pub fn new() -> Self {
        let mut siv = cursive::default();

        SINK.set(siv.cb_sink().clone()).expect("error setting sink");

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

        Self { root: siv }
    }

    fn player(&self) -> LinearLayout {
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
            .child(
                TextView::new("16 bits")
                    .h_align(HAlign::Right)
                    .with_name("bit_depth"),
            )
            .child(
                TextView::new("44.1 kHz")
                    .h_align(HAlign::Right)
                    .with_name("sample_rate"),
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
            tokio::spawn(
                async move { qobuz_player_controls::skip_to_position(i as u32, true).await },
            );
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

    fn global_events(&mut self) {
        self.root.clear_global_callbacks(Event::CtrlChar('c'));

        self.root.set_on_pre_event(Event::CtrlChar('c'), move |s| {
            let dialog = Dialog::text("Do you want to quit?")
                .button("Yes", move |s: &mut Cursive| {
                    s.quit();
                })
                .dismiss_button("No");

            s.add_layer(dialog);
        });

        self.root.add_global_callback('1', move |s| {
            s.set_screen(0);
        });

        self.root.add_global_callback('2', move |s| {
            s.set_screen(1);
        });

        self.root.add_global_callback('3', move |s| {
            s.set_screen(2);
        });

        self.root.add_global_callback(' ', move |_| {
            block_on(async { qobuz_player_controls::play_pause().await.expect("") });
        });

        self.root.add_global_callback('N', move |_| {
            block_on(async { qobuz_player_controls::next().await.expect("") });
        });

        self.root.add_global_callback('P', move |_| {
            block_on(async { qobuz_player_controls::previous().await.expect("") });
        });

        self.root.add_global_callback('l', move |_| {
            block_on(async { qobuz_player_controls::jump_forward().await.expect("") });
        });

        self.root.add_global_callback('h', move |_| {
            block_on(async { qobuz_player_controls::jump_backward().await.expect("") });
        });
    }

    fn menubar(&mut self) {
        self.root.set_autohide_menu(false);

        self.root
            .menubar()
            .add_leaf("Now Playing [1]", move |s| {
                s.set_screen(0);
            })
            .add_delimiter()
            .add_leaf("Playlists [2]", move |s| {
                s.set_screen(1);
            })
            .add_delimiter()
            .add_leaf("Search [3]", move |s| {
                s.set_screen(2);
            })
            .add_delimiter();

        self.root.add_global_callback('1', move |s| {
            s.set_screen(0);
        });
        self.root.add_global_callback('2', move |s| {
            s.set_screen(1);
        });
        self.root.add_global_callback('3', move |s| {
            s.set_screen(2);
        });
    }

    async fn my_playlists(&self) -> NamedView<LinearLayout> {
        let mut list_layout = LinearLayout::new(Orientation::Vertical);

        let mut user_playlists = SelectView::new().popup();
        user_playlists.add_item("Select Playlist", 0);

        let my_playlists = qobuz_player_controls::user_playlists().await;
        my_playlists.iter().for_each(|p| {
            user_playlists.add_item(p.title.clone(), p.id);
        });

        user_playlists.set_on_submit(move |s: &mut Cursive, item: &u32| {
            if item == &0 {
                s.call_on_name("play_button", |button: &mut Button| {
                    button.disable();
                });

                return;
            }

            let layout = submit_playlist(s, *item).wrap_with(Panel::new);

            s.call_on_name("user_playlist_layout", |l: &mut LinearLayout| {
                l.remove_child(1);
                l.add_child(layout);
            });

            s.call_on_name("play_button", |button: &mut Button| {
                button.enable();
            });
        });

        list_layout.add_child(
            Panel::new(
                user_playlists
                    .with_name("user_playlists")
                    .scrollable()
                    .scroll_y(true)
                    .resized(SizeConstraint::Full, SizeConstraint::Free),
            )
            .title("my playlists"),
        );

        list_layout.with_name("user_playlist_layout")
    }

    fn search(&mut self) -> LinearLayout {
        let mut layout = LinearLayout::new(Orientation::Vertical);

        let on_submit = move |s: &mut Cursive, item: &String| {
            load_search_results(item, s);
        };

        let search_type = SelectView::new()
            .item_str("Albums")
            .item_str("Artists")
            .item_str("Tracks")
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

    fn results_list(name: &str) -> ResultsPanel {
        let panel: ResultsPanel = SelectView::new()
            .with_name(name)
            .scrollable()
            .scroll_y(true)
            .scroll_x(true);

        panel
    }

    pub async fn run(&mut self) {
        let player = self.player();
        let search = self.search();
        let my_playlists = self.my_playlists().await;

        self.root
            .screen_mut()
            .add_fullscreen_layer(PaddedView::lrtb(
                0,
                0,
                1,
                0,
                player.resized(SizeConstraint::Full, SizeConstraint::Free),
            ));

        self.root.add_active_screen();
        self.root
            .screen_mut()
            .add_fullscreen_layer(PaddedView::lrtb(
                0,
                0,
                1,
                0,
                my_playlists.resized(SizeConstraint::Full, SizeConstraint::Free),
            ));

        self.root.add_active_screen();
        self.root
            .screen_mut()
            .add_fullscreen_layer(PaddedView::lrtb(
                0,
                0,
                1,
                0,
                search.resized(SizeConstraint::Full, SizeConstraint::Free),
            ));

        self.root.set_screen(0);

        self.global_events();
        self.menubar();
        self.root.run();
    }
}

impl Default for CursiveUI {
    fn default() -> Self {
        Self::new()
    }
}

type ResultsPanel = ScrollView<NamedView<SelectView<(i32, Option<String>)>>>;

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

                    search_results.set_on_submit(move |s: &mut Cursive, item: &String| {
                        let layout = submit_playlist(
                            s,
                            item.parse::<u32>().expect("failed to parse string"),
                        );

                        let event_panel =
                            OnEventView::new(layout).on_event(Event::Key(Key::Esc), move |s| {
                                s.screen_mut().pop_layer();
                            });

                        s.screen_mut().add_layer(Panel::new(event_panel));
                    });
                }
                _ => {}
            }
        }
    }
}

fn submit_playlist(_s: &mut Cursive, item: u32) -> LinearLayout {
    let mut layout = LinearLayout::vertical();

    let playlist_tracks =
        block_on(async { qobuz_player_controls::playlist_tracks(item as i64).await });

    let mut list = CursiveUI::results_list("playlist_items");
    let mut playlist_items = list.get_inner_mut().get_mut();

    for t in &playlist_tracks {
        let mut row = StyledString::plain(format!("{:02} ", t.position));

        row.append(t.list_item());

        let track_id = if t.available { t.id as i32 } else { -1 };

        let value = if let Some(album) = &t.album {
            let album_id = if album.available {
                album.id.clone()
            } else {
                UNSTREAMABLE.to_string()
            };

            (track_id, Some(album_id))
        } else {
            (track_id, None)
        };

        playlist_items.add_item(row, value);
    }

    let meta = LinearLayout::horizontal()
        .child(Button::new("play", move |_s| {
            tokio::spawn(async move { qobuz_player_controls::play_playlist(item as i64).await });
        }))
        .child(
            TextView::new(format!("total tracks: {}", playlist_tracks.len()))
                .h_align(HAlign::Right)
                .full_width(),
        );

    layout.add_child(meta);
    layout.add_child(list);

    layout
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

    if let (Some(mut bit_depth), Some(mut sample_rate)) = (
        s.find_name::<TextView>("bit_depth"),
        s.find_name::<TextView>("sample_rate"),
    ) {
        bit_depth.set_content(format!("{} bits", track.bit_depth));
        sample_rate.set_content(format!("{} kHz", track.sampling_rate));
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

pub async fn receive_notifications() {
    let mut receiver = qobuz_player_controls::notify_receiver();

    loop {
        select! {
            Ok(notification) = receiver.recv() => {
                match notification {
                    Notification::Quit => {
                        debug!("exiting tui notification thread");
                        return;
                    }
                    Notification::Loading { is_loading, target_state } => {
                        SINK.get().unwrap().send(Box::new(move |s| {
                                if let Some(mut view) = s.find_name::<TextView>("player_status") {
                                    if is_loading {
                                        view.set_content(format!(" {}", '\u{2B71}'));
                                    } else {
                                        view.set_content(get_state_icon(target_state));
                                    }
                                }
                        })).expect("failed to send update");
                    }
                    Notification::Status { status } => {
                        SINK.get()
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
                            .expect("failed to send update");
                    }
                    Notification::Position { clock } => {
                        SINK.get()
                            .unwrap()
                            .send(Box::new(move |s| {
                                if let Some(mut progress) = s.find_name::<ProgressBar>("progress") {
                                    progress.set_value(clock.seconds() as usize);
                                }
                            }))
                            .expect("failed to send update");
                    }
                    Notification::CurrentTrackList { list } => {
                        match list.list_type() {
                            TrackListType::Album => {
                                SINK.get()
                                    .unwrap()
                                    .send(Box::new(move |s| {
                                        if let Some(mut list_view) = s
                                            .find_name::<ScrollView<SelectView<usize>>>(
                                                "current_track_list",
                                            )
                                        {
                                            list_view.get_inner_mut().clear();

                                            list.queue.iter().filter(|t| t.1.status == TrackStatus::Unplayed).map(|t| t.1).for_each(|i| {
                                                list_view.get_inner_mut().add_item(
                                                    i.track_list_item(list.list_type(), false),
                                                    i.position as usize,
                                                );
                                            });

                                            list.queue.iter().filter(|t| t.1.status == TrackStatus::Played).map(|t| t.1).for_each(|i| {
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
                                            total_tracks
                                                .set_content(format!("{:03}", album.total_tracks));
                                        }

                                        for t in list.queue.values() {
                                            if t.status == TrackStatus::Playing {
                                                set_current_track(s, t, list.list_type());
                                                break;
                                            }
                                        }
                                    }))
                                    .expect("failed to send update");
                            }
                            TrackListType::Playlist => {
                                SINK.get()
                                    .unwrap()
                                    .send(Box::new(move |s| {
                                        if let Some(mut list_view) = s
                                            .find_name::<ScrollView<SelectView<usize>>>(
                                                "current_track_list",
                                            )
                                        {
                                            list_view.get_inner_mut().clear();

                                            list.queue.iter().filter(|t| t.1.status == TrackStatus::Unplayed).map(|t| t.1).for_each(|i| {
                                                list_view.get_inner_mut().add_item(
                                                    i.track_list_item(list.list_type(), false),
                                                    i.position as usize,
                                                );
                                            });

                                            list.queue.iter().filter(|t| t.1.status == TrackStatus::Played).map(|t| t.1).for_each(|i| {
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
                                    .expect("failed to send update");
                            }
                            TrackListType::Track => {
                                SINK.get()
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
                                    .expect("failed to send update");
                            }
                            _ => {}
                        }
                    }
                    Notification::Buffering {
                        is_buffering,
                        target_state,
                        percent,
                    } => {
                        SINK.get()
                            .unwrap()
                            .send(Box::new(move |s| {
                                s.call_on_name("player_status", |view: &mut TextView| {
                                    if is_buffering {
                                        view.set_content(format!("{}%", percent));
                                    } else {
                                        view.set_content(get_state_icon(target_state));
                                    }
                                });
                            }))
                            .expect("failed to send update");
                    }
                    Notification::Error { error: _ } => {}
                    Notification::Volume{ volume: _ } => {}
                }
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
