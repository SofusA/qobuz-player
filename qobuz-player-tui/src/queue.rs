use qobuz_player_controls::controls::Controls;
use qobuz_player_models::{Track, TrackStatus};
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    style::Styled,
    widgets::*,
};

use crate::{
    app::Output,
    ui::{basic_list_table, block, mark_explicit_and_hifi},
};

pub struct QueueState {
    items: Vec<Track>,
    state: TableState,
}

impl QueueState {
    pub fn new(tracks: Vec<Track>) -> Self {
        Self {
            items: tracks,
            state: Default::default(),
        }
    }
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let table = basic_list_table(
            self.items
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
        )
        .block(block(None));

        frame.render_stateful_widget(table, area, &mut self.state);
    }

    pub fn items(&self) -> &Vec<Track> {
        &self.items
    }

    pub fn set_items(&mut self, items: Vec<Track>) {
        self.items = items
    }

    pub async fn handle_events(&mut self, event: Event, controls: &Controls) -> Output {
        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match key_event.code {
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.state.select_next();
                        Output::Consumed
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.state.select_previous();
                        Output::Consumed
                    }
                    KeyCode::Char('d') => {
                        let index = self.state.selected();

                        if let Some(index) = index {
                            if index == self.items().len() - 1 {
                                return Output::Consumed;
                            }

                            let mut order: Vec<_> =
                                self.items().iter().enumerate().map(|x| x.0).collect();

                            order.swap(index, index + 1);
                            controls.reorder_queue(order);
                        }
                        Output::Consumed
                    }
                    KeyCode::Char('u') => {
                        let index = self.state.selected();

                        if let Some(index) = index {
                            if index == 0 {
                                return Output::Consumed;
                            }
                            let mut order: Vec<_> =
                                self.items().iter().enumerate().map(|x| x.0).collect();

                            order.swap(index, index - 1);
                            controls.reorder_queue(order);
                        }
                        Output::Consumed
                    }
                    KeyCode::Char('D') => {
                        let index = self.state.selected();

                        if let Some(index) = index {
                            controls.remove_index_from_queue(index);
                        }
                        Output::Consumed
                    }
                    KeyCode::Enter => {
                        let index = self.state.selected();

                        if let Some(index) = index {
                            controls.skip_to_position(index, false);
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
