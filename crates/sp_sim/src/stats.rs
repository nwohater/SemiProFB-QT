use std::collections::BTreeMap;

/// Per-player stat bag. Keys are short snake_case ids (see features/game-sim-engine.md sec 6).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlayerStats {
    m: BTreeMap<String, f64>,
}

impl PlayerStats {
    pub fn add(&mut self, k: &str, v: f64) {
        if let Some(x) = self.m.get_mut(k) {
            *x += v;
        } else {
            self.m.insert(k.to_string(), v);
        }
    }

    /// Adds one.
    pub fn inc(&mut self, k: &str) {
        self.add(k, 1.0);
    }

    pub fn get(&self, k: &str) -> f64 {
        self.m.get(k).copied().unwrap_or(0.0)
    }

    pub fn set_max(&mut self, k: &str, v: f64) {
        if v > self.get(k) {
            self.m.insert(k.to_string(), v);
        }
    }

    pub fn from_map(m: BTreeMap<String, f64>) -> Self {
        Self { m }
    }

    pub fn to_map(&self) -> BTreeMap<String, f64> {
        self.m.clone()
    }

    pub fn is_empty(&self) -> bool {
        self.m.is_empty()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TeamStats {
    pub plays: i32,
    pub pass_plays: i32,
    pub rush_plays: i32,
    pub pass_yards: i32,
    pub rush_yards: i32,
    pub first_downs: i32,
    pub third_att: i32,
    pub third_conv: i32,
    pub fourth_att: i32,
    pub fourth_conv: i32,
    pub turnovers: i32,
    pub interceptions_thrown: i32,
    pub fumbles_lost: i32,
    pub penalties: i32,
    pub penalty_yards: i32,
    pub sacks_taken: i32,
    pub sack_yards_lost: i32,
    pub sacks_made: i32,
    pub pass_att: i32,
    pub pass_cmp: i32,
    pub punts: i32,
    pub punt_yards: i32,
    pub fg_att: i32,
    pub fg_made: i32,
    pub xp_att: i32,
    pub xp_made: i32,
    pub drives: i32,
    pub seconds_of_possession: i32,
}

impl TeamStats {
    pub fn total_yards(&self) -> i32 {
        self.pass_yards + self.rush_yards
    }
}
