use serde::{Deserialize, Serialize};

/// The 8 visible attributes (rated 1-99). Each has a hidden potential counterpart.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Attr {
    Speed,
    Acceleration,
    Agility,
    Strength,
    Hands,
    Endurance,
    Intelligence,
    Discipline,
}

impl Attr {
    pub const ALL: [Attr; 8] = [
        Attr::Speed,
        Attr::Acceleration,
        Attr::Agility,
        Attr::Strength,
        Attr::Hands,
        Attr::Endurance,
        Attr::Intelligence,
        Attr::Discipline,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn code(self) -> &'static str {
        ["SP", "AC", "AG", "ST", "HA", "EN", "IN", "DI"][self as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Position {
    Qb,
    Rb,
    Fb,
    Wr,
    Te,
    Ot,
    Og,
    C,
    De,
    Dt,
    Olb,
    Mlb,
    Cb,
    S,
    K,
    P,
}

impl Position {
    pub const ALL: [Position; 16] = [
        Position::Qb,
        Position::Rb,
        Position::Fb,
        Position::Wr,
        Position::Te,
        Position::Ot,
        Position::Og,
        Position::C,
        Position::De,
        Position::Dt,
        Position::Olb,
        Position::Mlb,
        Position::Cb,
        Position::S,
        Position::K,
        Position::P,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn label(self) -> &'static str {
        ["QB", "RB", "FB", "WR", "TE", "OT", "OG", "C", "DE", "DT", "OLB", "MLB", "CB", "S", "K", "P"][self as usize]
    }

    /// Positions a player listed here can sensibly be moved to.
    pub fn can_switch_to(self) -> Vec<Position> {
        SWITCH_PAIRS
            .iter()
            .filter_map(|&(a, b)| if a == self { Some(b) } else if b == self { Some(a) } else { None })
            .collect()
    }
}

use Position::*;

/// Allowed position moves. Defined as pairs so every move works both ways.
const SWITCH_PAIRS: [(Position, Position); 40] = [
    (Qb, Rb), (Qb, Wr), (Qb, Te), (Qb, Cb), (Qb, S),
    (Rb, Fb), (Rb, Wr), (Rb, Te), (Rb, Olb), (Rb, Cb), (Rb, S),
    (Fb, Te), (Fb, Og), (Fb, Olb), (Fb, Mlb),
    (Wr, Te), (Wr, Cb), (Wr, S),
    (Te, Ot), (Te, Og), (Te, De), (Te, Olb),
    (Ot, Og), (Ot, C), (Ot, De), (Ot, Dt),
    (Og, C), (Og, Dt), (Og, De),
    (C, Dt), (C, De),
    (De, Dt), (De, Olb), (De, Mlb),
    (Dt, Mlb),
    (Olb, Mlb), (Olb, S),
    (Mlb, S),
    (Cb, S),
    (K, P),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Weather {
    Clear,
    Wind,
    Rain,
    Snow,
}
