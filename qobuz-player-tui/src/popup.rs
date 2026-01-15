use std::sync::Arc;

use qobuz_player_controls::client::Client;
use qobuz_player_models::{Album, AlbumSimple, Playlist, Track};
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    widgets::*,
};
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{
    app::{Output, PlayOutcome},
    ui::{
        basic_list_table, block, center, centered_rect_fixed, mark_explicit_and_hifi, render_input,
        track_table,
    },
};

pub(crate) struct ArtistPopupState {
    pub artist_name: String,
    pub albums: Vec<AlbumSimple>,
    pub state: ListState,
    pub client: Arc<Client>,
}

pub(crate) struct AlbumPopupState {
    pub album: Album,
    pub state: TableState,
}

impl AlbumPopupState {
    pub fn new(album: Album) -> Self {
        let is_empty = album.tracks.is_empty();
        let mut state = Self {
            album,
            state: Default::default(),
        };

        if !is_empty {
            state.state.select(Some(0));
        }
        state
    }
}

pub(crate) struct PlaylistPopupState {
    pub playlist: Playlist,
    pub shuffle: bool,
    pub state: TableState,
    pub client: Arc<Client>,
}

pub(crate) struct DeletePlaylistPopupstate {
    pub title: String,
    pub id: u32,
    pub confirm: bool,
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
    Album(AlbumPopupState),
    Playlist(PlaylistPopupState),
    Track(TrackPopupState),
    NewPlaylist(NewPlaylistPopupState),
    DeletePlaylist(DeletePlaylistPopupstate),
}

impl Popup {
    pub(crate) fn render(&mut self, frame: &mut Frame) {
        match self {
            Popup::Album(state) => {
                let area = center(
                    frame.area(),
                    Constraint::Percentage(50),
                    Constraint::Length(state.album.tracks.len() as u16 + 2),
                );

                let table = basic_list_table(
                    state
                        .album
                        .tracks
                        .iter()
                        .map(|track| {
                            Row::new(vec![mark_explicit_and_hifi(
                                track.title.clone(),
                                track.explicit,
                                track.hires_available,
                            )])
                        })
                        .collect(),
                    &state.album.title,
                    false,
                );

                frame.render_widget(Clear, area);
                frame.render_stateful_widget(table, area, &mut state.state);
            }
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
                let visible_rows = playlist_state.playlist.tracks.len().min(15) as u16;

                let inner_content_height = visible_rows + 2;
                let block_border_height = 2;

                let popup_height = (inner_content_height + block_border_height)
                    .clamp(4, frame.area().height.saturating_sub(2));

                let popup_width = (frame.area().width * 75 / 100).max(30);

                let area = centered_rect_fixed(popup_width, popup_height, frame.area());

                let buttons = Tabs::new(["Play", "Shuffle"])
                    .not_underlined()
                    .highlight_style(Style::default().bg(Color::Blue))
                    .select(if playlist_state.shuffle { 1 } else { 0 });

                let tracks = track_table(&playlist_state.playlist.tracks, None);

                let block = block(&playlist_state.playlist.title, false);

                frame.render_widget(Clear, area);

                let inner = block.inner(area);
                frame.render_widget(block, area);

                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Min(1),
                        Constraint::Length(1),
                        Constraint::Length(1),
                    ])
                    .split(inner);

