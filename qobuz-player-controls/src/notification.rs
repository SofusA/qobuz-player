use gstreamer::ClockTime;

use crate::tracklist::{self, Tracklist};

#[derive(Debug, Clone, PartialEq)]
pub enum Notification {
    Status { status: tracklist::Status },
    Position { clock: ClockTime },
    CurrentTrackList { list: Tracklist },
    Quit,
    Message { message: Message },
    Volume { volume: f64 },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Error(String),
    Warning(String),
    Success(String),
    Info(String),
}
