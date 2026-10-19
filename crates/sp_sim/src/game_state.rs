use serde::{Deserialize, Serialize};
use std::ops::{Index, IndexMut};

/// Which side of the ball.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Home,
    Away,
}

impl Side {
    pub const ALL: [Side; 2] = [Side::Home, Side::Away];

    pub fn other(self) -> Side {
        match self {
            Side::Home => Side::Away,
            Side::Away => Side::Home,
        }
    }

    pub fn idx(self) -> usize {
        self as usize
    }

    pub fn name(self) -> &'static str {
        match self {
            Side::Home => "home",
            Side::Away => "away",
        }
    }
}

/// One value per side, indexable by [`Side`].
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Sides<T> {
    pub home: T,
    pub away: T,
}

impl<T> Sides<T> {
    pub fn new(home: T, away: T) -> Self {
        Self { home, away }
    }

    pub fn map<U>(&self, mut f: impl FnMut(Side, &T) -> U) -> Sides<U> {
        Sides { home: f(Side::Home, &self.home), away: f(Side::Away, &self.away) }
    }
}

impl<T> Index<Side> for Sides<T> {
    type Output = T;
    fn index(&self, s: Side) -> &T {
        match s {
            Side::Home => &self.home,
            Side::Away => &self.away,
        }
    }
}

impl<T> IndexMut<Side> for Sides<T> {
    fn index_mut(&mut self, s: Side) -> &mut T {
        match s {
            Side::Home => &mut self.home,
            Side::Away => &mut self.away,
        }
    }
}

pub const QUARTER_SECONDS: i32 = 15 * 60; // D13
pub const QUARTERS_PER_GAME: i32 = 4;

/// Mutable state of one game. Field position is yards from the offense's own goal line (0-100).
#[derive(Clone, Debug)]
pub struct GameState {
    pub quarter: i32,
    /// Seconds left in the quarter.
    pub clock: i32,
    pub possession: Side,
    /// Offense's own yard line, 1..99.
    pub yard_line: i32,
    pub down: i32,
    pub to_go: i32,
    pub home_score: i32,
    pub away_score: i32,
    pub home_timeouts: i32,
    pub away_timeouts: i32,
    pub overtime: bool,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            quarter: 1,
            clock: QUARTER_SECONDS,
            possession: Side::Home,
            yard_line: 25,
            down: 1,
            to_go: 10,
            home_score: 0,
            away_score: 0,
            home_timeouts: 3,
            away_timeouts: 3,
            overtime: false,
        }
    }
}

impl GameState {
    pub fn yards_to_end_zone(&self) -> i32 {
        100 - self.yard_line
    }

    pub fn is_over(&self) -> bool {
        self.quarter > QUARTERS_PER_GAME && !self.overtime
    }

    pub fn add_score(&mut self, s: Side, pts: i32) {
        match s {
            Side::Home => self.home_score += pts,
            Side::Away => self.away_score += pts,
        }
    }

    /// Lead for `side` (negative = trailing).
    pub fn lead(&self, side: Side) -> i32 {
        match side {
            Side::Home => self.home_score - self.away_score,
            Side::Away => self.away_score - self.home_score,
        }
    }

    /// Burns clock, rolling into the next quarter when it hits zero. Returns true if the quarter changed.
    pub fn run_clock(&mut self, seconds: i32) -> bool {
        self.clock -= seconds;
        if self.overtime {
            if self.clock < 0 {
                self.clock = 0;
            }
            return self.clock == 0;
        }
        if self.clock > 0 {
            return false;
        }
        self.quarter += 1;
        self.clock = QUARTER_SECONDS;
        true
    }

    pub fn first_down(&mut self) {
        self.down = 1;
        self.to_go = self.yards_to_end_zone().min(10);
    }

    /// Flip possession; the new offense starts at `own_yard_line` with a fresh set of downs.
    pub fn change_possession(&mut self, own_yard_line: i32) {
        self.possession = self.possession.other();
        self.yard_line = own_yard_line.clamp(1, 99);
        self.first_down();
    }
}

impl std::fmt::Display for GameState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Q{} {}:{:02} {}&{} @own{} {} ball  H{}-A{}",
            self.quarter,
            self.clock / 60,
            self.clock % 60,
            self.down,
            self.to_go,
            self.yard_line,
            self.possession.name(),
            self.home_score,
            self.away_score
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quarter_rolls_over_and_clock_resets() {
        let mut s = GameState::default();
        assert_eq!(s.clock, 900);
        assert!(!s.run_clock(899));
        assert!(s.run_clock(5));
        assert_eq!((s.quarter, s.clock), (2, 900));
    }

    #[test]
    fn change_possession_flips_side_and_resets_downs() {
        let mut s = GameState { down: 4, ..Default::default() };
        s.change_possession(30);
        assert_eq!((s.possession, s.yard_line, s.down, s.to_go), (Side::Away, 30, 1, 10));
    }

    #[test]
    fn goal_to_go_distance() {
        let mut s = GameState { yard_line: 96, ..Default::default() };
        s.first_down();
        assert_eq!(s.to_go, 4);
    }

    #[test]
    fn overtime_clock_stops_at_zero() {
        let mut s = GameState { overtime: true, ..Default::default() };
        assert!(!s.run_clock(100));
        assert!(s.run_clock(10_000));
        assert_eq!((s.clock, s.quarter), (0, 1));
    }
}
