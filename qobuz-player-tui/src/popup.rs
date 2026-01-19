use std::sync::Arc;

use qobuz_player_controls::{Result, client::Client};
use qobuz_player_models::{Album, AlbumSimple, Artist, Playlist, Track};
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    widgets::*,
};
use tokio::try_join;
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{
    app::{Output, PlayOutcome, QueueOutcome},
    ui::{
        basic_list_table, block, center, centered_rect_fixed, mark_explicit_and_hifi, render_input,
        tab_bar, track_table,
    },
};

pub(crate) struct ArtistPopupState {
    pub artist_name: String,
    pub id: u32,
    pub albums: Vec<AlbumSimple>,
    pub state: TableState,
    pub show_top_track: bool,
    pub top_tracks: Vec<Track>,
    pub top_track_state: TableState,
    pub client: Arc<Client>,
}

impl ArtistPopupState {
    pub async fn new(artist: &Artist, client: Arc<Client>) -> Result<Self> {
        let id = artist.id;
        let (artist_page, artist_albums) =
            try_join!(client.artist_page(id), client.artist_albums(id))?;

        let is_album_empty = artist_albums.is_empty();
        let is_top_tracks_empty = artist_page.top_tracks.is_empty();

        let mut state = Self {
            artist_name: artist.name.clone(),
            id,
            albums: artist_albums,
            state: Default::default(),
            show_top_track: false,
            top_tracks: artist_page.top_tracks,
            top_track_state: Default::default(),
            client,
        };

        if !is_album_empty {
            state.state.select(Some(0));
        }
        if !is_top_tracks_empty {
            state.state.select(Some(0));
        }

        Ok(state)
    }
}

pub(crate) struct AlbumPopupState {
    pub album: Album,
    pub state: TableState,
    pub client: Arc<Client>,
}

