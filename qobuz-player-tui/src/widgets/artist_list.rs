use qobuz_player_controls::{client::Client, notification::Notification};
use qobuz_player_models::Artist;
use ratatui::{
    buffer::Buffer,
    crossterm::event::KeyCode,
    layout::Rect,
    text::Line,
    widgets::{Row, StatefulWidget},
};

use crate::{
    app::{FilteredListState, NotificationList, Output},
    popup::{ArtistPopupState, Popup},
    ui::basic_list_table,
};

#[derive(Default)]
pub struct ArtistList {
    items: FilteredListState<Artist>,
}

impl ArtistList {
    pub fn new(artists: Vec<Artist>) -> Self {
        let artists = FilteredListState::new(artists);
        Self { items: artists }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let table = basic_list_table(
            self.items
                .filter()
                .iter()
                .map(|artist| Row::new(Line::from(artist.name.clone())))
                .collect::<Vec<_>>(),
        );

        table.render(area, buf, &mut self.items.state);
    }

    pub fn set_filter(&mut self, items: Vec<Artist>) {
        self.items.set_filter(items);
    }

    pub fn all_items(&self) -> &Vec<Artist> {
        self.items.all_items()
    }

    pub fn set_all_items(&mut self, items: Vec<Artist>) {
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

            KeyCode::Char('A') => {
                let index = self.items.state.selected();
                let selected = index.and_then(|index| self.items.filter().get(index));

                if let Some(selected) = selected {
                    return match client.add_favorite_artist(selected.id).await {
                        Ok(_) => {
                            notifications.push(Notification::Info(format!(
                                "{} added to favorites",
                                selected.name
                            )));
                            Output::UpdateFavorites
                        }
                        Err(err) => Output::Error(err.to_string()),
                    };
                }
                Output::UpdateFavorites
            }

            KeyCode::Char('D') => {
                let index = self.items.state.selected();
                let selected = index.and_then(|index| self.items.filter().get(index));

                if let Some(selected) = selected {
                    return match client.remove_favorite_artist(selected.id).await {
                        Ok(_) => {
                            notifications.push(Notification::Info(format!(
                                "{} removed from favorites",
                                selected.name
                            )));
                            Output::UpdateFavorites
                        }
                        Err(err) => Output::Error(err.to_string()),
                    };
                }
                Output::UpdateFavorites
            }

            KeyCode::Enter => {
                let index = self.items.state.selected();
                let selected = index.and_then(|index| self.items.filter().get(index));

                let Some(selected) = selected else {
                    return Output::Consumed;
                };

                let state = ArtistPopupState::new(selected, client).await;
                let state = match state {
                    Ok(res) => res,
                    Err(err) => return Output::Error(err.to_string()),
                };

                Output::Popup(Popup::Artist(state))
            }

            _ => Output::NotConsumed,
        }
    }
}
