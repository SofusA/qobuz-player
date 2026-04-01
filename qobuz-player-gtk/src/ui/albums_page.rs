use adw::prelude::*;
use gtk4 as gtk;
use libadwaita as adw;
use qobuz_player_models::AlbumSimple;

pub struct AlbumsPage {
    pub widget: gtk::ScrolledWindow,
}

impl AlbumsPage {
    pub fn new(albums: Vec<AlbumSimple>) -> Self {
        let scroller = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .build();

        let flow = gtk::FlowBox::builder()
            .valign(gtk::Align::Start)
            .halign(gtk::Align::Start)
            .selection_mode(gtk::SelectionMode::None)
            .max_children_per_line(6)
            .min_children_per_line(2)
            .row_spacing(12)
            .column_spacing(12)
            .build();

        for album in albums {
            let tile = build_album_tile(&album);
            flow.insert(&tile, -1);
        }

        scroller.set_child(Some(&flow));

        Self { widget: scroller }
    }

    pub fn widget(&self) -> &gtk::ScrolledWindow {
        &self.widget
    }
}

fn build_album_tile(album: &AlbumSimple) -> gtk::Box {
    let vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .width_request(150)
        .build();

    let picture = gtk::Picture::builder()
        .width_request(150)
        .height_request(150)
        .build();

    picture.set_filename(Some(&album.image));

    let title = gtk::Label::builder()
        .label(&album.title)
        .xalign(0.0)
        .wrap(true)
        .max_width_chars(20)
        .build();

    let artist = gtk::Label::builder()
        .label(&album.artist.name)
        .xalign(0.0)
        .css_classes(vec![String::from("dim-label")])
        .wrap(true)
        .max_width_chars(20)
        .build();

    vbox.append(&picture);
    vbox.append(&title);
    vbox.append(&artist);

    vbox
}
