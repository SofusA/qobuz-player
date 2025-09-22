use qobuz_player_models::{AlbumSimple, Playlist};
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    widgets::*,
};

use crate::{
    app::{Output, PlayOutcome, UnfilteredListState},
    popup::{PlaylistPopupState, Popup},
    ui::{album_simple_table, basic_list_table},
};

pub(crate) struct DiscoverState {
    pub(crate) featured_albums: Vec<(String, UnfilteredListState<AlbumSimple>)>,
    pub(crate) featured_playlists: Vec<(String, UnfilteredListState<Playlist>)>,
    pub(crate) sub_tab: usize,
}

impl DiscoverState {
    pub(crate) fn render(&mut self, frame: &mut Frame, area: Rect) {
        let is_album = self.album_selected();

        let (table, state) = match is_album {
            true => {
                let list_state = &mut self.featured_albums[self.sub_tab];
                (
                    album_simple_table(&list_state.1.items, &list_state.0),
                    &mut list_state.1.state,
                )
            }
            false => {
                let list_state =
                    &mut self.featured_playlists[self.sub_tab - self.featured_albums.len()];
                (
                    basic_list_table(
                        list_state
                            .1
                            .items
                            .iter()
                            .map(|playlist| Row::new(Line::from(playlist.title.clone())))
                            .collect::<Vec<_>>(),
                        &list_state.0,
                    ),
                    &mut list_state.1.state,
                )
            }
        };

        frame.render_stateful_widget(table, area, state);
    }

    pub(crate) async fn handle_events(&mut self, event: Event) -> Output {
        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match key_event.code {
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
                    KeyCode::Enter => {
                        let selected_index = self.current_list_state().selected();
                        if let Some(selected_index) = selected_index {
                            let is_abum = self.album_selected();

                            match is_abum {
                                true => {
                                    let items = &self.featured_albums[self.sub_tab].1.items;
                                    let id = items[selected_index].id.clone();

                                    return Output::PlayOutcome(PlayOutcome::Album(id));
                                }
                                false => {
                                    let items = &self.featured_playlists
                                        [self.sub_tab - self.featured_albums.len()]
                                    .1
                                    .items;

                                    let playlist = &items[selected_index];

                                    return Output::Popup(Popup::Playlist(PlaylistPopupState {
                                        playlist_name: playlist.title.clone(),
                                        playlist_id: playlist.id,
                                        shuffle: false,
                                    }));
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
        self.sub_tab < self.featured_albums.len()
    }

    fn current_list_state(&mut self) -> &mut TableState {
        let is_album = self.album_selected();

        match is_album {
            true => &mut self.featured_albums[self.sub_tab].1.state,
            false => {
                &mut self.featured_playlists[self.sub_tab - self.featured_albums.len()]
                    .1
                    .state
            }
        }
    }

    fn cycle_subtab_backwards(&mut self) {
        let count = self.featured_albums.len() + self.featured_playlists.len();
        self.sub_tab = (self.sub_tab + count - 1) % count;
    }

    fn cycle_subtab(&mut self) {
        let count = self.featured_albums.len() + self.featured_playlists.len();
        self.sub_tab = (self.sub_tab + count + 1) % count;
    }
}
