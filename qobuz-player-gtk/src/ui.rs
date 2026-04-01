use gtk4::{Image, gdk, gio, prelude::*};
use qobuz_player_controls::{controls::Controls, models::AlbumSimple};

pub mod albums_page;
pub mod now_playing_bar;
pub mod search_page;

pub fn set_image_from_url(url: Option<&str>, image: &Image) {
    if let Some(url) = url {
        let file = gio::File::for_uri(url);

        match gdk::Texture::from_file(&file) {
            Ok(texture) => {
                image.set_paintable(Some(&texture));
            }
            Err(err) => {
                eprintln!("Failed to load image: {err}");
                image.set_paintable(None::<&gdk::Paintable>);
            }
        }
    } else {
        image.set_paintable(None::<&gdk::Paintable>);
    }
}

pub fn build_album_tile(album: &AlbumSimple, controls: Controls) -> gtk4::Box {
    let vbox = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(6)
        .width_request(150)
        .build();

    let cover = gtk4::Image::builder().pixel_size(200).build();
    set_image_from_url(Some(&album.image), &cover);
    let cover_frame = gtk4::Frame::builder().child(&cover).build();
    cover_frame.add_css_class("card");

    let title = gtk4::Label::builder()
        .label(&album.title)
        .xalign(0.0)
        .wrap(true)
        .max_width_chars(20)
        .build();

    let artist = gtk4::Label::builder()
        .label(&album.artist.name)
        .xalign(0.0)
        .css_classes(vec![String::from("dim-label")])
        .wrap(true)
        .max_width_chars(20)
        .build();

    vbox.append(&cover_frame);
    vbox.append(&title);
    vbox.append(&artist);

    let controls_clone = controls.clone();
    let album_id = album.id.clone();

    let click = gtk4::GestureClick::new();
    click.connect_pressed(move |_, _, _, _| {
        controls_clone.play_album(&album_id, 0);
    });

    vbox.add_controller(click);
    vbox
}
