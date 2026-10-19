use crate::ai_business::{ai_in_austerity, ai_payroll_budget, ai_reserve};
use crate::inbox::{InboxFeed, InboxKind, PostOpts};
use crate::league::{League, LeaguePhase};
use serde::{Deserialize, Serialize};
use sp_core::geo::town_miles;
use sp_core::team::{ACTIVE_ROSTER_SIZE, ROSTER_MIX, min_active_coverage};
use sp_core::{Player, Position};
use sp_economy::{Contract, LedgerEntry, LedgerKind, PayScale, rookie_scale::RookieScale};
use std::collections::BTreeSet;

/// Tuning for trades. Everything is "points": a rough measure of what an asset is worth to a team.
/// All values are tuning seeds.
pub struct TradeTuning;

impl TradeTuning {
    /// Regular-season trades close after this week (they stay open all offseason).
    pub const DEADLINE_WEEK: i32 = 10;
    /// A just-signed free agent cannot be flipped for this many weeks (otherwise sign-and-trade would print picks).
    pub const LOCK_AFTER_SIGNING: i32 = 6;
    pub const GAMES_PER_SEASON: i64 = 14;
    /// Dollars that equal one point (cash in a deal, and contract savings, are counted in points).
    pub const CASH_PER_POINT: f64 = 40.0;
    /// Talent above a replacement-level player (a free-agent walk-on), per season: scale x (e^(slope x (ovr - replacement)) - 1).
    pub const TALENT_SCALE: f64 = 60.0;
    pub const TALENT_SLOPE: f64 = 0.07;
    pub const REPLACEMENT_OVR: i32 = 45;
    /// Each later season of a contract counts a little less.
    pub const SEASON_DISCOUNT: f64 = 0.85;
    /// A pick that is a year further out is worth a little less (and is less certain).
    pub const PICK_YEAR_DISCOUNT: f64 = 0.9;
    /// The AI wants more than it gives: incoming >= outgoing x (1 + margin) + minGap.
    pub const MARGIN: f64 = 0.12;
    pub const MIN_GAP: f64 = 8.0;
    /// Players the team is built around (its best two) cost extra to pry away.
    pub const CORE_MARGIN: f64 = 0.30;
    /// Giving up a starter hurts; giving up a bench player hardly does. Taking a starter helps, a bench body barely.
    pub const STARTER_OUT: f64 = 1.15;
    pub const THIN_DEPTH_EXTRA: f64 = 0.25;
    pub const BENCH_OUT: f64 = 0.8;
    pub const BENCH_IN: f64 = 0.5;
    /// Quantity for quality does not work: the AI counts its incoming assets best-first with these weights.
    pub const CONSOLIDATION: [f64; 4] = [1.0, 0.85, 0.7, 0.55];
    pub const CONSOLIDATION_REST: f64 = 0.4;
    /// Rebuilding teams (bottom of the table) prize picks and young players, contenders prize veterans.
    pub const REBUILD_PICK: f64 = 1.15;
    pub const REBUILD_VET: f64 = 0.85;
    pub const CONTEND_VET: f64 = 1.1;
    pub const CONTEND_PICK: f64 = 0.9;
    pub const REBUILD_BOTTOM: usize = 4;
    pub const CONTEND_TOP: usize = 4;
    /// Injured players are worth less while they are out.
    pub const INJURY_CUT_PER_GAME: f64 = 0.04;
    pub const INJURY_CUT_MAX: f64 = 0.5;
    /// A cash-strapped AI team values cash more.
    pub const AUSTERITY_CASH: f64 = 1.5;
    /// Roster limits the AI keeps: never drops under `MIN_ROSTER`, never grows beyond the position mix + `ROSTER_SLACK`.
    pub const MIN_ROSTER: usize = 38;
    pub const ROSTER_SLACK: usize = 4;
    /// Picks of this many upcoming drafts can be traded.
    pub const PICK_SEASONS: i32 = 2;
    /// Picks are uncertain (the rookie may bust, he needs a roster spot): worth this share of the rookie's value.
    pub const PICK_SCALE: f64 = 0.7;
    /// Floor for the worth of any pick (the last rounds still bring a body).
    pub const PICK_FLOOR: f64 = 3.0;
    /// Expected rating of the player taken at an overall slot (the class is a bit better than the usual free agent). (slot, ovr)
    pub const SLOT_OVR: [(i32, f64); 8] = [(1, 70.0), (3, 65.0), (6, 63.0), (12, 59.0), (24, 52.0), (36, 47.0), (48, 44.0), (60, 41.0)];
}

pub fn acquired_trade(season: i32) -> String {
    format!("S{season} Trade")
}

/// One draft pick: the `season` of the draft, `round`, and the team it originally belonged to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PickRef {
    pub season: i32,
    pub round: i32,
    pub original_team: usize,
}

impl PickRef {
    pub fn key(&self) -> String {
        format!("{}:{}:{}", self.season, self.round, self.original_team)
    }
}

/// A proposal. "give" is what you send, "get" is what you ask for.
#[derive(Clone, Debug, Default)]
pub struct TradeOffer {
    pub partner: usize,
    pub give_players: BTreeSet<u32>,
    pub get_players: BTreeSet<u32>,
    pub give_picks: Vec<PickRef>,
    pub get_picks: Vec<PickRef>,
    pub give_cash: i64,
    pub get_cash: i64,
}

impl TradeOffer {
    pub fn new(partner: usize) -> Self {
        Self { partner, ..Default::default() }
    }

