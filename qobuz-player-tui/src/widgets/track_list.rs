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
    ui::{COLUMN_SPACING, ROW_HIGHLIGHT_STYLE, format_duration, mark_explicit_and_hifi},
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

    pub fn render(&mut self, area: Rect, buf: &mut Buffer, show_album: bool) {
        let table = track_table(self.items.filter(), show_album);
        table.render(area, buf, &mut self.items.state);
    }

    pub fn all_items(&self) -> &Vec<Track> {
        self.items.all_items()
    }

    pub fn set_filter(&mut self, items: Vec<Track>) {
        self.items.set_filter(items);
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

fn track_table<'a>(rows: &[Track], show_album: bool) -> Table<'a> {
    let body_rows: Vec<Row<'a>> = rows
        .iter()
        .map(|track| {
            let mut cols: Vec<Line<'a>> = Vec::with_capacity(if show_album { 4 } else { 3 });

            cols.push(mark_explicit_and_hifi(
                track.title.clone(),
                track.explicit,
                track.hires_available,
            ));

            cols.push(Line::from(track.artist_name.clone().unwrap_or_default()));

            if show_album {
                cols.push(Line::from(track.album_title.clone().unwrap_or_default()));
            }

            cols.push(Line::from(format_duration(track.duration_seconds)));

            Row::new(cols)
        })
        .collect();

    let is_empty = body_rows.is_empty();

    let constraints: Vec<Constraint> = if show_album {
        vec![
            Constraint::Ratio(2, 6),
            Constraint::Ratio(2, 6),
            Constraint::Ratio(1, 6),
            Constraint::Length(10),
        ]
    } else {
        vec![
            Constraint::Ratio(2, 5),
            Constraint::Ratio(2, 5),
            Constraint::Length(10),
        ]
    };

    let mut table = Table::new(body_rows, constraints)
        .row_highlight_style(ROW_HIGHLIGHT_STYLE)
        .column_spacing(COLUMN_SPACING);

    if !is_empty {
        let header = if show_album {
            Row::new(vec!["Title", "Artist", "Album", "Duration"])
        } else {
            Row::new(vec!["Title", "Artist", "Duration"])
        }
        .add_modifier(Modifier::BOLD);

        table = table.header(header);
    }

    table
}
