use crate::notification::Notification;
use snafu::prelude::*;

#[derive(Snafu, Debug)]
pub enum Error {
    #[snafu(display("{message}"))]
    FailedToPlay {
        message: String,
    },
    #[snafu(display("failed to seek"))]
    Seek,
    #[snafu(display("{message}"))]
    Client {
        message: String,
    },
    #[snafu(display("Unable to broadcast notification"))]
    Notification,
    #[snafu(display("Unable to start stream: {message}"))]
    StreamError {
        message: String,
    },
    PoisonError,
    SendError,
    #[snafu(display("Unable to init mpris. Is address already taken?"))]
    MprisInitError,
    #[snafu(display("Unable to set mpris property: {property}"))]
    MprisPropertyError {
        property: String,
    },
    #[snafu(display("Unable to connect to database"))]
    DatabaseConnectError,
    #[snafu(display("Unable to migrate database to latest version"))]
    DatabaseMigrationError,
    #[snafu(display("Unable to find database location"))]
    DatabaseLocationError,
    #[snafu(display("Database error: {source}"))]
    DatabaseError {
        #[snafu(source)]
        source: sqlx::Error,
    },
    #[snafu(display("Serialization error: {source}"))]
    SerializationError {
        #[snafu(source)]
        source: serde_json::Error,
    },
    #[snafu(display("Gpio pin {pin} is unavailable"))]
    GpioUnavailable {
        pin: u8,
    },
    #[snafu(display("Rfid prompt input error"))]
    RfidInputPanic,
    #[snafu(display("Port already in use: {port}"))]
    PortInUse {
        port: u16,
    },
}

impl From<sqlx::migrate::MigrateError> for Error {
    fn from(_value: sqlx::migrate::MigrateError) -> Self {
        Self::DatabaseMigrationError
    }
}

impl From<serde_json::Error> for Error {
    fn from(source: serde_json::Error) -> Self {
        Self::SerializationError { source }
    }
}

impl From<sqlx::Error> for Error {
    fn from(source: sqlx::Error) -> Self {
        Self::DatabaseError { source }
    }
}

impl<T> From<tokio::sync::watch::error::SendError<T>> for Error {
    fn from(_: tokio::sync::watch::error::SendError<T>) -> Self {
        Error::SendError
    }
}

impl<T> From<std::sync::PoisonError<T>> for Error {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        Error::PoisonError
    }
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

impl From<rodio::DevicesError> for Error {
    fn from(value: rodio::DevicesError) -> Self {
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
