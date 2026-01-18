use std::sync::Arc;

use app::{App, FilteredListState, UnfilteredListState, get_current_state};
use favorites::FavoritesState;
use qobuz_player_controls::{
    ExitSender, PositionReceiver, Result, StatusReceiver, TracklistReceiver, client::Client,
    controls::Controls, error::Error, notification::NotificationBroadcast,
};
use queue::QueueState;
use ratatui::{prelude::*, widgets::*};
use search::SearchState;
use tokio::try_join;
use ui::center;

mod app;
mod discover;
mod favorites;
mod now_playing;
mod popup;
mod queue;
mod search;
mod sub_tab;
mod ui;

#[allow(clippy::too_many_arguments)]
pub async fn init(
    client: Arc<Client>,
    broadcast: Arc<NotificationBroadcast>,
    controls: Controls,
    position_receiver: PositionReceiver,
    tracklist_receiver: TracklistReceiver,
    status_receiver: StatusReceiver,
    exit_sender: ExitSender,
    disable_tui_album_cover: bool,
) -> Result<()> {
    let mut terminal = ratatui::init();

    draw_loading_screen(&mut terminal);

    let (favorites, featured_albums, featured_playlists) = try_join!(
        client.favorites(),
        client.featured_albums(),
        client.featured_playlists(),
    )?;

    let featured_albums = featured_albums
        .into_iter()
        .map(|x| {
            (
                x.0,
                UnfilteredListState {
                    items: x.1,
                    state: Default::default(),
                },
            )
        })
        .collect();

    let featured_playlists = featured_playlists
        .into_iter()
        .map(|x| {
            (
                x.0,
                UnfilteredListState {
                    items: x.1,
                    state: Default::default(),
                },
            )
        })
        .collect();

    let tracklist_value = tracklist_receiver.borrow().clone();
    let status_value = *status_receiver.borrow();
    let queue = tracklist_value.queue().clone();
    let now_playing = get_current_state(tracklist_value, status_value).await;

    let client_clone = client.clone();

    let mut app = App {
        client: client_clone.clone(),
        broadcast,
        notifications: Default::default(),
        controls,
        now_playing,
        full_screen: false,
        position: position_receiver,
        tracklist: tracklist_receiver,
        status: status_receiver,
        current_screen: Default::default(),
        exit: Default::default(),
        should_draw: true,
        app_state: Default::default(),
        disable_tui_album_cover,
        favorites: FavoritesState {
            client: client_clone.clone(),
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
            tracks: FilteredListState {
                filter: favorites.tracks.clone(),
                all_items: favorites.tracks,
                state: Default::default(),
            },
            sub_tab: Default::default(),
        },
        search: SearchState {
            client: client_clone.clone(),
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
                items: queue,
                state: Default::default(),
            },
        },
        discover: discover::DiscoverState {
            client: client_clone,
            featured_albums,
            featured_playlists,
            selected_sub_tab: Default::default(),
        },
    };

    _ = app.run(&mut terminal).await;
    ratatui::restore();
    match exit_sender.send(true) {
        Ok(_) => Ok(()),
        Err(_) => Err(Error::Notification),
    }
}

fn draw_loading_screen<B: Backend>(terminal: &mut Terminal<B>) {
    let ascii_art = r#"
             _                     _                       
  __ _  ___ | |__  _   _ _____ __ | | __ _ _   _  ___ _ __ 
 / _` |/ _ \| '_ \| | | |_  / '_ \| |/ _` | | | |/ _ \ '__|
| (_| | (_) | |_) | |_| |/ /| |_) | | (_| | |_| |  __/ |   
 \__, |\___/|_.__/ \__,_/___| .__/|_|\__,_|\__, |\___|_|   
    |_|                     |_|            |___/           
"#;

    terminal
        .draw(|f| {
            let area = center(f.area(), Constraint::Length(64), Constraint::Length(7));
            let paragraph = Paragraph::new(ascii_art)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: false });
            f.render_widget(paragraph, area);
        })
        .expect("infallible");
}
