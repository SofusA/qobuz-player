use std::path::Path;

use handlebars::{Handlebars, handlebars_helper};
use qobuz_player_controls::Status;

macro_rules! views {
    ( $( $name:ident => $path:expr ),+ $(,)? ) => {
        #[derive(Clone, Copy, Debug)]
        pub(crate) enum View {
            $( $name ),+
        }

        impl View {
            pub fn path(self) -> &'static str {
                match self {
                    $( View::$name => $path ),+
                }
            }

            pub fn iter() -> impl Iterator<Item = View> {
                [ $( View::$name ),+ ].into_iter()
            }
        }
    }
}

views! {
    Page => "page.hbs",
    ErrorPage => "error-page.hbs",
    Error => "error.hbs",
    NowPlaying => "now-playing.hbs",
    LoadingSpinner => "icons/loading-spinner.hbs",
    VolumeSlider => "volume-slider.hbs",
    PlayPause => "play-pause.hbs",
    Play => "play.hbs",
    Pause => "pause.hbs",
    Next => "next.hbs",
    Previous => "previous.hbs",
    Progress => "progress.hbs",
    PlayerState => "player-state.hbs",
    Info => "info.hbs",
    BackwardIcon => "icons/backward-icon.hbs",
    ForwardIcon => "icons/forward-icon.hbs",
    PlayIcon => "icons/play-icon.hbs",
    PauseIcon => "icons/pause-icon.hbs",
    PlayCircleIcon => "icons/play-circle.hbs",
    Megaphone => "icons/megaphone.hbs",
    QueueList => "icons/queue-list.hbs",
    Star => "icons/star.hbs",
    StarSolid => "icons/star-solid.hbs",
    MagnifyingGlass => "icons/magnifying-glass.hbs",
    Navigation => "navigation.hbs",
    Controls => "controls.hbs",
    Queue => "queue.hbs",
    List => "list.hbs",
    ListItem => "list-item.hbs",
    ListTracks => "list-tracks.hbs",
    Favorites => "favorites.hbs"
}

impl View {
    pub(crate) fn name(&self) -> String {
        self.path()
            .split("/")
            .last()
            .unwrap()
            .trim_end_matches(".hbs")
            .into()
    }
}

#[cfg(not(debug_assertions))]
#[derive(rust_embed::Embed)]
#[folder = "templates"]
struct Templates;

pub(crate) fn templates(root_dir: &Path) -> Handlebars<'static> {
    let mut reg = Handlebars::new();
    #[cfg(debug_assertions)]
    reg.set_dev_mode(true);

    for file in View::iter() {
        let name = file.name();

        #[cfg(debug_assertions)]
        {
            let mut path = root_dir.to_path_buf();
            path.push(file.path());
            reg.register_template_file(&name, path).unwrap();
        }

        #[cfg(not(debug_assertions))]
        {
            let content = Templates::get(file.path()).unwrap();
            let content = String::from_utf8_lossy(&content.data);
            reg.register_template_string(&name, content).unwrap();
        }
    }

    reg.register_helper("msec-to-mmss", Box::new(mseconds_to_mm_ss));
    reg.register_helper("multiply", Box::new(multiply));
    reg.register_helper("ternary", Box::new(ternary));
    reg.register_helper("play-pause-api", Box::new(play_pause_api_string));
    reg.register_helper("list-callback", Box::new(list_callback));

    reg
}

handlebars_helper!(mseconds_to_mm_ss: |a: i64| {
    let seconds: i64= a / 1000;

    let minutes = seconds / 60;
    let seconds = seconds % 60;
    format!("{minutes:02}:{seconds:02}")
});

handlebars_helper!(multiply: |a: i64, b: i64| a * b);
handlebars_helper!(play_pause_api_string: |a: Status| {
    match a {
        Status::Paused | Status::Buffering => "/api/play",
        Status::Playing => "/api/pause"
    }
});

handlebars_helper!(ternary: |cond: bool, a: String, b: String| {
    match cond {
        true => a,
        false => b,
    }
});

handlebars_helper!(list_callback: |template: String, index: i32| {
    template.replace("@index", &format!("{index}"))
});
