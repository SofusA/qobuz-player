#[derive(Debug, Default, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum TrackStatus {
    Played,
    Playing,
    #[default]
    Unplayed,
    Unplayable,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Track {
    pub id: u32,
    pub title: String,
    pub number: u32,
    pub explicit: bool,
    pub hires_available: bool,
    pub available: bool,
    pub status: TrackStatus,
    pub image: Option<String>,
    pub image_thumbnail: Option<String>,
    pub duration_seconds: u32,
    pub artist_name: Option<String>,
    pub artist_id: Option<u32>,
    pub album_title: Option<String>,
    pub album_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Album {
    pub id: String,
    pub title: String,
    pub artist: Artist,
    pub release_year: u32,
    pub hires_available: bool,
    pub explicit: bool,
    pub total_tracks: u32,
    pub tracks: Vec<Track>,
    pub available: bool,
    pub image: String,
    pub image_thumbnail: String,
    pub duration_seconds: u32,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AlbumSimple {
    pub id: String,
    pub title: String,
    pub artist: Artist,
    pub image: String,
    pub available: bool,
    pub hires_available: bool,
    pub explicit: bool,
}

#[derive(Default, Debug, Clone)]
pub struct SearchResults {
    pub query: String,
    pub albums: Vec<Album>,
    pub artists: Vec<Artist>,
    pub playlists: Vec<Playlist>,
    pub tracks: Vec<Track>,
}

#[derive(Default, Debug, Clone)]
pub struct Favorites {
    pub albums: Vec<Album>,
    pub artists: Vec<Artist>,
    pub playlists: Vec<Playlist>,
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct Artist {
    pub id: u32,
    pub name: String,
    pub image: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct ArtistPage {
    pub id: u32,
    pub name: String,
    pub image: Option<String>,
    pub top_tracks: Vec<Track>,
    pub description: Option<String>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Playlist {
    pub is_owned: bool,
    pub title: String,
    pub duration_seconds: u32,
    pub tracks_count: u32,
    pub id: u32,
    pub image: Option<String>,
    pub tracks: Vec<Track>,
}
