use std::fmt;

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum SubTab {
    #[default]
    Albums = 0,
    Artists = 1,
    Playlists = 2,
    Tracks = 3,
}

impl SubTab {
    pub(crate) const COUNT: u8 = 4;

    pub(crate) fn selected(self) -> u8 {
        self as u8
    }

    pub(crate) fn from_u8(v: u8) -> Self {
        match v % Self::COUNT {
            0 => Self::Albums,
            1 => Self::Artists,
            2 => Self::Playlists,
            _ => Self::Tracks,
        }
    }

    pub(crate) fn next(self) -> Self {
        Self::from_u8(self.selected().wrapping_add(1))
    }

    pub(crate) fn previous(self) -> Self {
        Self::from_u8(self.selected().wrapping_add(Self::COUNT - 1))
    }

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Albums => "Albums",
            Self::Artists => "Artists",
            Self::Playlists => "Playlists",
            Self::Tracks => "Tracks",
        }
    }

    pub(crate) const VALUES: [Self; Self::COUNT as usize] =
        [Self::Albums, Self::Artists, Self::Playlists, Self::Tracks];

    pub(crate) fn labels() -> Vec<&'static str> {
        Self::VALUES.iter().map(|tab| tab.as_str()).collect()
    }
}

impl fmt::Display for SubTab {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
