use qobuz_player_controls::{client::Client, notification::Notification};
use qobuz_player_models::AlbumSimple;
use ratatui::{
    buffer::Buffer,
    crossterm::event::KeyCode,
    layout::{Constraint, Rect},
    style::{Modifier, Stylize},
    text::Line,
    widgets::{Row, StatefulWidget, Table},
};

use crate::{
    app::{FilteredListState, NotificationList, Output},
    popup::{AlbumPopupState, Popup},
    ui::{ROW_HIGHLIGHT_STYLE, mark_explicit_and_hifi},
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
        let table = album_simple_table(self.items.filter());
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
                    return match client.add_favorite_album(&selected.id).await {
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
                    return match client.remove_favorite_album(&selected.id).await {
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

                Output::Consumed
            }

            KeyCode::Enter => {
                let index = self.items.state.selected();

                let id = index
                    .and_then(|index| self.items.filter().get(index))
                    .map(|album| album.id.clone());

                if let Some(id) = id {
                    let album = match client.album(&id).await {
                        Ok(res) => res,
                        Err(err) => return Output::Error(err.to_string()),
                    };

                    return Output::Popup(Popup::Album(AlbumPopupState::new(album)));
                }
                Output::Consumed
            }

            _ => Output::NotConsumed,
        }
    }
}

fn album_simple_table<'a>(rows: &[AlbumSimple]) -> Table<'a> {
    let rows: Vec<_> = rows
        .iter()
        .map(|album| {
            Row::new(vec![
                mark_explicit_and_hifi(album.title.clone(), album.explicit, album.hires_available),
                Line::from(album.artist.name.clone()),
            ])
        })
        .collect();

    let is_empty = rows.is_empty();
    let mut table = Table::new(rows, [Constraint::Ratio(2, 3), Constraint::Ratio(1, 3)])
        .row_highlight_style(ROW_HIGHLIGHT_STYLE);

    if !is_empty {
        table = table.header(Row::new(["Title", "Artist"]).add_modifier(Modifier::BOLD));
    }
    table
}
