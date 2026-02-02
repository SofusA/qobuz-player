use qobuz_player_controls::{Result, client::Client, notification::Notification};
use qobuz_player_models::AlbumSimple;
use ratatui::{buffer::Buffer, crossterm::event::KeyCode, layout::Rect, widgets::StatefulWidget};

use crate::{
    app::{FilteredListState, NotificationList, Output},
    popup::{AlbumPopupState, Popup},
    widgets::album_list::album_table,
};

pub struct AlbumSimpleList {
    items: FilteredListState<AlbumSimple>,
}

impl AlbumSimpleList {
    pub fn new(albums: Vec<AlbumSimple>) -> Self {
        let albums = FilteredListState::new(albums);
        Self { items: albums }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let table = album_table(self.items.filter());
        table.render(area, buf, &mut self.items.state);
    }

    pub fn select_first(&mut self) {
        self.items.state.select(Some(0));
    }

    pub fn filter(&self) -> &Vec<AlbumSimple> {
        self.items.filter()
    }

    pub async fn handle_events(
        &mut self,
        event: KeyCode,
        client: &Client,
        notifications: &mut NotificationList,
    ) -> Result<Output> {
        match event {
            KeyCode::Down | KeyCode::Char('j') => {
                self.items.state.select_next();
                Ok(Output::Consumed)
            }

            KeyCode::Up | KeyCode::Char('k') => {
                self.items.state.select_previous();
                Ok(Output::Consumed)
            }

            KeyCode::Char('A') => {
                let index = self.items.state.selected();

                let selected = index.and_then(|index| self.items.filter().get(index));

                if let Some(selected) = selected {
                    client.add_favorite_album(&selected.id).await?;

                    notifications.push(Notification::Info(format!(
                        "{} added to favorites",
                        selected.title
                    )));
                    return Ok(Output::UpdateFavorites);
                };

                Ok(Output::Consumed)
            }

            KeyCode::Char('D') => {
                let index = self.items.state.selected();

                let selected = index.and_then(|index| self.items.filter().get(index));

                if let Some(selected) = selected {
                    client.remove_favorite_album(&selected.id).await?;

                    notifications.push(Notification::Info(format!(
                        "{} removed from favorites",
                        selected.title
                    )));
                    return Ok(Output::UpdateFavorites);
                }

                Ok(Output::Consumed)
            }

            KeyCode::Enter => {
                let index = self.items.state.selected();

                let id = index
                    .and_then(|index| self.items.filter().get(index))
                    .map(|album| album.id.clone());

                if let Some(id) = id {
                    let album = client.album(&id).await?;

                    return Ok(Output::Popup(Popup::Album(AlbumPopupState::new(album))));
                }
                Ok(Output::Consumed)
            }

            _ => Ok(Output::NotConsumed),
        }
    }
}
