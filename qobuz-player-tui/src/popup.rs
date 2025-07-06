use crossterm::event::KeyCode;
use qobuz_player_controls::models::AlbumSimple;
use ratatui::{prelude::*, widgets::*};

use crate::ui::{block, center};

#[derive(PartialEq)]
pub(crate) struct ArtistPopupState {
    pub artist_name: String,
    pub albums: Vec<AlbumSimple>,
    pub state: ListState,
}

#[derive(PartialEq)]
pub(crate) struct PlaylistPopupState {
    pub playlist_name: String,
    pub playlist_id: u32,
    pub shuffle: bool,
}

#[derive(PartialEq)]
pub(crate) enum Popup {
    Artist(ArtistPopupState),
    Playlist(PlaylistPopupState),
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
                    .map(|album| ListItem::from(Line::from(album.title.clone())))
                    .collect();

                let list = List::new(list)
                    .block(block(&artist.artist_name))
                    .highlight_style(Style::default().bg(Color::Blue))
                    .highlight_symbol(">")
                    .highlight_spacing(HighlightSpacing::Always);

                frame.render_widget(Clear, area);
                frame.render_stateful_widget(list, area, &mut artist.state);
            }
            Popup::Playlist(playlist) => {
                let area = center(frame.area(), Constraint::Length(18), Constraint::Length(3));
                let tabs = Tabs::new(["Play", "Shuffle"])
                    .block(block(&playlist.playlist_name))
                    .not_underlined()
                    .highlight_style(Style::default().bg(Color::Blue))
                    .select(if playlist.shuffle { 1 } else { 0 })
                    .divider(symbols::line::VERTICAL);

                frame.render_widget(Clear, area);
                frame.render_widget(tabs, area);
            }
        };
    }

    pub(crate) async fn handle_event(&mut self, key: KeyCode) -> bool {
        match self {
            Popup::Artist(artist_popup_state) => match key {
                KeyCode::Up => {
                    artist_popup_state.state.select_previous();
                    false
                }
                KeyCode::Down => {
                    artist_popup_state.state.select_next();
                    false
                }
                KeyCode::Enter => {
                    let index = artist_popup_state.state.selected();

                    let id = index
                        .map(|index| &artist_popup_state.albums[index])
                        .map(|album| album.id.clone());

                    if let Some(id) = id {
                        qobuz_player_controls::play_album(&id, 0).await.unwrap();
                        return true;
                    }
                    false
                }
                _ => false,
            },
            Popup::Playlist(playlist_popup_state) => match key {
                KeyCode::Left => {
                    playlist_popup_state.shuffle = !playlist_popup_state.shuffle;
                    false
                }
                KeyCode::Right => {
                    playlist_popup_state.shuffle = !playlist_popup_state.shuffle;
                    false
                }
                KeyCode::Enter => {
                    let id = playlist_popup_state.playlist_id;

                    qobuz_player_controls::play_playlist(id, 0, playlist_popup_state.shuffle)
                        .await
                        .unwrap();
                    true
                }
                _ => false,
            },
        }
    }
}
