use std::sync::Arc;

use qobuz_player_controls::{
    database::LinkRequest,
    notification::NotificationBroadcast,
};

use crate::RfidState;

#[cfg(feature = "rfid")]
pub async fn link(
    state: RfidState,
    request: LinkRequest,
    broadcast: Arc<NotificationBroadcast>,
) {
    qobuz_player_rfid::link(state, request, broadcast).await;
}

#[cfg(not(feature = "rfid"))]
pub async fn link(
    _state: RfidState,
    _request: LinkRequest,
    _broadcast: Arc<NotificationBroadcast>,
) {
    // RFID non compilato: no-op.
}
