use qobuz_player_models::{Track, TrackStatus};
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    style::Styled,
    widgets::*,
};

use crate::{
    app::{Output, PlayOutcome, QueueOutcome, UnfilteredListState},
    ui::{basic_list_table, mark_explicit_and_hifi},
};

pub(crate) struct QueueState {
    pub queue: UnfilteredListState<Track>,
}

impl QueueState {
    pub(crate) fn render(&mut self, frame: &mut Frame, area: Rect) {
        let table = basic_list_table(
            self.queue
                .items
                .iter()
                .enumerate()
                .map(|(index, track)| {
                    let style = match track.status {
                        TrackStatus::Played => Style::default().add_modifier(Modifier::CROSSED_OUT),
                        TrackStatus::Playing => Style::default().add_modifier(Modifier::BOLD),
                        TrackStatus::Unplayed => Style::default(),
                        TrackStatus::Unplayable => {
                            Style::default().add_modifier(Modifier::CROSSED_OUT)
                        }
                    };
                    Row::new(Line::from(vec![
                        format!(
                            "{} {}",
                            index + 1,
                            mark_explicit_and_hifi(
                                track.title.clone(),
                                track.explicit,
                                track.hires_available
                            )
                        )
                        .set_style(style),
                    ]))
                })
                .collect(),
            Some("Queue"),
            false,
        );

        frame.render_stateful_widget(table, area, &mut self.queue.state);
    }

    pub(crate) async fn handle_events(&mut self, event: Event) -> Output {
        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match key_event.code {
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.queue.state.select_next();
                        Output::Consumed
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.queue.state.select_previous();
                        Output::Consumed
                    }
                    KeyCode::Char('d') => {
                        let index = self.queue.state.selected();

                        if let Some(index) = index {
                            return Output::Queue(QueueOutcome::MoveIndexDown(index));
                        }
                        Output::Consumed
                    }
                    KeyCode::Char('u') => {
                        let index = self.queue.state.selected();

                        if let Some(index) = index {
                            return Output::Queue(QueueOutcome::MoveIndexUp(index));
                        }
                        Output::Consumed
                    }
                    KeyCode::Char('D') => {
                        let index = self.queue.state.selected();

                        if let Some(index) = index {
                            return Output::Queue(QueueOutcome::RemoveIndex(index));
                        }
                        Output::Consumed
                    }
                    KeyCode::Enter => {
                        let index = self.queue.state.selected();

                        if let Some(index) = index {
                            return Output::PlayOutcome(PlayOutcome::SkipToPosition(index));
                        }
                        Output::Consumed
                    }

                    _ => Output::NotConsumed,
                }
            }
            _ => Output::NotConsumed,
        }
    }
}
