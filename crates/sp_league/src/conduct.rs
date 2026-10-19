use crate::inbox::*;
use crate::league::League;
use crate::text::{commas, conduct_asks, conduct_incidents, fill, Incident};
use serde::{Deserialize, Serialize};
use sp_core::team::min_active_coverage;
use sp_core::{Attr, Player, Rng};
use sp_economy::{EconTuning, LedgerEntry, LedgerKind};

/// Tuning for off-field trouble. Players with low discipline (and, less, low intelligence) sometimes do something dumb.
/// The owner pays to make it go away, or the player sits. All values are tuning seeds.
pub struct ConductTuning;

pub struct ConductTier {
    pub min_cost: i64,
    pub max_cost: i64,
    pub games: i32,
    pub weight: f64,
}

impl ConductTuning {
    /// Weight of discipline vs intelligence in the "sense" score (0..100).
    pub const DISCIPLINE_WEIGHT: f64 = 0.65;
    /// Sense at or above this never gets in trouble; at 20 and below the weekly risk is at its highest.
    pub const SAFE_ABOVE: f64 = 45.0;
    pub const WORST_AT: f64 = 20.0;
    /// Weekly chance at the worst sense (scaled down by `RISK_CURVE` as sense improves).
    pub const MAX_WEEKLY_RISK: f64 = 0.02;
    pub const RISK_CURVE: f64 = 1.5;
    /// At most this many incidents a week, and a player is left alone for this many weeks after one.
    pub const MAX_PER_WEEK: usize = 1;
    pub const COOLDOWN_WEEKS: i32 = 4;
    /// AI clubs that can afford it (3x the cost in cash) pay this often for a starter / a backup.
    pub const AI_PAY_STARTER: f64 = 0.85;
    pub const AI_PAY_BACKUP: f64 = 0.15;
    /// If you do not cover it up, the town hears about it: fan interest drops by this much per game he is suspended.
    pub const FAN_HIT_PER_GAME: f64 = 0.75;

    /// Tier -> (min cost, max cost, games he sits if you do not pay, weight). Costs are rounded to $50 and rise with the cost index.
    pub const TIERS: [ConductTier; 3] = [
        ConductTier { min_cost: 250, max_cost: 700, games: 1, weight: 5.0 },
        ConductTier { min_cost: 800, max_cost: 1800, games: 2, weight: 3.0 },
        ConductTier { min_cost: 2000, max_cost: 4000, games: 3, weight: 1.5 },
    ];
}

/// One bit of gossip about a player in trouble: what happened, what the lawyers wanted, and whether the club paid.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConductGossip {
    pub team: usize,
    pub player_id: u32,
    pub title: String,
    pub body: String,
    pub cost: i64,
    pub games: i32,
    pub paid: bool,
}

/// 0 (no risk) to 1 (worst) for how likely a player is to do something dumb.
pub fn trouble_score(p: &Player) -> f64 {
    let sense = ConductTuning::DISCIPLINE_WEIGHT * p.ratings[Attr::Discipline] as f64 + (1.0 - ConductTuning::DISCIPLINE_WEIGHT) * p.ratings[Attr::Intelligence] as f64;
    ((ConductTuning::SAFE_ABOVE - sense) / (ConductTuning::SAFE_ABOVE - ConductTuning::WORST_AT)).clamp(0.0, 1.0)
}

pub fn weekly_risk(p: &Player) -> f64 {
    ConductTuning::MAX_WEEKLY_RISK * trouble_score(p).powf(ConductTuning::RISK_CURVE)
}

fn shuffled(mut v: Vec<Player>, r: &mut Rng) -> Vec<Player> {
    for i in (1..v.len()).rev() {
        let j = r.range(0, i as i32) as usize;
        v.swap(i, j);
    }
    v
}

