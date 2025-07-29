use core::fmt;

use qobuz_player_controls::models::{AlbumSimple, Playlist};
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    widgets::*,
};

use crate::{
    app::{Output, PlayOutcome, UnfilteredListState},
    popup::{PlaylistPopupState, Popup},
    ui::{album_simple_table, basic_list_table},
};

pub(crate) struct DiscoverState {
    pub(crate) press_awards: UnfilteredListState<AlbumSimple>,
    pub(crate) new_releases: UnfilteredListState<AlbumSimple>,
    pub(crate) qobuzissims: UnfilteredListState<AlbumSimple>,
    pub(crate) ideal_discography: UnfilteredListState<AlbumSimple>,
    pub(crate) editor_picks: UnfilteredListState<Playlist>,
    pub(crate) sub_tab: SubTab,
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SubTab {
    #[default]
    PressAwards,
    NewReleases,
    Qobuzissims,
    IdealDiscography,
    EditorPicks,
}

impl fmt::Display for SubTab {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::PressAwards => write!(f, "Press awards"),
            Self::NewReleases => write!(f, "New releases"),
            Self::Qobuzissims => write!(f, "Qobuzissims"),
            Self::IdealDiscography => write!(f, "Ideal discography"),
            Self::EditorPicks => write!(f, "Editor picks"),
        }
    }
}

impl SubTab {
    pub(crate) const VALUES: [Self; 5] = [
        Self::PressAwards,
        Self::NewReleases,
        Self::Qobuzissims,
        Self::IdealDiscography,
        Self::EditorPicks,
    ];

    pub(crate) fn next(self) -> Self {
        let index = Self::VALUES.iter().position(|&x| x == self).unwrap();
        Self::VALUES[(index + 1) % Self::VALUES.len()]
    }

    pub(crate) fn previous(self) -> Self {
        let index = Self::VALUES.iter().position(|&x| x == self).unwrap();
        let len = Self::VALUES.len();
        Self::VALUES[(index + len - 1) % len]
    }
}

impl DiscoverState {
    pub(crate) fn render(&mut self, frame: &mut Frame, area: Rect) {
        let title = format!("<{}>", self.sub_tab);
        let (table, state) = match self.sub_tab {
            SubTab::PressAwards => (
                album_simple_table(&self.press_awards.items, &title),
                &mut self.press_awards.state,
            ),
            SubTab::NewReleases => (
                album_simple_table(&self.new_releases.items, &title),
                &mut self.new_releases.state,
            ),
            SubTab::Qobuzissims => (
                album_simple_table(&self.qobuzissims.items, &title),
                &mut self.qobuzissims.state,
            ),
            SubTab::IdealDiscography => (
                album_simple_table(&self.ideal_discography.items, &title),
                &mut self.ideal_discography.state,
            ),
            SubTab::EditorPicks => (
                basic_list_table(
                    self.editor_picks
                        .items
                        .iter()
                        .map(|playlist| Row::new(Line::from(playlist.title.clone())))
                        .collect::<Vec<_>>(),
                    &title,
                ),
                &mut self.editor_picks.state,
            ),
        };

        frame.render_stateful_widget(table, area, state);
    }

    pub(crate) async fn handle_events(&mut self, event: Event) -> Output {
        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match key_event.code {
                    KeyCode::Left => {
                        self.cycle_subtab_backwards();
                        Output::Consumed
                    }
                    KeyCode::Right => {
                        self.cycle_subtab();
                        Output::Consumed
                    }
                    KeyCode::Down => {
                        self.current_list_state().select_next();
                        Output::Consumed
                    }
                    KeyCode::Up => {
                        self.current_list_state().select_previous();
                        Output::Consumed
                    }
                    KeyCode::Enter => {
                        let selected_index = self.current_list_state().selected();
                        if let Some(selected_index) = selected_index {
                            match self.sub_tab {
                                SubTab::PressAwards => {
                                    let id = self.press_awards.items[selected_index].id.clone();
                                    return Output::PlayOutcome(PlayOutcome::Album(id));
                                }
                                SubTab::NewReleases => {
                                    let id = self.new_releases.items[selected_index].id.clone();
                                    return Output::PlayOutcome(PlayOutcome::Album(id));
                                }
                                SubTab::Qobuzissims => {
                                    let id = self.qobuzissims.items[selected_index].id.clone();
                                    return Output::PlayOutcome(PlayOutcome::Album(id));
                                }
                                SubTab::IdealDiscography => {
                                    let id =
                                        self.ideal_discography.items[selected_index].id.clone();
                                    return Output::PlayOutcome(PlayOutcome::Album(id));
                                }
                                SubTab::EditorPicks => {
                                    let selected = &self.editor_picks.items[selected_index];
                                    return Output::Popup(Popup::Playlist(PlaylistPopupState {
                                        playlist_name: selected.title.clone(),
                                        playlist_id: selected.id,
                                        shuffle: false,
                                    }));
                                }
                            }
                        }
                        Output::Consumed
                    }

                    _ => Output::NotConsumed,
                }
            }
            _ => Output::NotConsumed,
        }
    }

    fn current_list_state(&mut self) -> &mut TableState {
        match self.sub_tab {
            SubTab::PressAwards => &mut self.press_awards.state,
            SubTab::NewReleases => &mut self.new_releases.state,
            SubTab::Qobuzissims => &mut self.qobuzissims.state,
            SubTab::IdealDiscography => &mut self.ideal_discography.state,
            SubTab::EditorPicks => &mut self.editor_picks.state,
        }
    }

    fn cycle_subtab_backwards(&mut self) {
        self.sub_tab = self.sub_tab.previous();
    }

    fn cycle_subtab(&mut self) {
        self.sub_tab = self.sub_tab.next();
    }
}
