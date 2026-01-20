use qobuz_player_controls::{
    Result, client::Client, controls::Controls, notification::Notification,
};
use qobuz_player_models::Track;
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
    ui::{ROW_HIGHLIGHT_STYLE, mark_explicit_and_hifi},
};

#[derive(Default)]
pub struct TrackList {
    items: FilteredListState<Track>,
}

pub enum TrackListEvent {
    Track,
    Album(String),
    Playlist(u32, bool),
    Artist(u32),
}

impl TrackList {
    pub fn new(tracks: Vec<Track>) -> Self {
        let tracks = FilteredListState::new(tracks);
        Self { items: tracks }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let table = track_table(self.items.filter());
        table.render(area, buf, &mut self.items.state);
    }

    pub fn select_first(&mut self) {
        self.items.state.select(Some(0));
    }

    pub fn set_all_items(&mut self, items: Vec<Track>) {
        self.items.set_all_items(items);
    }

    pub fn filter(&self) -> &Vec<Track> {
        self.items.filter()
    }

    pub async fn handle_events(
        &mut self,
        event: KeyCode,
        client: &Client,
        controls: &Controls,
        notifications: &mut NotificationList,
        event_type: TrackListEvent,
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

            KeyCode::Char('a') => {
                let index = self.items.state.selected();

                let track = index.and_then(|index| self.items.filter().get(index));

                if let Some(id) = track {
                    return Ok(Output::AddTrackToPlaylist(id.clone()));
                }
                Ok(Output::Consumed)
            }

            KeyCode::Char('N') => {
                let index = self.items.state.selected();
                let selected = index.and_then(|index| self.items.filter().get(index));

                let Some(selected) = selected else {
                    return Ok(Output::Consumed);
                };

                controls.play_track_next(selected.id);
                Ok(Output::Consumed)
            }

            KeyCode::Char('B') => {
                let index = self.items.state.selected();
                let selected = index.and_then(|index| self.items.filter().get(index));

                if let Some(selected) = selected {
                    controls.add_track_to_queue(selected.id);
                };
                Ok(Output::Consumed)
            }

            KeyCode::Char('A') => {
                let index = self.items.state.selected();
                let selected = index.and_then(|index| self.items.filter().get(index));

                if let Some(selected) = selected {
                    client.add_favorite_track(selected.id).await?;
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
                    client.remove_favorite_track(selected.id).await?;
                    notifications.push(Notification::Info(format!(
                        "{} removed from favorites",
                        selected.title
                    )));
                    return Ok(Output::UpdateFavorites);
                }
                Ok(Output::Consumed)
            }

            KeyCode::Enter => {
                let Some(index) = self.items.state.selected() else {
                    return Ok(Output::Consumed);
                };

                match event_type {
                    TrackListEvent::Track => {
                        let selected = self.items.filter().get(index);
                        if let Some(selected) = selected {
                            controls.play_track(selected.id);
                        }
                    }
                    TrackListEvent::Album(id) => controls.play_album(&id, index),
                    TrackListEvent::Playlist(id, shuffle) => {
                        controls.play_playlist(id, index, shuffle)
                    }
                    TrackListEvent::Artist(id) => controls.play_top_tracks(id, index),
                }

                Ok(Output::Consumed)
            }

            _ => Ok(Output::NotConsumed),
        }
    }
}

fn track_table<'a>(rows: &[Track]) -> Table<'a> {
    let rows: Vec<_> = rows
        .iter()
        .map(|track| {
            Row::new(vec![
                mark_explicit_and_hifi(track.title.clone(), track.explicit, track.hires_available),
                Line::from(track.artist_name.clone().unwrap_or_default()),
                Line::from(track.album_title.clone().unwrap_or_default()),
            ])
        })
        .collect();

    let is_empty = rows.is_empty();
    let mut table = Table::new(
        rows,
        [
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ],
    )
    .row_highlight_style(ROW_HIGHLIGHT_STYLE);

    if !is_empty {
        table = table.header(Row::new(["Title", "Artist", "Album"]).add_modifier(Modifier::BOLD));
    }
    table
}