impl League {
    /// Called after a week is played. Rolls for trouble among your players and writes the letter. Uses its own Rng.
    pub fn roll_conduct(&mut self, week_no: i32) {
        let Some(u) = self.user_team else { return };
        self.roll_ai_conduct(week_no);
        let mut r = Rng::new(self.season as u64 * 8191 + week_no as u64 * 131 + 29);
        let mut made = 0;
        let candidates = shuffled(self.teams[u].roster.clone(), &mut r);
        for p in candidates {
            if made >= ConductTuning::MAX_PER_WEEK {
                break;
            }
            if self.injured_games.get(&p.id).copied().unwrap_or(0) > 0 {
                continue;
            }
            let season = self.season;
            if self.inbox.iter().any(|m| m.kind == InboxKind::Conduct && m.player_id == Some(p.id) && m.season == season && week_no - m.week < ConductTuning::COOLDOWN_WEEKS && m.week <= week_no) {
                continue;
            }
            if !r.chance(weekly_risk(&p)) {
                continue;
            }
            self.post_conduct(&p, week_no, &mut r);
            made += 1;
        }
    }

    fn post_conduct(&mut self, p: &Player, week_no: i32, r: &mut Rng) {
        let (inc, cost, games) = self.roll_incident(r);
        let town = self.me().town.name.clone();
        let vars = [
            ("p", p.name.clone()),
            ("pos", p.position.label().to_string()),
            ("town", town),
            ("job", p.day_job.to_lowercase()),
            ("amt", format!("${}", commas(cost))),
            ("g", games.to_string()),
            ("gm", if games == 1 { "1 game".to_string() } else { format!("{games} games") }),
        ];
        let ask = r.pick(conduct_asks()).clone();
        let text = format!("{}\n\n{}", fill(&inc.body, &vars), fill(&ask, &vars));
        self.post_inbox(
            InboxFeed::Travel,
            InboxKind::Conduct,
            &format!("{} {}: {}", p.position.label(), p.name, inc.title),
            &text,
            PostOpts { at_week: Some(week_no), player_id: Some(p.id), conduct_cost: cost, conduct_games: games, ..Default::default() },
        );
    }

    fn roll_incident(&self, r: &mut Rng) -> (Incident, i64, i32) {
        let weights: Vec<(usize, f64)> = ConductTuning::TIERS.iter().enumerate().map(|(i, t)| (i, t.weight)).collect();
        let tier = r.weighted(&weights);
        let t = &ConductTuning::TIERS[tier];
        let pool: Vec<&Incident> = conduct_incidents().iter().filter(|i| i.tier as usize == tier).collect();
        let inc = (*r.pick(&pool)).clone();
        let ix = EconTuning::cost_index(self.season);
        let cost = (((t.min_cost as f64 + r.next_f64() * (t.max_cost - t.min_cost) as f64) * ix / 50.0).round() as i64) * 50;
        (inc, cost, t.games)
    }

    /// AI clubs get in trouble too. The club pays if it can comfortably afford it; otherwise the player sits. Either way it is gossip.
    fn roll_ai_conduct(&mut self, week_no: i32) {
        let mut r = Rng::new(self.season as u64 * 9157 + week_no as u64 * 137 + 31);
        for ti in 0..self.teams.len() {
            if Some(ti) == self.user_team {
                continue;
            }
            let candidates = shuffled(self.teams[ti].roster.clone(), &mut r);
            for p in &candidates {
                if self.injured_games.get(&p.id).copied().unwrap_or(0) > 0 {
                    continue;
                }
                if !r.chance(weekly_risk(p)) {
                    continue;
                }
                let (inc, cost, games) = self.roll_incident(&mut r);
                let starters_n = 1.max(min_active_coverage(p.position));
                let rank = self.teams[ti].roster.iter().filter(|x| x.position == p.position && x.ovr() > p.ovr()).count();
                let starter = rank < starters_n;
                // With 3x the cost in the bank, a club pays to keep a starter 85% of the time and a backup 15% of the time.
                let f = &self.finances[ti];
                let pays = !f.bankrupt && f.cash >= cost * 3 && r.chance(if starter { ConductTuning::AI_PAY_STARTER } else { ConductTuning::AI_PAY_BACKUP });
                let vars = [("p", p.name.clone()), ("pos", p.position.label().to_string()), ("town", self.teams[ti].town.name.clone()), ("job", p.day_job.to_lowercase())];
                if pays {
                    self.finances[ti].cash -= cost;
                } else {
                    let cur = self.injured_games.get(&p.id).copied().unwrap_or(0);
                    self.injured_games.insert(p.id, cur.max(games));
                    self.out_reason.insert(p.id, format!("Suspended: {}", inc.title));
                    self.record_absence(p.id, week_no, false, &format!("Suspended ({})", inc.title.to_lowercase()), games);
                }
                self.conduct_gossip.push(ConductGossip { team: ti, player_id: p.id, title: inc.title.clone(), body: fill(&inc.body, &vars), cost, games, paid: pays });
                break;
            }
        }
    }

