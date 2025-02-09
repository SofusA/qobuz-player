use serde::{Deserialize, Serialize};

use super::{
    artist::OtherArtists,
    release::{Artist, AudioInfo, PhysicalSupport, Rights},
};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtistName {
    pub display: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtistPage {
    pub id: u32,
    pub name: ArtistName,
    pub images: Images,
    pub top_tracks: Vec<Track>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub id: u32,
    pub album: Album,
    pub isrc: String,
    pub title: String,
    pub artist: Artist,
    pub artists: Vec<OtherArtists>,
    pub duration: u32,
    pub parental_warning: bool,
    pub audio_info: AudioInfo,
    pub rights: Rights,
    pub physical_support: PhysicalSupport,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Album {
    pub id: String,
    pub title: String,
    pub image: super::Image,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Biography {
    pub content: String,
    language: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Images {
    pub portrait: Option<Image>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Image {
    pub hash: String,
    pub format: String,
}