    pub fn is_empty(&self) -> bool {
        self.give_players.is_empty() && self.get_players.is_empty() && self.give_picks.is_empty() && self.get_picks.is_empty() && self.give_cash == 0 && self.get_cash == 0
    }

    pub fn has_give(&self) -> bool {
        !self.give_players.is_empty() || !self.give_picks.is_empty() || self.give_cash > 0
    }

    pub fn has_get(&self) -> bool {
        !self.get_players.is_empty() || !self.get_picks.is_empty() || self.get_cash > 0
    }

    pub fn plus(&self, a: &TradeAdd) -> TradeOffer {
        let mut o = self.clone();
        match a {
            TradeAdd::Player { player_id, .. } => {
                o.give_players.insert(*player_id);
            }
            TradeAdd::Pick { pick, .. } => o.give_picks.push(*pick),
            TradeAdd::Cash { cash, .. } => o.give_cash += cash,
        }
        o
    }
}

/// Something the AI would ask you to add.
#[derive(Clone, Debug, PartialEq)]
pub enum TradeAdd {
    Player { label: String, player_id: u32 },
    Pick { label: String, pick: PickRef },
    Cash { label: String, cash: i64 },
}

impl TradeAdd {
    pub fn label(&self) -> &str {
        match self {
            TradeAdd::Player { label, .. } | TradeAdd::Pick { label, .. } | TradeAdd::Cash { label, .. } => label,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TradeMood {
    Yes,
    Close,
    No,
}

/// The partner's answer to a proposal.
#[derive(Clone, Debug)]
pub struct TradeVerdict {
    pub accepted: bool,
    pub mood: TradeMood,
    /// A hard rule that blocks the deal (window closed, roster limits, cash). `None` when the deal is legal.
    pub blocked: Option<String>,
    /// What the deal is worth to them: what they receive vs what they give up (both adjusted for their needs), in points.
    pub receive: f64,
    pub give: f64,
    /// How far short the offer is, in points (0 when it works).
    pub short_by: f64,
    pub line: String,
    pub notes: Vec<String>,
    pub ask: Option<TradeAdd>,
}

impl TradeVerdict {
    /// 1.0 = exactly what they want; the bar the UI shows.
    pub fn ratio(&self) -> f64 {
        if self.give <= 0.0 { if self.receive > 0.0 { 2.0 } else { 0.0 } } else { self.receive / self.give }
    }
}

/// What a player looks like in a trade for the team that would get him (for the trade screens).
#[derive(Clone, Debug)]
pub struct TradePlayerLine {
    pub player: Player,
    pub contract: Option<Contract>,
    pub from_team: usize,
    pub to_team: usize,
    pub old_commute: i64,
    pub new_commute: i64,
    /// Per game while he dresses, with the commute for `to_team`.
    pub pay_now: i64,
    /// What he would ask `to_team` on a 3-year deal.
    pub market: i64,
    pub seasons_left: i32,
    /// Total remaining pay at `to_team`.
    pub commitment: i64,
    /// What cutting him would cost `to_team`.
    pub release_cost: i64,
}

impl TradePlayerLine {
    /// Market ask minus pay: positive = a bargain for the team that gets him.
    pub fn surplus_per_game(&self) -> i64 {
        self.market - self.pay_now
    }
}

/// The money side of a proposal, from your point of view.
#[derive(Clone, Debug)]
pub struct TradeBook {
    pub incoming: Vec<TradePlayerLine>,
    pub outgoing: Vec<TradePlayerLine>,
    /// Change in your per-game pay if everyone dressed (+ costs more).
    pub payroll_delta: i64,
    pub commitment_in: i64,
    pub commitment_out: i64,
    /// Cash you receive minus cash you send.
    pub cash_delta: i64,
    pub cash_after: i64,
    pub roster_after: usize,
    pub warnings: Vec<String>,
}

impl League {
    // ------------------------------------------------------------- clock + windows

    /// A counter that only goes up: 20 per season, regular-season weeks 1-16, the offseason sits at 18.
    pub fn trade_clock(&self) -> i32 {
        (self.season - 1) * 20 + if self.phase == LeaguePhase::Offseason { 18 } else { self.week }
    }

    /// Why trades are closed right now (`None` when open).
    pub fn trade_closed_reason(&self) -> Option<String> {
        if self.user_team.is_none() {
            return Some("There is no owner to trade.".into());
        }
        match self.phase {
            LeaguePhase::Offseason => None,
            LeaguePhase::RegularSeason => {
                if self.week > TradeTuning::DEADLINE_WEEK {
                    Some(format!("The trade deadline passed after week {}. Trades reopen in the offseason.", TradeTuning::DEADLINE_WEEK))
                } else {
                    None
                }
            }
            _ => Some("Trades are closed during the playoffs. They reopen in the offseason.".into()),
        }
    }

    /// Weeks until a just-signed player can be traded (0 = free to move).
    pub fn trade_lock_weeks(&self, player_id: u32) -> i32 {
        (self.trade_lock.get(&player_id).copied().unwrap_or(0) - self.trade_clock()).max(0)
    }

    /// Why `p` on `team` cannot be traded (`None` if he can).
    pub fn untradeable_reason(&self, team: usize, p: &Player) -> Option<String> {
        if Some(team) == self.user_team && self.pending_expired.contains(&p.id) {
            return Some("His contract is up".into());
        }
        let w = self.trade_lock_weeks(p.id);
        if w > 0 {
            return Some(format!("Just signed: {w} more week{}", if w == 1 { "" } else { "s" }));
        }
        None
    }

    // ---------------------------------------------------------------- picks

    /// The upcoming drafts whose picks can be traded: the live one (while it still has picks) plus the next.
    pub fn tradable_pick_seasons(&self) -> Vec<i32> {
        let first = if self.draft.as_ref().map(|d| d.complete()).unwrap_or(false) { self.season + 1 } else { self.season };
        (0..TradeTuning::PICK_SEASONS).map(|i| first + i).collect()
    }

    pub fn pick_owner(&self, p: &PickRef) -> usize {
        self.pick_owners.get(&p.key()).copied().unwrap_or(p.original_team)
    }

    /// Every pick `team` holds right now, soonest and highest round first.
    pub fn picks_of(&self, team: usize) -> Vec<PickRef> {
        let mut out = Vec::new();
        for s in self.tradable_pick_seasons() {
            for r in 1..=RookieScale::ROUNDS {
                for o in 0..self.teams.len() {
                    let p = PickRef { season: s, round: r, original_team: o };
                    if self.pick_owner(&p) != team || self.pick_spent(&p) {
                        continue;
                    }
                    out.push(p);
                }
            }
        }
        out
    }

    /// Draft slot (1-based overall) of `p` once the order is known, else `None`.
    pub fn pick_slot(&self, p: &PickRef) -> Option<usize> {
        let d = self.draft.as_ref()?;
        if d.season != p.season {
            return None;
        }
        let i = d.order.iter().position(|&t| t == p.original_team)?;
        Some((p.round as usize - 1) * d.teams() + i + 1)
    }

    fn pick_spent(&self, p: &PickRef) -> bool {
        match (&self.draft, self.pick_slot(p)) {
            (Some(d), Some(slot)) if d.season == p.season => slot <= d.made(),
            _ => false,
        }
    }

    /// Label for screens: "Season 2 round 1 (Fresno)", with the slot when it is known.
    pub fn pick_label(&self, p: &PickRef) -> String {
        let slot = self.pick_slot(p);
        let from = if Some(p.original_team) == self.user_team { "yours".to_string() } else { self.teams[p.original_team].town.name.clone() };
        let y = format!("Season {}", p.season);
        match slot {
            Some(s) => format!("{y} round {}, pick #{s} ({from})", p.round),
            None => format!("{y} round {} ({from})", p.round),
        }
    }

    // ------------------------------------------------------------ valuation

    fn talent(&self, ovr: i32, pos: Position, age: i32) -> f64 {
        let base = TradeTuning::TALENT_SCALE * ((TradeTuning::TALENT_SLOPE * (ovr - TradeTuning::REPLACEMENT_OVR) as f64).exp() - 1.0);
        if base <= 0.0 {
            return 0.0;
        }
        let age_mod = if age <= 23 {
            1.1
        } else if age <= 26 {
            1.0
        } else if age <= 28 {
            0.92
        } else if age <= 30 {
            0.78
        } else if age <= 32 {
            0.6
        } else {
            0.4
        };
        base * PayScale::position_mod(pos) * age_mod
    }

    /// Points for a player over a contract: talent each season plus the money saved against the market. Neutral view (no roster fit).
    fn contract_value(&self, ovr: i32, pos: Position, age: i32, ppg: i64, commute: i64, seasons: i32, market: i64) -> f64 {
        let (mut v, mut d) = (0.0, 1.0);
        for s in 0..seasons {
            let a = age + s;
            let o = ovr + if age <= 25 { 0 } else if age >= 30 { -s } else { 0 }; // young players hold; old ones slide
            v += d * (self.talent(o, pos, a) + (market - ppg - commute) as f64 * TradeTuning::GAMES_PER_SEASON as f64 / TradeTuning::CASH_PER_POINT);
            d *= TradeTuning::SEASON_DISCOUNT;
        }
        v.max(0.0)
    }

    fn commute_for(&self, p: &Player, team: usize) -> i64 {
        PayScale::commute(town_miles(&p.hometown, &self.teams[team].town.name))
    }

    /// What a player is worth, neutral to team fit, to the team `viewer` (contract and commute as that team would pay).
    pub fn player_worth(&self, p: &Player, owner: usize, viewer: usize) -> f64 {
        let c = self.finances[owner].contracts.get(&p.id);
        let town = &self.teams[viewer].town;
        let seasons = 1.max(c.map(|c| c.seasons_left).unwrap_or(1));
        let market = PayScale::ask(p, town, 3);
        let ppg = c.map(|c| c.ppg).unwrap_or(market);
        let mut v = self.contract_value(p.ovr(), p.position, p.age, ppg, self.commute_for(p, viewer), seasons, market);
        let out = self.injured_games.get(&p.id).copied().unwrap_or(0);
        if out > 0 {
            v *= 1.0 - TradeTuning::INJURY_CUT_MAX.min(out as f64 * TradeTuning::INJURY_CUT_PER_GAME);
        }
        v
    }

    fn expected_ovr(&self, slot: i32) -> f64 {
        let t = &TradeTuning::SLOT_OVR;
        if slot <= t[0].0 {
            return t[0].1;
        }
        for i in 1..t.len() {
            if slot <= t[i].0 {
                let ((s0, o0), (s1, o1)) = (t[i - 1], t[i]);
                return o0 + (o1 - o0) * (slot - s0) as f64 / (s1 - s0) as f64;
            }
        }
        t[t.len() - 1].1
    }

    /// Strength of each team (higher = stronger), from this season's record (once played) blended with roster quality.
    fn strength(&self) -> Vec<f64> {
        let roster = |t: &sp_core::Team| {
            let mut o: Vec<i32> = t.roster.iter().map(|p| p.ovr()).collect();
            o.sort_by(|a, b| b.cmp(a));
            let top: Vec<i32> = o.into_iter().take(22).collect();
            if top.is_empty() { 0.0 } else { top.iter().sum::<i32>() as f64 / top.len() as f64 }
        };
        let r: Vec<f64> = self.teams.iter().map(roster).collect();
        let lo = r.iter().cloned().fold(f64::INFINITY, f64::min);
        let hi = r.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        (0..self.teams.len())
            .map(|i| {
                let roster_score = if hi == lo { 0.5 } else { (r[i] - lo) / (hi - lo) };
                let rec = &self.standings.records[i];
                let w = (rec.games() as f64 / 14.0).clamp(0.0, 1.0) * 0.6;
                roster_score * (1.0 - w) + rec.pct() * w
            })
            .collect()
    }

    /// The overall slot we expect a pick to land at (known once the order is set; projected from team strength before).
    fn expected_slot(&self, p: &PickRef, strength: &[f64]) -> f64 {
        if let Some(k) = self.pick_slot(p) {
            return k as f64;
        }
        let mut order: Vec<usize> = (0..self.teams.len()).collect();
        order.sort_by(|&a, &b| strength[a].total_cmp(&strength[b]));
        let rank = order.iter().position(|&t| t == p.original_team).unwrap() as f64; // 0 = weakest = picks first
        let years_out = (p.season - self.tradable_pick_seasons()[0]).max(0) as f64;
        let regress = (0.25 * years_out + 0.1).min(1.0); // the further out, the closer to the middle
        let mid = (self.teams.len() as f64 - 1.0) / 2.0;
        let pos = rank + (mid - rank) * regress;
        (p.round - 1) as f64 * self.teams.len() as f64 + pos + 1.0
    }

    /// Neutral worth of a pick: the rookie it should bring (rating, a cheap fixed-pay deal) in points.
    pub fn pick_worth(&self, p: &PickRef, strength: Option<&[f64]>) -> f64 {
        let owned;
        let s: &[f64] = match strength {
            Some(s) => s,
            None => {
                owned = self.strength();
                &owned
            }
        };
        let slot = self.expected_slot(p, s);
        let n_slot = (slot.round() as i32).clamp(1, RookieScale::ROUNDS * self.teams.len() as i32);
        let mut ovr = self.expected_ovr(n_slot);
        if let Some(d) = &self.draft {
            if d.season == p.season && !d.prospects.is_empty() {
                // The class is on the board: use the real prospects still there.
                let k = ((n_slot - 1 - d.made() as i32).max(0) as usize).min(d.prospects.len() - 1);
                let mut sorted: Vec<i32> = d.prospects.iter().map(|x| x.ovr()).collect();
                sorted.sort_by(|a, b| b.cmp(a));
                ovr = (ovr + sorted[k] as f64) / 2.0;
            }
        }
        let round = p.round;
        let pick_in_round = ((n_slot - 1) % self.teams.len() as i32) + 1;
        let ppg = RookieScale::ppg(round, pick_in_round, self.teams.len() as i32);
        let seasons = RookieScale::seasons_for_round(round);
        let market = (PayScale::ovr_curve(ovr.round() as i32) * 0.9).round() as i64;
        let v = self.contract_value(ovr.round() as i32, Position::Wr, 24, ppg, 0, seasons, market);
        let years_out = (p.season - self.tradable_pick_seasons()[0]).max(0);
        TradeTuning::PICK_FLOOR.max(v * TradeTuning::PICK_SCALE * TradeTuning::PICK_YEAR_DISCOUNT.powi(years_out))
    }

    // ----------------------------------------------------- the AI's view

    /// "Rebuilding", "Contending" or "Retooling": how the team sees the season.
    pub fn trade_mood(&self, team: usize) -> &'static str {
        let st = self.strength();
        let mut order: Vec<usize> = (0..self.teams.len()).collect();
        order.sort_by(|&a, &b| st[a].total_cmp(&st[b]));
        let rank = order.iter().position(|&t| t == team).unwrap();
        if rank < TradeTuning::REBUILD_BOTTOM {
            "Rebuilding"
        } else if rank >= self.teams.len() - TradeTuning::CONTEND_TOP {
            "Contending"
        } else {
            "Retooling"
        }
    }

    fn rev(&self, i: usize) -> i64 {
        let f = &self.finances[i];
        if f.games.is_empty() {
            (f.town.population as i64).clamp(1000, 12000)
        } else {
            (f.games.iter().map(|g| g.total_revenue()).sum::<i64>() as f64 / f.games.len() as f64).round() as i64
        }
    }

    /// How the team sorts its position group: best to worst.
    fn group(&self, team: usize, pos: Position) -> Vec<&Player> {
        let mut v: Vec<&Player> = self.teams[team].roster.iter().filter(|p| p.position == pos).collect();
        v.sort_by_key(|p| std::cmp::Reverse(p.ovr()));
        v
    }

    /// Worth of `p` LEAVING `team` (what the team gives up).
    fn worth_leaving(&self, team: usize, p: &Player, mood: &str) -> f64 {
        let mut v = self.player_worth(p, team, team);
        let group = self.group(team, p.position);
        let starters_n = 1.max(min_active_coverage(p.position));
        let rank = group.iter().position(|x| x.id == p.id);
        let starter = rank.map(|r| r < starters_n).unwrap_or(false);
        if starter {
            v *= TradeTuning::STARTER_OUT;
            if group.len() <= starters_n {
                v *= 1.0 + TradeTuning::THIN_DEPTH_EXTRA; // nobody behind him
            }
        } else {
            v *= TradeTuning::BENCH_OUT;
        }
        v * vet_mood(p, mood)
    }

    /// Worth of `p` ARRIVING at `team` (contract and commute as that team would pay).
    fn worth_arriving(&self, from: usize, team: usize, p: &Player, mood: &str) -> f64 {
        let mut v = self.player_worth(p, from, team);
        let group = self.group(team, p.position);
        let starters_n = 1.max(min_active_coverage(p.position));
        let would_start = group.len() < starters_n || p.ovr() > group[starters_n - 1].ovr();
        if !would_start {
            v *= TradeTuning::BENCH_IN;
        }
        v * vet_mood(p, mood)
    }

    fn cash_points(&self, team: usize, dollars: i64) -> f64 {
        let f = &self.finances[team];
        let austere = Some(team) != self.user_team && ai_in_austerity(self.rev(team) as f64, f.cash);
        dollars as f64 / TradeTuning::CASH_PER_POINT * if austere { TradeTuning::AUSTERITY_CASH } else { 1.0 }
    }

    /// Sums asset values best-first with the quantity-for-quality weights.
    fn consolidated(values: &[f64]) -> f64 {
        let mut v = values.to_vec();
        v.sort_by(|a, b| b.total_cmp(a));
        v.iter().enumerate().map(|(i, x)| x * if i < TradeTuning::CONSOLIDATION.len() { TradeTuning::CONSOLIDATION[i] } else { TradeTuning::CONSOLIDATION_REST }).sum()
    }

    fn player_on(&self, team: usize, id: u32) -> Option<&Player> {
        self.teams[team].roster.iter().find(|p| p.id == id)
    }

    // ------------------------------------------------------ evaluate a deal

    /// Hard rules first: window, ownership, locks, cash and roster limits. `None` when the deal is legal.
    fn blocked_reason(&self, o: &TradeOffer) -> Option<String> {
        if let Some(closed) = self.trade_closed_reason() {
            return Some(closed);
        }
        let me = self.user();
        let other = o.partner;
        if other == me || other >= self.teams.len() {
            return Some("Pick another team.".into());
        }
        if self.finances[other].bankrupt {
            return Some(format!("{} is bankrupt.", self.teams[other].town.team_name()));
        }
        if o.is_empty() || (!o.has_give() && !o.has_get()) {
            return Some("Add something to the deal.".into());
        }
        for &id in &o.give_players {
            let Some(p) = self.player_on(me, id) else { return Some("One of your players is no longer on the roster.".into()) };
            if let Some(why) = self.untradeable_reason(me, p) {
                return Some(format!("{}: {why}.", p.name));
            }
        }
        for &id in &o.get_players {
            let Some(p) = self.player_on(other, id) else { return Some("One of their players is no longer on their roster.".into()) };
            if let Some(why) = self.untradeable_reason(other, p) {
                return Some(format!("{}: {why}.", p.name));
            }
        }
        for p in &o.give_picks {
            if self.pick_owner(p) != me || self.pick_spent(p) || !self.tradable_pick_seasons().contains(&p.season) {
                return Some(format!("You no longer hold the {} pick.", self.pick_label(p)));
            }
        }
        for p in &o.get_picks {
            if self.pick_owner(p) != other || self.pick_spent(p) || !self.tradable_pick_seasons().contains(&p.season) {
                return Some(format!("They no longer hold the {} pick.", self.pick_label(p)));
            }
        }
        if o.give_cash < 0 || o.get_cash < 0 {
            return Some("Cash cannot be negative.".into());
        }
        if o.give_cash > 0 && o.get_cash > 0 {
            return Some("Send cash or ask for cash, not both.".into());
        }
        if o.give_cash > self.finances[me].cash {
            return Some(format!("You do not have {} in cash to send.", o.give_cash));
        }
        if o.get_cash > 0 {
            let f = &self.finances[other];
            let spare = f.cash - ai_reserve(self.rev(other) as f64).round() as i64;
            if spare < o.get_cash {
                return Some(format!("{} cannot spare that much cash.", self.teams[other].town.name));
            }
        }
        // Roster limits for the partner.
        let t = &self.teams[other];
        let n = t.roster.len() + o.give_players.len() - o.get_players.len();
        let cap = ROSTER_MIX.iter().map(|m| m.1).sum::<usize>() + TradeTuning::ROSTER_SLACK;
        if n < TradeTuning::MIN_ROSTER {
            return Some(format!("{} would be left with only {n} players.", t.town.name));
        }
        if n > cap {
            return Some(format!("{} has no room for that many players.", t.town.name));
        }
        for &pos in &Position::ALL {
            let need = min_active_coverage(pos);
            let have = t.roster.iter().filter(|p| p.position == pos && !o.get_players.contains(&p.id)).count()
                + o.give_players.iter().filter_map(|id| self.player_on(me, *id)).filter(|p| p.position == pos).count();
            // Only refuse a deal that makes a hole worse: a club already short (retirements, before it refills) can still take a gift.
            let now = t.roster.iter().filter(|p| p.position == pos).count();
            if have < need && have < now {
                return Some(format!("{} would be short at {}.", t.town.name, pos.label()));
            }
        }
        None
    }


    /// The partner's answer. No side effects.
    pub fn evaluate_trade(&self, o: &TradeOffer) -> TradeVerdict {
        if let Some(blocked) = self.blocked_reason(o) {
            return TradeVerdict { accepted: false, mood: TradeMood::No, blocked: Some(blocked.clone()), receive: 0.0, give: 0.0, short_by: 0.0, line: blocked, notes: vec![], ask: None };
        }
        let me = self.user();
        let other = o.partner;
        let mood = self.trade_mood(other);
        let st = self.strength();
        let mut notes: Vec<String> = Vec::new();

        // What the partner receives (from you) and gives up.
        let mut incoming: Vec<f64> = Vec::new();
        for &id in &o.give_players {
            incoming.push(self.worth_arriving(me, other, self.player_on(me, id).unwrap(), mood));
        }
        for p in &o.give_picks {
            incoming.push(self.pick_worth(p, Some(&st)) * pick_mood(mood));
        }
        let mut receive = Self::consolidated(&incoming) + if o.give_cash > 0 { self.cash_points(other, o.give_cash) } else { 0.0 };

        let mut give = 0.0;
        let mut core = false;
        let mut by_ovr: Vec<&Player> = self.teams[other].roster.iter().collect();
        by_ovr.sort_by_key(|p| std::cmp::Reverse(p.ovr()));
        let top: Vec<u32> = by_ovr.iter().take(2).map(|p| p.id).collect();
        for &id in &o.get_players {
            let p = self.player_on(other, id).unwrap();
            give += self.worth_leaving(other, p, mood);
            if top.contains(&id) {
                core = true;
                notes.push(format!("{} is one of the players they are built around.", p.name));
            }
        }
        for p in &o.get_picks {
            give += self.pick_worth(p, Some(&st)) * pick_mood(mood);
        }
        if o.get_cash > 0 {
            give += self.cash_points(other, o.get_cash);
        }

        // Payroll: taking on more pay than they send out matters when the budget is tight.
        let fin = &self.finances[other];
        let (mut in_pay, mut out_pay) = (0i64, 0i64);
        for &id in &o.give_players {
            let p = self.player_on(me, id).unwrap();
            let c = self.finances[me].contracts.get(&id);
            in_pay += c.map(|c| c.ppg).unwrap_or(0) + self.commute_for(p, other);
        }
        for &id in &o.get_players {
            out_pay += fin.contracts.get(&id).map(|c| c.active_pay()).unwrap_or(0);
        }
        let rev = self.rev(other) as f64;
        let budget = ai_payroll_budget(rev, fin.cash);
        let pay_after = (fin.projected_payroll(&self.teams[other]) + (in_pay - out_pay)) as f64;
        if in_pay > out_pay && pay_after > budget {
            if ai_in_austerity(rev, fin.cash) {
                notes.push("They are watching every dollar and cannot take on that pay.".into());
                return TradeVerdict {
                    accepted: false,
                    mood: TradeMood::No,
                    blocked: Some(format!("{} cannot afford the extra pay right now.", self.teams[other].town.name)),
                    receive,
                    give,
                    short_by: 0.0,
                    line: "We can't afford the salaries. Try sending less pay or more cash.".into(),
                    notes,
                    ask: None,
                };
            }
            let overshoot = (pay_after - budget) * TradeTuning::GAMES_PER_SEASON as f64 / TradeTuning::CASH_PER_POINT;
            receive -= overshoot;
            notes.push("The extra pay strains their payroll budget.".into());
        }

        let need = give * (1.0 + if core { TradeTuning::CORE_MARGIN } else { TradeTuning::MARGIN }) + if give > 0.0 { TradeTuning::MIN_GAP } else { 0.0 };
        // A gift is a gift; they still need the roster fit and the money check above.
        let ok = receive >= need;
        let short = if ok { 0.0 } else { need - receive };
        let ratio = if need <= 0.0 { 2.0 } else { receive / need };
        let m = if ok { TradeMood::Yes } else if ratio >= 0.8 { TradeMood::Close } else { TradeMood::No };
        if !ok {
            notes.push(format!("{} are {}.", self.teams[other].town.nickname, mood.to_lowercase()));
        }
        let line = if ok { yes_line(other) } else if m == TradeMood::Close { close_line(other) } else { no_line(other, ratio) };
        TradeVerdict { accepted: ok, mood: m, blocked: None, receive, give: need, short_by: short, line, notes, ask: if ok { None } else { self.suggest(o, short) } }
    }

    // ---------------------------------------------------------- suggestions

    /// The smallest thing you own that closes the gap `short_by` (in the partner's points): a pick, cash or a player.
    fn suggest(&self, o: &TradeOffer, short_by: f64) -> Option<TradeAdd> {
        if short_by <= 0.0 {
            return None;
        }
        let me = self.user();
        let other = o.partner;
        let mood = self.trade_mood(other);
        let st = self.strength();
        let mut cands: Vec<(TradeAdd, f64)> = Vec::new();
        for p in self.picks_of(me) {
            if o.give_picks.contains(&p) {
                continue;
            }
            cands.push((TradeAdd::Pick { label: self.pick_label(&p), pick: p }, self.pick_worth(&p, Some(&st)) * pick_mood(mood)));
        }
        let spendable = self.finances[me].cash - o.give_cash;
        for step in [500i64, 1000, 2000, 3000, 5000, 8000, 12000] {
            if spendable >= step + 5000 {
                cands.push((TradeAdd::Cash { label: format!("${step} in cash"), cash: step }, self.cash_points(other, step)));
            }
        }
        let mut by_ovr: Vec<&Player> = self.teams[me].roster.iter().collect();
        by_ovr.sort_by_key(|p| std::cmp::Reverse(p.ovr()));
        let best: Vec<u32> = by_ovr.iter().take(5).map(|p| p.id).collect();
        for p in &self.teams[me].roster {
            if o.give_players.contains(&p.id) || best.contains(&p.id) || self.untradeable_reason(me, p).is_some() {
                continue;
            }
            cands.push((TradeAdd::Player { label: format!("{} {} ({})", p.position.label(), p.name, p.ovr()), player_id: p.id }, self.worth_arriving(me, other, p, mood) * 0.85));
        }
        // Because later assets count for less, ask for a bit more than the raw gap.
        let target = short_by / 0.85;
        let mut enough: Vec<&(TradeAdd, f64)> = cands.iter().filter(|c| c.1 >= target).collect();
        enough.sort_by(|a, b| a.1.total_cmp(&b.1));
        if let Some(e) = enough.first() {
            return Some(e.0.clone());
        }
        cands.sort_by(|a, b| b.1.total_cmp(&a.1));
        cands.into_iter().next().map(|c| c.0)
    }

    // ---------------------------------------------------------- the books

    pub fn trade_line(&self, p: &Player, from_team: usize, to_team: usize) -> TradePlayerLine {
        let c = self.finances[from_team].contracts.get(&p.id).cloned();
        let town = &self.teams[to_team].town;
        let market = PayScale::ask(p, town, 3);
        let new_commute = self.commute_for(p, to_team);
        let ppg = c.as_ref().map(|c| c.ppg).unwrap_or(market);
        let seasons = c.as_ref().map(|c| c.seasons_left).unwrap_or(1);
        let pay = ppg + new_commute;
        TradePlayerLine {
            player: p.clone(),
            old_commute: c.as_ref().map(|c| c.commute).unwrap_or_else(|| self.commute_for(p, from_team)),
            contract: c,
            from_team,
            to_team,
            new_commute,
            pay_now: pay,
            market,
            seasons_left: seasons,
            commitment: pay * TradeTuning::GAMES_PER_SEASON * seasons.max(0) as i64,
            release_cost: (seasons.max(0) as i64 as f64 * TradeTuning::GAMES_PER_SEASON as f64 * ppg as f64 * Contract::RELEASE_PCT).round() as i64,
        }
    }

    /// The money for you: pay, commitments, cash and roster size after the deal.
    pub fn trade_book(&self, o: &TradeOffer) -> TradeBook {
        let me = self.user();
        let other = o.partner;
        let inc: Vec<TradePlayerLine> = o.get_players.iter().filter_map(|id| self.player_on(other, *id)).map(|p| self.trade_line(p, other, me)).collect();
        let out: Vec<TradePlayerLine> = o.give_players.iter().filter_map(|id| self.player_on(me, *id)).map(|p| self.trade_line(p, me, other)).collect();
        // Your side of an outgoing line: what you stop paying (old commute).
        let out_pay: Vec<i64> = out.iter().map(|l| l.contract.as_ref().map(|c| c.ppg).unwrap_or(0) + l.old_commute).collect();
        let payroll_delta = inc.iter().map(|l| l.pay_now).sum::<i64>() - out_pay.iter().sum::<i64>();
        let commitment_in: i64 = inc.iter().map(|l| l.commitment).sum();
        let commitment_out: i64 = out.iter().enumerate().map(|(i, l)| out_pay[i] * TradeTuning::GAMES_PER_SEASON * l.seasons_left.max(0) as i64).sum();
        let cash_delta = o.get_cash - o.give_cash;
        let roster_after = self.teams[me].roster.len() + inc.len() - out.len();
        let mut warn = Vec::new();
        if roster_after < ACTIVE_ROSTER_SIZE {
            warn.push(format!("You would have {roster_after} players, fewer than the {ACTIVE_ROSTER_SIZE} you dress."));
        }
        for &pos in &Position::ALL {
            let need = min_active_coverage(pos);
            let have = self.teams[me].roster.iter().filter(|p| p.position == pos && !o.give_players.contains(&p.id)).count() + inc.iter().filter(|l| l.player.position == pos).count();
            if have < need {
                warn.push(format!("You would be short at {} ({need} needed).", pos.label()));
            }
        }
        let cash_after = self.finances[me].cash + cash_delta;
        if cash_after < 0 {
            warn.push("This would put you in the red.".into());
        }
        if payroll_delta > 0 && self.finances[me].cash + cash_delta < (self.finances[me].projected_payroll(&self.teams[me]) + payroll_delta) * 2 {
            warn.push("Your cash would be thin for the new payroll.".into());
        }
        TradeBook { incoming: inc, outgoing: out, payroll_delta, commitment_in, commitment_out, cash_delta, cash_after, roster_after, warnings: warn }
    }

    // ------------------------------------------------------------ execution

    /// Makes the trade if the partner agrees. Returns the verdict (accepted = it happened).
    pub fn propose_trade(&mut self, o: &TradeOffer) -> TradeVerdict {
        let v = self.evaluate_trade(o);
        if !v.accepted {
            return v;
        }
        let me = self.user();
        let other = o.partner;
        let mut names: Vec<String> = Vec::new();
        let mut got: Vec<String> = Vec::new();
        for &id in &o.give_players {
            let p = self.player_on(me, id).unwrap().clone();
            names.push(format!("{} {}", p.position.label(), p.name));
            self.move_player(&p, me, other);
        }
        for &id in &o.get_players {
            let p = self.player_on(other, id).unwrap().clone();
            got.push(format!("{} {}", p.position.label(), p.name));
            self.move_player(&p, other, me);
        }
        for p in &o.give_picks {
            names.push(format!("{} round {} pick", p.season, p.round));
            self.set_pick_owner(p, other);
        }
        for p in &o.get_picks {
            got.push(format!("{} round {} pick ({})", p.season, p.round, self.teams[p.original_team].town.name));
            self.set_pick_owner(p, me);
        }
        let partner_town = self.teams[other].town.name.clone();
        if o.give_cash > 0 {
            self.move_cash(me, other, o.give_cash, &format!("trade with {partner_town}"));
        }
        if o.get_cash > 0 {
            self.move_cash(other, me, o.get_cash, &format!("trade with {partner_town}"));
        }
        if o.give_cash > 0 {
            names.push(format!("${} cash", o.give_cash));
        }
        if o.get_cash > 0 {
            got.push(format!("${} cash", o.get_cash));
        }
        self.sync_draft_owners();
        if self.auto_set_roster {
            self.auto_set_active();
        }
        let text = format!(
            "You sent {} to {} and got {}. {}",
            if names.is_empty() { "nothing".to_string() } else { names.join(", ") },
            self.teams[other].town.team_name(),
            if got.is_empty() { "nothing".to_string() } else { got.join(", ") },
            v.line
        );
        self.post_inbox(InboxFeed::Money, InboxKind::Report, &format!("Trade with {partner_town}"), &text, PostOpts::default());
        v
    }

    fn move_player(&mut self, p: &Player, from: usize, to: usize) {
        let me = self.user_team;
        let c = self.finances[from].contracts.remove(&p.id);
        let season = self.season;
        let from_team = &mut self.teams[from];
        from_team.roster.retain(|x| x.id != p.id);
        from_team.active_ids.remove(&p.id);
        if let Some(d) = from_team.depth.get_mut(&p.position) {
            d.retain(|id| *id != p.id);
        }
        let mut moved = p.clone();
        moved.acquired = acquired_trade(season);
        let town = self.teams[to].town.clone();
        let commute = PayScale::commute(town_miles(&p.hometown, &town.name));
        let to_team = &mut self.teams[to];
        to_team.roster.push(moved);
        self.finances[to].contracts.insert(
            p.id,
            Contract {
                player_id: p.id,
                ppg: c.as_ref().map(|c| c.ppg).unwrap_or_else(|| PayScale::ask(p, &town, 3)),
                seasons_left: 1.max(c.as_ref().map(|c| c.seasons_left).unwrap_or(1)),
                win_bonus: c.as_ref().map(|c| c.win_bonus).unwrap_or(0),
                commute,
            },
        );
        if Some(to) == me && self.teams[to].active_ids.len() < ACTIVE_ROSTER_SIZE {
            self.teams[to].active_ids.insert(p.id);
        }
    }

    fn set_pick_owner(&mut self, p: &PickRef, owner: usize) {
        if owner == p.original_team {
            self.pick_owners.remove(&p.key());
        } else {
            self.pick_owners.insert(p.key(), owner);
        }
    }

    fn move_cash(&mut self, from: usize, to: usize, amount: i64, memo: &str) {
        self.finances[from].cash -= amount;
        self.finances[to].cash += amount;
        let (season, week) = (self.season, self.week);
        if Some(from) == self.user_team {
            self.finances[from].ledger.push(LedgerEntry::new(season, week, LedgerKind::Other, -amount, memo));
        }
        if Some(to) == self.user_team {
            self.finances[to].ledger.push(LedgerEntry::new(season, week, LedgerKind::Other, amount, memo));
        }
    }
}

fn vet_mood(p: &Player, mood: &str) -> f64 {
    if p.age < 29 {
        return 1.0;
    }
    match mood {
        "Rebuilding" => TradeTuning::REBUILD_VET,
        "Contending" => TradeTuning::CONTEND_VET,
        _ => 1.0,
    }
}

fn pick_mood(mood: &str) -> f64 {
    match mood {
        "Rebuilding" => TradeTuning::REBUILD_PICK,
        "Contending" => TradeTuning::CONTEND_PICK,
        _ => 1.0,
    }
}

fn yes_line(other: usize) -> String {
    ["Deal. Our owner already has the pen out.", "You drive a fair bargain. Shake on it.", "We can live with that. Done."][other % 3].to_string()
}

fn close_line(other: usize) -> String {
    ["We're close. Something small would get this done.", "Not far off. Sweeten it a little.", "Almost. Our owner wants a little more for the trouble."][other % 3].to_string()
}

fn no_line(other: usize, ratio: f64) -> String {
    if ratio < 0.4 {
        [
            "Is this a joke? We were laughing so loud the team bus came to check.",
            "Not even close. Our owner wants to know if you are feeling alright.",
            "We have heard better offers from a guy selling flags at an intersection.",
        ][other % 3]
            .to_string()
    } else {
        ["That does not do it for us. Come back with more.", "No. We need to see more value.", "We would be giving up too much. Try again."][other % 3].to_string()
    }
}
