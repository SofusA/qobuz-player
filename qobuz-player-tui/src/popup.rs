use std::sync::Arc;

use qobuz_player_controls::client::Client;
use qobuz_player_models::{AlbumSimple, Playlist, Track};
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    widgets::*,
};
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{
    app::PlayOutcome,
    ui::{basic_list_table, block, center, mark_explicit_and_hifi, render_input, track_table},
};

#[derive(PartialEq)]
pub(crate) struct ArtistPopupState {
    pub artist_name: String,
    pub albums: Vec<AlbumSimple>,
    pub state: ListState,
}

pub(crate) struct PlaylistPopupState {
    pub playlist: Playlist,
    pub shuffle: bool,
    pub state: TableState,
    pub client: Arc<Client>,
}

pub(crate) struct TrackPopupState {
    pub playlists: Vec<Playlist>,
    pub track: Track,
    pub state: TableState,
    pub client: Arc<Client>,
}

pub(crate) struct NewPlaylistPopupState {
    pub name: Input,
    pub client: Arc<Client>,
}

pub(crate) enum Popup {
    Artist(ArtistPopupState),
    Playlist(PlaylistPopupState),
    Track(TrackPopupState),
    NewPlaylist(NewPlaylistPopupState),
}

impl Popup {
    pub(crate) fn render(&mut self, frame: &mut Frame) {
        match self {
            Popup::Artist(artist) => {
                let area = center(
                    frame.area(),
                    Constraint::Percentage(50),
                    Constraint::Length(artist.albums.len() as u16 + 2),
                );

                let list: Vec<ListItem> = artist
                    .albums
                    .iter()
                    .map(|album| {
                        ListItem::from(mark_explicit_and_hifi(
                            album.title.clone(),
                            album.explicit,
                            album.hires_available,
                        ))
                    })
                    .collect();

                let list = List::new(list)
                    .block(block(&artist.artist_name, false))
                    .highlight_style(Style::default().bg(Color::Blue))
                    .highlight_symbol(">")
                    .highlight_spacing(HighlightSpacing::Always);

                frame.render_widget(Clear, area);
                frame.render_stateful_widget(list, area, &mut artist.state);
            }
            Popup::Playlist(playlist_state) => {
                let area = center(
                    frame.area(),
                    Constraint::Percentage(75),
                    Constraint::Percentage(50),
                );

                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(1), Constraint::Length(3)])
                    .split(area);

                let tabs = Tabs::new(["Play", "Shuffle"])
                    .not_underlined()
                    .highlight_style(Style::default().bg(Color::Blue))
                    .select(if playlist_state.shuffle { 1 } else { 0 })
                    .divider(symbols::line::VERTICAL);

                let tracks = track_table(&playlist_state.playlist.tracks, None);

                let block = block(&playlist_state.playlist.title, false);

                frame.render_widget(Clear, area);
                block.render(area.outer(Margin::new(1, 1)), frame.buffer_mut());
                frame.render_stateful_widget(tracks, chunks[0], &mut playlist_state.state);
                frame.render_widget(tabs, chunks[1]);
            }
            Popup::Track(track_state) => {
                let area = center(
                    frame.area(),
                    Constraint::Percentage(75),
                    Constraint::Percentage(50),
                );

                let block_title = format!("Add {} to playlist", track_state.track.title);
                let playlists = basic_list_table(
                    track_state
                        .playlists
                        .iter()
                        .map(|playlist| Row::new(Line::from(playlist.title.clone())))
                        .collect::<Vec<_>>(),
                    &block_title,
                    true,
                );

                frame.render_widget(Clear, area);
                frame.render_stateful_widget(playlists, area, &mut track_state.state);
            }
            Popup::NewPlaylist(state) => {
                let area = center(
                    frame.area(),
                    Constraint::Percentage(75),
                    Constraint::Length(3),
                );

                frame.render_widget(Clear, area);
                render_input(&state.name, false, area, frame, "Create playlist");
            }
        };
    }

    pub(crate) async fn handle_event(&mut self, event: Event) -> Option<PlayOutcome> {
        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => match self {
                Popup::Artist(artist_popup_state) => match key_event.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        artist_popup_state.state.select_previous();
                        None
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        artist_popup_state.state.select_next();
                        None
                    }
                    KeyCode::Enter => {
                        let index = artist_popup_state.state.selected();
                        let id = index
                            .and_then(|index| artist_popup_state.albums.get(index))
                            .map(|album| album.id.clone());

                        if let Some(id) = id {
                            return Some(PlayOutcome::Album(id));
                        }

                        None
                    }
                    _ => None,
                },
                Popup::Playlist(playlist_popup_state) => match key_event.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        playlist_popup_state.state.select_previous();
                        None
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        playlist_popup_state.state.select_next();
                        None
                    }
                    KeyCode::Left | KeyCode::Char('h') => {
                        playlist_popup_state.shuffle = !playlist_popup_state.shuffle;
                        None
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        playlist_popup_state.shuffle = !playlist_popup_state.shuffle;
                        None
                    }
                    KeyCode::Char('u') => {
                        if let Some(index) = playlist_popup_state.state.selected() {
                            let playlist_track_id = playlist_popup_state
                                .playlist
                                .tracks
                                .get(index)
                                .and_then(|x| x.playlist_track_id)?;

                            _ = playlist_popup_state
                                .client
                                .update_playlist_track_position(
                                    index,
                                    playlist_popup_state.playlist.id,
                                    playlist_track_id,
                                )
                                .await;

                            if let Ok(updated_playlist) = playlist_popup_state
                                .client
                                .playlist(playlist_popup_state.playlist.id)
                                .await
                            {
                                playlist_popup_state.playlist = updated_playlist;
                                playlist_popup_state.state.select_previous();
                            };
                        }
                        None
                    }
                    KeyCode::Char('d') => {
                        if let Some(index) = playlist_popup_state.state.selected() {
                            let playlist_track_id = playlist_popup_state
                                .playlist
                                .tracks
                                .get(index)
                                .and_then(|x| x.playlist_track_id)?;

                            _ = playlist_popup_state
                                .client
                                .update_playlist_track_position(
                                    index + 3,
                                    playlist_popup_state.playlist.id,
                                    playlist_track_id,
                                )
                                .await;

                            if let Ok(updated_playlist) = playlist_popup_state
                                .client
                                .playlist(playlist_popup_state.playlist.id)
                                .await
                            {
                                playlist_popup_state.playlist = updated_playlist;
                                playlist_popup_state.state.select_next();
                            };
                        }
                        None
                    }
                    KeyCode::Char('D') => {
                        if let Some(playlist_track_id) = playlist_popup_state
                            .state
                            .selected()
                            .and_then(|index| playlist_popup_state.playlist.tracks.get(index))
                            .and_then(|t| t.playlist_track_id)
                        {
                            _ = playlist_popup_state
                                .client
                                .playlist_delete_track(
                                    playlist_popup_state.playlist.id,
                                    &[playlist_track_id],
                                )
                                .await;

                            if let Ok(updated_playlist) = playlist_popup_state
                                .client
                                .playlist(playlist_popup_state.playlist.id)
                                .await
                            {
                                playlist_popup_state.playlist = updated_playlist;
                            };
                        }
                        None
                    }
                    KeyCode::Char('a') => {
                        if let Some(index) = playlist_popup_state.state.selected() {
                            let track = playlist_popup_state.playlist.tracks.get(index)?;
                            return Some(PlayOutcome::AddTrackToPlaylist(track.clone()));
                        };
                        None
                    }
                    KeyCode::Enter => {
                        let id = playlist_popup_state.playlist.id;
                        let index = playlist_popup_state.state.selected().unwrap_or(0);
                        Some(PlayOutcome::Playlist((
                            id,
                            playlist_popup_state.shuffle,
                            index,
                        )))
                    }
                    _ => None,
                },
                Popup::Track(track_popup_state) => match key_event.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        track_popup_state.state.select_previous();
                        None
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        track_popup_state.state.select_next();
                        None
                    }
                    KeyCode::Enter => {
                        let index = track_popup_state.state.selected();
                        let id = index
                            .and_then(|index| track_popup_state.playlists.get(index))
                            .map(|p| p.id);

                        if let Some(id) = id {
                            _ = track_popup_state
                                .client
                                .playlist_add_track(id, &[track_popup_state.track.id])
                                .await;
                            return Some(PlayOutcome::Consumed);
                        }

                        None
                    }
                    _ => None,
                },
                Popup::NewPlaylist(state) => match key_event.code {
                    KeyCode::Enter => {
                        let input = state.name.value();
                        match state
                            .client
                            .create_playlist(input.to_string(), false, Default::default(), None)
                            .await
                        {
                            Ok(_) => Some(PlayOutcome::Consumed),
                            Err(_) => None,
                        }
                    }
                    _ => {
                        state.name.handle_event(&event);
                        None
                    }
                },
            },
            _ => None,
        }
    }
}
