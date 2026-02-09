use qobuz_player_controls::{
    AppResult, TracklistReceiver,
    controls::Controls,
    database::{Database, LinkRequest, ReferenceType},
    error::Error,
    notification::NotificationBroadcast,
    tracklist,
};
use std::sync::Arc;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt},
    sync::Mutex,
};

#[derive(Debug, Clone, Default)]
pub struct RfidState {
    link_request: Arc<Mutex<Option<LinkRequest>>>,
}

pub async fn init(
    state: RfidState,
    tracklist_receiver: TracklistReceiver,
    controls: Controls,
    database: Arc<Database>,
    broadcast: Arc<NotificationBroadcast>,
) -> AppResult<()> {
    let mut reader = tokio::io::BufReader::new(tokio::io::stdin());
    let mut out = tokio::io::stdout();
    let mut line = String::new();

    loop {
        out.write_all(b"Scan RFID: ")
            .await
            .or(Err(Error::RfidInputPanic))?;
        out.flush().await.or(Err(Error::RfidInputPanic))?;

        line.clear();

        let n = reader
            .read_line(&mut line)
            .await
            .or(Err(Error::RfidInputPanic))?;
        if n == 0 {
            continue;
        }

        let res = line.trim();

        let maybe_request = {
            let guard = state.link_request.lock().await;
            guard.clone()
        };

        match maybe_request {
            Some(LinkRequest::Album(album_id)) => submit_link_album(
                state.clone(),
                database.clone(),
                broadcast.clone(),
                res,
                &album_id,
            ),
            Some(LinkRequest::Playlist(playlist_id)) => submit_link_playlist(
                state.clone(),
                database.clone(),
                broadcast.clone(),
                res,
                playlist_id,
            ),
            None => {
                handle_play_scan(&database, &controls, res, &tracklist_receiver).await;
            }
        };
    }
}

async fn handle_play_scan(
    database: &Arc<Database>,
    controls: &Controls,
    res: &str,
    tracklist_receiver: &TracklistReceiver,
) {
    let reference = match database.get_reference(res).await {
        Some(reference) => reference,
        None => {
            return;
        }
    };

    let tracklist = tracklist_receiver.borrow();
    let now_playing = tracklist.list_type();
    match reference {
        LinkRequest::Album(id) => {
            if let tracklist::TracklistType::Album(now_playing) = now_playing
                && now_playing.id == id
            {
                controls.play_pause();
                return;
            }
            controls.play_album(&id, 0);
        }
        LinkRequest::Playlist(id) => {
            if let tracklist::TracklistType::Playlist(now_playing) = now_playing
                && now_playing.id == id
            {
                controls.play_pause();
                return;
            }
            controls.play_playlist(id, 0, false);
        }
    }
}

pub async fn link(state: RfidState, request: LinkRequest, broadcast: Arc<NotificationBroadcast>) {
    set_state(&state, Some(request.clone())).await;

    let type_string = match request {
        LinkRequest::Album(_) => "album",
        LinkRequest::Playlist(_) => "playlist",
    };

    broadcast.send(qobuz_player_controls::notification::Notification::Info(
        format!("Scan rfid to link {type_string}"),
    ));

    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        let request_ongoing = state.link_request.lock().await.is_some();

        if request_ongoing {
            broadcast.send(qobuz_player_controls::notification::Notification::Warning(
                "Scan cancelled".to_string(),
            ));
            set_state(&state, None).await;
        }
    });
}

async fn set_state(state: &RfidState, request: Option<LinkRequest>) {
    let mut request_lock = state.link_request.lock().await;
    *request_lock = request;
}

fn submit_link_album(
    state: RfidState,
    database: Arc<Database>,
    broadcast: Arc<NotificationBroadcast>,
    rfid_id: &str,
    id: &str,
) {
    let reference = ReferenceType::Album(id.to_owned());
    submit_link(state, database, broadcast, rfid_id, reference);
}

fn submit_link_playlist(
    state: RfidState,
    database: Arc<Database>,
    broadcast: Arc<NotificationBroadcast>,
    rfid_id: &str,
    id: u32,
) {
    let reference = ReferenceType::Playlist(id);
    submit_link(state, database, broadcast, rfid_id, reference);
}

fn submit_link(
    state: RfidState,
    database: Arc<Database>,
    broadcast: Arc<NotificationBroadcast>,
    rfid_id: &str,
    reference: ReferenceType,
) {
    let rfid_id = rfid_id.to_owned();
    tokio::spawn(async move {
        match database.add_rfid_reference(rfid_id, reference).await {
            Ok(_) => {
                broadcast.send(qobuz_player_controls::notification::Notification::Success(
                    "Link completed".to_string(),
                ));
                set_state(&state, None).await;
            }
            Err(e) => {
                broadcast.send(qobuz_player_controls::notification::Notification::Error(
                    format!("{e}"),
                ));
                tracing::error!("{e}");
            }
        };
    });
}
