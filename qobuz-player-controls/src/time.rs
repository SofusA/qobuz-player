use std::ops::{Add, Sub};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Debug)]
pub struct Time(u64);

impl Time {
    pub fn from_mseconds(value: u64) -> Self {
        Self(value)
    }

    pub fn from_seconds(value: u64) -> Self {
        Self(value * 1000)
    }

    pub fn mseconds(&self) -> u64 {
        self.0
    }

    pub fn seconds(&self) -> u64 {
        self.0 / 1000
    }
}

impl Add for Time {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Time::from_mseconds(self.0 + rhs.0)
    }
}

impl Sub for Time {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Time::from_mseconds(self.0 - rhs.0)
    }
}
