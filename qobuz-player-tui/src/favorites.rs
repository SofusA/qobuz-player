use std::{fmt, sync::Arc};

use qobuz_player_controls::client::Client;
use qobuz_player_models::{Album, Artist, Playlist, Track};
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    widgets::*,
};
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{
    app::{FilteredListState, Output, PlayOutcome, QueueOutcome},
    popup::{AlbumPopupState, ArtistPopupState, NewPlaylistPopupState, PlaylistPopupState, Popup},
    ui::{album_table, basic_list_table, render_input, track_table},
};

pub(crate) struct FavoritesState {
    pub client: Arc<Client>,
    pub editing: bool,
    pub filter: Input,
    pub albums: FilteredListState<Album>,
    pub artists: FilteredListState<Artist>,
    pub playlists: FilteredListState<Playlist>,
    pub tracks: FilteredListState<Track>,
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
        let index = Self::VALUES
            .iter()
            .position(|&x| x == self)
            .expect("infallible");
        Self::VALUES[(index + 1) % Self::VALUES.len()]
    }

    pub(crate) fn previous(self) -> Self {
        let index = Self::VALUES
            .iter()
            .position(|&x| x == self)
            .expect("infallible");
        let len = Self::VALUES.len();
        Self::VALUES[(index + len - 1) % len]
    }
}

impl FavoritesState {
    pub(crate) fn render(&mut self, frame: &mut Frame, area: Rect) {
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

        let tab_content_area = tab_content_area_split[1];
        let title = format!("Favorites: {}", self.sub_tab);

        let (table, state) = match self.sub_tab {
            SubTab::Albums => (
                album_table(&self.albums.filter, "Favorite: Albums"),
                &mut self.albums.state,
            ),
            SubTab::Artists => (
                basic_list_table(
                    self.artists
                        .filter
                        .iter()
                        .map(|artist| Row::new(Line::from(artist.name.clone())))
                        .collect::<Vec<_>>(),
                    title.as_str(),
                    true,
                ),
                &mut self.artists.state,
            ),
            SubTab::Playlists => (
                // TODO: Render owned
                basic_list_table(
                    self.playlists
                        .filter
                        .iter()
                        .map(|playlist| Row::new(Line::from(playlist.title.clone())))
                        .collect::<Vec<_>>(),
                    title.as_str(),
                    true,
                ),
                &mut self.playlists.state,
            ),
            SubTab::Tracks => (
                track_table(&self.tracks.filter, Some(&title)),
                &mut self.tracks.state,
            ),
        };

        frame.render_stateful_widget(table, tab_content_area, state);
    }

