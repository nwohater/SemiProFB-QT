use crate::enums::{Attr, Position};
use crate::ovr::overall;
use crate::ratings::Ratings;
use serde::{Deserialize, Serialize};

/// The ways a player can have joined the league (the draft is `S{season} Draft #{pick}`).
pub const ACQUIRED_WALK_ON: &str = "Walk-On";
pub const ACQUIRED_TRY_OUT: &str = "Try Out";
pub const ACQUIRED_FREE_AGENCY: &str = "Free Agency";

pub fn acquired_draft(season: i32, overall: i32) -> String {
    format!("S{season} Draft #{overall}")
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Player {
    pub id: u32,
    pub name: String,
    pub age: i32,
    /// Preferred label only; OVR is computed per position.
    pub position: Position,
    /// K/P dual role (D33).
    pub also_kicks_punts: bool,
    pub ratings: Ratings,
    /// Hidden potentials PSP..PDI (D6/D11). Never shown in UI.
    pub potentials: Ratings,
    pub hometown: String,
    pub day_job: String,
    pub height_in: i32,
    pub weight_lb: i32,
    /// Seasons of pro experience. 0 = a rookie (has not finished a season yet).
    pub years_pro: i32,
    /// How he came into the league: 'S2 Draft #11', 'Walk-On', 'Try Out' or 'Free Agency'.
    pub acquired: String,
    /// The number he last wore. It travels with him; `Team::numbers` is the authority on a roster.
    pub jersey: Option<i32>,
    /// Individual awards won (MVP and the like, not titles). Raises what he asks to be paid.
    pub awards: i32,
}

impl Player {
    #[allow(clippy::too_many_arguments)]
    pub fn new(id: u32, name: String, age: i32, position: Position, ratings: Ratings, potentials: Ratings, hometown: String, day_job: String) -> Self {
        for a in Attr::ALL {
            debug_assert!(
                ratings[a] >= 1 && ratings[a] <= potentials[a] && potentials[a] <= 99,
                "invariant 1 <= attr <= potential <= 99 violated for {}",
                a.code()
            );
        }
        Self {
            id,
            name,
            age,
            position,
            also_kicks_punts: false,
            ratings,
            potentials,
            hometown,
            day_job,
            height_in: 72,
            weight_lb: 210,
            years_pro: 0,
            acquired: ACQUIRED_WALK_ON.to_string(),
            jersey: None,
            awards: 0,
        }
    }

    pub fn is_rookie(&self) -> bool {
        self.years_pro == 0
    }

    /// Short experience label: "R" for a rookie, otherwise the number of seasons.
    pub fn years_label(&self) -> String {
        if self.is_rookie() { "R".into() } else { self.years_pro.to_string() }
    }

    pub fn size_label(&self) -> String {
        format!("{}'{}\" {} lb", self.height_in / 12, self.height_in % 12, self.weight_lb)
    }

    pub fn ovr_at(&self, pos: Position) -> i32 {
        overall(&self.ratings, pos)
    }

    pub fn ovr(&self) -> i32 {
        self.ovr_at(self.position)
    }
}

impl std::fmt::Display for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} age{} OVR{} [{}]", self.name, self.position.label(), self.age, self.ovr(), self.ratings)
    }
}
