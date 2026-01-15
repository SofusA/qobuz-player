use crate::{
    discover::DiscoverState,
    favorites::FavoritesState,
    now_playing::NowPlayingState,
    popup::{Popup, TrackPopupState},
    queue::QueueState,
    search::SearchState,
};
use core::fmt;
use image::load_from_memory;
use qobuz_player_controls::{
    PositionReceiver, Status, StatusReceiver, TracklistReceiver,
    client::Client,
    controls::Controls,
    notification::{Notification, NotificationBroadcast},
    tracklist::Tracklist,
};
use qobuz_player_models::Track;
use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    widgets::*,
};
use ratatui_image::{picker::Picker, protocol::StatefulProtocol};
use std::{io, sync::Arc, time::Instant};
use tokio::time::{self, Duration};

pub(crate) struct App {
    pub(crate) client: Arc<Client>,
    pub(crate) controls: Controls,
    pub(crate) position: PositionReceiver,
    pub(crate) tracklist: TracklistReceiver,
    pub(crate) status: StatusReceiver,
    pub(crate) current_screen: Tab,
    pub(crate) exit: bool,
    pub(crate) should_draw: bool,
    pub(crate) app_state: AppState,
    pub(crate) now_playing: NowPlayingState,
    pub(crate) favorites: FavoritesState,
    pub(crate) search: SearchState,
    pub(crate) queue: QueueState,
    pub(crate) discover: DiscoverState,
    pub(crate) broadcast: Arc<NotificationBroadcast>,
    pub(crate) notifications: Vec<(Notification, Instant)>,
    pub(crate) full_screen: bool,
    pub(crate) disable_tui_album_cover: bool,
}

#[allow(clippy::large_enum_variant)] // TODO remove
#[derive(Default)]
pub(crate) enum AppState {
    #[default]
    Normal,
    Popup(Vec<Popup>),
    Help,
}

pub(crate) enum Output {
    Consumed,
    NotConsumed,
    UpdateFavorites,
    Popup(Popup),
    PopPopup,
    PopPoputUpdateFavorites,
    PlayOutcome(PlayOutcome),
    Error(String),
    Queue(QueueOutcome),
}

pub(crate) enum QueueOutcome {
    MoveIndexUp(usize),
    MoveIndexDown(usize),
    RemoveIndex(usize),
    PlayTrackNext(u32),
    AddTrackToQueue(u32),
}

pub(crate) enum PlayOutcome {
    Album(String, usize),
    Playlist((u32, bool, usize)),
    Track(u32),
    SkipToPosition(usize),
    AddTrackToPlaylist(Track),
}

#[derive(Default, PartialEq)]
pub(crate) enum Tab {
    #[default]
    Favorites,
    Search,
    Queue,
    Discover,
}

impl fmt::Display for Tab {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Tab::Favorites => write!(f, "Favorites"),
            Tab::Search => write!(f, "Search"),
            Tab::Queue => write!(f, "Queue"),
            Tab::Discover => write!(f, "Discover"),
        }
    }
}

impl Tab {
    pub(crate) const VALUES: [Self; 4] = [Tab::Favorites, Tab::Search, Tab::Queue, Tab::Discover];
}

pub(crate) struct FilteredListState<T> {
    pub(crate) filter: Vec<T>,
    pub(crate) all_items: Vec<T>,
    pub(crate) state: TableState,
}

pub(crate) struct UnfilteredListState<T> {
    pub(crate) items: Vec<T>,
    pub(crate) state: TableState,
}

impl App {
    pub(crate) async fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let mut tick_interval = time::interval(Duration::from_millis(10));
        let mut receiver = self.broadcast.subscribe();

        while !self.exit {
            tokio::select! {
                Ok(_) = self.position.changed() => {
                    self.now_playing.duration_ms = self.position.borrow_and_update().as_millis() as u32;
                    self.should_draw = true;
                },

                Ok(_) = self.tracklist.changed() => {
                    let tracklist = self.tracklist.borrow_and_update().clone();
                    self.queue.queue.items = tracklist.queue().to_vec();
                    let status = self.now_playing.status;
                    self.now_playing = get_current_state(tracklist, status).await;
                    self.should_draw = true;
                },

                Ok(_) = self.status.changed() => {
                    let status = self.status.borrow_and_update();
                    self.now_playing.status = *status;
                    self.should_draw = true;
                }

                _ = tick_interval.tick() => {
                    if event::poll(Duration::from_millis(0))? {
                        self.handle_events().await.expect("infallible");
                    }
                }

                notification = receiver.recv() => {
                    if let Ok(message) = notification {
                        self.notifications.push((message, Instant::now()));
                        self.should_draw = true;
                    }
                }
            }

            let notifications_before_clean = self.notifications.len();
            self.notifications
                .retain(|notification| notification.1.elapsed() < Duration::from_secs(5));
            let notifications_after_clean = self.notifications.len();

            if notifications_before_clean != notifications_after_clean {
                self.should_draw = true;
            }

            if self.should_draw {
                terminal.draw(|frame| self.render(frame))?;
                self.should_draw = false;
            }
        }

