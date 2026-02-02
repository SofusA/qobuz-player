use qobuz_player_controls::Result;
use qobuz_player_controls::client::Client;
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
};
use tokio::try_join;

use crate::{
    app::{NotificationList, Output},
    ui::{block, tab_bar},
    widgets::{album_list::AlbumList, playlist_list::PlaylistList},
};

pub struct DiscoverState {
    featured_albums: Vec<(String, AlbumList)>,
    featured_playlists: Vec<(String, PlaylistList)>,
    selected_sub_tab: usize,
}

impl DiscoverState {
    pub async fn new(client: &Client) -> Result<Self> {
        let (featured_albums, featured_playlists) =
            try_join!(client.featured_albums(), client.featured_playlists(),)?;

        let featured_albums = featured_albums
            .into_iter()
            .map(|x| (x.0, AlbumList::new(x.1)))
            .collect();

        let featured_playlists = featured_playlists
            .into_iter()
            .map(|x| (x.0, PlaylistList::new(x.1)))
            .collect();

        Ok(Self {
            featured_albums,
            featured_playlists,
            selected_sub_tab: 0,
        })
    }
}

impl DiscoverState {
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let block = block(None);
        frame.render_widget(block, area);

        let tab_content_area = area.inner(Margin::new(1, 1));

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(1)])
            .split(tab_content_area);

        let album_labels: Vec<_> = self
            .featured_albums
            .iter()
            .map(|fa| fa.0.as_str())
            .collect();
        let playlist_labels: Vec<_> = self
            .featured_playlists
            .iter()
            .map(|fp| fp.0.as_str())
            .collect();
        let labels = [album_labels, playlist_labels].concat();

        let tabs = tab_bar(labels, self.selected_sub_tab);
        frame.render_widget(tabs, chunks[0]);

        let is_album = self.album_selected();

        match is_album {
            true => {
                let list_state = &mut self.featured_albums[self.selected_sub_tab];
                list_state.1.render(chunks[1], frame.buffer_mut());
            }
            false => {
                let list_state = &mut self.featured_playlists
                    [self.selected_sub_tab - self.featured_albums.len()];

                list_state.1.render(chunks[1], frame.buffer_mut());
            }
        };
    }

    pub async fn handle_events(
        &mut self,
        event: Event,
        client: &Client,
        notifications: &mut NotificationList,
    ) -> Result<Output> {
        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match key_event.code {
                    KeyCode::Left | KeyCode::Char('h') => {
                        self.cycle_subtab_backwards();
                        Ok(Output::Consumed)
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        self.cycle_subtab();
                        Ok(Output::Consumed)
                    }
                    _ => {
                        let is_album = self.album_selected();

                        match is_album {
                            true => {
                                return self.featured_albums[self.selected_sub_tab]
                                    .1
                                    .handle_events(key_event.code, client, notifications)
                                    .await;
                            }
                            false => {
                                return self.featured_playlists
                                    [self.selected_sub_tab - self.featured_albums.len()]
                                .1
                                .handle_events(key_event.code, client, notifications)
                                .await;
                            }
                        }
                    }
                }
            }
            _ => Ok(Output::NotConsumed),
        }
    }

    fn album_selected(&self) -> bool {
        self.selected_sub_tab < self.featured_albums.len()
    }

    fn cycle_subtab_backwards(&mut self) {
        let count = self.featured_albums.len() + self.featured_playlists.len();
        self.selected_sub_tab = (self.selected_sub_tab + count - 1) % count;
    }

    fn cycle_subtab(&mut self) {
        let count = self.featured_albums.len() + self.featured_playlists.len();
        self.selected_sub_tab = (self.selected_sub_tab + count + 1) % count;
    }
}
