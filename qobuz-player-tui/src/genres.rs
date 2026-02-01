use qobuz_player_controls::Result;
use qobuz_player_controls::client::Client;
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

use crate::{
    app::{NotificationList, Output},
    ui::{block, tab_bar},
    widgets::album_simple_list::AlbumSimpleList,
};

pub struct GenresState {
    genres: Vec<GenreItem>,
    selected_genre: usize,
    selected_sub_tab: usize,
    mode: GenresMode,
}

struct GenreItem {
    id: u32,
    name: String,
    albums: Vec<(String, AlbumSimpleList)>,
}

#[derive(PartialEq)]
enum GenresMode {
    GenreList,
    GenreDetail,
}

impl GenresState {
    pub async fn new(client: &Client) -> Result<Self> {
        let genres_list = client.genres().await?;

        let genres = genres_list
            .into_iter()
            .map(|g| GenreItem {
                id: g.id,
                name: g.name,
                albums: Vec::new(),
            })
            .collect();

        Ok(Self {
            genres,
            selected_genre: 0,
            selected_sub_tab: 0,
            mode: GenresMode::GenreList,
        })
    }

    async fn load_genre_albums(&mut self, client: &Client) -> Result<()> {
        if self.genres[self.selected_genre].albums.is_empty() {
            let genre_id = self.genres[self.selected_genre].id;
            let albums = client.genre_albums(genre_id).await?;

            self.genres[self.selected_genre].albums = albums
                .into_iter()
                .map(|x| (x.0, AlbumSimpleList::new(x.1)))
                .collect();
        }
        Ok(())
    }
}

impl GenresState {
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let block = block(None);
        frame.render_widget(block, area);

        let tab_content_area = area.inner(Margin::new(1, 1));

        match self.mode {
            GenresMode::GenreList => self.render_genre_list(frame, tab_content_area),
            GenresMode::GenreDetail => self.render_genre_detail(frame, tab_content_area),
        }
    }

    fn render_genre_list(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .split(area);

        let title = Paragraph::new("Select a Genre")
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center);
        frame.render_widget(title, chunks[0]);

        let items_per_row = 2;
        let rows_needed = self.genres.len().div_ceil(items_per_row);

        let mut constraints = vec![];
        for _ in 0..rows_needed {
            constraints.push(Constraint::Length(3));
        }

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(chunks[1]);

        for (row_idx, row_area) in rows.iter().enumerate() {
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(*row_area);

            for col_idx in 0..items_per_row {
                let genre_idx = row_idx * items_per_row + col_idx;
                if genre_idx < self.genres.len() {
                    let is_selected = genre_idx == self.selected_genre;
                    let style = if is_selected {
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };

                    let genre_block = Paragraph::new(self.genres[genre_idx].name.as_str())
                        .style(style)
                        .alignment(Alignment::Center)
                        .block(Block::default().borders(Borders::ALL).border_style(
                            if is_selected {
                                Style::default().fg(Color::Cyan)
                            } else {
                                Style::default().fg(Color::DarkGray)
                            },
                        ));

                    frame.render_widget(genre_block, cols[col_idx]);
                }
            }
        }
    }

    fn render_genre_detail(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(2),
                Constraint::Min(1),
            ])
            .split(area);

        // Back button and title
        let title = format!("← Back  |  {}", self.genres[self.selected_genre].name);
        let title_widget = Paragraph::new(title)
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Left);
        frame.render_widget(title_widget, chunks[0]);

        // Sub tabs
        let albums = &self.genres[self.selected_genre].albums;
        let labels: Vec<_> = albums.iter().map(|a| a.0.as_str()).collect();

        if !labels.is_empty() {
            let tabs = tab_bar(labels, self.selected_sub_tab);
            frame.render_widget(tabs, chunks[1]);

            // Album list
            if self.selected_sub_tab < albums.len() {
                let list_state =
                    &mut self.genres[self.selected_genre].albums[self.selected_sub_tab];
                list_state.1.render(chunks[2], frame.buffer_mut());
            }
        } else {
            let loading = Paragraph::new("Loading albums...")
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Center);
            frame.render_widget(loading, chunks[2]);
        }
    }

    pub async fn handle_events(
        &mut self,
        event: Event,
        client: &Client,
        notifications: &mut NotificationList,
    ) -> Result<Output> {
        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => match self.mode {
                GenresMode::GenreList => {
                    self.handle_genre_list_events(key_event.code, client).await
                }
                GenresMode::GenreDetail => {
                    self.handle_genre_detail_events(key_event.code, client, notifications)
                        .await
                }
            },
            _ => Ok(Output::NotConsumed),
        }
    }

    async fn handle_genre_list_events(&mut self, code: KeyCode, client: &Client) -> Result<Output> {
        match code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_genre >= 2 {
                    self.selected_genre -= 2;
                }
                Ok(Output::Consumed)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_genre + 2 < self.genres.len() {
                    self.selected_genre += 2;
                }
                Ok(Output::Consumed)
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if self.selected_genre > 0 {
                    self.selected_genre -= 1;
                }
                Ok(Output::Consumed)
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if self.selected_genre + 1 < self.genres.len() {
                    self.selected_genre += 1;
                }
                Ok(Output::Consumed)
            }
            KeyCode::Enter => {
                self.load_genre_albums(client).await?;
                self.mode = GenresMode::GenreDetail;
                self.selected_sub_tab = 0;
                Ok(Output::Consumed)
            }
            _ => Ok(Output::NotConsumed),
        }
    }

    async fn handle_genre_detail_events(
        &mut self,
        code: KeyCode,
        client: &Client,
        notifications: &mut NotificationList,
    ) -> Result<Output> {
        match code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = GenresMode::GenreList;
                Ok(Output::Consumed)
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.cycle_subtab_backwards();
                Ok(Output::Consumed)
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.cycle_subtab();
                Ok(Output::Consumed)
            }
            _ => {
                let albums = &mut self.genres[self.selected_genre].albums;
                if self.selected_sub_tab < albums.len() {
                    albums[self.selected_sub_tab]
                        .1
                        .handle_events(code, client, notifications)
                        .await
                } else {
                    Ok(Output::NotConsumed)
                }
            }
        }
    }

    fn cycle_subtab_backwards(&mut self) {
        let count = self.genres[self.selected_genre].albums.len();
        if count > 0 {
            self.selected_sub_tab = (self.selected_sub_tab + count - 1) % count;
        }
    }

    fn cycle_subtab(&mut self) {
        let count = self.genres[self.selected_genre].albums.len();
        if count > 0 {
            self.selected_sub_tab = (self.selected_sub_tab + 1) % count;
        }
    }
}
