use std::sync::Arc;

use libadwaita::{Application, ApplicationWindow, prelude::*};
use qobuz_player_controls::{AppResult, client::Client};

mod ui;

pub async fn init(_client: Arc<Client>) -> AppResult<()> {
    libadwaita::init().unwrap();

    let application = libadwaita::Application::builder()
        .application_id("com.github.sofusa.qobuz-player")
        .build();

    application.connect_activate(build_ui);
    let args: &[&str] = &[];
    application.run_with_args(args);

    Ok(())
}

fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Qobuz Player")
        .default_width(900)
        .default_height(600)
        .build();

    let stack = libadwaita::ViewStack::builder().build();

    stack
        .add_titled(
            &gtk4::Label::new(Some("Albums Page")),
            Some("albums"),
            "Albums",
        )
        .set_icon_name(Some("media-optical-symbolic"));

    stack
        .add_titled(
            &gtk4::Label::new(Some("Artists Page")),
            Some("artists"),
            "Artists",
        )
        .set_icon_name(Some("avatar-default-symbolic"));

    stack
        .add_titled(
            &gtk4::Label::new(Some("Playlists Page")),
            Some("playlists"),
            "Playlists",
        )
        .set_icon_name(Some("media-playlist-repeat-symbolic"));

    stack
        .add_titled(
            &gtk4::Label::new(Some("Tracks Page")),
            Some("tracks"),
            "Tracks",
        )
        .set_icon_name(Some("audio-x-generic-symbolic"));

    stack
        .add_titled(
            &gtk4::Label::new(Some("Search Page")),
            Some("search"),
            "Search",
        )
        .set_icon_name(Some("system-search-symbolic"));

    let view_switcher = libadwaita::ViewSwitcher::builder()
        .stack(&stack)
        .policy(libadwaita::ViewSwitcherPolicy::Wide)
        .build();

    let header = libadwaita::HeaderBar::builder()
        .title_widget(&view_switcher)
        .build();

    let vbox = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .build();

    vbox.append(&header);
    vbox.append(&stack);

    window.set_content(Some(&vbox));
    window.show();
}
