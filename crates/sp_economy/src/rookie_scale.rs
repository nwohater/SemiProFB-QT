use crate::Money;

/// Rookie contracts for the Premium Player Draft: fixed by where the player is picked, not negotiated.
/// Earlier picks cost more and sign for longer. Pay is set well under what the same players would ask on the market,
/// so the draft is a way for teams to get talent cheaply. All values are tuning seeds.
pub struct RookieScale;

impl RookieScale {
    pub const ROUNDS: i32 = 5;

    /// Pay per game for the (first, last) pick of each round; picks in between slide down evenly.
    pub const PPG_BY_ROUND: [(Money, Money); 5] = [(230, 150), (140, 105), (100, 80), (76, 62), (60, 50)];

    /// Round 1: 3 seasons, round 2: 2 seasons, round 3 and later: 1 season.
    pub fn seasons_for_round(round: i32) -> i32 {
        match round {
            1 => 3,
            2 => 2,
            _ => 1,
        }
    }

    /// Pay per game for `pick_in_round` (1-based) of `round` (1-based) with `teams` picks per round.
    pub fn ppg(round: i32, pick_in_round: i32, teams: i32) -> Money {
        let (first, last) = Self::PPG_BY_ROUND[((round - 1).clamp(0, Self::PPG_BY_ROUND.len() as i32 - 1)) as usize];
        if teams <= 1 {
            return first;
        }
        (first as f64 + (last - first) as f64 * (pick_in_round - 1) as f64 / (teams - 1) as f64).round() as Money
    }
}
