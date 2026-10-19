use crate::league::League;
use crate::text::{fill, inbox_text};
use sp_core::generator::GenOptions;
use sp_core::geo::town_miles;
use sp_core::player::ACQUIRED_FREE_AGENCY;
use sp_core::{Player, Position, Rng, Team, Tier};
use std::collections::HashSet;

/// Tuning for fan favorites: a small, league-wide set of players the crowd turns out for.
pub struct FanFavoriteTuning;

impl FanFavoriteTuning {
    /// The league never runs below this many...
    pub const MIN_LEAGUE: usize = 12;
    /// ...or above this many.
    pub const MAX_LEAGUE: usize = 20;
    pub const START_COUNT: usize = 16;
    /// Per favorite lost to retirement, in the offseason.
    pub const REGEN_CHANCE: f64 = 0.65;
    /// A regenerated favorite is a brand-new local kid this often, else an existing player.
    pub const NEW_PLAYER_SHARE: f64 = 0.5;
    /// An existing player must still have a few years left.
    pub const MAX_AGE: i32 = 31;
    /// Home attendance at OVR 45...
    pub const BASE_BOOST: f64 = 0.015;
    /// ...up to OVR 75.
    pub const TOP_BOOST: f64 = 0.035;
    /// Total per team.
    pub const CAP: f64 = 0.10;

    /// How much the crowd cares about a position (QBs sell tickets). Offensive linemen, kickers and punters never become favorites.
    pub fn visibility(p: Position) -> Option<f64> {
        use Position::*;
        match p {
            Qb => Some(1.4),
            Rb | Wr => Some(1.2),
            Te | Olb | Mlb | De => Some(1.0),
            Fb | Cb | S => Some(0.9),
            Dt => Some(0.8),
            _ => None,
        }
    }

    /// Positions a brand-new favorite can be generated at (weights).
    pub const NEW_POSITIONS: [(Position, f64); 9] = [
        (Position::Qb, 4.0),
        (Position::Rb, 4.0),
        (Position::Wr, 4.0),
        (Position::Te, 2.0),
        (Position::Olb, 2.0),
        (Position::Mlb, 2.0),
        (Position::De, 2.0),
        (Position::Cb, 2.0),
        (Position::S, 1.0),
    ];
}

#[derive(Clone, Debug)]
pub struct FanFavorite {
    pub player_id: u32,
    pub name: String,
    /// Fraction of home attendance, e.g. 0.03 = +3%.
    pub boost: f64,
    pub reason: &'static str,
}

/// One line of offseason fan-favorite news. `player_id` is set when the line is about a new local hero in free agency.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FanNote {
    pub text: String,
    pub player_id: Option<u32>,
}

pub fn can_be_fan_favorite(p: &Player) -> bool {
    FanFavoriteTuning::visibility(p.position).is_some()
}

impl League {
    /// The crowd pull of `p` on `team`, or `None` when he is not one of the league's favorites.
    pub fn fan_favorite_of(&self, team: &Team, p: &Player) -> Option<FanFavorite> {
        if !self.fan_favorite_ids.contains(&p.id) || !can_be_fan_favorite(p) {
            return None;
        }
        let skill = ((p.ovr() as f64 - 45.0) / 30.0).clamp(0.0, 1.0);
        let boost = (FanFavoriteTuning::BASE_BOOST + (FanFavoriteTuning::TOP_BOOST - FanFavoriteTuning::BASE_BOOST) * skill) * FanFavoriteTuning::visibility(p.position).unwrap();
        let local = town_miles(&p.hometown, &team.town.name) <= 1.0;
        Some(FanFavorite { player_id: p.id, name: p.name.clone(), boost, reason: if local { "hometown hero" } else { "fan favorite" } })
    }

    /// Favorites on `team` who would actually draw a crowd: dressed (active) and not injured. Biggest pull first.
    pub fn fan_favorites(&self, team: &Team, unavailable: &HashSet<u32>, active_only: bool) -> Vec<FanFavorite> {
        let mut out = Vec::new();
        for p in &team.roster {
            if unavailable.contains(&p.id) {
                continue;
            }
            if active_only && !team.active_ids.contains(&p.id) {
                continue;
            }
            if let Some(f) = self.fan_favorite_of(team, p) {
                out.push(f);
            }
        }
        out.sort_by(|a, b| b.boost.total_cmp(&a.boost));
        out
    }

    /// Attendance multiplier from favorites (1.0 = none), capped.
    pub fn fan_favorite_mult(&self, team: &Team, unavailable: &HashSet<u32>) -> f64 {
        let sum: f64 = self.fan_favorites(team, unavailable, true).iter().map(|f| f.boost).sum();
        1.0 + sum.min(FanFavoriteTuning::CAP)
    }

