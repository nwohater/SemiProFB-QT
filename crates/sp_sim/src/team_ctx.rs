use crate::game_state::Side;
use crate::stats::{PlayerStats, TeamStats};
use sp_core::returners::{RETURNER_POSITIONS, ReturnRole, return_score};
use sp_core::{Attr, Player, Position, Team};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OffPackage {
    Base,
    TwoTe,
    Heavy,
    Spread,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DefPackage {
    FourThree,
    Nickel,
    ThreeFour,
    GoalLine,
}

pub struct OffUnit<'a> {
    pub qb: &'a Player,
    /// RB + FB.
    pub rbs: Vec<&'a Player>,
    pub wrs: Vec<&'a Player>,
    pub tes: Vec<&'a Player>,
    pub ol: Vec<&'a Player>,
}

impl<'a> OffUnit<'a> {
    pub fn all(&self) -> Vec<&'a Player> {
        let mut v = vec![self.qb];
        v.extend(&self.rbs);
        v.extend(&self.wrs);
        v.extend(&self.tes);
        v.extend(&self.ol);
        v
    }

    pub fn targets(&self) -> Vec<&'a Player> {
        let mut v = self.wrs.clone();
        v.extend(&self.tes);
        v.extend(&self.rbs);
        v
    }
}

pub struct DefUnit<'a> {
    pub dl: Vec<&'a Player>,
    pub lb: Vec<&'a Player>,
    pub db: Vec<&'a Player>,
}

impl<'a> DefUnit<'a> {
    pub fn all(&self) -> Vec<&'a Player> {
        let mut v = self.dl.clone();
        v.extend(&self.lb);
        v.extend(&self.db);
        v
    }
}

/// Per-team runtime state inside one game: fatigue, injuries, stats, unit selection.
pub struct TeamCtx<'a> {
    pub team: &'a Team,
    pub side: Side,
    pub fatigue: BTreeMap<u32, f64>,
    pub injured: HashSet<u32>,
    pub stats: BTreeMap<u32, PlayerStats>,
    pub team_stats: TeamStats,
    pub pool: Vec<&'a Player>,
    by_id: HashMap<u32, &'a Player>,
}

fn sort_desc_by<T>(v: &mut [T], key: impl Fn(&T) -> f64) {
    v.sort_by(|a, b| key(b).total_cmp(&key(a)));
}

impl<'a> TeamCtx<'a> {
    pub fn new(team: &'a Team, side: Side, unavailable: &HashSet<u32>) -> Self {
        let pool: Vec<&Player> = team.active().collect();
        let fatigue = pool.iter().map(|p| (p.id, 0.0)).collect();
        let by_id = pool.iter().map(|&p| (p.id, p)).collect();
        Self { team, side, fatigue, injured: unavailable.clone(), stats: BTreeMap::new(), team_stats: TeamStats::default(), pool, by_id }
    }

    pub fn st(&mut self, p: &Player) -> &mut PlayerStats {
        self.stats.entry(p.id).or_default()
    }

    pub fn mult(&self, p: &Player) -> f64 {
        1.0 - 0.22 * self.fatigue.get(&p.id).copied().unwrap_or(0.0)
    }

    /// Fatigue-adjusted rating.
    pub fn ev(&self, p: &Player, a: Attr) -> f64 {
        p.ratings[a] as f64 * self.mult(p)
    }

    fn available(&self, p: &Player, used: &HashSet<u32>) -> bool {
        !self.injured.contains(&p.id) && !used.contains(&p.id)
    }

    fn fatigue_of(&self, p: &Player) -> f64 {
        self.fatigue.get(&p.id).copied().unwrap_or(0.0)
    }

