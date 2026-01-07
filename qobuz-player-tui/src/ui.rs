use qobuz_player_controls::notification::Notification;
use qobuz_player_models::{Album, AlbumSimple, Track};
use ratatui::{layout::Flex, prelude::*, widgets::*};
use tui_input::Input;

use crate::{
    app::{App, AppState, Tab},
    now_playing::{self},
};

impl App {
    pub(crate) fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        self.render_inner(frame);

        if matches!(self.app_state, AppState::Help) {
            render_help(frame);
        }

        self.render_notifications(frame, area);
    }

    fn render_inner(&mut self, frame: &mut Frame) {
        let area = frame.area();
        if self.full_screen {
            let area = center(area, Constraint::Percentage(80), Constraint::Length(10));
            now_playing::render(
                frame,
                area,
                &mut self.now_playing,
                self.full_screen,
                self.disable_tui_album_cover,
            );
            return;
        }

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
            now_playing::render(
                frame,
                chunks[2],
                &mut self.now_playing,
                self.full_screen,
                self.disable_tui_album_cover,
            );
        }

        let tab_content_area = if self.now_playing.playing_track.is_some() {
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

        if let AppState::Popup(popup) = &mut self.app_state {
            popup.render(frame);
        }
    }

    fn render_notifications(&self, frame: &mut Frame, area: Rect) {
        let notifications: Vec<_> = self.notifications.iter().map(|x| &x.0).collect();

        if notifications.is_empty() {
            return;
        }

        let messages = notifications
            .into_iter()
            .map(|notification| match notification {
                Notification::Error(msg) => ("Error", msg, Color::Red),
                Notification::Warning(msg) => ("Warning", msg, Color::Yellow),
                Notification::Success(msg) => ("Success", msg, Color::Green),
                Notification::Info(msg) => ("Info", msg, Color::Blue),
            });

        let inner_width = 60;
        let box_width = inner_width;
        let x = area.x + area.width.saturating_sub(box_width);
        let mut y = area.y;

        for msg in messages.rev() {
            let lines = (msg.1.len() as u16).div_ceil(inner_width);
            let box_height = lines + 2;

            if y + box_height > area.y + area.height {
                break;
            }

            let rect = Rect {
                x,
                y,
                width: box_width,
                height: box_height,
            };

            let paragraph = Paragraph::new(msg.1.as_str())
                .block(
                    Block::new()
                        .borders(Borders::ALL)
                        .border_style(msg.2)
                        .border_type(BorderType::Rounded)
                        .title(msg.0)
                        .title_alignment(Alignment::Center)
                        .title_style(msg.2),
                )
                .wrap(Wrap { trim: true });

            frame.render_widget(Clear, rect);
            frame.render_widget(paragraph, rect);

            y += box_height;
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
        ["Toggle focus mode", "F"],
        ["Next song", "n"],
        ["Previous song", "p"],
        ["Jump forward", "f"],
        ["Jump backwards", "b"],
        ["Edit filter", "e"],
        ["Stop edit filter", "esc"],
        ["Select in list", "Up/Down"],
        ["Select selected item", "Enter"],
        ["Cycle subgroup", "Left/right"],
        ["Add track to queue", "B"],
        ["Play track next", "N"],
        ["Delete from queue", "D"],
        ["Move up in queue", "u"],
        ["Move down in queue", "d"],
        ["Remove from favorites", "D"],
        ["Add to favorites", "A"],
        ["Create playlist", "C (playlist page)"],
        ["Delete playlist", "D (playlist page)"],
        ["Add track to playlist", "a"],
        ["Move playlist track up", "u"],
        ["Move playlist track down", "d"],
        ["Exit", "q"],
    ];

    let max_left = rows.iter().map(|x| x[0].len()).max().expect("infallible");
    let max_right = rows.iter().map(|x| x[1].len()).max().expect("infallible");
    let max = std::cmp::max(max_left, max_right);
    let max = max + max;

    let rows: Vec<_> = rows.into_iter().map(Row::new).collect();

    let area = center(
        frame.area(),
        Constraint::Length(max as u16 + 2 + 1),
        Constraint::Length(rows.len() as u16 + 2),
    );

    let block = block("Help", false);

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
        .block(block(title, false));

    frame.render_widget(input_paragraph, area);

    if editing {
        let x = input.visual_cursor().max(scroll) - scroll + 1;
        frame.set_cursor_position((area.x + x as u16, area.y + 1))
    }
}

