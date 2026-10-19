use crate::enums::{Attr, Position};
use crate::player::Player;
use serde::{Deserialize, Serialize};

/// The two return jobs on special teams. Each has its own depth list on the team.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReturnRole {
    Kick,
    Punt,
}

impl ReturnRole {
    pub const ALL: [ReturnRole; 2] = [ReturnRole::Kick, ReturnRole::Punt];

    pub fn label(self) -> &'static str {
        match self {
            ReturnRole::Kick => "Kick returner",
            ReturnRole::Punt => "Punt returner",
        }
    }

    pub fn tag(self) -> &'static str {
        match self {
            ReturnRole::Kick => "KR",
            ReturnRole::Punt => "PR",
        }
    }
}

/// Positions the automatic pick draws from (you can still list anyone else by hand).
pub const RETURNER_POSITIONS: [Position; 4] = [Position::Wr, Position::Rb, Position::Cb, Position::S];

/// The ratings that make a return man, shown on his row.
pub const RETURNER_ATTRS: [Attr; 4] = [Attr::Speed, Attr::Acceleration, Attr::Agility, Attr::Hands];

/// How good a player is as a returner: speed, acceleration, agility and hands. The same for kicks and punts.
pub fn return_score(p: &Player) -> f64 {
    p.ratings[Attr::Speed] as f64 * 0.3
        + p.ratings[Attr::Acceleration] as f64 * 0.25
        + p.ratings[Attr::Agility] as f64 * 0.25
        + p.ratings[Attr::Hands] as f64 * 0.2
}
