use std::sync::Arc;

use qobuz_player_controls::client::Client;
use qobuz_player_models::{AlbumSimple, Playlist};
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    widgets::*,
};

use crate::{
    app::{Output, UnfilteredListState},
    popup::{AlbumPopupState, PlaylistPopupState, Popup},
    ui::{album_simple_table, basic_list_table, block, tab_bar},
};

pub(crate) struct DiscoverState {
    pub(crate) client: Arc<Client>,
    pub(crate) featured_albums: Vec<(String, UnfilteredListState<AlbumSimple>)>,
    pub(crate) featured_playlists: Vec<(String, UnfilteredListState<Playlist>)>,
    pub(crate) selected_sub_tab: usize,
}

impl DiscoverState {
    pub(crate) fn render(&mut self, frame: &mut Frame, area: Rect) {
        let block = block(None);
        frame.render_widget(block, area);

        let tab_content_area = area.inner(Margin::new(1, 1));

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(1)])
            .split(tab_content_area);

        let album_labels: Vec<_> = self
            .featured_albums
            .iter()
            .map(|fa| fa.0.as_str())
            .collect();
        let playlist_labels: Vec<_> = self
            .featured_playlists
            .iter()
            .map(|fp| fp.0.as_str())
            .collect();
        let labels = [album_labels, playlist_labels].concat();

        let tabs = tab_bar(labels, self.selected_sub_tab);
        frame.render_widget(tabs, chunks[0]);

        let is_album = self.album_selected();

        let (table, state) = match is_album {
            true => {
                let list_state = &mut self.featured_albums[self.selected_sub_tab];
                (
                    album_simple_table(&list_state.1.items),
                    &mut list_state.1.state,
                )
            }
            false => {
                let list_state = &mut self.featured_playlists
                    [self.selected_sub_tab - self.featured_albums.len()];
                (
                    basic_list_table(
                        list_state
                            .1
                            .items
                            .iter()
                            .map(|playlist| Row::new(Line::from(playlist.title.clone())))
                            .collect::<Vec<_>>(),
                        None,
                    ),
                    &mut list_state.1.state,
                )
            }
        };

        frame.render_stateful_widget(table, chunks[1], state);
    }

    pub(crate) async fn handle_events(&mut self, event: Event) -> Output {
        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match key_event.code {
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
                    KeyCode::Enter => {
                        let selected_index = self.current_list_state().selected();
                        if let Some(selected_index) = selected_index {
                            let is_abum = self.album_selected();

                            match is_abum {
                                true => {
                                    let items = self.featured_albums.get(self.selected_sub_tab);
                                    let Some(items) = items else {
                                        return Output::NotConsumed;
                                    };

                                    let id =
                                        items.1.items.get(selected_index).map(|x| x.id.clone());

                                    let Some(id) = id else {
                                        return Output::NotConsumed;
                                    };

                                    let album = match self.client.album(&id).await {
                                        Ok(res) => res,
                                        Err(err) => return Output::Error(err.to_string()),
                                    };

                                    return Output::Popup(Popup::Album(AlbumPopupState::new(
                                        album,
                                        self.client.clone(),
                                    )));
                                }
                                false => {
                                    let items = &self.featured_playlists
                                        [self.selected_sub_tab - self.featured_albums.len()]
                                    .1
                                    .items;

                                    let playlist = &items[selected_index];

                                    return Output::Popup(Popup::Playlist(PlaylistPopupState {
                                        playlist: playlist.clone(),
                                        shuffle: false,
                                        state: Default::default(),
                                        client: self.client.clone(),
                                    }));
                                }
                            }
                        }
                        Output::Consumed
                    }
                    KeyCode::Char('A') => {
                        let selected_index = self.current_list_state().selected();
                        if let Some(selected_index) = selected_index {
                            let is_abum = self.album_selected();

                            match is_abum {
                                true => {
                                    let items = self.featured_albums.get(self.selected_sub_tab);
                                    let Some(items) = items else {
                                        return Output::NotConsumed;
                                    };

                                    let id =
                                        items.1.items.get(selected_index).map(|x| x.id.clone());

                                    if let Some(id) = id {
                                        _ = self.client.add_favorite_album(&id).await;
                                        return Output::UpdateFavorites;
                                    };

                                    return Output::Consumed;
                                }
                                false => {
                                    let items = &self.featured_playlists
                                        [self.selected_sub_tab - self.featured_albums.len()]
                                    .1
                                    .items;

                                    let playlist = &items[selected_index];

                                    _ = self.client.add_favorite_playlist(playlist.id).await;
                                    return Output::UpdateFavorites;
                                }
                            }
                        }
                        Output::Consumed
                    }

                    _ => Output::NotConsumed,
                }
            }
            _ => Output::NotConsumed,
        }
    }

    fn album_selected(&self) -> bool {
        self.selected_sub_tab < self.featured_albums.len()
    }

    fn current_list_state(&mut self) -> &mut TableState {
        let is_album = self.album_selected();

        match is_album {
            true => &mut self.featured_albums[self.selected_sub_tab].1.state,
            false => {
                &mut self.featured_playlists[self.selected_sub_tab - self.featured_albums.len()]
                    .1
                    .state
            }
        }
    }

    fn cycle_subtab_backwards(&mut self) {
        let count = self.featured_albums.len() + self.featured_playlists.len();
        self.selected_sub_tab = (self.selected_sub_tab + count - 1) % count;
    }

    fn cycle_subtab(&mut self) {
        let count = self.featured_albums.len() + self.featured_playlists.len();
        self.selected_sub_tab = (self.selected_sub_tab + count + 1) % count;
    }
}
