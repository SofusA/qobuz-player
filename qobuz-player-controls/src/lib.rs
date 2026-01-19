use crate::{error::Error, tracklist::Tracklist};

use std::time::Duration;
use tokio::sync::{broadcast, watch};

pub use qobuz_player_client::client::AudioQuality;

pub mod client;
pub mod controls;
pub mod database;
pub mod downloader;
pub mod error;
pub mod notification;
pub mod player;
pub mod simple_cache;
pub mod sink;
pub mod tracklist;

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub type PositionReceiver = watch::Receiver<Duration>;
pub type VolumeReceiver = watch::Receiver<f32>;
pub type StatusReceiver = watch::Receiver<Status>;
pub type TracklistReceiver = watch::Receiver<Tracklist>;

#[derive(Default, Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Status {
    Playing,
    Buffering,
    #[default]
    Paused,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum Notification {
    Error(String),
    Warning(String),
    Success(String),
    Info(String),
}

pub type ExitReceiver = broadcast::Receiver<bool>;
pub type ExitSender = broadcast::Sender<bool>;
