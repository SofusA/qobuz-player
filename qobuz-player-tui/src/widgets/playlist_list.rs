use qobuz_player_controls::{client::Client, notification::Notification};
use qobuz_player_models::Playlist;
use ratatui::{
    buffer::Buffer,
    crossterm::event::KeyCode,
    layout::Rect,
    widgets::{Row, StatefulWidget},
};

use crate::{
    app::{FilteredListState, NotificationList, Output},
    popup::{DeletePlaylistPopupstate, NewPlaylistPopupState, PlaylistPopupState, Popup},
    ui::{basic_list_table, mark_as_owned},
};

#[derive(Default)]
pub struct PlaylistList {
    items: FilteredListState<Playlist>,
}

impl PlaylistList {
    pub fn new(playlists: Vec<Playlist>) -> Self {
        let playlists = FilteredListState::new(playlists);
        Self { items: playlists }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let table = basic_list_table(
            self.items
                .filter()
                .iter()
                .map(|playlist| {
                    Row::new(vec![mark_as_owned(
                        playlist.title.clone(),
                        playlist.is_owned,
                    )])
                })
                .collect::<Vec<_>>(),
        );
        table.render(area, buf, &mut self.items.state);
    }

    pub fn set_filter(&mut self, items: Vec<Playlist>) {
        self.items.set_filter(items);
    }

    pub fn all_items(&self) -> &Vec<Playlist> {
        self.items.all_items()
    }

    pub fn set_all_items(&mut self, items: Vec<Playlist>) {
        self.items.set_all_items(items);
    }

    pub async fn handle_events(
        &mut self,
        event: KeyCode,
        client: &Client,
        notifications: &mut NotificationList,
    ) -> Output {
        match event {
            KeyCode::Down | KeyCode::Char('j') => {
                self.items.state.select_next();
                Output::Consumed
            }

            KeyCode::Up | KeyCode::Char('k') => {
                self.items.state.select_previous();
                Output::Consumed
            }

            KeyCode::Char('C') => Output::Popup(Popup::NewPlaylist(NewPlaylistPopupState::new())),

            KeyCode::Char('A') => {
                let index = self.items.state.selected();
                let selected = index.and_then(|index| self.items.filter().get(index));

                if let Some(selected) = selected
                    && !selected.is_owned
                {
                    return match client.add_favorite_playlist(selected.id).await {
                        Ok(_) => {
                            notifications.push(Notification::Info(format!(
                                "{} added to favorites",
                                selected.title
                            )));
                            Output::UpdateFavorites
                        }
                        Err(err) => Output::Error(err.to_string()),
                    };
                }

                Output::Consumed
            }

            KeyCode::Char('D') => {
                let index = self.items.state.selected();
                let selected = index.and_then(|index| self.items.filter().get(index));

                if let Some(selected) = selected {
                    match selected.is_owned {
                        true => {
                            return Output::Popup(Popup::DeletePlaylist(
                                DeletePlaylistPopupstate::new(selected.clone()),
                            ));
                        }
                        false => {
                            return match client.remove_favorite_playlist(selected.id).await {
                                Ok(_) => {
                                    notifications.push(Notification::Info(format!(
                                        "{} removed from favorites",
                                        selected.title
                                    )));
                                    Output::UpdateFavorites
                                }
                                Err(err) => Output::Error(err.to_string()),
                            };
                        }
                    }
                }

                Output::Consumed
            }

            KeyCode::Enter => {
                let index = self.items.state.selected();
                let selected = index.and_then(|index| self.items.filter().get(index));

                let Some(selected) = selected else {
                    return Output::Consumed;
                };

                let playlist = match client.playlist(selected.id).await {
                    Ok(res) => res,
                    Err(err) => return Output::Error(err.to_string()),
                };

                Output::Popup(Popup::Playlist(PlaylistPopupState::new(playlist)))
            }

            _ => Output::NotConsumed,
        }
    }
}