const ROW_HIGHLIGHT_STYLE: Style = Style::new().bg(Color::Blue);

pub(crate) fn block(title: &str, selectable: bool) -> Block<'_> {
    let title = match selectable {
        true => format!(" <{title}> "),
        false => format!(" {title} "),
    };

    Block::bordered()
        .title(title)
        .title_alignment(Alignment::Center)
        .border_type(BorderType::Rounded)
}

pub(crate) fn album_table<'a>(rows: &[Album], title: &'a str) -> Table<'a> {
    let rows: Vec<_> = rows
        .iter()
        .map(|album| {
            Row::new(vec![
                mark_explicit_and_hifi(album.title.clone(), album.explicit, album.hires_available),
                Line::from(album.artist.name.clone()),
                Line::from(album.release_year.to_string()),
            ])
        })
        .collect();

    let is_empty = rows.is_empty();
    let mut table = Table::new(
        rows,
        [
            Constraint::Ratio(2, 3),
            Constraint::Ratio(1, 3),
            Constraint::Length(4),
        ],
    )
    .block(block(title, true))
    .row_highlight_style(ROW_HIGHLIGHT_STYLE);

    if !is_empty {
        table = table.header(Row::new(["Title", "Artist", "Year"]).add_modifier(Modifier::BOLD));
    }
    table
}

pub(crate) fn album_simple_table<'a>(rows: &[AlbumSimple], title: &'a str) -> Table<'a> {
    let rows: Vec<_> = rows
        .iter()
        .map(|album| {
            Row::new(vec![
                mark_explicit_and_hifi(album.title.clone(), album.explicit, album.hires_available),
                Line::from(album.artist.name.clone()),
            ])
        })
        .collect();

    let is_empty = rows.is_empty();
    let mut table = Table::new(rows, [Constraint::Ratio(2, 3), Constraint::Ratio(1, 3)])
        .block(block(title, true))
        .row_highlight_style(ROW_HIGHLIGHT_STYLE);

    if !is_empty {
        table = table.header(Row::new(["Title", "Artist"]).add_modifier(Modifier::BOLD));
    }
    table
}

pub(crate) fn basic_list_table<'a>(
    rows: Vec<Row<'a>>,
    title: &'a str,
    selectable: bool,
) -> Table<'a> {
    Table::new(rows, [Constraint::Min(1)])
        .block(block(title, selectable))
        .row_highlight_style(ROW_HIGHLIGHT_STYLE)
}

pub(crate) fn track_table<'a>(rows: &[Track], block_title: Option<&'a str>) -> Table<'a> {
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

    if let Some(title) = block_title {
        table = table.block(block(title, true));
    }

    if !is_empty {
        table = table.header(Row::new(["Title", "Artist", "Album"]).add_modifier(Modifier::BOLD));
    }
    table
}

pub fn mark_explicit_and_hifi(
    title: String,
    explicit: bool,
    hires_available: bool,
) -> Line<'static> {
    let mut parts: Vec<Span<'static>> = Vec::new();

    parts.push(Span::raw(title));

    if explicit {
        parts.push(Span::raw(" "));
        parts.push(Span::styled(
            "\u{f0b0c}",
            Style::default().add_modifier(Modifier::DIM),
        ));
    }

    if hires_available {
        parts.push(Span::raw(" "));
        parts.push(Span::styled(
            "\u{f0435}",
            Style::default().add_modifier(Modifier::DIM),
        ));
    }

    Line::from(parts)
}