    pub(crate) async fn handle_events(&mut self, event: Event) -> Output {
        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match &mut self.editing {
                    false => match key_event.code {
                        KeyCode::Char('e') => {
                            self.start_editing();
                            Output::Consumed
                        }
                        KeyCode::Left | KeyCode::Char('h') => {
                            self.cycle_subtab_backwards();
                            Output::Consumed
                        }
                        KeyCode::Right | KeyCode::Char('l') => {
                            self.cycle_subtab();
                            Output::Consumed
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            self.current_list_state().select_next();
                            Output::Consumed
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            self.current_list_state().select_previous();
                            Output::Consumed
                        }
                        KeyCode::Char('C') => match self.sub_tab {
                            SubTab::Playlists => {
                                Output::Popup(Popup::NewPlaylist(NewPlaylistPopupState {
                                    name: Default::default(),
                                    client: self.client.clone(),
                                }))
                            }
                            _ => Output::NotConsumed,
                        },
                        KeyCode::Char('a') => match self.sub_tab {
                            SubTab::Tracks => {
                                let index = self.tracks.state.selected();

                                let track = index.and_then(|index| self.tracks.filter.get(index));

                                if let Some(id) = track {
                                    return Output::PlayOutcome(PlayOutcome::AddTrackToPlaylist(
                                        id.clone(),
                                    ));
                                }
                                Output::Consumed
                            }
                            _ => Output::NotConsumed,
                        },
                        KeyCode::Char('N') => {
                            if self.sub_tab != SubTab::Tracks {
                                return Output::Consumed;
                            }
                            let index = self.tracks.state.selected();
                            let selected = index.and_then(|index| self.tracks.filter.get(index));

                            let Some(selected) = selected else {
                                return Output::Consumed;
                            };

                            Output::Queue(QueueOutcome::PlayTrackNext(selected.id))
                        }
                        KeyCode::Char('B') => {
                            if self.sub_tab != SubTab::Tracks {
                                return Output::Consumed;
                            }

                            let index = self.tracks.state.selected();
                            let selected = index.and_then(|index| self.tracks.filter.get(index));

                            let Some(selected) = selected else {
                                return Output::Consumed;
                            };

                            Output::Queue(QueueOutcome::AddTrackToQueue(selected.id))
                        }
                        KeyCode::Char('D') => match self.sub_tab {
                            SubTab::Albums => {
                                let index = self.albums.state.selected();

                                let id = index
                                    .and_then(|index| self.albums.filter.get(index))
                                    .map(|album| album.id.clone());

                                if let Some(id) = id {
                                    _ = self.client.remove_favorite_album(&id).await;
                                }

                                Output::UpdateFavorites
                            }
                            SubTab::Artists => {
                                let index = self.artists.state.selected();
                                let selected =
                                    index.and_then(|index| self.artists.filter.get(index));

                                if let Some(selected) = selected {
                                    _ = self.client.remove_favorite_artist(selected.id).await;
                                }
                                Output::UpdateFavorites
                            }
                            SubTab::Playlists => {
                                let index = self.playlists.state.selected();
                                let selected =
                                    index.and_then(|index| self.playlists.filter.get(index));

                                if let Some(selected) = selected {
                                    match selected.is_owned {
                                        // TODO: Add confirmation
                                        true => _ = self.client.delete_playlist(selected.id).await,
                                        false => {
                                            _ = self
                                                .client
                                                .remove_favorite_playlist(selected.id)
                                                .await
                                        }
                                    }
                                }

                                Output::UpdateFavorites
                            }
                            SubTab::Tracks => {
                                let index = self.tracks.state.selected();
                                let selected =
                                    index.and_then(|index| self.tracks.filter.get(index));

                                if let Some(selected) = selected {
                                    _ = self.client.remove_favorite_track(selected.id).await;
                                }
                                Output::UpdateFavorites
                            }
                        },
                        KeyCode::Enter => match self.sub_tab {
                            SubTab::Albums => {
                                let index = self.albums.state.selected();

                                let id = index
                                    .and_then(|index| self.albums.filter.get(index))
                                    .map(|album| album.id.clone());

                                if let Some(id) = id {
                                    let album = match self.client.album(&id).await {
                                        Ok(res) => res,
                                        Err(err) => return Output::Error(format!("{err}")),
                                    };

                                    return Output::Popup(Popup::Album(AlbumPopupState::new(
                                        album,
                                    )));
                                }
                                Output::Consumed
                            }
                            SubTab::Artists => {
                                let index = self.artists.state.selected();
                                let selected =
                                    index.and_then(|index| self.artists.filter.get(index));

                                let Some(selected) = selected else {
                                    return Output::Consumed;
                                };

                                let artist_albums =
                                    match self.client.artist_albums(selected.id).await {
                                        Ok(res) => res,
                                        Err(err) => return Output::Error(format!("{err}")),
                                    };

                                Output::Popup(Popup::Artist(ArtistPopupState {
                                    artist_name: selected.name.clone(),
                                    albums: artist_albums,
                                    state: Default::default(),
                                }))
                            }
                            SubTab::Playlists => {
                                let index = self.playlists.state.selected();
                                let selected =
                                    index.and_then(|index| self.playlists.filter.get(index));

                                let Some(selected) = selected else {
                                    return Output::Consumed;
                                };

                                let playlist = match self.client.playlist(selected.id).await {
                                    Ok(res) => res,
                                    Err(err) => return Output::Error(format!("{err}")),
                                };

                                Output::Popup(Popup::Playlist(PlaylistPopupState {
                                    playlist,
                                    shuffle: false,
                                    state: Default::default(),
                                    client: self.client.clone(),
                                }))
                            }
                            SubTab::Tracks => {
                                let index = self.tracks.state.selected();
                                let selected =
                                    index.and_then(|index| self.tracks.filter.get(index));

                                let Some(selected) = selected else {
                                    return Output::Consumed;
                                };

                                Output::PlayOutcome(PlayOutcome::Track(selected.id))
                            }
                        },
                        _ => Output::NotConsumed,
                    },
                    true => match key_event.code {
                        KeyCode::Esc | KeyCode::Enter => {
                            self.stop_editing();
                            Output::Consumed
                        }
                        _ => {
                            self.filter.handle_event(&event);

                            self.albums.filter = self
                                .albums
                                .all_items
                                .iter()
                                .filter(|x| {
                                    x.title
                                        .to_lowercase()
                                        .contains(&self.filter.value().to_lowercase())
                                        || x.artist
                                            .name
                                            .to_lowercase()
                                            .contains(&self.filter.value().to_lowercase())
                                })
                                .cloned()
                                .collect();

                            self.artists.filter = self
                                .artists
                                .all_items
                                .iter()
                                .filter(|x| {
                                    x.name
                                        .to_lowercase()
                                        .contains(&self.filter.value().to_lowercase())
                                })
                                .cloned()
                                .collect();

                            self.playlists.filter = self
                                .playlists
                                .all_items
                                .iter()
                                .filter(|x| {
                                    x.title
                                        .to_lowercase()
                                        .contains(&self.filter.value().to_lowercase())
                                })
                                .cloned()
                                .collect();
                            Output::Consumed
                        }
                    },
                }
            }
            _ => Output::NotConsumed,
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
