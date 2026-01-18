use std::sync::Arc;

use qobuz_player_controls::{Result, client::Client};
use qobuz_player_models::{Album, Artist, Playlist, Track};
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    widgets::*,
};
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{
    app::{Output, PlayOutcome, QueueOutcome, UnfilteredListState},
    popup::{AlbumPopupState, ArtistPopupState, PlaylistPopupState, Popup},
    sub_tab::SubTab,
    ui::{album_table, basic_list_table, block, render_input, tab_bar, track_table},
};

pub(crate) struct SearchState {
    pub client: Arc<Client>,
    pub editing: bool,
    pub filter: Input,
    pub albums: UnfilteredListState<Album>,
    pub artists: UnfilteredListState<Artist>,
    pub playlists: UnfilteredListState<Playlist>,
    pub tracks: UnfilteredListState<Track>,
    pub sub_tab: SubTab,
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

        let block = block(None);
        frame.render_widget(block, tab_content_area_split[1]);

        let tab_content_area = tab_content_area_split[1].inner(Margin::new(1, 1));

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(1)])
            .split(tab_content_area);

        let tabs = tab_bar(SubTab::labels(), self.sub_tab.selected().into());
        frame.render_widget(tabs, chunks[0]);

        let (table, state) = match self.sub_tab {
            SubTab::Albums => (album_table(&self.albums.items), &mut self.albums.state),
            SubTab::Artists => (
                basic_list_table(
                    self.artists
                        .items
                        .iter()
                        .map(|artist| Row::new(Line::from(artist.name.clone())))
                        .collect::<Vec<_>>(),
                    None,
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
                    None,
                ),
                &mut self.playlists.state,
            ),
            SubTab::Tracks => (
                track_table(&self.tracks.items, None),
                &mut self.tracks.state,
            ),
        };

        frame.render_stateful_widget(table, chunks[1], state);
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
                        KeyCode::Char('N') => {
                            if self.sub_tab != SubTab::Tracks {
                                return Output::Consumed;
                            }
                            let index = self.tracks.state.selected();
                            let selected = index.and_then(|index| self.tracks.items.get(index));

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
                            let selected = index.and_then(|index| self.tracks.items.get(index));

                            let Some(selected) = selected else {
                                return Output::Consumed;
                            };

                            Output::Queue(QueueOutcome::AddTrackToQueue(selected.id))
                        }
                        KeyCode::Char('A') => match self.sub_tab {
                            SubTab::Albums => {
                                let index = self.albums.state.selected();

                                let id = index
                                    .and_then(|index| self.albums.items.get(index))
                                    .map(|album| album.id.clone());

                                if let Some(id) = id {
                                    _ = self.client.add_favorite_album(&id).await;
                                    return Output::UpdateFavorites;
                                }

                                Output::Consumed
                            }
                            SubTab::Artists => {
                                let index = self.artists.state.selected();
                                let selected =
                                    index.and_then(|index| self.artists.items.get(index));

                                if let Some(selected) = selected {
                                    _ = self.client.add_favorite_artist(selected.id).await;
                                    return Output::UpdateFavorites;
                                }

                                Output::Consumed
                            }
                            SubTab::Playlists => {
                                let index = self.playlists.state.selected();
                                let selected =
                                    index.and_then(|index| self.playlists.items.get(index));

                                if let Some(selected) = selected {
                                    _ = self.client.add_favorite_playlist(selected.id).await;
                                    return Output::UpdateFavorites;
                                }

                                Output::Consumed
                            }
                            SubTab::Tracks => {
                                let index = self.tracks.state.selected();

                                let id = index
                                    .and_then(|index| self.tracks.items.get(index))
                                    .map(|track| track.id);

                                if let Some(id) = id {
                                    _ = self.client.add_favorite_track(id).await;
                                    return Output::UpdateFavorites;
                                }

                                Output::Consumed
                            }
                        },
                        KeyCode::Char('a') => match self.sub_tab {
                            SubTab::Tracks => {
                                let index = self.tracks.state.selected();

                                let track = index.and_then(|index| self.tracks.items.get(index));

                                if let Some(id) = track {
                                    return Output::PlayOutcome(PlayOutcome::AddTrackToPlaylist(
                                        id.clone(),
                                    ));
                                }
                                Output::Consumed
                            }
                            _ => Output::NotConsumed,
                        },
                        KeyCode::Enter => match self.sub_tab {
                            SubTab::Albums => {
                                let index = self.albums.state.selected();

                                let id = index
                                    .and_then(|index| self.albums.items.get(index))
                                    .map(|album| album.id.clone());

                                if let Some(id) = id {
                                    let album = match self.client.album(&id).await {
                                        Ok(res) => res,
                                        Err(err) => return Output::Error(err.to_string()),
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
                                    index.and_then(|index| self.artists.items.get(index));

                                let Some(selected) = selected else {
                                    return Output::Consumed;
                                };

                                let state =
                                    ArtistPopupState::new(selected, self.client.clone()).await;
                                let state = match state {
                                    Ok(res) => res,
                                    Err(err) => return Output::Error(err.to_string()),
                                };

                                Output::Popup(Popup::Artist(state))
                            }
                            SubTab::Playlists => {
                                let index = self.playlists.state.selected();
                                let selected =
                                    index.and_then(|index| self.playlists.items.get(index));

                                let Some(selected) = selected else {
                                    return Output::Consumed;
                                };

                                let playlist = match self.client.playlist(selected.id).await {
                                    Ok(res) => res,
                                    Err(err) => return Output::Error(err.to_string()),
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

                                let id = index
                                    .and_then(|index| self.tracks.items.get(index))
                                    .map(|track| track.id);

                                if let Some(id) = id {
                                    return Output::PlayOutcome(PlayOutcome::Track(id));
                                }
                                Output::Consumed
                            }
                        },
                        _ => Output::NotConsumed,
                    },
                    true => match key_event.code {
                        KeyCode::Esc | KeyCode::Enter => {
                            self.stop_editing();
                            if let Err(err) = self.update_search().await {
                                return Output::Error(err.to_string());
                            };
                            Output::Consumed
                        }
                        _ => {
                            self.filter.handle_event(&event);
                            Output::Consumed
                        }
                    },
                }
            }
            _ => Output::NotConsumed,
        }
    }

    async fn update_search(&mut self) -> Result<()> {
        if !self.filter.value().trim().is_empty() {
            let search_results = self.client.search(self.filter.value().to_string()).await?;

            self.albums.items = search_results.albums;
            self.artists.items = search_results.artists;
            self.playlists.items = search_results.playlists;
            self.tracks.items = search_results.tracks;
        }

        Ok(())
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
