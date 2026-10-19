use crate::game_state::Side;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlayType {
    Run,
    Pass,
    Sack,
    Scramble,
    FieldGoal,
    ExtraPoint,
    Punt,
    Kickoff,
    Penalty,
    Kneel,
}

/// Single source of truth for what happened; stats and narrative derive from these.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayEvent {
    pub quarter: i32,
    pub clock: i32,
    pub offense: Side,
    pub down: i32,
    pub to_go: i32,
    pub yard_line: i32,
    pub play_type: PlayType,
    pub yards: i32,
    /// Short code: complete, incomplete, int, fumble_lost, td, sack, ...
    pub result: String,
    /// Role -> player name (blocked_by, pass_rusher, tackler, ...).
    pub roles: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DriveSummary {
    pub offense: Side,
    pub quarter: i32,
    pub clock_start: i32,
    pub start_yard_line: i32,
    pub plays: i32,
    pub yards: i32,
    pub result: String,
}

impl DriveSummary {
    pub fn new(offense: Side, quarter: i32, clock_start: i32, start_yard_line: i32) -> Self {
        Self { offense, quarter, clock_start, start_yard_line, plays: 0, yards: 0, result: String::new() }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InjuryRecord {
    pub side: Side,
    pub player_id: u32,
    pub name: String,
    /// shaken | minor | multiWeek | seasonEnding
    pub severity: String,
    pub games_out: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScoringPlay {
    pub quarter: i32,
    pub clock: i32,
    pub side: Side,
    pub points: i32,
    pub text: String,
    pub home_score: i32,
    pub away_score: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KeyPlay {
    pub quarter: i32,
    pub clock: i32,
    pub side: Side,
    pub text: String,
}
