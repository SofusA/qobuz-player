use qobuz_player_controls::{Result, client::Client, controls::Controls};
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
};
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{
    app::{NotificationList, Output},
    sub_tab::SubTab,
    ui::{block, render_input, tab_bar},
    widgets::{
        album_list::AlbumList,
        artist_list::ArtistList,
        playlist_list::PlaylistList,
        track_list::{TrackList, TrackListEvent},
    },
};

pub struct FavoritesState {
    pub editing: bool,
    pub filter: Input,
    pub albums: AlbumList,
    pub artists: ArtistList,
    pub playlists: PlaylistList,
    pub tracks: TrackList,
    pub sub_tab: SubTab,
}

impl FavoritesState {
    pub async fn new(client: &Client) -> Result<Self> {
        let favorites = client.favorites().await?;

        Ok(Self {
            editing: Default::default(),
            filter: Default::default(),
            albums: AlbumList::new(favorites.albums),
            artists: ArtistList::new(favorites.artists),
            playlists: PlaylistList::new(favorites.playlists),
            tracks: TrackList::new(favorites.tracks),
            sub_tab: Default::default(),
        })
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let tab_content_area_split = Layout::default()
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .split(area);

        render_input(
            &self.filter,
            self.editing,
            tab_content_area_split[0],
            frame,
            "Filter",
        );

        let block = block(None);
        frame.render_widget(block, tab_content_area_split[1]);

        let tab_content_area = tab_content_area_split[1].inner(Margin::new(1, 1));

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(1)])
            .split(tab_content_area);

        let tabs = tab_bar(SubTab::labels(), self.sub_tab.selected().into());
        frame.render_widget(tabs, chunks[0]);

        match self.sub_tab {
            SubTab::Albums => self.albums.render(chunks[1], frame.buffer_mut()),
            SubTab::Artists => self.artists.render(chunks[1], frame.buffer_mut()),
            SubTab::Playlists => self.playlists.render(chunks[1], frame.buffer_mut()),
            SubTab::Tracks => self.tracks.render(chunks[1], frame.buffer_mut(), true),
        };
    }

    pub async fn handle_events(
        &mut self,
        event: Event,
        client: &Client,
        controls: &Controls,
        notifications: &mut NotificationList,
    ) -> Result<Output> {
        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match &mut self.editing {
                    false => match key_event.code {
                        KeyCode::Char('e') => {
                            self.start_editing();
                            Ok(Output::Consumed)
                        }
                        KeyCode::Left | KeyCode::Char('h') => {
                            self.cycle_subtab_backwards();
                            Ok(Output::Consumed)
                        }
                        KeyCode::Right | KeyCode::Char('l') => {
                            self.cycle_subtab();
                            Ok(Output::Consumed)
                        }
                        _ => match self.sub_tab {
                            SubTab::Albums => {
                                return self
                                    .albums
                                    .handle_events(key_event.code, client, notifications)
                                    .await;
                            }
                            SubTab::Artists => {
                                return self
                                    .artists
                                    .handle_events(key_event.code, client, notifications)
                                    .await;
                            }
                            SubTab::Playlists => {
                                return self
                                    .playlists
                                    .handle_events(key_event.code, client, notifications)
                                    .await;
                            }
                            SubTab::Tracks => {
                                return self
                                    .tracks
                                    .handle_events(
                                        key_event.code,
                                        client,
                                        controls,
                                        notifications,
                                        TrackListEvent::Track,
                                    )
                                    .await;
                            }
                        },
                    },
                    true => match key_event.code {
                        KeyCode::Esc | KeyCode::Enter => {
                            self.stop_editing();
                            Ok(Output::Consumed)
                        }
                        _ => {
                            self.filter.handle_event(&event);

                            let match_in = |s: &str| {
                                s.to_lowercase()
                                    .contains(&self.filter.value().to_lowercase())
                            };

                            self.albums.set_filter(
                                self.albums
                                    .all_items()
                                    .iter()
                                    .filter(|album| {
                                        match_in(&album.title) || match_in(&album.artist.name)
                                    })
                                    .cloned()
                                    .collect(),
                            );

                            self.artists.set_filter(
                                self.artists
                                    .all_items()
                                    .iter()
                                    .filter(|artist| match_in(&artist.name))
                                    .cloned()
                                    .collect(),
                            );

                            self.playlists.set_filter(
                                self.playlists
                                    .all_items()
                                    .iter()
                                    .filter(|playlist| match_in(&playlist.title))
                                    .cloned()
                                    .collect(),
                            );

                            self.tracks.set_filter(
                                self.tracks
                                    .all_items()
                                    .iter()
                                    .filter(|track| {
                                        match_in(&track.title)
                                            || track
                                                .artist_name
                                                .as_ref()
                                                .is_some_and(|artist| match_in(artist))
                                            || track
                                                .album_title
                                                .as_ref()
                                                .is_some_and(|album| match_in(album))
                                    })
                                    .cloned()
                                    .collect(),
                            );

                            Ok(Output::Consumed)
                        }
                    },
                }
            }
            _ => Ok(Output::NotConsumed),
        }
    }

    fn start_editing(&mut self) {
        self.editing = true;
    }

    fn stop_editing(&mut self) {
        self.editing = false;
    }

    fn cycle_subtab_backwards(&mut self) {
        self.sub_tab = self.sub_tab.previous();
    }

    fn cycle_subtab(&mut self) {
        self.sub_tab = self.sub_tab.next();
    }
}
