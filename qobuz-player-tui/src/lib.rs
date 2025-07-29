use std::sync::Arc;

use app::{App, FilteredListState, UnfilteredListState, get_current_state};
use favorites::FavoritesState;
use qobuz_player_controls::tracklist::Tracklist;
use queue::QueueState;
use ratatui::{prelude::*, widgets::*};
use search::SearchState;
use tokio::{sync::RwLock, try_join};
use ui::center;

mod app;
mod discover;
mod favorites;
mod now_playing;
mod popup;
mod queue;
mod search;
mod ui;

pub async fn init(tracklist: Arc<RwLock<Tracklist>>) {
    let mut terminal = ratatui::init();

    draw_loading_screen(&mut terminal);

    let (favorites, press_awards, new_releases, qobuzissims, ideal_discography, editor_picks) =
        try_join!(
            qobuz_player_controls::favorites(),
            qobuz_player_controls::featured_albums(
                qobuz_player_controls::AlbumFeaturedType::PressAwards
            ),
            qobuz_player_controls::featured_albums(
                qobuz_player_controls::AlbumFeaturedType::NewReleasesFull
            ),
            qobuz_player_controls::featured_albums(
                qobuz_player_controls::AlbumFeaturedType::Qobuzissims
            ),
            qobuz_player_controls::featured_albums(
                qobuz_player_controls::AlbumFeaturedType::IdealDiscography
            ),
            qobuz_player_controls::featured_playlists(
                qobuz_player_controls::PlaylistFeaturedType::EditorPicks
            ),
        )
        .unwrap();

    let now_playing = get_current_state(&*tracklist.read().await).await;

    let mut app = App {
        tracklist,
        now_playing,
        current_screen: Default::default(),
        exit: Default::default(),
        should_draw: true,
        state: Default::default(),
        favorites: FavoritesState {
            editing: Default::default(),
            filter: Default::default(),
            albums: FilteredListState {
                filter: favorites.albums.clone(),
                all_items: favorites.albums,
                state: Default::default(),
            },
            artists: FilteredListState {
                filter: favorites.artists.clone(),
                all_items: favorites.artists,
                state: Default::default(),
            },
            playlists: FilteredListState {
                filter: favorites.playlists.clone(),
                all_items: favorites.playlists,
                state: Default::default(),
            },
            sub_tab: Default::default(),
        },
        search: SearchState {
            editing: Default::default(),
            filter: Default::default(),
            albums: UnfilteredListState {
                items: Default::default(),
                state: Default::default(),
            },
            artists: UnfilteredListState {
                items: Default::default(),
                state: Default::default(),
            },
            playlists: UnfilteredListState {
                items: Default::default(),
                state: Default::default(),
            },
            tracks: UnfilteredListState {
                items: Default::default(),
                state: Default::default(),
            },
            sub_tab: Default::default(),
        },
        queue: QueueState {
            queue: UnfilteredListState {
                items: Default::default(),
                state: Default::default(),
            },
        },
        discover: discover::DiscoverState {
            press_awards: UnfilteredListState {
                items: press_awards,
                state: Default::default(),
            },
            new_releases: UnfilteredListState {
                items: new_releases,
                state: Default::default(),
            },
            qobuzissims: UnfilteredListState {
                items: qobuzissims,
                state: Default::default(),
            },
            ideal_discography: UnfilteredListState {
                items: ideal_discography,
                state: Default::default(),
            },
            editor_picks: UnfilteredListState {
                items: editor_picks,
                state: Default::default(),
            },
            sub_tab: Default::default(),
        },
    };

    let _app_result = app.run(&mut terminal).await;
    ratatui::restore();
}

fn draw_loading_screen<B: Backend>(terminal: &mut Terminal<B>) {
    let ascii_art = r#"
              _                       _                       
   __ _  ___ | |__  _   _ ___   _ __ | | __ _ _   _  ___ _ __ 
  / _` |/ _ \| '_ \| | | / __| | '_ \| |/ _` | | | |/ _ \ '__|
 | (_| | (_) | |_) | |_| \__ \ | |_) | | (_| | |_| |  __/ |   
  \__, |\___/|_.__/ \__,_|___/ | .__/|_|\__,_|\__, |\___|_|   
     |_|                       |_|            |___/           
"#;

    terminal
        .draw(|f| {
            let area = center(f.area(), Constraint::Length(64), Constraint::Length(7));
            let paragraph = Paragraph::new(ascii_art)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: false });
            f.render_widget(paragraph, area);
        })
        .unwrap();
}
