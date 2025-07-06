use crate::ui::block;
use qobuz_player_controls::{models::Track, tracklist};
use ratatui::{prelude::*, widgets::*};
use ratatui_image::{StatefulImage, protocol::StatefulProtocol};

#[derive(Default)]
pub(crate) struct NowPlayingState {
    pub(crate) image: Option<(StatefulProtocol, f32)>,
    pub(crate) entity_title: Option<String>,
    pub(crate) playing_track: Option<Track>,
    pub(crate) tracklist_length: u32,
    pub(crate) tracklist_position: u32,
    pub(crate) show_tracklist_position: bool,
    pub(crate) status: tracklist::Status,
    pub(crate) duration_s: u32,
}

pub(crate) fn render(frame: &mut Frame, area: Rect, state: &mut NowPlayingState) {
    let track = match &state.playing_track {
        Some(t) => t,
        None => return,
    };

    let title = format!("Playing {}", get_status_icon(state.status));
    let block = block(&title);

    let length = state
        .image
        .as_ref()
        .map(|image| image.1 * (area.height * 2 - 1) as f32)
        .map(|x| x as u16)
        .unwrap_or(0);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(length), Constraint::Min(1)])
        .split(block.inner(area));

    frame.render_widget(block, area);

    if let Some(image) = &mut state.image {
        let stateful_image = StatefulImage::default();
        frame.render_stateful_widget(stateful_image, chunks[0], &mut image.0);
    }

    let info_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(chunks[1]);

    let mut lines = vec![];

    if let Some(album) = &track.album_title {
        lines.push(Line::from(album.clone()).style(Style::new().bold()));
    }

    if let Some(artist) = &track.artist_name {
        lines.push(Line::from(artist.clone()));
    }

    lines.push(Line::from(track.title.clone()));

    let track_number = if state.show_tracklist_position {
        state.tracklist_position + 1
    } else {
        track.number
    };

    lines.push(Line::from(format!(
        "{} of {}",
        track_number, state.tracklist_length
    )));

    let duration = if state.duration_s < track.duration_seconds {
        state.duration_s
    } else {
        track.duration_seconds
    };

    let ratio = duration as f64 / track.duration_seconds as f64;

    let label = format!(
        "{} / {}",
        format_seconds(state.duration_s),
        format_seconds(track.duration_seconds),
    );

    let gauge = Gauge::default()
        .ratio(ratio)
        .gauge_style(Style::default().fg(Color::Blue))
        .label(label);

    frame.render_widget(gauge, info_chunks[1]);
    frame.render_widget(Text::from(lines), info_chunks[0]);
}

fn get_status_icon(state: tracklist::Status) -> String {
    match state {
        tracklist::Status::Playing => "⏵".to_string(),
        tracklist::Status::Paused => "⏸ ".to_string(),
        tracklist::Status::Stopped => "⏹ ".to_string(),
    }
}

fn format_seconds(seconds: u32) -> String {
    let minutes = seconds / 60;
    let seconds = seconds % 60;
    format!("{:02}:{:02}", minutes, seconds)
}
