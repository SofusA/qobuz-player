use std::time::Duration;

pub use qobuz_player_client::client::AudioQuality;
use tokio::sync::watch;

use crate::{error::Error, tracklist::Tracklist};
pub mod client;
pub mod controls;
pub mod database;
pub mod error;
pub mod notification;
pub mod player;
pub(crate) mod simple_cache;
pub mod sink;
pub mod timer;
pub mod tracklist;

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub type PositionReceiver = watch::Receiver<Duration>;
pub type VolumeReceiver = watch::Receiver<f32>;
pub type StatusReceiver = watch::Receiver<Status>;
pub type TracklistReceiver = watch::Receiver<Tracklist>;

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub enum Status {
    Playing,
    Buffering,
    #[default]
    Paused,
}
