use serde::{Deserialize, Serialize};

/// Team strategy: the player's only in-game control (D9). All sliders 0..1 unless noted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OffenseScheme {
    ProStyle,
    Spread,
    AirRaid,
    GroundAndPound,
    Option,
    Balanced,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DefenseFront {
    FourThree,
    ThreeFour,
    Nickel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Coverage {
    Man,
    Zone,
    Mix,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PocketStyle {
    Pocket,
    Rolling,
    Scramble,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OffenseStrategy {
    pub scheme: OffenseScheme,
    /// 0 = all run, 1 = all pass.
    pub run_pass_bias: f64,
    /// 0 = short, 1 = deep.
    pub pass_depth: f64,
    /// 0 = slow, 1 = hurry-up.
    pub tempo: f64,
    pub pocket: PocketStyle,
    pub fourth_down_aggression: f64,
    pub play_action: f64,
}

impl Default for OffenseStrategy {
    fn default() -> Self {
        Self {
            scheme: OffenseScheme::Balanced,
            run_pass_bias: 0.5,
            pass_depth: 0.5,
            tempo: 0.5,
            pocket: PocketStyle::Pocket,
            fourth_down_aggression: 0.3,
            play_action: 0.3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefenseStrategy {
    pub front: DefenseFront,
    pub coverage: Coverage,
    pub blitz_frequency: f64,
    /// Box loading vs deep safeties.
    pub run_stop_commitment: f64,
    /// Strip/pick risk vs gap discipline.
    pub turnover_hunting: f64,
}

impl Default for DefenseStrategy {
    fn default() -> Self {
        Self { front: DefenseFront::FourThree, coverage: Coverage::Mix, blitz_frequency: 0.2, run_stop_commitment: 0.5, turnover_hunting: 0.3 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecialTeamsStrategy {
    pub onside_tendency: f64,
    /// Yards beyond which the team won't try a FG.
    pub field_goal_range_limit: f64,
}

impl Default for SpecialTeamsStrategy {
    fn default() -> Self {
        Self { onside_tendency: 0.02, field_goal_range_limit: 52.0 }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Strategy {
    pub offense: OffenseStrategy,
    pub defense: DefenseStrategy,
    pub special_teams: SpecialTeamsStrategy,
}
