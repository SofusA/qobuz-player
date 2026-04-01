use crate::ui::build_album_tile;

use std::sync::Arc;

use gtk4::prelude::*;
use gtk4::{Align, glib};
use qobuz_player_controls::client::Client;
use qobuz_player_controls::controls::Controls;

pub struct SearchPage {
    root: gtk4::Box,
}

impl SearchPage {
    pub fn new(client: Arc<Client>, controls: Controls) -> Self {
        let search_entry = gtk4::SearchEntry::builder()
            .placeholder_text("Search albums…")
            .hexpand(true)
            .build();

        let flow = gtk4::FlowBox::builder()
            .valign(Align::Start)
            .halign(Align::Center)
            .selection_mode(gtk4::SelectionMode::None)
            .row_spacing(12)
            .column_spacing(12)
            .hexpand(true)
            .build();

        let scroller = gtk4::ScrolledWindow::builder()
            .child(&flow)
            .vexpand(true)
            .hexpand(true)
            .build();

        let root = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();

        root.append(&search_entry);
        root.append(&scroller);

        let client = client.clone();
        let controls = controls.clone();
        let flow = flow.clone();

        search_entry.connect_activate(move |entry| {
            let query = entry.text().to_string();

            if query.trim().len() < 2 {
                clear_flowbox(&flow);
                return;
            }

            clear_flowbox(&flow);

            let spinner = gtk4::Spinner::new();
            spinner.start();
            flow.insert(&spinner, 0);

            let client = client.clone();
            let controls = controls.clone();
            let flow = flow.clone();

            glib::MainContext::default().spawn_local(async move {
                match client.search(query).await {
                    Ok(search) => {
                        clear_flowbox(&flow);

                        for album in search.albums {
                            let tile = build_album_tile(&album.into(), controls.clone());
                            flow.insert(&tile, -1);
                        }
                    }
                    Err(err) => {
                        clear_flowbox(&flow);
                        eprintln!("Search failed: {err}");
                    }
                }
            });
        });

        Self { root }
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.root
    }
}

fn clear_flowbox(flow: &gtk4::FlowBox) {
    while let Some(child) = flow.first_child() {
        flow.remove(&child);
    }
}
