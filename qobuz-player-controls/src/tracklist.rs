use std::ops::Index;

use qobuz_player_models::{Track, TrackStatus};

#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct AlbumTracklist {
    pub title: String,
    pub id: String,
    pub image: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct PlaylistTracklist {
    pub title: String,
    pub id: u32,
    pub image: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct TopTracklist {
    pub artist_name: String,
    pub id: u32,
    pub image: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct SingleTracklist {
    pub track_title: String,
    pub album_id: Option<String>,
    pub image: Option<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum TracklistType {
    Album(AlbumTracklist),
    Playlist(PlaylistTracklist),
    TopTracks(TopTracklist),
    Track(SingleTracklist),
    #[default]
    None,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Tracklist {
    pub(crate) queue: Vec<Track>,
    pub(crate) list_type: TracklistType,
}

impl Tracklist {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn queue(&self) -> &Vec<Track> {
        &self.queue
    }

    pub fn total(&self) -> u32 {
        self.queue.len() as u32
    }

    pub fn currently_playing(&self) -> Option<u32> {
        self.queue
            .iter()
            .find(|t| t.status == TrackStatus::Playing)
            .map(|x| x.id)
    }

    pub fn current_position(&self) -> u32 {
        self.queue
            .iter()
            .enumerate()
            .find(|t| t.1.status == TrackStatus::Playing)
            .map(|x| x.0 as u32)
            .unwrap_or(0)
    }

    pub fn list_type(&self) -> &TracklistType {
        &self.list_type
    }

    pub fn reset(&mut self) {
        for track in self.queue.iter_mut() {
            if track.status == TrackStatus::Played || track.status == TrackStatus::Playing {
                track.status = TrackStatus::Unplayed;
            }
        }

        if let Some(first_track) = self
            .queue
            .iter_mut()
            .find(|t| t.status == TrackStatus::Unplayed)
        {
            first_track.status = TrackStatus::Playing;
        }
    }

    pub fn next_track(&self) -> Option<&Track> {
        let current_position = self.current_position();
        let next_position = current_position + 1;
        if self.total() <= next_position {
            return None;
        }

        Some(self.queue.index(next_position as usize))
    }

    pub fn current_track(&self) -> Option<&Track> {
        self.queue.iter().find(|t| t.status == TrackStatus::Playing)
    }

    pub(crate) fn skip_to_track(&mut self, new_position: u32) -> Option<&Track> {
        let mut new_track: Option<&Track> = None;

        for queue_item in self.queue.iter_mut().enumerate() {
            let queue_item_position = queue_item.0 as u32;

            match queue_item_position.cmp(&new_position) {
                std::cmp::Ordering::Less => {
                    queue_item.1.status = TrackStatus::Played;
                }

                std::cmp::Ordering::Equal => {
                    queue_item.1.status = TrackStatus::Playing;

                    new_track = Some(queue_item.1)
                }

                std::cmp::Ordering::Greater => {
                    queue_item.1.status = TrackStatus::Unplayed;
                }
            }
        }

        new_track
    }
}