    /// Existing players who could be picked: right position, not already a favorite, not too old. Locals and good players weigh more.
    fn fan_candidates(&self, pool: &[(&Player, Option<&Team>)]) -> Vec<(u32, f64)> {
        let mut out = Vec::new();
        for (p, t) in pool {
            if !can_be_fan_favorite(p) || self.fan_favorite_ids.contains(&p.id) || p.age > FanFavoriteTuning::MAX_AGE {
                continue;
            }
            let local = t.map(|t| town_miles(&p.hometown, &t.town.name) <= 1.0).unwrap_or(false);
            let w = (p.ovr() - 30).clamp(1, 60) as f64 * if local { 2.0 } else { 1.0 } * if p.age <= 27 { 1.3 } else { 1.0 };
            out.push((p.id, w));
        }
        out
    }

    fn league_pool(&self) -> Vec<(&Player, Option<&Team>)> {
        self.teams.iter().flat_map(|t| t.roster.iter().map(move |p| (p, Some(t)))).collect()
    }

    /// First-time seeding: at least one favorite on every team, topped up to the starting count across the league.
    pub fn seed_fan_favorites(&mut self, r: &mut Rng) {
        self.fan_favorite_ids.clear();
        let mut picks = Vec::new();
        for t in &self.teams {
            let pool: Vec<(&Player, Option<&Team>)> = t.roster.iter().map(|p| (p, Some(t))).collect();
            let c = self.fan_candidates(&pool);
            if c.is_empty() {
                continue;
            }
            picks.push(r.weighted(&c));
        }
        self.fan_favorite_ids.extend(picks);
        let mut all = self.fan_candidates(&self.league_pool());
        while self.fan_favorite_ids.len() < FanFavoriteTuning::START_COUNT && !all.is_empty() {
            let id = r.weighted(&all);
            all.retain(|(x, _)| *x != id);
            self.fan_favorite_ids.insert(id);
        }
    }

    /// Offseason: retired favorites drop off, and new ones sometimes emerge (an existing player breaks out, or a local kid turns up).
    /// Keeps the league between `MIN_LEAGUE` and `MAX_LEAGUE`.
    pub fn roll_fan_favorites(&mut self, r: &mut Rng) {
        self.fan_news.clear();
        let alive: HashSet<u32> = self.teams.iter().flat_map(|t| t.roster.iter().map(|p| p.id)).chain(self.free_agents.iter().map(|p| p.id)).collect();
        let before = self.fan_favorite_ids.len();
        self.fan_favorite_ids.retain(|id| alive.contains(id));
        let lost = before - self.fan_favorite_ids.len();

        let mut make: i64 = 0;
        for _ in 0..lost {
            if r.chance(FanFavoriteTuning::REGEN_CHANCE) {
                make += 1;
            }
        }
        let target = self.fan_favorite_ids.len() as i64 + make;
        if target < FanFavoriteTuning::MIN_LEAGUE as i64 {
            make += FanFavoriteTuning::MIN_LEAGUE as i64 - target;
        }
        if self.fan_favorite_ids.len() as i64 + make > FanFavoriteTuning::MAX_LEAGUE as i64 {
            make = FanFavoriteTuning::MAX_LEAGUE as i64 - self.fan_favorite_ids.len() as i64;
        }

        for _ in 0..make.max(0) {
            if r.chance(FanFavoriteTuning::NEW_PLAYER_SHARE) {
                self.new_local_hero(r);
            } else {
                let c = self.fan_candidates(&self.league_pool());
                if c.is_empty() {
                    self.new_local_hero(r);
                    continue;
                }
                let id = r.weighted(&c);
                self.fan_favorite_ids.insert(id);
                let (p, town) = self
                    .teams
                    .iter()
                    .find_map(|t| t.roster.iter().find(|x| x.id == id).map(|p| (p.clone(), t.town.name.clone())))
                    .expect("candidate is on a roster");
                let line = fill(inbox_text().pick("fanFavoriteBreakout", r), &[("p", p.name.clone()), ("pos", p.position.label().to_string()), ("town", town)]);
                self.fan_news.push(FanNote { text: line, player_id: None });
            }
        }
    }

    /// A brand-new young local star: joins the free-agent pool (any team can sign him) already beloved.
    fn new_local_hero(&mut self, r: &mut Rng) {
        let pos = r.weighted(&FanFavoriteTuning::NEW_POSITIONS);
        let towns: Vec<String> = self.region.all_towns().map(|t| t.name.clone()).collect();
        let home = r.pick(&towns).clone();
        let age = r.range(20, 24);
        let p = self.tg.players.generate(pos, GenOptions::default().tier(Tier::ExCollege).hometown(&home).age(age).years_pro(0).acquired(ACQUIRED_FREE_AGENCY));
        let line = fill(
            inbox_text().pick("fanFavoriteLocal", r),
            &[("p", p.name.clone()), ("pos", p.position.label().to_string()), ("age", p.age.to_string()), ("town", home)],
        );
        self.fan_favorite_ids.insert(p.id);
        self.fan_news.push(FanNote { text: line, player_id: Some(p.id) });
        self.free_agents.push(p);
    }
}
