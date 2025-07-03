use crossterm::event::{Event, KeyCode, KeyEventKind};
use qobuz_player_controls::models::Track;
use ratatui::{prelude::*, style::Styled, widgets::*};

use crate::{
    app::{Output, UnfilteredListState},
    ui::basic_list_table,
};

pub struct QueueState {
    pub queue: UnfilteredListState<Track>,
}

impl QueueState {
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let table = basic_list_table(
            self.queue
                .items
                .iter()
                .enumerate()
                .map(|(index, track)| {
                    let style = match track.status {
                        qobuz_player_controls::models::TrackStatus::Played => {
                            Style::default().add_modifier(Modifier::CROSSED_OUT)
                        }
                        qobuz_player_controls::models::TrackStatus::Playing => {
                            Style::default().add_modifier(Modifier::BOLD)
                        }
                        qobuz_player_controls::models::TrackStatus::Unplayed => Style::default(),
                        qobuz_player_controls::models::TrackStatus::Unplayable => {
                            Style::default().add_modifier(Modifier::CROSSED_OUT)
                        }
                    };
                    Row::new(Line::from(vec![
                        format!("{} {}", index + 1, track.title.clone()).set_style(style),
                    ]))
                })
                .collect(),
            " Queue ",
        );

        frame.render_stateful_widget(table, area, &mut self.queue.state);
    }

    pub async fn handle_events(&mut self, event: Event) -> Output {
        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match key_event.code {
                    KeyCode::Down => {
                        self.queue.state.select_next();
                        Output::Consumed
                    }
                    KeyCode::Up => {
                        self.queue.state.select_previous();
                        Output::Consumed
                    }
                    KeyCode::Enter => {
                        let index = self.queue.state.selected();

                        if let Some(index) = index {
                            qobuz_player_controls::skip_to_position(index as u32, true)
                                .await
                                .unwrap();
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
