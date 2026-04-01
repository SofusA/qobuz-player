use std::sync::Arc;

use gtk4::prelude::*;
use qobuz_player_controls::{client::Client, controls::Controls};

use crate::ui::build_album_tile;

pub struct AlbumsPage {
    widget: gtk4::Stack,
    flow: gtk4::FlowBox,
    controls: Controls,
    client: Arc<Client>,
}

impl AlbumsPage {
    pub fn new(controls: Controls, client: Arc<Client>) -> Self {
        let spinner = gtk4::Spinner::new();
        spinner.start();

        let spinner_box = gtk4::Box::builder()
            .vexpand(true)
            .hexpand(true)
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .build();

        spinner_box.append(&spinner);

        let flow = gtk4::FlowBox::builder()
            .valign(gtk4::Align::Start)
            .halign(gtk4::Align::Center)
            .selection_mode(gtk4::SelectionMode::None)
            .row_spacing(12)
            .column_spacing(12)
            .build();

        let scroller = gtk4::ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .child(&flow)
            .build();

        let stack = gtk4::Stack::builder()
            .transition_type(gtk4::StackTransitionType::Crossfade)
            .build();

        stack.add_named(&spinner_box, Some("loading"));
        stack.add_named(&scroller, Some("content"));
        stack.set_visible_child_name("loading");

        Self {
            widget: stack,
            flow,
            controls,
            client,
        }
    }

    pub fn widget(&self) -> &gtk4::Stack {
        &self.widget
    }

    pub fn load(&self) {
        let flow = self.flow.clone();
        let controls = self.controls.clone();
        let client = self.client.clone();
        let stack = self.widget.clone();

        stack.set_visible_child_name("loading");

        glib::spawn_future_local(async move {
            match client.favorites().await {
                Ok(favorites) => {
                    clear_flowbox(&flow);

                    for album in favorites.albums {
                        let tile = build_album_tile(&album, controls.clone());
                        flow.insert(&tile, -1);
                    }

                    stack.set_visible_child_name("content");
                }
                Err(err) => {
                    clear_flowbox(&flow);
                    eprintln!("Favorites failed: {err}");

                    stack.set_visible_child_name("content");
                }
            }
        });
    }
}

fn clear_flowbox(flow: &gtk4::FlowBox) {
    while let Some(child) = flow.first_child() {
        flow.remove(&child);
    }
}
