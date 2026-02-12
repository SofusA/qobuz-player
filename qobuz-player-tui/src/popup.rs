use qobuz_player_controls::{AppResult, client::Client, controls::Controls};
use qobuz_player_models::{Album, Artist, Playlist, PlaylistSimple, Track};
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    widgets::*,
};
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{
    app::{NotificationList, Output},
    ui::{block, center, centered_rect_fixed, render_input, tab_bar},
    widgets::{
        album_list::AlbumList,
        playlist_list::PlaylistList,
        track_list::{TrackList, TrackListEvent},
    },
};

pub struct ArtistPopupState {
    artist_name: String,
    albums: AlbumList,
    singles: AlbumList,
    live: AlbumList,
    compilations: AlbumList,
    selected_sub_tab: usize,
    top_tracks: TrackList,
    id: u32,
}

enum SelectedArtistPopupSubtabMut<'a> {
    Albums(&'a mut AlbumList),
    TopTracks(&'a mut TrackList),
}

enum SelectedArtistPopupSubtab<'a> {
    Albums(&'a AlbumList),
    TopTracks(&'a TrackList),
}

struct Tab<'a> {
    name: &'a str,
    is_empty: bool,
}

enum TabKind {
    Albums,
    TopTracks,
    Singles,
    Live,
    Compilations,
}

impl ArtistPopupState {
    pub async fn new(artist: &Artist, client: &Client) -> AppResult<Self> {
        let id = artist.id;
        let artist_page = client.artist_page(id).await?;

        let is_album_empty = artist_page.albums.is_empty();
        let is_top_tracks_empty = artist_page.top_tracks.is_empty();

        let mut state = Self {
            artist_name: artist.name.clone(),
            albums: AlbumList::new(artist_page.albums),
            singles: AlbumList::new(artist_page.singles),
            live: AlbumList::new(artist_page.live),
            compilations: AlbumList::new(artist_page.compilations),
            selected_sub_tab: 0,
            top_tracks: TrackList::new(artist_page.top_tracks),
            id: artist.id,
        };

        if !is_album_empty {
            state.albums.select_first();
        }
        if !is_top_tracks_empty {
            state.top_tracks.select_first();
        }

        Ok(state)
    }

    fn cycle_subtab_backwards(&mut self) {
        let count = self.tabs().len();
        self.selected_sub_tab = (self.selected_sub_tab + count - 1) % count;
    }

    fn cycle_subtab(&mut self) {
        let count = self.tabs().len();
        self.selected_sub_tab = (self.selected_sub_tab + count + 1) % count;
    }

    fn visible_tab_kinds(&self) -> Vec<TabKind> {
        let mut tabs = vec![];

        if !self.albums.filter().is_empty() {
            tabs.push(TabKind::Albums);
        }
        if !self.top_tracks.filter().is_empty() {
            tabs.push(TabKind::TopTracks);
        }
        if !self.singles.filter().is_empty() {
            tabs.push(TabKind::Singles);
        }
        if !self.live.filter().is_empty() {
            tabs.push(TabKind::Live);
        }
        if !self.compilations.filter().is_empty() {
            tabs.push(TabKind::Compilations);
        }

        tabs
    }

    fn current_state_mut(&'_ mut self) -> Option<SelectedArtistPopupSubtabMut<'_>> {
        let visible_tabs = self.visible_tab_kinds();
        let tab = visible_tabs.get(self.selected_sub_tab)?;

        match tab {
            TabKind::Albums => Some(SelectedArtistPopupSubtabMut::Albums(&mut self.albums)),
            TabKind::TopTracks => Some(SelectedArtistPopupSubtabMut::TopTracks(
                &mut self.top_tracks,
            )),
            TabKind::Singles => Some(SelectedArtistPopupSubtabMut::Albums(&mut self.singles)),
            TabKind::Live => Some(SelectedArtistPopupSubtabMut::Albums(&mut self.live)),
            TabKind::Compilations => {
                Some(SelectedArtistPopupSubtabMut::Albums(&mut self.compilations))
            }
        }
    }

    fn current_state(&self) -> Option<SelectedArtistPopupSubtab<'_>> {
        let visible_tabs = self.visible_tab_kinds();
        let tab = visible_tabs.get(self.selected_sub_tab)?;

        match tab {
            TabKind::Albums => Some(SelectedArtistPopupSubtab::Albums(&self.albums)),
            TabKind::TopTracks => Some(SelectedArtistPopupSubtab::TopTracks(&self.top_tracks)),
            TabKind::Singles => Some(SelectedArtistPopupSubtab::Albums(&self.singles)),
            TabKind::Live => Some(SelectedArtistPopupSubtab::Albums(&self.live)),
            TabKind::Compilations => Some(SelectedArtistPopupSubtab::Albums(&self.compilations)),
        }
    }

    fn current_row_count(&self) -> usize {
        let current_state = self.current_state();
        match current_state {
            Some(state) => match state {
                SelectedArtistPopupSubtab::Albums(album_list) => album_list.filter().len(),
                SelectedArtistPopupSubtab::TopTracks(track_list) => track_list.filter().len(),
            },
            None => 0,
        }
    }

    fn tabs(&self) -> Vec<&str> {
        vec![
            Tab {
                name: "Albums",
                is_empty: self.albums.filter().is_empty(),
            },
            Tab {
                name: "Top Tracks",
                is_empty: self.top_tracks.filter().is_empty(),
            },
            Tab {
                name: "Singles",
                is_empty: self.singles.filter().is_empty(),
            },
            Tab {
                name: "Live",
                is_empty: self.live.filter().is_empty(),
            },
            Tab {
                name: "Compilations",
                is_empty: self.compilations.filter().is_empty(),
            },
        ]
        .into_iter()
        .filter(|t| !t.is_empty)
        .map(|x| x.name)
        .collect()
    }
}

pub struct AlbumPopupState {
    title: String,
    tracks: TrackList,
    id: String,
}

impl AlbumPopupState {
    pub fn new(album: Album) -> Self {
        let is_empty = album.tracks.is_empty();
        let mut state = Self {
            title: album.title,
            tracks: TrackList::new(album.tracks),
            id: album.id,
        };

        if !is_empty {
            state.tracks.select_first();
        }
        state
    }
}

pub struct PlaylistPopupState {
    shuffle: bool,
    tracks: TrackList,
    title: String,
    id: u32,
}

impl PlaylistPopupState {
    pub fn new(playlist: Playlist) -> Self {
        let is_empty = playlist.tracks.is_empty();
        let mut state = Self {
            tracks: TrackList::new(playlist.tracks),
            title: playlist.title,
            shuffle: false,
            id: playlist.id,
        };

        if !is_empty {
            state.tracks.select_first();
        }
        state
    }
}

pub struct DeletePlaylistPopupstate {
    title: String,
    id: u32,
    confirm: bool,
}

impl DeletePlaylistPopupstate {
    pub fn new(playlist: PlaylistSimple) -> Self {
        Self {
            title: playlist.title,
            id: playlist.id,
            confirm: false,
        }
    }
}

pub struct TrackPopupState {
    playlists: PlaylistList,
    track: Track,
}

impl TrackPopupState {
    pub fn new(track: Track, owned_playlists: Vec<PlaylistSimple>) -> Self {
        Self {
            playlists: PlaylistList::new(owned_playlists),
            track,
        }
    }

    fn select_next(&mut self) {
        self.playlists.select_next();
    }

    fn select_previous(&mut self) {
        self.playlists.select_previous();
    }
}

pub struct NewPlaylistPopupState {
    name: Input,
}

impl NewPlaylistPopupState {
    pub fn new() -> Self {
        Self {
            name: Default::default(),
        }
    }
}

#[allow(clippy::large_enum_variant)]
pub enum Popup {
    Artist(ArtistPopupState),
    Album(AlbumPopupState),
    Playlist(PlaylistPopupState),
    Track(TrackPopupState),
    NewPlaylist(NewPlaylistPopupState),
    DeletePlaylist(DeletePlaylistPopupstate),
}

impl Popup {
    pub fn render(&mut self, frame: &mut Frame) {
        match self {
            Popup::Album(state) => {
                let area = center(
                    frame.area(),
                    Constraint::Percentage(50),
                    Constraint::Length(state.tracks.filter().len() as u16 + 2),
                );

                let block = block(Some(&state.title));

                frame.render_widget(Clear, area);
                frame.render_widget(&block, area);
                state
                    .tracks
                    .render(block.inner(area), frame.buffer_mut(), false);
            }
            Popup::Artist(artist) => {
                let visible_rows = (artist.current_row_count() + 1).min(15) as u16;

                let tabs_height: u16 = 2;
                let border_height: u16 = 2;
                let min_height: u16 = 4;

                let popup_height = (visible_rows + border_height + tabs_height)
                    .clamp(min_height, frame.area().height.saturating_sub(2));

                let popup_width = (frame.area().width * 75 / 100).max(30);

                let area = centered_rect_fixed(popup_width, popup_height, frame.area());

                let outer_block = block(Some(&artist.artist_name));

                let tabs = tab_bar(artist.tabs(), artist.selected_sub_tab);

                frame.render_widget(Clear, area);
                frame.render_widget(&outer_block, area);

                let inner = outer_block.inner(area);

                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(tabs_height), Constraint::Min(1)])
                    .split(inner);

                frame.render_widget(tabs, chunks[0]);

                if let Some(state) = artist.current_state_mut() {
                    match state {
                        SelectedArtistPopupSubtabMut::Albums(album_list) => {
                            album_list.render(chunks[1], frame.buffer_mut())
                        }
                        SelectedArtistPopupSubtabMut::TopTracks(track_list) => {
                            track_list.render(chunks[1], frame.buffer_mut(), true)
                        }
                    }
                }
            }
            Popup::Playlist(playlist_state) => {
                let visible_rows = playlist_state.tracks.filter().len().min(15) as u16;

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

                let block = block(Some(&playlist_state.title));

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

                playlist_state
                    .tracks
                    .render(chunks[0], frame.buffer_mut(), true);
                frame.render_widget(buttons, chunks[2]);
            }
            Popup::Track(track_state) => {
                let area = center(
                    frame.area(),
                    Constraint::Percentage(75),
                    Constraint::Percentage(50),
                );

                let block_title = format!("Add {} to playlist", track_state.track.title);
                let block = block(Some(&block_title));

                frame.render_widget(Clear, area);
                frame.render_widget(&block, area);
                track_state
                    .playlists
                    .render(block.inner(area), frame.buffer_mut());
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

    pub async fn handle_event(
        &mut self,
        event: Event,
        client: &Client,
        controls: &Controls,
        notifications: &mut NotificationList,
    ) -> AppResult<Output> {
        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => match self {
                Popup::Album(album_state) => {
                    album_state
                        .tracks
                        .handle_events(
                            key_event.code,
                            client,
                            controls,
                            notifications,
                            TrackListEvent::Album(album_state.id.clone()),
                        )
                        .await
                }
                Popup::Artist(artist_popup_state) => match key_event.code {
                    KeyCode::Left | KeyCode::Char('h') => {
                        artist_popup_state.cycle_subtab_backwards();
                        Ok(Output::Consumed)
                    }

                    KeyCode::Right | KeyCode::Char('l') => {
                        artist_popup_state.cycle_subtab();
                        Ok(Output::Consumed)
                    }
                    _ => {
                        let artist_id = artist_popup_state.id;
                        let current_state = artist_popup_state.current_state_mut();
                        match current_state {
                            Some(state) => match state {
                                SelectedArtistPopupSubtabMut::Albums(album_list) => {
                                    album_list
                                        .handle_events(key_event.code, client, notifications)
                                        .await
                                }
                                SelectedArtistPopupSubtabMut::TopTracks(track_list) => {
                                    track_list
                                        .handle_events(
                                            key_event.code,
                                            client,
                                            controls,
                                            notifications,
                                            TrackListEvent::Artist(artist_id),
                                        )
                                        .await
                                }
                            },
                            None => Ok(Output::Consumed),
                        }
                    }
                },
                Popup::Playlist(playlist_popup_state) => match key_event.code {
                    KeyCode::Left | KeyCode::Char('h') | KeyCode::Right | KeyCode::Char('l') => {
                        playlist_popup_state.shuffle = !playlist_popup_state.shuffle;
                        Ok(Output::Consumed)
                    }
                    _ => {
                        playlist_popup_state
                            .tracks
                            .handle_events(
                                key_event.code,
                                client,
                                controls,
                                notifications,
                                TrackListEvent::Playlist(
                                    playlist_popup_state.id,
                                    playlist_popup_state.shuffle,
                                ),
                            )
                            .await
                    }
                },
                Popup::Track(track_popup_state) => match key_event.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        track_popup_state.select_previous();
                        Ok(Output::Consumed)
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        track_popup_state.select_next();
                        Ok(Output::Consumed)
                    }
                    KeyCode::Enter => {
                        let index = track_popup_state.playlists.selected();
                        let id = index
                            .and_then(|index| track_popup_state.playlists.get(index))
                            .map(|p| p.id);

                        if let Some(id) = id {
                            return Ok(Output::AddTrackToPlaylistAndPopPopup((
                                track_popup_state.track.id,
                                id,
                            )));
                        }

                        Ok(Output::Consumed)
                    }
                    _ => Ok(Output::NotConsumed),
                },
                Popup::NewPlaylist(state) => match key_event.code {
                    KeyCode::Enter => {
                        let input = state.name.value();
                        client
                            .create_playlist(input.to_string(), false, Default::default(), None)
                            .await?;
                        Ok(Output::PopPopupUpdateFavorites)
                    }
                    _ => {
                        state.name.handle_event(&event);
                        Ok(Output::Consumed)
                    }
                },
                Popup::DeletePlaylist(state) => match key_event.code {
                    KeyCode::Enter => {
                        if state.confirm {
                            client.delete_playlist(state.id).await?;
                            return Ok(Output::PopPopupUpdateFavorites);
                        }

                        Ok(Output::PopPopupUpdateFavorites)
                    }
                    KeyCode::Left | KeyCode::Right => {
                        state.confirm = !state.confirm;
                        Ok(Output::Consumed)
                    }
                    _ => Ok(Output::Consumed),
                },
            },
            _ => Ok(Output::Consumed),
        }
    }
}
