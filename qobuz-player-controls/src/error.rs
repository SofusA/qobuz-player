use crate::notification::Notification;
use snafu::prelude::*;

#[derive(Snafu, Debug, Clone, PartialEq)]
pub enum Error {
    #[snafu(display("{message}"))]
    FailedToPlay {
        message: String,
    },
    #[snafu(display("failed to retrieve a track url"))]
    TrackURL,
    #[snafu(display("failed to seek"))]
    Seek,
    #[snafu(display("sorry, could not resume previous session"))]
    Resume,
    #[snafu(display("{message}"))]
    Client {
        message: String,
    },
    Notification,
    App,
    StreamError {
        message: String,
    },
}

impl From<rodio::source::SeekError> for Error {
    fn from(_: rodio::source::SeekError) -> Self {
        Error::Seek
    }
}

impl From<rodio::StreamError> for Error {
    fn from(value: rodio::StreamError) -> Self {
        Self::StreamError {
            message: value.to_string(),
        }
    }
}

impl From<rodio::decoder::DecoderError> for Error {
    fn from(value: rodio::decoder::DecoderError) -> Self {
        Self::StreamError {
            message: value.to_string(),
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        Self::StreamError {
            message: value.to_string(),
        }
    }
}

impl From<qobuz_player_client::Error> for Error {
    fn from(value: qobuz_player_client::Error) -> Self {
        Error::Client {
            message: value.to_string(),
        }
    }
}

impl From<tokio::sync::broadcast::error::SendError<Notification>> for Error {
    fn from(_value: tokio::sync::broadcast::error::SendError<Notification>) -> Self {
        Self::Notification
    }
}