                frame.render_stateful_widget(tracks, chunks[0], &mut playlist_state.state);
                frame.render_widget(buttons, chunks[2]);
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
            Popup::DeletePlaylist(state) => {
                let block_title = format!("Delete {}?", state.title);
                let area = center(
                    frame.area(),
                    Constraint::Length(block_title.len() as u16 + 6),
                    Constraint::Length(3),
                );
                let tabs = Tabs::new(["Delete", "Cancel"])
                    .block(block(&block_title, false))
                    .not_underlined()
                    .highlight_style(Style::default().bg(Color::Blue))
                    .select(if state.confirm { 0 } else { 1 })
                    .divider(symbols::line::VERTICAL);

                frame.render_widget(Clear, area);
                frame.render_widget(tabs, area);
            }
        };
    }

    pub(crate) async fn handle_event(&mut self, event: Event) -> Output {
        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => match self {
                Popup::Album(album_state) => match key_event.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        album_state.state.select_previous();
                        Output::Consumed
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        album_state.state.select_next();
                        Output::Consumed
                    }
                    KeyCode::Enter => {
                        let index = album_state.state.selected();

                        if let Some(index) = index {
                            return Output::PlayOutcome(PlayOutcome::Album(
                                album_state.album.id.clone(),
                                index,
                            ));
                        }

                        Output::PopPopup
                    }
                    _ => Output::Consumed,
                },
                Popup::Artist(artist_popup_state) => match key_event.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        artist_popup_state.state.select_previous();
                        Output::Consumed
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        artist_popup_state.state.select_next();
                        Output::Consumed
                    }
                    KeyCode::Enter => {
                        let index = artist_popup_state.state.selected();
                        let id = index
                            .and_then(|index| artist_popup_state.albums.get(index))
                            .map(|album| album.id.clone());

                        if let Some(id) = id {
                            let album = artist_popup_state.client.album(&id).await;
                            match album {
                                Ok(album) => {
                                    return Output::Popup(Popup::Album(AlbumPopupState::new(
                                        album,
                                    )));
                                }
                                Err(err) => return Output::Error(err.to_string()),
                            }
                        }

                        Output::PopPopup
                    }
                    _ => Output::Consumed,
                },
                Popup::Playlist(playlist_popup_state) => match key_event.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        playlist_popup_state.state.select_previous();
                        Output::Consumed
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        playlist_popup_state.state.select_next();
                        Output::Consumed
                    }
                    KeyCode::Left | KeyCode::Char('h') => {
                        playlist_popup_state.shuffle = !playlist_popup_state.shuffle;
                        Output::Consumed
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        playlist_popup_state.shuffle = !playlist_popup_state.shuffle;
                        Output::Consumed
                    }
                    KeyCode::Char('u') => {
                        if let Some(index) = playlist_popup_state.state.selected() {
                            let playlist_track_id = playlist_popup_state
                                .playlist
                                .tracks
                                .get(index)
                                .and_then(|x| x.playlist_track_id)
                                .expect("infallible");

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
                        Output::Consumed
                    }
                    KeyCode::Char('d') => {
                        if let Some(index) = playlist_popup_state.state.selected() {
                            let playlist_track_id = playlist_popup_state
                                .playlist
                                .tracks
                                .get(index)
                                .and_then(|x| x.playlist_track_id)
                                .expect("infallible");

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
                        Output::Consumed
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
                        Output::Consumed
                    }
                    KeyCode::Char('a') => {
                        if let Some(index) = playlist_popup_state.state.selected() {
                            let track = playlist_popup_state
                                .playlist
                                .tracks
                                .get(index)
                                .expect("infallible");

                            return Output::PlayOutcome(PlayOutcome::AddTrackToPlaylist(
                                track.clone(),
                            ));
                        };
                        Output::Consumed
                    }
                    KeyCode::Enter => {
                        let id = playlist_popup_state.playlist.id;
                        let index = playlist_popup_state.state.selected().unwrap_or(0);
                        Output::PlayOutcome(PlayOutcome::Playlist((
                            id,
                            playlist_popup_state.shuffle,
                            index,
                        )))
                    }
                    _ => Output::Consumed,
                },
                Popup::Track(track_popup_state) => match key_event.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        track_popup_state.state.select_previous();
                        Output::Consumed
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        track_popup_state.state.select_next();
                        Output::Consumed
                    }
                    KeyCode::Enter => {
                        let index = track_popup_state.state.selected();
                        let id = index
                            .and_then(|index| track_popup_state.playlists.get(index))
                            .map(|p| p.id);

                        if let Some(id) = id {
                            match track_popup_state
                                .client
                                .playlist_add_track(id, &[track_popup_state.track.id])
                                .await
                            {
                                Ok(_) => return Output::PopPopup,
                                Err(err) => return Output::Error(err.to_string()),
                            };
                        }

                        Output::PopPopup
                    }
                    _ => Output::Consumed,
                },
                Popup::NewPlaylist(state) => match key_event.code {
                    KeyCode::Enter => {
                        let input = state.name.value();
                        match state
                            .client
                            .create_playlist(input.to_string(), false, Default::default(), None)
                            .await
                        {
                            Ok(_) => Output::PopPoputUpdateFavorites,
                            Err(err) => Output::Error(err.to_string()),
                        }
                    }
                    _ => {
                        state.name.handle_event(&event);
                        Output::Consumed
                    }
                },
                Popup::DeletePlaylist(state) => match key_event.code {
                    KeyCode::Enter => {
                        if state.confirm {
                            match state.client.delete_playlist(state.id).await {
                                Ok(_) => return Output::PopPoputUpdateFavorites,
                                Err(err) => {
                                    return Output::Error(err.to_string());
                                }
                            }
                        }

                        Output::PopPoputUpdateFavorites
                    }
                    KeyCode::Left | KeyCode::Right => {
                        state.confirm = !state.confirm;
                        Output::Consumed
                    }
                    _ => Output::Consumed,
                },
            },
            _ => Output::Consumed,
        }
    }
}
