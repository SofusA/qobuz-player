use snafu::prelude::*;

pub mod client;
pub mod qobuz_models;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("No username provided."))]
    NoPassword,
    #[snafu(display("No password provided."))]
    NoUsername,
    #[snafu(display("No username or password provided."))]
    NoCredentials,
    #[snafu(display("Failed to get a usable secret from Qobuz."))]
    ActiveSecret,
    #[snafu(display("Failed to get an app id from Qobuz."))]
    AppID,
    #[snafu(display("Failed to login."))]
    Login,
    #[snafu(display("Authorization missing."))]
    Authorization,
    #[snafu(display("Failed to create client"))]
    Create,
    #[snafu(display("{message}"))]
    Api { message: String },
    #[snafu(display("Failed to deserialize json: {message}"))]
    DeserializeJSON { message: String },
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        let status = error.status();

        match status {
            Some(status) => Error::Api {
                message: status.to_string(),
            },
            None => Error::Api {
                message: "Error calling the API".to_string(),
            },
        }
    }
}
