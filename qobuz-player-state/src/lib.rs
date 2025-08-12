use std::sync::Arc;

use database::{Database, LinkRequest};
use qobuz_player_controls::{
    Broadcast, ReadOnly,
    client::Client,
    sink::Sink,
    tracklist::{self, Tracklist},
};
use tokio::sync::{Mutex, RwLock};

pub mod database;

pub struct State {
    pub client: Arc<Client>,
    pub rfid: bool,
    pub web_interface: String,
    pub web_secret: Option<String>,
    pub database: Database,
    pub link_request: Mutex<Option<LinkRequest>>,
    pub tracklist: ReadOnly<Tracklist>,
    pub target_status: ReadOnly<tracklist::Status>,
    pub broadcast: Arc<Broadcast>,
    pub sink: Arc<Sink>,
}

impl State {
    #[allow(clippy::too_many_arguments)]
    pub async fn new(
        client: Arc<Client>,
        rfid: bool,
        web_interface: String,
        web_secret: Option<String>,
        tracklist: Arc<RwLock<Tracklist>>,
        database: Database,
        target_status: ReadOnly<tracklist::Status>,
        broadcast: Arc<Broadcast>,
        sink: Arc<Sink>,
    ) -> Self {
        let link_request = Mutex::new(None);

        Self {
            client,
            rfid,
            web_interface,
            web_secret,
            database,
            link_request,
            tracklist: tracklist.into(),
            target_status,
            broadcast,
            sink,
        }
    }
}
