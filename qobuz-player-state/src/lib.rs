use std::sync::Arc;

use database::{Database, LinkRequest};
use qobuz_player_controls::tracklist::Tracklist;
use tokio::sync::{Mutex, RwLock};

pub mod database;

pub struct State {
    pub rfid: bool,
    pub web_interface: String,
    pub web_secret: Option<String>,
    pub database: Database,
    pub link_request: Mutex<Option<LinkRequest>>,
    pub tracklist: ReadOnly<Tracklist>,
}

impl State {
    pub async fn new(
        rfid: bool,
        web_interface: String,
        web_secret: Option<String>,
        tracklist: Arc<RwLock<Tracklist>>,
        database: Database,
    ) -> Self {
        let link_request = Mutex::new(None);

        Self {
            rfid,
            web_interface,
            web_secret,
            database,
            link_request,
            tracklist: tracklist.into(),
        }
    }
}

pub struct ReadOnly<T>(Arc<RwLock<T>>);

impl<T> Clone for ReadOnly<T> {
    fn clone(&self) -> Self {
        ReadOnly(self.0.clone())
    }
}

impl<T> ReadOnly<T> {
    pub async fn read(&self) -> tokio::sync::RwLockReadGuard<'_, T> {
        self.0.read().await
    }
}

impl<T> From<Arc<RwLock<T>>> for ReadOnly<T> {
    fn from(arc: Arc<RwLock<T>>) -> Self {
        ReadOnly(arc)
    }
}
