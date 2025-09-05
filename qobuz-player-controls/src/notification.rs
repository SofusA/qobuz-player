use crate::{
    Time,
    tracklist::{self, Tracklist},
};

#[derive(Debug, Clone, PartialEq)]
pub enum Notification {
    Play(PlayNotification),
    Status { status: tracklist::Status },
    Position { position: Time },
    CurrentTrackList { tracklist: Tracklist },
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

#[derive(Debug, Clone, PartialEq)]
pub enum PlayNotification {
    Album { id: String, index: u32 },
    Playlist { id: u32, index: u32, shuffle: bool },
    ArtistTopTracks { artist_id: u32, index: u32 },
    Track { id: u32 },
    SkipToPosition { new_position: u32, force: bool },
    Next,
    Previous,
    PlayPause,
    Play,
    Pause,
    JumpForward,
    JumpBackward,
    Seek { time: Time },
    TrackFinished,
}
