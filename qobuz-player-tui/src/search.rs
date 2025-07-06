use std::fmt;

use crossterm::event::{Event, KeyCode, KeyEventKind};
use qobuz_player_controls::models::{Album, Artist, Playlist, Track};
use ratatui::{prelude::*, widgets::*};
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{
    app::{Output, UnfilteredListState},
    popup::{ArtistPopupState, PlaylistPopupState, Popup},
    ui::{album_table, basic_list_table, render_input},
};

pub(crate) struct SearchState {
    pub editing: bool,
    pub filter: Input,
    pub albums: UnfilteredListState<Album>,
    pub artists: UnfilteredListState<Artist>,
    pub playlists: UnfilteredListState<Playlist>,
    pub tracks: UnfilteredListState<Track>,
    pub sub_tab: SubTab,
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SubTab {
    #[default]
    Albums,
    Artists,
    Playlists,
    Tracks,
}

impl fmt::Display for SubTab {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Albums => write!(f, "Albums"),
            Self::Artists => write!(f, "Artists"),
            Self::Playlists => write!(f, "Playlists"),
            Self::Tracks => write!(f, "Tracks"),
        }
    }
}

impl SubTab {
    pub(crate) const VALUES: [Self; 4] =
        [Self::Albums, Self::Artists, Self::Playlists, Self::Tracks];

    pub(crate) fn next(self) -> Self {
        let index = Self::VALUES.iter().position(|&x| x == self).unwrap();
        Self::VALUES[(index + 1) % Self::VALUES.len()]
    }

    pub(crate) fn previous(self) -> Self {
        let index = Self::VALUES.iter().position(|&x| x == self).unwrap();
        let len = Self::VALUES.len();
        Self::VALUES[(index + len - 1) % len]
    }
}

impl SearchState {
    pub(crate) fn render(&mut self, frame: &mut Frame, area: Rect) {
        let tab_content_area_split = Layout::default()
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .split(area);

        render_input(
            &self.filter,
            self.editing,
            tab_content_area_split[0],
            frame,
            "Search",
        );

        let tab_content_area = tab_content_area_split[1];
        let title = format!("<Search: {}>", self.sub_tab);

        let (table, state) = match self.sub_tab {
            SubTab::Albums => (
                album_table(&self.albums.items, "<Favorite: Albums>"),
                &mut self.albums.state,
            ),
            SubTab::Artists => (
                basic_list_table(
                    self.artists
                        .items
                        .iter()
                        .map(|artist| Row::new(Line::from(artist.name.clone())))
                        .collect::<Vec<_>>(),
                    &title,
                ),
                &mut self.artists.state,
            ),
            SubTab::Playlists => (
                basic_list_table(
                    self.playlists
                        .items
                        .iter()
                        .map(|playlist| Row::new(Line::from(playlist.title.clone())))
                        .collect::<Vec<_>>(),
                    &title,
                ),
                &mut self.playlists.state,
            ),
            SubTab::Tracks => (
                basic_list_table(
                    self.tracks
                        .items
                        .iter()
                        .map(|track| Row::new(Line::from(track.title.clone())))
                        .collect::<Vec<_>>(),
                    &title,
                ),
                &mut self.tracks.state,
            ),
        };

        frame.render_stateful_widget(table, tab_content_area, state);
    }

    pub(crate) async fn handle_events(&mut self, event: Event) -> Output {
        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => match &mut self
                .editing
            {
                false => match key_event.code {
                    KeyCode::Char('e') => {
                        self.start_editing();
                        Output::Consumed
                    }
                    KeyCode::Left => {
                        self.cycle_subtab_backwards();
                        Output::Consumed
                    }
                    KeyCode::Right => {
                        self.cycle_subtab();
                        Output::Consumed
                    }
                    KeyCode::Down => {
                        self.current_list_state().select_next();
                        Output::Consumed
                    }
                    KeyCode::Up => {
                        self.current_list_state().select_previous();
                        Output::Consumed
                    }
                    KeyCode::Enter => match self.sub_tab {
                        SubTab::Albums => {
                            let index = self.albums.state.selected();

                            let id = index
                                .map(|index| &self.albums.items[index])
                                .map(|album| album.id.clone());

                            if let Some(id) = id {
                                qobuz_player_controls::play_album(&id, 0).await.unwrap();
                            }
                            Output::Consumed
                        }
                        SubTab::Artists => {
                            let index = self.artists.state.selected();
                            let selected = index.map(|index| &self.artists.items[index]);

                            let Some(selected) = selected else {
                                return Output::Consumed;
                            };

                            let artist_albums = qobuz_player_controls::artist_albums(selected.id)
                                .await
                                .unwrap();

                            Output::Popup(Popup::Artist(ArtistPopupState {
                                artist_name: selected.name.clone(),
                                albums: artist_albums,
                                state: Default::default(),
                            }))
                        }
                        SubTab::Playlists => {
                            let index = self.playlists.state.selected();
                            let selected = index.map(|index| &self.playlists.items[index]);

                            let Some(selected) = selected else {
                                return Output::Consumed;
                            };

                            Output::Popup(Popup::Playlist(PlaylistPopupState {
                                playlist_name: selected.title.clone(),
                                playlist_id: selected.id,
                                shuffle: false,
                            }))
                        }
                        SubTab::Tracks => {
                            let index = self.tracks.state.selected();

                            let id = index
                                .map(|index| &self.tracks.items[index])
                                .map(|track| track.id);

                            if let Some(id) = id {
                                qobuz_player_controls::play_track(id).await.unwrap();
                            }
                            Output::Consumed
                        }
                    },
                    _ => Output::NotConsumed,
                },
                true => match key_event.code {
                    KeyCode::Esc | KeyCode::Enter => {
                        self.stop_editing();
                        self.update_search().await;
                        Output::Consumed
                    }
                    _ => {
                        self.filter.handle_event(&event);
                        Output::Consumed
                    }
                },
            },
            _ => Output::NotConsumed,
        }
    }

    async fn update_search(&mut self) {
        if !self.filter.value().trim().is_empty() {
            let search_results = qobuz_player_controls::search(self.filter.value().to_string())
                .await
                .unwrap();

            self.albums.items = search_results.albums;
            self.artists.items = search_results.artists;
            self.playlists.items = search_results.playlists;
            self.tracks.items = search_results.tracks;
        }
    }

    fn start_editing(&mut self) {
        self.editing = true;
    }

    fn stop_editing(&mut self) {
        self.editing = false;
    }

    fn current_list_state(&mut self) -> &mut TableState {
        match self.sub_tab {
            SubTab::Albums => &mut self.albums.state,
            SubTab::Artists => &mut self.artists.state,
            SubTab::Playlists => &mut self.playlists.state,
            SubTab::Tracks => &mut self.tracks.state,
        }
    }

    fn cycle_subtab_backwards(&mut self) {
        self.sub_tab = self.sub_tab.previous();
    }

    fn cycle_subtab(&mut self) {
        self.sub_tab = self.sub_tab.next();
    }
}
