use crate::Money;
use serde::{Deserialize, Serialize};
use sp_core::{Rng, Town};
use std::sync::OnceLock;

const ECONOMY_JSON: &str = include_str!("../../../data/economy.json");

/// Promo nights: spend money for a bigger home crowd (with a chance of something going wrong).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromoTemplate {
    pub id: String,
    pub name: String,
    pub base_cost: i64,
    pub attendance_mult: f64,
    pub per_cap_mult: f64,
    pub merch_mult: f64,
    pub risk_chance: f64,
    pub risk_text: String,
    /// When it goes wrong: how much of the crowd lift survives, and the cleanup/fine as a multiple of the promo's cost.
    pub mishap_att_mult: f64,
    pub mishap_fine_mult: f64,
}

impl PromoTemplate {
    /// Bigger stadiums cost more to dress up.
    pub fn cost(&self, t: &Town) -> Money {
        (self.base_cost as f64 * (t.capacity as f64 / 2500.0).powf(0.8)).round() as Money
    }
}

/// Stadium upgrades (capex + upkeep).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Facility {
    Seating,
    Concessions,
    Scoreboard,
    Lights,
    Training,
}

impl Facility {
    pub const ALL: [Facility; 5] = [Facility::Seating, Facility::Concessions, Facility::Scoreboard, Facility::Lights, Facility::Training];

    fn key(self) -> &'static str {
        match self {
            Facility::Seating => "seating",
            Facility::Concessions => "concessions",
            Facility::Scoreboard => "scoreboard",
            Facility::Lights => "lights",
            Facility::Training => "training",
        }
    }

    pub fn spec(self) -> &'static FacilitySpec {
        economy().facilities.iter().find(|f| f.key == self.key()).expect("facility spec")
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FacilitySpec {
    pub key: String,
    pub name: String,
    pub base_cost: i64,
    pub max_level: i32,
    /// Per game.
    pub upkeep_per_level: i64,
    pub effect: String,
}

impl FacilitySpec {
    /// Each level costs more than the last.
    pub fn cost_at(&self, current_level: i32) -> Money {
        (self.base_cost as f64 * (1.0 + current_level as f64 * 0.4)).round() as Money
    }
}

/// A sponsor's strings: what it pays and what it does to your team. Effects are applied by the economy engine.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SponsorClause {
    pub id: String,
    /// Flavor line.
    pub text: String,
    /// Multiplier on the base per-game pay.
    pub pay_mult: f64,
    /// Deal ends at season rollover if you win less than this share (0 = none).
    pub min_win_pct: f64,
    /// Fan interest change after every game.
    pub fan_per_game: f64,
    /// Home crowd multiplier.
    pub att_mult: f64,
    /// Extra cost every home game.
    pub cost_per_game: i64,
    /// Extra pay for every win.
    pub win_bonus: i64,
}

impl SponsorClause {
    /// Plain-language summary of every effect, shown next to the offer.
    pub fn effects(&self) -> String {
        let mut bits: Vec<String> = Vec::new();
        let pct = ((self.pay_mult - 1.0) * 100.0).round() as i32;
        if pct != 0 {
            bits.push(if pct > 0 { format!("Pays {pct}% more than a standard deal") } else { format!("Pays {}% less than a standard deal", -pct) });
        }
        if self.win_bonus > 0 {
            bits.push(format!("Bonus of ${} for every win", self.win_bonus));
        }
        if self.min_win_pct > 0.0 {
            bits.push(format!("Walks away at season end if you win under {}% of games", (self.min_win_pct * 100.0).round() as i32));
        }
        if self.fan_per_game != 0.0 {
            bits.push(format!("Fan interest {} {:.1} after every game", if self.fan_per_game > 0.0 { "rises" } else { "drops" }, self.fan_per_game.abs()));
        }
        if self.att_mult != 1.0 {
            bits.push(format!("Home crowds {}% {}", ((self.att_mult - 1.0).abs() * 100.0).round() as i32, if self.att_mult > 1.0 { "bigger" } else { "smaller" }));
        }
        if self.cost_per_game > 0 {
            bits.push(format!("Costs you ${} every home game", self.cost_per_game));
        }
        if bits.is_empty() { "No downside".to_string() } else { bits.join(" · ") }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EconomyData {
    sponsor_clauses: Vec<SponsorClause>,
    promos: Vec<PromoTemplate>,
    facilities: Vec<FacilitySpec>,
    sponsor_names: Vec<String>,
}

fn economy() -> &'static EconomyData {
    static D: OnceLock<EconomyData> = OnceLock::new();
    D.get_or_init(|| serde_json::from_str(ECONOMY_JSON).expect("economy.json"))
}

pub fn promo_templates() -> &'static [PromoTemplate] {
    &economy().promos
}

