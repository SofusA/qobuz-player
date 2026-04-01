use std::time::Duration;

use libadwaita::prelude::*;
use qobuz_player_controls::{Status, controls::Controls, models::Track};

use crate::ui::set_image_from_url;

#[derive(Clone)]
pub struct NowPlayingBar {
    pub revealer: gtk4::Revealer,
    pub title_label: gtk4::Label,
    pub subtitle_label: gtk4::Label,
    pub cover: gtk4::Image,
    pub play_button: gtk4::Button,

    pub progress_scale: gtk4::Scale,
    pub progress_current_label: gtk4::Label,
    pub progress_total_label: gtk4::Label,
}

pub fn build_now_playing_bar(controls: Controls) -> NowPlayingBar {
    let title_label = gtk4::Label::builder()
        .xalign(0.0)
        .ellipsize(gtk4::pango::EllipsizeMode::End)
        .wrap(false)
        .build();
    title_label.add_css_class("title-3");

    let subtitle_label = gtk4::Label::builder()
        .xalign(0.0)
        .ellipsize(gtk4::pango::EllipsizeMode::End)
        .wrap(false)
        .build();
    subtitle_label.add_css_class("dim-label");

    let text_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(4)
        .hexpand(true)
        .build();
    text_box.append(&title_label);
    text_box.append(&subtitle_label);

    let controls_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(12)
        .halign(gtk4::Align::Center)
        .build();

    let controls_prev = controls.clone();
    let prev_button = gtk4::Button::builder()
        .icon_name("media-seek-backward-symbolic")
        .build();
    prev_button.add_css_class("flat");
    prev_button.connect_clicked(move |_| controls_prev.previous());

    let controls_play_pause = controls.clone();
    let play_button = gtk4::Button::builder()
        .icon_name("media-playback-start-symbolic")
        .build();
    play_button.add_css_class("flat");
    play_button.connect_clicked(move |_| controls_play_pause.play_pause());

    let controls_next = controls.clone();
    let next_button = gtk4::Button::builder()
        .icon_name("media-seek-forward-symbolic")
        .build();
    next_button.add_css_class("flat");
    next_button.connect_clicked(move |_| controls_next.next());

    controls_box.append(&prev_button);
    controls_box.append(&play_button);
    controls_box.append(&next_button);

    let progress_current_label = gtk4::Label::builder()
        .label("0:00")
        .width_chars(6)
        .xalign(0.0)
        .build();

    let progress_total_label = gtk4::Label::builder()
        .label("0:00")
        .width_chars(6)
        .xalign(1.0)
        .build();

    let progress_scale = gtk4::Scale::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .hexpand(true)
        .draw_value(false)
        .focusable(false)
        .build();

    let controls_seek = controls.clone();
    progress_scale.connect_change_value(move |_, _, value| {
        controls_seek.seek(Duration::from_millis(value as u64));
        glib::Propagation::Stop
    });

    let progress_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .hexpand(true)
        .build();

    progress_box.append(&progress_current_label);
    progress_box.append(&progress_scale);
    progress_box.append(&progress_total_label);

    let left_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(8)
        .hexpand(true)
        .valign(gtk4::Align::Center)
        .build();

    left_box.append(&text_box);
    left_box.append(&controls_box);
    left_box.append(&progress_box);

    let cover = gtk4::Image::builder().pixel_size(200).build();
    let cover_frame = gtk4::Frame::builder().child(&cover).build();
    cover_frame.add_css_class("card");

    let content = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .margin_start(8)
        .margin_end(8)
        .margin_top(8)
        .margin_bottom(8)
        .build();

    content.append(&left_box);
    content.append(&cover_frame);

    let frame = gtk4::Frame::builder().child(&content).build();
    frame.add_css_class("content");

    let revealer = gtk4::Revealer::builder()
        .transition_type(gtk4::RevealerTransitionType::SlideUp)
        .child(&frame)
        .reveal_child(false)
        .build();

    NowPlayingBar {
        revealer,
        title_label,
        subtitle_label,
        cover,
        play_button,
        progress_scale,
        progress_current_label,
        progress_total_label,
    }
}

pub fn update_now_playing(bar: &NowPlayingBar, track: &Track) {
    bar.title_label.set_text(&track.title);

    let subtitle = match (&track.album_title, &track.artist_name) {
        (Some(album), Some(artist)) => format!("{album} · {artist}"),
        (Some(album), None) => album.clone(),
        (None, Some(artist)) => artist.clone(),
        _ => String::new(),
    };
    bar.subtitle_label.set_text(&subtitle);

    bar.progress_scale
        .set_range(0.0, (track.duration_seconds * 1000) as f64);
    bar.progress_total_label
        .set_text(&format_time(track.duration_seconds));

    set_image_from_url(track.image.as_deref(), &bar.cover);

    bar.revealer.set_reveal_child(true);
}

pub fn update_progress(bar: &NowPlayingBar, position: &Duration) {
    animate_scale_to(&bar.progress_scale, position.as_millis() as f64, 120);

    bar.progress_current_label
        .set_text(&format_time(position.as_secs() as u32));
}

pub fn update_now_playing_button_icon(status: &Status, button: &gtk4::Button) {
    match status {
        Status::Playing => button.set_icon_name("media-playback-pause-symbolic"),
        Status::Buffering => button.set_icon_name("content-loading-symbolic"),
        Status::Paused => button.set_icon_name("media-playback-start-symbolic"),
    }
}

fn format_time(seconds: u32) -> String {
    let m = seconds / 60;
    let s = seconds % 60;
    format!("{m}:{s:02}")
}

fn animate_scale_to(scale: &gtk4::Scale, target: f64, duration_ms: u32) {
    let adjustment = scale.adjustment();
    let start = adjustment.value();
    let delta = target - start;

    let start_time = std::time::Instant::now();

    scale.add_tick_callback(move |_, _| {
        let elapsed = start_time.elapsed().as_millis() as u32;
        let t = (elapsed as f64 / duration_ms as f64).min(1.0);

        let eased = 1.0 - (1.0 - t).powi(3);

        adjustment.set_value(start + delta * eased);

        if t >= 1.0 {
            gtk4::glib::ControlFlow::Break
        } else {
            gtk4::glib::ControlFlow::Continue
        }
    });
}
