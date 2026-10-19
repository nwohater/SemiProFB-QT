use crate::Money;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contract {
    pub player_id: u32,
    /// Pay per game while active.
    pub ppg: Money,
    /// 1..3 (D23).
    pub seasons_left: i32,
    pub win_bonus: Money,
    /// Per-game add-on for out-of-town players.
    pub commute: Money,
}

impl Contract {
    pub fn new(player_id: u32, ppg: Money, seasons_left: i32) -> Self {
        Self { player_id, ppg, seasons_left, win_bonus: 0, commute: 0 }
    }

    pub fn active_pay(&self) -> Money {
        self.ppg + self.commute
    }

    /// D25: release costs a % of the remaining contract (lenient; exact % is a balance knob).
    pub const RELEASE_PCT: f64 = 0.125;

    pub fn release_cost(&self) -> Money {
        self.release_cost_with(14, 0)
    }

    pub fn release_cost_with(&self, games_per_season: i32, games_played_this_season: i32) -> Money {
        let remaining = (self.seasons_left * games_per_season - games_played_this_season).clamp(0, 999);
        (remaining as f64 * self.ppg as f64 * Self::RELEASE_PCT).round() as Money
    }
}