pub fn promo_by_id(id: &str) -> Option<&'static PromoTemplate> {
    economy().promos.iter().find(|p| p.id == id)
}

pub fn sponsor_clauses() -> &'static [SponsorClause] {
    &economy().sponsor_clauses
}

pub fn sponsor_clause_by_id(id: &str) -> Option<&'static SponsorClause> {
    economy().sponsor_clauses.iter().find(|c| c.id == id)
}

/// Older saves only stored the flavor text.
pub fn sponsor_clause_by_text(text: &str) -> Option<&'static SponsorClause> {
    economy().sponsor_clauses.iter().find(|c| c.text == text)
}

/// A sponsor offer with strings attached.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SponsorOffer {
    pub name: String,
    pub per_game: Money,
    pub seasons: i32,
    pub clause: String,
    /// 0 = no performance clause.
    pub min_win_pct: f64,
    pub clause_id: String,
    /// Mid-season offers vanish after this week (0 = lasts until the next season).
    pub expires_week: i32,
}

impl SponsorOffer {
    pub fn effects(&self) -> Option<&'static SponsorClause> {
        sponsor_clause_by_id(&self.clause_id).or_else(|| sponsor_clause_by_text(&self.clause))
    }
}

pub struct SponsorMarket;

impl SponsorMarket {
    pub const MAX_SLOTS: usize = 4;

    /// Fan interest needed for each number of sponsor slots (businesses only pay for a team the town cares about).
    pub const SLOT_THRESHOLDS: [(usize, f64); 4] = [(1, 0.0), (2, 30.0), (3, 45.0), (4, 65.0)];

    pub fn slots_for(fan_interest: f64) -> usize {
        let mut n = 1;
        for (slots, need) in Self::SLOT_THRESHOLDS {
            if fan_interest >= need {
                n = slots;
            }
        }
        n
    }

    pub fn names() -> &'static [String] {
        &economy().sponsor_names
    }

    /// `avoid_names` and `avoid_clauses` keep new offers from repeating a sponsor you already have or an offer already on the table.
    pub fn generate(t: &Town, rng: &mut Rng, n: usize, expires_week: i32, avoid_names: &[String], avoid_clauses: &[String]) -> Vec<SponsorOffer> {
        let clauses = sponsor_clauses();
        let names = Self::names();
        let mut out = Vec::new();
        let mut used_clauses: std::collections::HashSet<String> = avoid_clauses.iter().cloned().collect();
        let mut used_names: std::collections::HashSet<String> = avoid_names.iter().cloned().collect();
        for _ in 0..n {
            let mut clause = rng.pick(clauses);
            let mut tries = 0;
            while tries < 12 && used_clauses.contains(&clause.id) {
                clause = rng.pick(clauses);
                tries += 1;
            }
            used_clauses.insert(clause.id.clone());
            let mut name = rng.pick(names);
            let mut tries = 0;
            while tries < 24 && used_names.contains(name) {
                name = rng.pick(names);
                tries += 1;
            }
            used_names.insert(name.clone());
            let base = t.biz as f64 * 55.0 * rng.gaussian(1.0, 0.12);
            let per_game = 40.max((base * clause.pay_mult).round() as Money);
            let seasons = rng.range(1, 3);
            out.push(SponsorOffer { name: name.clone(), per_game, seasons, clause: clause.text.clone(), min_win_pct: clause.min_win_pct, clause_id: clause.id.clone(), expires_week });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_load() {
        assert_eq!(sponsor_clauses().len(), 27);
        assert_eq!(promo_templates().len(), 6);
        assert_eq!(SponsorMarket::names().len(), 50);
        assert_eq!(Facility::Seating.spec().max_level, 4);
        assert_eq!(promo_by_id("beer").unwrap().name, "$2 Beer Night");
        assert_eq!(sponsor_clause_by_id("nephew").unwrap().text, "Field is renamed after the owner's nephew.");
    }

    #[test]
    fn every_facility_has_a_spec() {
        for f in Facility::ALL {
            assert!(f.spec().base_cost > 0);
        }
        assert_eq!(Facility::Seating.spec().cost_at(1), 25200);
    }

    #[test]
    fn clause_effect_text() {
        assert_eq!(sponsor_clause_by_id("hat").unwrap().effects(), "No downside");
        assert!(sponsor_clause_by_id("perwin").unwrap().effects().contains("Bonus of $160 for every win"));
        assert!(sponsor_clause_by_id("win55").unwrap().effects().contains("under 55% of games"));
    }
}