    /// Healthy RBs not on the field, best first: the change-of-pace backs in a running-back rotation.
    pub fn rotation_backs(&self, ou: &OffUnit<'a>) -> Vec<&'a Player> {
        let on: HashSet<u32> = ou.all().iter().map(|p| p.id).collect();
        let mut v: Vec<&Player> = self.pool.iter().copied().filter(|p| p.position == Position::Rb && self.available(p, &on)).collect();
        sort_desc_by(&mut v, |p| p.ovr_at(Position::Rb) as f64);
        v
    }

    /// Uses your depth chart for `pos`: listed players in order, then the rest of the position by rating.
    /// Tired starters are rotated out for a rested backup (not QB, K, P).
    fn pick_from_depth(&self, pos: Position, n: usize, used: &mut HashSet<u32>, ids: &[u32]) -> Vec<&'a Player> {
        let mut listed: Vec<&Player> = Vec::new();
        for id in ids {
            if let Some(&p) = self.by_id.get(id) {
                if self.available(p, used) && !listed.iter().any(|x| x.id == p.id) {
                    listed.push(p);
                }
            }
        }
        let mut rest: Vec<&Player> = self.pool.iter().copied().filter(|p| p.position == pos && self.available(p, used) && !listed.iter().any(|x| x.id == p.id)).collect();
        sort_desc_by(&mut rest, |p| p.ovr_at(pos) as f64);
        let mut order: Vec<&Player> = listed;
        order.extend(rest);
        if order.len() < n {
            let mut others: Vec<&Player> = self.pool.iter().copied().filter(|p| p.position != pos && self.available(p, used) && !order.iter().any(|x| x.id == p.id)).collect();
            sort_desc_by(&mut others, |p| p.ovr_at(pos) as f64 * self.mult(p));
            order.extend(others);
        }
        let mut out: Vec<&Player> = order.iter().take(n).copied().collect();
        let no_rotate = matches!(pos, Position::Qb | Position::K | Position::P);
        if !no_rotate {
            let mut bench: Vec<&Player> = order.iter().skip(n).copied().collect();
            for slot in out.iter_mut() {
                let tired = self.fatigue_of(slot);
                if tired <= 0.6 {
                    continue;
                }
                if let Some(j) = bench.iter().position(|b| self.fatigue_of(b) < tired - 0.25) {
                    let b = bench.remove(j);
                    bench.push(*slot);
                    *slot = b;
                }
            }
        }
        used.extend(out.iter().map(|p| p.id));
        out
    }