    /// Pay to make it go away. Returns false if it is not open or you cannot afford it.
    pub fn pay_conduct(&mut self, item_id: u32) -> bool {
        let u = self.user();
        let Some(item) = self.inbox.iter().find(|i| i.id == item_id).cloned() else { return false };
        if item.kind != InboxKind::Conduct || item.conduct_state != ConductState::Open || self.finances[u].cash < item.conduct_cost {
            return false;
        }
        let name = self.teams[u].roster.iter().find(|p| Some(p.id) == item.player_id).map(|p| p.name.clone()).unwrap_or_else(|| "a player".to_string());
        let (season, week) = (self.season, self.week);
        let f = &mut self.finances[u];
        f.cash -= item.conduct_cost;
        f.ledger.push(LedgerEntry::new(season, week, LedgerKind::Other, -item.conduct_cost, &format!("off-field trouble: {name}")));
        if let Some(i) = self.inbox_item_mut(item_id) {
            i.conduct_state = ConductState::Paid;
            i.unread = false;
        }
        true
    }

    /// Let him face it: he sits out `conduct_games` games (at least one).
    pub fn let_him_sit(&mut self, item_id: u32) -> bool {
        let u = self.user();
        let Some(item) = self.inbox.iter().find(|i| i.id == item_id).cloned() else { return false };
        if item.kind != InboxKind::Conduct || item.conduct_state != ConductState::Open {
            return false;
        }
        if let Some(i) = self.inbox_item_mut(item_id) {
            i.conduct_state = ConductState::Sits;
            i.unread = false;
        }
        let Some(id) = item.player_id else { return true };
        if !self.teams[u].roster.iter().any(|p| p.id == id) {
            return true;
        }
        let games = 1.max(item.conduct_games);
        let what = item.title.split(": ").last().unwrap_or("").to_lowercase();
        self.conduct_gossip.push(ConductGossip { team: u, player_id: id, title: what.clone(), body: item.text.split("\n\n").next().unwrap_or("").to_string(), cost: item.conduct_cost, games, paid: false });
        let f = &mut self.finances[u];
        f.fan_interest = (f.fan_interest - ConductTuning::FAN_HIT_PER_GAME * games as f64).clamp(0.0, 100.0);
        let cur = self.injured_games.get(&id).copied().unwrap_or(0);
        self.injured_games.insert(id, cur.max(games));
        self.out_reason.insert(id, format!("Suspended: {what}"));
        self.record_absence(id, item.week, false, &format!("Suspended ({what})"), games);
        true
    }

    /// A letter you ignore counts as "let him sit" when the next game is played.
    pub fn resolve_open_conduct(&mut self) {
        let ids: Vec<u32> = self.inbox.iter().filter(|m| m.kind == InboxKind::Conduct && m.conduct_state == ConductState::Open).map(|m| m.id).collect();
        for id in ids {
            self.let_him_sit(id);
        }
    }
}
