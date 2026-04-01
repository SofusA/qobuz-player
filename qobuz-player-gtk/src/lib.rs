use std::{sync::Arc, time::Duration};

use libadwaita::{Application, ApplicationWindow, prelude::*};
use qobuz_player_controls::{
    PositionReceiver, Status, StatusReceiver, TracklistReceiver, client::Client,
    controls::Controls, tracklist::Tracklist,
};

use crate::ui::{
    albums_page::AlbumsPage,
    now_playing_bar::{
        NowPlayingBar, build_now_playing_bar, update_now_playing, update_now_playing_button_icon,
        update_progress,
    },
    search_page::SearchPage,
};

mod ui;

pub fn init(
    client: Arc<Client>,
    tracklist_receiver: TracklistReceiver,
    status_receiver: StatusReceiver,
    position_receiver: PositionReceiver,
    controls: Controls,
) {
    libadwaita::init().unwrap();

    let application = libadwaita::Application::builder()
        .application_id("com.github.sofusa.qobuz-player")
        .build();

    application.connect_activate(move |app| {
        build_ui(
            app,
            tracklist_receiver.clone(),
            status_receiver.clone(),
            position_receiver.clone(),
            controls.clone(),
            client.clone(),
        );
    });

    let args: &[&str] = &[];
    application.run_with_args(args);
}

fn build_ui(
    app: &Application,
    tracklist_receiver: TracklistReceiver,
    status_receiver: StatusReceiver,
    position_receiver: PositionReceiver,
    controls: Controls,
    client: Arc<Client>,
) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Qobuz Player")
        .default_width(900)
        .default_height(600)
        .build();

    let stack = libadwaita::ViewStack::builder().vexpand(true).build();
    let album_page = AlbumsPage::new(controls.clone(), client.clone());
    album_page.load();

    stack
        .add_titled(album_page.widget(), Some("albums"), "Albums")
        .set_icon_name(Some("media-optical-symbolic"));
    let search_page = SearchPage::new(client, controls.clone());

    stack
        .add_titled(search_page.widget(), Some("search"), "Search")
        .set_icon_name(Some("system-search-symbolic"));

    let view_switcher = libadwaita::ViewSwitcher::builder()
        .stack(&stack)
        .policy(libadwaita::ViewSwitcherPolicy::Wide)
        .build();

    let header = libadwaita::HeaderBar::builder()
        .title_widget(&view_switcher)
        .build();

    let now_playing = build_now_playing_bar(controls);

    let vbox = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .build();

    vbox.append(&header);
    vbox.append(&stack);
    vbox.append(&now_playing.revealer);

    window.set_content(Some(&vbox));
    window.show();

    let tracklist_value = tracklist_receiver.borrow().clone();
    let current_track = tracklist_value.current_track();
    if let Some(track) = current_track {
        update_now_playing(&now_playing, track);
    }

    setup_tracklist_listener(
        tracklist_receiver,
        status_receiver,
        position_receiver,
        now_playing,
    );
}

fn setup_tracklist_listener(
    mut tracklist_receiver: TracklistReceiver,
    mut status_receiver: StatusReceiver,
    mut position_receiver: PositionReceiver,
    now_playing_bar: NowPlayingBar,
) {
    let (sender, receiver) = async_channel::unbounded::<UiEvent>();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                Ok(_) = tracklist_receiver.changed() => {
                    let tracklist = tracklist_receiver.borrow_and_update().clone();
                    sender.send(UiEvent::Tracklist(tracklist)).await.unwrap();
                }

                Ok(_) = status_receiver.changed() => {
                    let status = *status_receiver.borrow_and_update();
                    sender.send(UiEvent::Status(status)).await.unwrap();
                }

                Ok(_) = position_receiver.changed() => {
                    let position = *position_receiver.borrow_and_update();
                    sender.send(UiEvent::Position(position)).await.unwrap();
                }
            }
        }
    });

    glib::spawn_future_local(async move {
        loop {
            match receiver.recv().await {
                Ok(update) => match update {
                    UiEvent::Tracklist(tracklist) => {
                        if let Some(track) = tracklist.current_track() {
                            update_now_playing(&now_playing_bar, track);
                        }
                    }
                    UiEvent::Status(status) => {
                        update_now_playing_button_icon(&status, &now_playing_bar.play_button);
                    }
                    UiEvent::Position(duration) => {
                        update_progress(&now_playing_bar, &duration);
                    }
                },
                Err(err) => {
                    tracing::error!("{err}");
                }
            }
        }
    });
}

enum UiEvent {
    Tracklist(Tracklist),
    Status(Status),
    Position(Duration),
}