    /// Best `n` players for `pos` by fatigue-adjusted OVR; same-position first, others fill (out-of-position penalty).
    fn pick(&self, pos: Position, n: usize, used: &mut HashSet<u32>) -> Vec<&'a Player> {
        if let Some(ids) = self.team.depth.get(&pos) {
            if !ids.is_empty() {
                return self.pick_from_depth(pos, n, used, ids);
            }
        }
        // QBs, kickers and punters never rotate out for fatigue.
        let no_rotate = matches!(pos, Position::Qb | Position::K | Position::P);
        let m = |p: &Player| if no_rotate { 1.0 } else { self.mult(p) };
        let mut same: Vec<&Player> = self.pool.iter().copied().filter(|p| p.position == pos && self.available(p, used)).collect();
        sort_desc_by(&mut same, |p| p.ovr_at(pos) as f64 * m(p));
        let mut out: Vec<&Player> = same.into_iter().take(n).collect();
        if out.len() < n {
            let mut others: Vec<&Player> = self.pool.iter().copied().filter(|p| p.position != pos && self.available(p, used) && !out.iter().any(|x| x.id == p.id)).collect();
            sort_desc_by(&mut others, |p| p.ovr_at(pos) as f64 * m(p));
            let need = n - out.len();
            out.extend(others.into_iter().take(need));
        }
        used.extend(out.iter().map(|p| p.id));
        out
    }

    pub fn offense(&self, pkg: OffPackage) -> OffUnit<'a> {
        let mut used = HashSet::new();
        let qb = self.pick(Position::Qb, 1, &mut used)[0];
        let mut ol = self.pick(Position::Ot, 2, &mut used);
        ol.extend(self.pick(Position::Og, 2, &mut used));
        ol.extend(self.pick(Position::C, 1, &mut used));
        match pkg {
            OffPackage::Base => {
                let rbs = self.pick(Position::Rb, 1, &mut used);
                let wrs = self.pick(Position::Wr, 3, &mut used);
                let tes = self.pick(Position::Te, 1, &mut used);
                OffUnit { qb, rbs, wrs, tes, ol }
            }
            OffPackage::TwoTe => {
                let rbs = self.pick(Position::Rb, 1, &mut used);
                let wrs = self.pick(Position::Wr, 2, &mut used);
                let tes = self.pick(Position::Te, 2, &mut used);
                OffUnit { qb, rbs, wrs, tes, ol }
            }
            OffPackage::Heavy => {
                let mut rbs = self.pick(Position::Rb, 1, &mut used);
                rbs.extend(self.pick(Position::Fb, 1, &mut used));
                let wrs = self.pick(Position::Wr, 1, &mut used);
                let tes = self.pick(Position::Te, 2, &mut used);
                OffUnit { qb, rbs, wrs, tes, ol }
            }
            OffPackage::Spread => {
                let rbs = self.pick(Position::Rb, 1, &mut used);
                let wrs = self.pick(Position::Wr, 4, &mut used);
                OffUnit { qb, rbs, wrs, tes: vec![], ol }
            }
        }
    }

    pub fn defense(&self, pkg: DefPackage) -> DefUnit<'a> {
        use Position::*;
        let mut used = HashSet::new();
        let (de, dt, mlb, olb, cb, s) = match pkg {
            DefPackage::FourThree => (2, 2, 1, 2, 2, 2),
            DefPackage::Nickel => (2, 2, 1, 1, 3, 2),
            DefPackage::ThreeFour => (2, 1, 2, 2, 2, 2),
            DefPackage::GoalLine => (2, 2, 2, 2, 2, 1),
        };
        let mut dl = self.pick(De, de, &mut used);
        dl.extend(self.pick(Dt, dt, &mut used));
        let mut lb = self.pick(Mlb, mlb, &mut used);
        lb.extend(self.pick(Olb, olb, &mut used));
        let mut db = self.pick(Cb, cb, &mut used);
        db.extend(self.pick(S, s, &mut used));
        DefUnit { dl, lb, db }
    }

    pub fn kicker(&self) -> &'a Player {
        self.specialist(Position::K)
    }

    pub fn punter(&self) -> &'a Player {
        self.specialist(Position::P)
    }

    fn specialist(&self, pos: Position) -> &'a Player {
        let mut c: Vec<&Player> = self
            .pool
            .iter()
            .copied()
            .filter(|p| (p.position == pos || p.also_kicks_punts && matches!(p.position, Position::K | Position::P)) && !self.injured.contains(&p.id))
            .collect();
        if !c.is_empty() {
            sort_desc_by(&mut c, |p| p.ovr_at(pos) as f64);
            return c[0];
        }
        let mut all: Vec<&Player> = self.pool.iter().copied().filter(|p| !self.injured.contains(&p.id)).collect();
        sort_desc_by(&mut all, |p| p.ovr_at(pos) as f64);
        all[0]
    }

    /// The return man for a kick or a punt: the first healthy man on your list for that role, otherwise the best by
    /// return score among non-lineman skill players.
    pub fn returner_for(&self, role: ReturnRole) -> &'a Player {
        for id in self.team.return_depth.get(&role).into_iter().flatten() {
            if let Some(&p) = self.by_id.get(id) {
                if !self.injured.contains(id) {
                    return p;
                }
            }
        }
        let mut c: Vec<&Player> = self.pool.iter().copied().filter(|p| !self.injured.contains(&p.id) && RETURNER_POSITIONS.contains(&p.position)).collect();
        sort_desc_by(&mut c, |p| return_score(p));
        c.first().copied().unwrap_or(self.pool[0])
    }

    /// Called per snap: `on_field` gains fatigue, everyone else recovers.
    pub fn apply_fatigue(&mut self, on_field: &[&Player], intensity: f64) {
        let on: HashSet<u32> = on_field.iter().map(|p| p.id).collect();
        for p in &self.pool {
            let f = self.fatigue.get(&p.id).copied().unwrap_or(0.0);
            let en = p.ratings[Attr::Endurance] as f64 / 100.0;
            let v = if on.contains(&p.id) {
                let role_mult = match p.position {
                    Position::Ot | Position::Og | Position::C | Position::De | Position::Dt | Position::Rb | Position::Fb => 1.15,
                    Position::Qb | Position::K | Position::P => 0.5,
                    _ => 1.0,
                };
                (f + 0.011 * intensity * role_mult * (1.55 - en)).clamp(0.0, 1.0)
            } else {
                (f - 0.0035 * (0.5 + en)).clamp(0.0, 1.0)
            };
            self.fatigue.insert(p.id, v);
        }
    }

    pub fn halftime_recovery(&mut self) {
        for p in &self.pool {
            let f = self.fatigue.get(&p.id).copied().unwrap_or(0.0);
            self.fatigue.insert(p.id, f * 0.5);
        }
    }
}
