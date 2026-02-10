use std::time::Duration;

#[derive(Debug)]
pub enum ControlCommand {
    Album {
        id: String,
        index: usize,
    },
    Playlist {
        id: u32,
        index: usize,
        shuffle: bool,
    },
    ArtistTopTracks {
        artist_id: u32,
        index: usize,
    },
    Track {
        id: u32,
    },
    SkipToPosition {
        new_position: usize,
        force: bool,
    },
    Next,
    Previous,
    PlayPause,
    Play,
    Pause,
    JumpForward,
    JumpBackward,
    Seek {
        time: Duration,
    },
    SetVolume {
        volume: f32,
    },
    AddTrackToQueue {
        id: u32,
    },
    RemoveIndexFromQueue {
        index: usize,
    },
    PlayTrackNext {
        id: u32,
    },
    ReorderQueue {
        new_order: Vec<usize>,
    },
    NewQueue {
        tracks: Vec<u32>,
        play: bool,
    },
    ClearQueue,
}

#[derive(Debug, Clone)]
pub struct Controls {
    tx: tokio::sync::mpsc::UnboundedSender<ControlCommand>,
}

impl Controls {
    pub fn new(tx: tokio::sync::mpsc::UnboundedSender<ControlCommand>) -> Self {
        Self { tx }
    }

    pub fn next(&self) {
        self.tx.send(ControlCommand::Next).expect("infallible");
    }

    pub fn previous(&self) {
        self.tx.send(ControlCommand::Previous).expect("infallible");
    }

    pub fn play_pause(&self) {
        self.tx.send(ControlCommand::PlayPause).expect("infallible");
    }

    pub fn play(&self) {
        self.tx.send(ControlCommand::Play).expect("infallible");
    }

    pub fn pause(&self) {
        self.tx.send(ControlCommand::Pause).expect("infallible");
    }

    pub fn play_album(&self, id: &str, index: usize) {
        self.tx
            .send(ControlCommand::Album {
                id: id.to_string(),
                index,
            })
            .expect("infallible");
    }

    pub fn play_playlist(&self, id: u32, index: usize, shuffle: bool) {
        self.tx
            .send(ControlCommand::Playlist { id, index, shuffle })
            .expect("infallible");
    }

    pub fn play_track(&self, id: u32) {
        self.tx
            .send(ControlCommand::Track { id })
            .expect("infallible");
    }

    pub fn add_track_to_queue(&self, id: u32) {
        self.tx
            .send(ControlCommand::AddTrackToQueue { id })
            .expect("infallible");
    }

    pub fn remove_index_from_queue(&self, index: usize) {
        self.tx
            .send(ControlCommand::RemoveIndexFromQueue { index })
            .expect("infallible");
    }

    pub fn play_track_next(&self, id: u32) {
        self.tx
            .send(ControlCommand::PlayTrackNext { id })
            .expect("infallible");
    }

    pub fn play_top_tracks(&self, artist_id: u32, index: usize) {
        self.tx
            .send(ControlCommand::ArtistTopTracks { artist_id, index })
            .expect("infallible");
    }

    pub fn skip_to_position(&self, index: usize, force: bool) {
        self.tx
            .send(ControlCommand::SkipToPosition {
                new_position: index,
                force,
            })
            .expect("infallible");
    }

    pub fn set_volume(&self, volume: f32) {
        self.tx
            .send(ControlCommand::SetVolume { volume })
            .expect("infallible");
    }

    pub fn seek(&self, time: Duration) {
        self.tx
            .send(ControlCommand::Seek { time })
            .expect("infallible");
    }

    pub fn jump_forward(&self) {
        self.tx
            .send(ControlCommand::JumpForward)
            .expect("infallible");
    }

    pub fn jump_backward(&self) {
        self.tx
            .send(ControlCommand::JumpBackward)
            .expect("infallible");
    }

    pub fn reorder_queue(&self, new_order: Vec<usize>) {
        self.tx
            .send(ControlCommand::ReorderQueue { new_order })
            .expect("infallible");
    }

    pub fn new_queue(&self, tracks: Vec<u32>, play: bool) {
        self.tx
            .send(ControlCommand::NewQueue { tracks, play })
            .expect("infallible");
    }

    pub fn clear_queue(&self) {
        self.tx
            .send(ControlCommand::ClearQueue)
            .expect("infallible");
    }
}
