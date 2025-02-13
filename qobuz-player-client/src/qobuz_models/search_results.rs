use serde::{Deserialize, Serialize};

use crate::qobuz_models::{album::Albums, artist::Artist, playlist::Playlists, track::Track};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchAllResults {
    pub query: String,
    pub albums: Albums,
    pub tracks: Tracks,
    pub artists: Artists,
    pub playlists: Playlists,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Analytics {
    #[serde(rename = "search_external_id")]
    pub search_external_id: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tracks {
    pub limit: i64,
    pub offset: i64,
    pub analytics: Analytics,
    pub total: i64,
    pub items: Vec<Track>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artists {
    pub limit: i64,
    pub offset: i64,
    pub analytics: Analytics,
    pub total: i64,
    pub items: Vec<Artist>,
}
