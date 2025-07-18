use qobuz_player_controls::models::{Album, AlbumSimple};
use ratatui::{layout::Flex, prelude::*, widgets::*};
use tui_input::Input;

use crate::{
    app::{App, State, Tab},
    now_playing::{self},
};

impl App {
    pub(crate) fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(10),
            ])
            .split(area);

        let tabs = Tabs::new(
            Tab::VALUES
                .iter()
                .enumerate()
                .map(|(i, tab)| format!("[{}] {}", i + 1, tab)),
        )
        .block(Block::bordered().border_type(BorderType::Rounded))
        .highlight_style(Style::default().bg(Color::Blue))
        .select(
            Tab::VALUES
                .iter()
                .position(|tab| tab == &self.current_screen)
                .unwrap_or(0),
        )
        .divider(symbols::line::VERTICAL);
        frame.render_widget(tabs, chunks[0]);

        if self.now_playing.playing_track.is_some() {
            now_playing::render(frame, chunks[2], &mut self.now_playing);
        }

        let tab_content_area = if self.now_playing.entity_title.is_some() {
            chunks[1]
        } else {
            chunks[1].union(chunks[2])
        };

        match self.current_screen {
            Tab::Favorites => self.favorites.render(frame, tab_content_area),
            Tab::Search => self.search.render(frame, tab_content_area),
            Tab::Queue => self.queue.render(frame, tab_content_area),
            Tab::Discover => self.discover.render(frame, tab_content_area),
        }

        if let State::Popup(popup) = &mut self.state {
            popup.render(frame);
        }

        if matches!(self.state, State::Help) {
            render_help(frame);
        }
    }
}

pub(crate) fn center(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [area] = Layout::horizontal([horizontal])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
    area
}

fn render_help(frame: &mut Frame) {
    let rows = [
        ["Next song", "n"],
        ["Previous song", "p"],
        ["Jump forward", "f"],
        ["Jump backwards", "b"],
        ["e", "Edit filter"],
        ["esc", "Stop edit filter"],
        ["Up/Down", "Select in list"],
        ["Enter", "Select selected item"],
        ["Left/right", "Cycle subgrup"],
        ["q", "Exit"],
    ];

    let max_left = rows.iter().map(|x| x[0].len()).max().unwrap();
    let max_right = rows.iter().map(|x| x[1].len()).max().unwrap();
    let max = max_left + max_right;

    let rows: Vec<_> = rows.into_iter().map(Row::new).collect();

    let area = center(
        frame.area(),
        Constraint::Length(max as u16 + 2 + 9),
        Constraint::Length(rows.len() as u16 + 2),
    );

    let block = block("Help");

    let table = Table::default().rows(rows).block(block);

    frame.render_widget(Clear, area);
    frame.render_widget(table, area);
}

pub(crate) fn render_input(
    input: &Input,
    editing: bool,
    area: Rect,
    frame: &mut Frame,
    title: &str,
) {
    let width = area.width.max(3) - 3;
    let scroll = input.visual_scroll(width as usize);
    let style = match editing {
        true => Color::Blue.into(),
        _ => Style::default(),
    };

    let input_paragraph = Paragraph::new(input.value())
        .style(style)
        .scroll((0, scroll as u16))
        .block(block(title));

    frame.render_widget(input_paragraph, area);

    if editing {
        let x = input.visual_cursor().max(scroll) - scroll + 1;
        frame.set_cursor_position((area.x + x as u16, area.y + 1))
    }
}

const ROW_HIGHLIGHT_STYLE: Style = Style::new().bg(Color::Blue);

pub(crate) fn block(title: &str) -> Block {
    Block::bordered()
        .title(format!(" {title} "))
        .title_alignment(Alignment::Center)
        .border_type(BorderType::Rounded)
}

pub(crate) fn album_table<'a>(rows: &[Album], title: &'a str) -> Table<'a> {
    let max_title_length = rows
        .iter()
        .map(|album| album.title.len())
        .max()
        .unwrap_or(0);

    let max_artist_name_length = rows
        .iter()
        .map(|album| album.artist.name.len())
        .max()
        .unwrap_or(0);

    let rows: Vec<_> = rows
        .iter()
        .map(|album| {
            Row::new(vec![
                Span::from(album.title.clone()),
                Span::from(album.artist.name.clone()),
                Span::from(album.release_year.to_string()),
            ])
        })
        .collect();

    let is_empty = rows.is_empty();
    let mut table = Table::new(
        rows,
        [
            Constraint::Min(max_title_length as u16),
            Constraint::Min(max_artist_name_length as u16),
            Constraint::Length(4),
        ],
    )
    .block(block(title))
    .row_highlight_style(ROW_HIGHLIGHT_STYLE);

    if !is_empty {
        table = table.header(Row::new(["Title", "Artist", "Year"]).add_modifier(Modifier::BOLD));
    }
    table
}

pub(crate) fn album_simple_table<'a>(rows: &[AlbumSimple], title: &'a str) -> Table<'a> {
    let max_title_length = rows
        .iter()
        .map(|album| album.title.len())
        .max()
        .unwrap_or(0);

    let max_artist_name_length = rows
        .iter()
        .map(|album| album.artist.name.len())
        .max()
        .unwrap_or(0);

    let rows: Vec<_> = rows
        .iter()
        .map(|album| {
            Row::new(vec![
                Span::from(album.title.clone()),
                Span::from(album.artist.name.clone()),
            ])
        })
        .collect();

    let is_empty = rows.is_empty();
    let mut table = Table::new(
        rows,
        [
            Constraint::Min(max_title_length as u16),
            Constraint::Min(max_artist_name_length as u16),
        ],
    )
    .block(block(title))
    .row_highlight_style(ROW_HIGHLIGHT_STYLE);

    if !is_empty {
        table = table.header(Row::new(["Title", "Artist"]).add_modifier(Modifier::BOLD));
    }
    table
}

pub(crate) fn basic_list_table<'a>(rows: Vec<Row<'a>>, title: &'a str) -> Table<'a> {
    Table::new(rows, [Constraint::Min(1)])
        .block(block(title))
        .row_highlight_style(ROW_HIGHLIGHT_STYLE)
}
