use qobuz_player_controls::{AppResult, client::Client, notification::Notification};
use qobuz_player_models::PlaylistSimple;
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
    popup::{DeletePlaylistPopupstate, NewPlaylistPopupState, PlaylistPopupState, Popup},
    ui::{COLUMN_SPACING, ROW_HIGHLIGHT_STYLE, format_duration, mark_as_owned},
};

#[derive(Default)]
pub struct PlaylistList {
    items: FilteredListState<PlaylistSimple>,
}

impl PlaylistList {
    pub fn new(playlists: Vec<PlaylistSimple>) -> Self {
        let playlists = FilteredListState::new(playlists);
        Self { items: playlists }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let table = playlist_list(self.items.filter());
        table.render(area, buf, &mut self.items.state);
    }

    pub fn set_filter(&mut self, items: Vec<PlaylistSimple>) {
        self.items.set_filter(items);
    }

    pub fn all_items(&self) -> &Vec<PlaylistSimple> {
        self.items.all_items()
    }

    pub fn set_all_items(&mut self, items: Vec<PlaylistSimple>) {
        self.items.set_all_items(items);
    }

    pub fn selected(&self) -> Option<usize> {
        self.items.state.selected()
    }

    pub fn get(&self, index: usize) -> Option<&PlaylistSimple> {
        self.items.filter().get(index)
    }

    pub fn select_next(&mut self) {
        self.items.state.select_next();
    }

    pub fn select_previous(&mut self) {
        self.items.state.select_previous();
    }

    pub async fn handle_events(
        &mut self,
        event: KeyCode,
        client: &Client,
        notifications: &mut NotificationList,
    ) -> AppResult<Output> {
        match event {
            KeyCode::Down | KeyCode::Char('j') => {
                self.items.state.select_next();
                Ok(Output::Consumed)
            }

            KeyCode::Up | KeyCode::Char('k') => {
                self.items.state.select_previous();
                Ok(Output::Consumed)
            }

            KeyCode::Char('C') => Ok(Output::Popup(Popup::NewPlaylist(
                NewPlaylistPopupState::new(),
            ))),

            KeyCode::Char('A') => {
                let index = self.items.state.selected();
                let selected = index.and_then(|index| self.items.filter().get(index));

                if let Some(selected) = selected
                    && !selected.is_owned
                {
                    client.add_favorite_playlist(selected.id).await?;

                    notifications.push(Notification::Info(format!(
                        "{} added to favorites",
                        selected.title
                    )));
                    return Ok(Output::UpdateFavorites);
                }

                Ok(Output::Consumed)
            }

            KeyCode::Char('D') => {
                let index = self.items.state.selected();
                let selected = index.and_then(|index| self.items.filter().get(index));

                if let Some(selected) = selected {
                    match selected.is_owned {
                        true => {
                            return Ok(Output::Popup(Popup::DeletePlaylist(
                                DeletePlaylistPopupstate::new(selected.clone()),
                            )));
                        }
                        false => {
                            client.remove_favorite_playlist(selected.id).await?;

                            notifications.push(Notification::Info(format!(
                                "{} removed from favorites",
                                selected.title
                            )));
                            return Ok(Output::UpdateFavorites);
                        }
                    }
                }

                Ok(Output::Consumed)
            }

            KeyCode::Enter => {
                let index = self.items.state.selected();
                let selected = index.and_then(|index| self.items.filter().get(index));

                let Some(selected) = selected else {
                    return Ok(Output::Consumed);
                };

                let playlist = client.playlist(selected.id).await?;

                Ok(Output::Popup(Popup::Playlist(PlaylistPopupState::new(
                    playlist,
                ))))
            }

            _ => Ok(Output::NotConsumed),
        }
    }
}

fn playlist_list<'a>(rows: &[PlaylistSimple]) -> Table<'a> {
    let body_rows: Vec<Row<'a>> = rows
        .iter()
        .map(|playlist| {
            Row::new(vec![
                mark_as_owned(playlist.title.clone(), playlist.is_owned),
                Line::from(format_duration(playlist.duration_seconds)),
            ])
        })
        .collect();

    let is_empty = body_rows.is_empty();

    let constraints = [Constraint::Ratio(2, 3), Constraint::Length(10)];

    let mut table = Table::new(body_rows, constraints)
        .row_highlight_style(ROW_HIGHLIGHT_STYLE)
        .column_spacing(COLUMN_SPACING);

    if !is_empty {
        table = table.header(Row::new(["Title", "Duration"]).add_modifier(Modifier::BOLD));
    }

    table
}