impl AlbumPopupState {
    pub fn new(album: Album, client: Arc<Client>) -> Self {
        let is_empty = album.tracks.is_empty();
        let mut state = Self {
            album,
            state: Default::default(),
            client,
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
                    Some(&state.album.title),
                );

                frame.render_widget(Clear, area);
                frame.render_stateful_widget(table, area, &mut state.state);
            }
            Popup::Artist(artist) => {
                let max_visible_rows: u16 = 15;
                let album_rows = (artist.albums.len() as u16).min(max_visible_rows);
                let top_track_rows = (artist.top_tracks.len() as u16).min(max_visible_rows);
                let visible_rows = if artist.show_top_track {
                    top_track_rows
                } else {
                    album_rows
                };

                let tabs_height: u16 = 2;
                let border_height: u16 = 2;
                let min_height: u16 = 4;

                let popup_height = (visible_rows + border_height + tabs_height)
                    .clamp(min_height, frame.area().height.saturating_sub(2));

                let popup_width = (frame.area().width * 75 / 100).max(30);

                let area = centered_rect_fixed(popup_width, popup_height, frame.area());

                let outer_block = block(Some(&artist.artist_name));

                let tabs = tab_bar(
                    ["Albums", "Top Tracks"].into(),
                    if artist.show_top_track { 1 } else { 0 },
                );

                let top_tracks = track_table(&artist.top_tracks, None);

                let list = basic_list_table(
                    artist
                        .albums
                        .iter()
                        .map(|album| Row::new(Line::from(album.title.clone())))
                        .collect(),
                    None,
                );

                frame.render_widget(Clear, area);
                frame.render_widget(&outer_block, area);

                let inner = outer_block.inner(area);

                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(tabs_height), Constraint::Min(1)])
                    .split(inner);

                frame.render_widget(tabs, chunks[0]);

                if artist.show_top_track {
                    frame.render_stateful_widget(
                        top_tracks,
                        chunks[1],
                        &mut artist.top_track_state,
                    );
                } else {
                    frame.render_stateful_widget(list, chunks[1], &mut artist.state);
                }
            }
            Popup::Playlist(playlist_state) => {
                let visible_rows = playlist_state.playlist.tracks.len().min(15) as u16;

                let inner_content_height = visible_rows + 2;
                let block_border_height = 2;

                let popup_height = (inner_content_height + block_border_height)
                    .clamp(4, frame.area().height.saturating_sub(2));

                let popup_width = (frame.area().width * 75 / 100).max(30);

                let area = centered_rect_fixed(popup_width, popup_height, frame.area());

                let buttons = tab_bar(
                    ["Play", "Shuffle"].into(),
                    if playlist_state.shuffle { 1 } else { 0 },
                );

                let tracks = track_table(&playlist_state.playlist.tracks, None);

                let block = block(Some(&playlist_state.playlist.title));

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
                    Some(&block_title),
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
                    Constraint::Length(block_title.chars().count() as u16 + 6),
                    Constraint::Length(3),
                );

                let tabs = tab_bar(
                    ["Delete", "Cancel"].into(),
                    if state.confirm { 0 } else { 1 },
                )
                .block(block(Some(&block_title)));

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
                    KeyCode::Char('N') => {
                        let index = album_state.state.selected();
                        let selected = index.and_then(|index| album_state.album.tracks.get(index));

                        let Some(selected) = selected else {
                            return Output::Consumed;
                        };

                        Output::Queue(QueueOutcome::PlayTrackNext(selected.id))
                    }
                    KeyCode::Char('B') => {
                        let index = album_state.state.selected();
                        let selected = index.and_then(|index| album_state.album.tracks.get(index));

                        let Some(selected) = selected else {
                            return Output::Consumed;
                        };

                        Output::Queue(QueueOutcome::AddTrackToQueue(selected.id))
                    }
                    KeyCode::Char('A') => {
                        let index = album_state.state.selected();

                        let id = index
                            .and_then(|index| album_state.album.tracks.get(index))
                            .map(|track| track.id);

                        if let Some(id) = id {
                            _ = album_state.client.add_favorite_track(id).await;
                            return Output::UpdateFavorites;
                        }

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
                        match artist_popup_state.show_top_track {
                            true => artist_popup_state.top_track_state.select_previous(),
                            false => artist_popup_state.state.select_previous(),
                        }
                        Output::Consumed
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        match artist_popup_state.show_top_track {
                            true => artist_popup_state.top_track_state.select_next(),
                            false => artist_popup_state.state.select_next(),
                        }
                        Output::Consumed
                    }
                    KeyCode::Left | KeyCode::Char('h') | KeyCode::Right | KeyCode::Char('l') => {
                        artist_popup_state.show_top_track = !artist_popup_state.show_top_track;
                        Output::Consumed
                    }
                    KeyCode::Char('A') => {
                        let index = artist_popup_state.state.selected();

                        let id = index
                            .and_then(|index| artist_popup_state.albums.get(index))
                            .map(|album| album.id.clone());

                        if let Some(id) = id {
                            _ = artist_popup_state.client.add_favorite_album(&id).await;
                            return Output::UpdateFavorites;
                        }

                        Output::Consumed
                    }
                    KeyCode::Enter => {
                        match artist_popup_state.show_top_track {
                            true => {
                                let index = artist_popup_state.top_track_state.selected();
                                if let Some(index) = index {
                                    return Output::PlayOutcome(PlayOutcome::TopTracks(
                                        artist_popup_state.id,
                                        index,
                                    ));
                                }
                            }
                            false => {
                                let index = artist_popup_state.state.selected();
                                let id = index
                                    .and_then(|index| artist_popup_state.albums.get(index))
                                    .map(|album| album.id.clone());

                                if let Some(id) = id {
                                    let album = artist_popup_state.client.album(&id).await;
                                    match album {
                                        Ok(album) => {
                                            return Output::Popup(Popup::Album(
                                                AlbumPopupState::new(
                                                    album,
                                                    artist_popup_state.client.clone(),
                                                ),
                                            ));
                                        }
                                        Err(err) => return Output::Error(err.to_string()),
                                    }
                                }
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
