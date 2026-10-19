use serde::{Deserialize, Serialize};

/// One stretch a player was out: hurt in a game ("team") or kept away by his day job ("work").
/// The league keeps these forever so a player's bio can show his whole history.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbsenceEvent {
    pub player_id: u32,
    pub season: i32,
    /// The week just played when it happened (after 14 it was a playoff game).
    pub week: i32,
    /// True for a day-job injury or work trip, false for a football injury.
    pub work: bool,
    /// Short label: 'Hurt at work (roofer)', 'Work trip', 'Season-ending injury'.
    pub what: String,
    /// Games he was ruled out for.
    pub games: i32,
    /// Games he has actually sat so far.
    pub missed: i32,
    /// Still out (or still counting down).
    pub open: bool,
}

impl AbsenceEvent {
    pub fn new(player_id: u32, season: i32, week: i32, work: bool, what: &str, games: i32) -> Self {
        Self { player_id, season, week, work, what: what.to_string(), games, missed: 0, open: true }
    }
}

/// Football injury severities as shown in the log.
pub fn injury_label(severity: &str) -> &'static str {
    match severity {
        "shaken" => "Shaken up",
        "minor" => "Minor injury",
        "multiWeek" => "Multi-week injury",
        "seasonEnding" => "Season-ending injury",
        _ => "Injury",
    }
}
