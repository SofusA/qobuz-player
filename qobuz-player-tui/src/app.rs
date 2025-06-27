use core::fmt;
use image::load_from_memory;
use qobuz_player_controls::{
    models::{Favorites, SearchResults, Track},
    tracklist::Tracklist,
};
use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    prelude::*,
};
use ratatui_image::{picker::Picker, protocol::StatefulProtocol};
use reqwest::Client;
use std::io;

#[derive(Default)]
pub struct App {
    pub current_screen: Tab,
    pub exit: bool,
    pub should_draw: bool,
    pub favorites: Favorites,
    pub search_results: SearchResults,
    // TODO: album filter
    // TODO: artist filter
    // TODO: playlist filter
    // TODO: search query
}

#[derive(Default)]
pub struct AppState {
    pub now_playing: NowPlayingState,
}

#[derive(Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    FavoriteAlbums,
    FavoriteArtists,
    // TODO: Playlists
    // TODO: Search
}

impl fmt::Display for Tab {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Tab::FavoriteAlbums => write!(f, "Favorite albums"),
            Tab::FavoriteArtists => write!(f, "Favorite artists"),
        }
    }
}

impl Tab {
    pub const VALUES: [Self; 2] = [Tab::FavoriteAlbums, Tab::FavoriteArtists];
}

#[derive(Default)]
pub struct NowPlayingState {
    // stateful image widget here
    pub image: Option<StatefulProtocol>,
    pub entity_tittle: Option<String>,
    pub playing_track: Option<Track>,
}

use tokio::time::{self, Duration};
impl App {
    pub async fn run(
        &mut self,
        state: &mut AppState,
        terminal: &mut DefaultTerminal,
    ) -> io::Result<()> {
        self.should_draw = true;
        let mut receiver = qobuz_player_controls::notify_receiver();
        let mut tick_interval = time::interval(Duration::from_millis(100));

        while !self.exit {
            tokio::select! {
                maybe_notification = receiver.recv() => {
                    if let Ok(notification) = maybe_notification {
                        match notification {
                            qobuz_player_controls::notification::Notification::Status { status: _ } => (),
                            qobuz_player_controls::notification::Notification::Position { clock: _ } => (),
                            qobuz_player_controls::notification::Notification::CurrentTrackList { list } => {
                                state.now_playing = get_current_state(list).await;
                                self.should_draw = true;
                            }
                            qobuz_player_controls::notification::Notification::Quit => {
                                self.exit = true;
                            }
                            qobuz_player_controls::notification::Notification::Message { message: _ } => (),
                            qobuz_player_controls::notification::Notification::Volume { volume: _ } => (),
                        }
                    }
                }

                _ = tick_interval.tick() => {
                    if event::poll(Duration::from_millis(0))? {
                        self.handle_events().unwrap();
                    }
                }
            }

            if self.should_draw {
                terminal.draw(|frame| self.draw(frame, state))?;
                self.should_draw = false;
            }
        }

        Ok(())
    }

    fn draw(&self, frame: &mut Frame, state: &mut AppState) {
        frame.render_stateful_widget(self, frame.area(), state);
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => {
                self.should_draw = true;
                self.exit()
            }
            KeyCode::Char('1') => {
                self.should_draw = true;
                self.navigate_to_favorites_albums()
            }
            KeyCode::Char('2') => {
                self.should_draw = true;
                self.navigate_to_favorites_artists()
            }
            _ => {}
        }
    }

    fn navigate_to_favorites_albums(&mut self) {
        self.current_screen = Tab::FavoriteAlbums;
    }

    fn navigate_to_favorites_artists(&mut self) {
        self.current_screen = Tab::FavoriteArtists;
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

async fn fetch_image(image_url: &str) -> Option<StatefulProtocol> {
    let client = Client::new();
    let response = client.get(image_url).send().await.ok()?;
    let img_bytes = response.bytes().await.ok()?;

    let image = load_from_memory(&img_bytes).ok()?;

    let picker = Picker::from_query_stdio().ok()?;
    Some(picker.new_resize_protocol(image))
}

async fn get_current_state(tracklist: Tracklist) -> NowPlayingState {
    let (entity, image_url) = match &tracklist.list_type {
        qobuz_player_controls::tracklist::TracklistType::Album(tracklist) => {
            (Some(tracklist.title.clone()), tracklist.image.clone())
        }
        qobuz_player_controls::tracklist::TracklistType::Playlist(tracklist) => {
            (Some(tracklist.title.clone()), tracklist.image.clone())
        }
        qobuz_player_controls::tracklist::TracklistType::TopTracks(tracklist) => {
            (Some(tracklist.artist_name.clone()), tracklist.image.clone())
        }
        qobuz_player_controls::tracklist::TracklistType::Track(tracklist) => {
            (None, tracklist.image.clone())
        }
        qobuz_player_controls::tracklist::TracklistType::None => (None, None),
    };

    let track = tracklist.current_track().cloned();

    let image = if let Some(image_url) = image_url {
        Some(fetch_image(&image_url).await)
    } else {
        None
    }
    .flatten();

    NowPlayingState {
        image,
        entity_tittle: entity,
        playing_track: track,
    }
}