        Ok(())
    }

    async fn update_favorites(&mut self) {
        let favorites = self.client.favorites().await;
        let Ok(favorites) = favorites else {
            return;
        };

        self.favorites.albums.all_items = favorites.albums;
        self.favorites.albums.filter = self.favorites.albums.all_items.clone();
        self.favorites.artists.all_items = favorites.artists;
        self.favorites.artists.filter = self.favorites.artists.all_items.clone();
        self.favorites.playlists.all_items = favorites.playlists;
        self.favorites.playlists.filter = self.favorites.playlists.all_items.clone();
        self.favorites.tracks.all_items = favorites.tracks;
        self.favorites.tracks.filter = self.favorites.tracks.all_items.clone();
        self.favorites.filter.reset();
    }

    async fn handle_output(&mut self, output: Output) {
        match output {
            Output::Consumed => {
                self.should_draw = true;
            }
            Output::UpdateFavorites => {
                self.update_favorites().await;
                self.should_draw = true;
            }
            Output::NotConsumed => {}
            Output::Popup(popup) => {
                let mut popups = match std::mem::take(&mut self.app_state) {
                    AppState::Popup(popups) => popups,
                    _ => Vec::new(),
                };

                popups.push(popup);

                self.app_state = AppState::Popup(popups);
                self.should_draw = true;
            }
            Output::PlayOutcome(outcome) => {
                self.handle_playoutcome(outcome).await;
            }
            Output::Error(err) => {
                self.broadcast.send_error(err);
            }
            Output::Queue(queue_outcome) => match queue_outcome {
                QueueOutcome::MoveIndexUp(index) => {
                    if index == 0 {
                        return;
                    }
                    let mut order: Vec<_> = self
                        .queue
                        .queue
                        .items
                        .iter()
                        .enumerate()
                        .map(|x| x.0)
                        .collect();

                    order.swap(index, index - 1);
                    self.controls.reorder_queue(order);
                }
                QueueOutcome::MoveIndexDown(index) => {
                    if index == self.queue.queue.items.len() - 1 {
                        return;
                    }

                    let mut order: Vec<_> = self
                        .queue
                        .queue
                        .items
                        .iter()
                        .enumerate()
                        .map(|x| x.0)
                        .collect();

                    order.swap(index, index + 1);
                    self.controls.reorder_queue(order);
                }
                QueueOutcome::RemoveIndex(index) => {
                    self.controls.remove_index_from_queue(index);
                }
                QueueOutcome::PlayTrackNext(id) => {
                    self.controls.play_track_next(id);
                }
                QueueOutcome::AddTrackToQueue(id) => {
                    self.controls.add_track_to_queue(id);
                }
            },
            Output::PopPopup => {
                if let AppState::Popup(popups) = &mut self.app_state {
                    popups.pop();
                    if popups.is_empty() {
                        self.app_state = AppState::Normal;
                    }
                    self.should_draw = true;
                }
            }
            Output::PopPoputUpdateFavorites => {
                if let AppState::Popup(popups) = &mut self.app_state {
                    popups.pop();
                    if popups.is_empty() {
                        self.app_state = AppState::Normal;
                    }
                    self.update_favorites().await;
                    self.should_draw = true;
                }
            }
        }
    }

    async fn handle_events(&mut self) -> io::Result<()> {
        let event = event::read()?;

        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match &mut self.app_state {
                    AppState::Help => {
                        self.app_state = AppState::Normal;
                        self.should_draw = true;
                        return Ok(());
                    }
                    AppState::Popup(popups) => {
                        if key_event.code == KeyCode::Esc {
                            _ = popups.pop();
                            if popups.is_empty() {
                                self.app_state = AppState::Normal;
                            }
                            self.should_draw = true;
                            return Ok(());
                        }

                        let outcome_opt = {
                            if let AppState::Popup(popups) = &mut self.app_state {
                                if let Some(popup) = popups.last_mut() {
                                    popup.handle_event(event).await
                                } else {
                                    Output::NotConsumed
                                }
                            } else {
                                Output::NotConsumed
                            }
                        };

                        self.handle_output(outcome_opt).await;

                        self.should_draw = true;
                        return Ok(());
                    }
                    _ => {}
                };

                let screen_output = match self.current_screen {
                    Tab::Favorites => self.favorites.handle_events(event).await,
                    Tab::Search => self.search.handle_events(event).await,
                    Tab::Queue => self.queue.handle_events(event).await,
                    Tab::Discover => self.discover.handle_events(event).await,
                };

                self.handle_output(screen_output).await;

                match key_event.code {
                    KeyCode::Char('?') => {
                        self.app_state = AppState::Help;
                        self.should_draw = true;
                    }
                    KeyCode::Char('q') => {
                        self.should_draw = true;
                        self.exit()
                    }
                    KeyCode::Char('1') => {
                        self.navigate_to_favorites();
                        self.should_draw = true;
                    }
                    KeyCode::Char('2') => {
                        self.navigate_to_search();
                        self.should_draw = true;
                    }
                    KeyCode::Char('3') => {
                        self.navigate_to_queue();
                        self.should_draw = true;
                    }
                    KeyCode::Char('4') => {
                        self.navigate_to_discover();
                        self.should_draw = true;
                    }
                    KeyCode::Char(' ') => {
                        self.controls.play_pause();
                        self.should_draw = true;
                    }
                    KeyCode::Char('n') => {
                        self.controls.next();
                        self.should_draw = true;
                    }
                    KeyCode::Char('p') => {
                        self.controls.previous();
                        self.should_draw = true;
                    }
                    KeyCode::Char('f') => {
                        self.controls.jump_forward();
                        self.should_draw = true;
                    }
                    KeyCode::Char('b') => {
                        self.controls.jump_backward();
                        self.should_draw = true;
                    }
                    KeyCode::Char('F') => {
                        self.full_screen = !self.full_screen;
                        self.should_draw = true;
                    }
                    _ => {}
                };
            }

            Event::Resize(_, _) => self.should_draw = true,
            _ => {}
        };
        Ok(())
    }

    async fn handle_playoutcome(&mut self, outcome: PlayOutcome) -> bool {
        match outcome {
            PlayOutcome::Album(id, index) => {
                self.controls.play_album(&id, index);
            }
            PlayOutcome::Playlist(outcome) => {
                self.controls.play_playlist(outcome.0, outcome.2, outcome.1);
            }
            PlayOutcome::Track(id) => {
                self.controls.play_track(id);
            }
            PlayOutcome::SkipToPosition(index) => {
                self.controls.skip_to_position(index, true);
            }
            PlayOutcome::AddTrackToPlaylist(track) => {
                let playlists_res = self.client.favorites().await.map(|favs| {
                    favs.playlists
                        .into_iter()
                        .filter(|p| p.is_owned)
                        .collect::<Vec<_>>()
                });

                if let Ok(playlists) = playlists_res {
                    let mut popups = match std::mem::take(&mut self.app_state) {
                        AppState::Popup(v) => v,
                        other => {
                            self.app_state = other;
                            Vec::new()
                        }
                    };

                    popups.push(Popup::Track(TrackPopupState {
                        playlists,
                        track,
                        state: Default::default(),
                        client: self.client.clone(),
                    }));

                    self.app_state = AppState::Popup(popups);
                    return false;
                }
            }
        }
        true
    }

    fn navigate_to_favorites(&mut self) {
        self.current_screen = Tab::Favorites;
    }

    fn navigate_to_search(&mut self) {
        self.search.editing = true;
        self.current_screen = Tab::Search;
    }

    fn navigate_to_queue(&mut self) {
        self.current_screen = Tab::Queue;
    }

    fn navigate_to_discover(&mut self) {
        self.current_screen = Tab::Discover;
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

async fn fetch_image(image_url: &str) -> Option<(StatefulProtocol, f32)> {
    let client = reqwest::Client::new();
    let response = client.get(image_url).send().await.ok()?;
    let img_bytes = response.bytes().await.ok()?;

    let image = load_from_memory(&img_bytes).ok()?;
    let ratio = image.width() as f32 / image.height() as f32;

    let picker = Picker::from_query_stdio().ok()?;
    Some((picker.new_resize_protocol(image), ratio))
}

pub(crate) async fn get_current_state(tracklist: Tracklist, status: Status) -> NowPlayingState {
    let entity = tracklist.entity_playing();
    let track = tracklist.current_track().cloned();
    let image = if let Some(image_url) = entity.cover_link {
        Some(fetch_image(&image_url).await)
    } else {
        None
    }
    .flatten();

    let tracklist_length = tracklist.total();

    NowPlayingState {
        image,
        entity_title: entity.title,
        playing_track: track,
        tracklist_length,
        status,
        tracklist_position: tracklist.current_position(),
        duration_ms: 0,
    }
}
