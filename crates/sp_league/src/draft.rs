use crate::ai_business::{ai_in_austerity, ai_payroll_budget};
use crate::inbox::{InboxFeed, InboxKind, PostOpts};
use crate::league::League;
use crate::text::inbox_text;
use serde::{Deserialize, Serialize};
use sp_core::generator::GenOptions;
use sp_core::geo::town_miles;
use sp_core::player::{ACQUIRED_FREE_AGENCY, acquired_draft};
use sp_core::team::{ACTIVE_ROSTER_SIZE, ROSTER_MIX, min_active_coverage};
use sp_core::{Player, Position, Rng, Team, Tier};
use sp_economy::{Negotiation, PayScale, rookie_scale::RookieScale};
use std::collections::BTreeMap;

use crate::text::fill;

/// Tuning for the Premium Player Draft and how AI teams pick. All values are tuning seeds.
pub struct DraftTuning;

impl DraftTuning {
    /// Share of the draft class whose hometowns are spread evenly across the league's towns.
    pub const LOCAL_SHARE: f64 = 0.6;
    /// Prospects generated per team and round, plus spares, so even the last pick has a choice.
    pub const SPARES: usize = 30;
    /// How many of the best undrafted players stay around as free agents (the rest return to their day jobs).
    pub const UNDRAFTED_TO_FREE_AGENCY: usize = 4;
    /// Quality mix of the class: college grads with real experience, far better than the walk-ons that fill the free-agent pool.
    /// Kept modest on purpose: the draft feeds good players into every roster each year, so the league settles a few
    /// points higher than it started instead of inflating forever.
    pub const TIERS: [(Tier, f64); 4] = [(Tier::ExCollege, 25.0), (Tier::Regular, 45.0), (Tier::LocalAmateur, 20.0), (Tier::WalkOn, 10.0)];
    pub const STAR_TIERS: [(Tier, f64); 2] = [(Tier::ExCollege, 50.0), (Tier::ExPro, 50.0)];
    pub const MIN_AGE: i32 = 22;
    pub const MAX_AGE: i32 = 26;
    /// Class balance: every position gets at least this share weight (roster-mix units), so a C or K is never absent.
    pub const MIN_POSITION_WEIGHT: f64 = 2.5;
    /// Each prospect aims at a common ovr curve; raw position templates differ, so we keep the candidate nearest the target.
    pub const TARGET_MEAN: f64 = 50.0;
    pub const TARGET_SD: f64 = 6.0;
    pub const CANDIDATES: usize = 5;

    pub fn position_offset(p: Position) -> f64 {
        match p {
            Position::K => -3.0,
            Position::P => -5.0,
            Position::Fb => -3.0,
            _ => 0.0,
        }
    }

    /// Chance of a blue-chip prospect at any position, and the bonus he adds to his target.
    pub const STAR_CHANCE: f64 = 0.05;
    pub const STAR_BONUS: (i32, i32) = (8, 15);

    /// How much a position is worth in the draft (kickers and punters last, QBs a little extra).
    pub fn position_value(p: Position) -> f64 {
        match p {
            Position::Qb => 1.10,
            Position::K | Position::P => 0.60,
            Position::Fb => 0.72,
            _ => 1.0,
        }
    }

    /// Commute cost counts as ovr points: (commute per game / rookie pay) x this.
    pub const COMMUTE_POINTS: f64 = 14.0;
    /// A team with a lot of open spots leans on need; one that is fine takes the best player.
    pub const NEED_WEIGHT_HOLES: f64 = 1.0;
    pub const NEED_WEIGHT_STEADY: f64 = 0.6;
    pub const HOLES_DEFICIT: i32 = 4;
    /// Cash-strapped teams care about commute more.
    pub const COMMUTE_WEIGHT_TIGHT: f64 = 1.6;
    pub const COMMUTE_WEIGHT_COMFY: f64 = 0.7;
}

/// One selection (or forfeited pick) in the draft.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftPickRecord {
    /// 1-based.
    pub overall: usize,
    pub round: usize,
    pub pick_in_round: usize,
    pub team: usize,
    pub forfeited: bool,
    /// Why a pick was passed.
    pub reason: Option<String>,
    pub player_id: Option<u32>,
    pub name: String,
    pub pos: String,
    pub ovr: i32,
    pub age: i32,
    pub ppg: i64,
    pub commute: i64,
    pub seasons: i32,
}

/// The draft board: who is still available, the pick order and everything picked so far.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Draft {
    pub season: i32,
    /// Team indexes in pick order for one round (worst finish first, champion last). Every round uses the same order.
    pub order: Vec<usize>,
    /// Still available.
    pub prospects: Vec<Player>,
    pub picks: Vec<DraftPickRecord>,
    /// Who owns each pick, by overall slot (index 0 = pick #1). Starts as the original order; trades change it.
    pub owners: Vec<usize>,
}

impl Draft {
    pub fn new(season: i32, order: Vec<usize>, prospects: Vec<Player>) -> Self {
        let owners = (0..RookieScale::ROUNDS as usize * order.len()).map(|i| order[i % order.len()]).collect();
        Self { season, order, prospects, picks: Vec::new(), owners }
    }

    pub fn teams(&self) -> usize {
        self.order.len()
    }

    pub fn total_picks(&self) -> usize {
        RookieScale::ROUNDS as usize * self.teams()
    }

    pub fn made(&self) -> usize {
        self.picks.len()
    }

    pub fn complete(&self) -> bool {
        self.made() >= self.total_picks()
    }

    pub fn overall(&self) -> usize {
        self.made() + 1
    }

    pub fn round(&self) -> usize {
        if self.complete() { RookieScale::ROUNDS as usize } else { self.made() / self.teams() + 1 }
    }

    pub fn pick_in_round(&self) -> usize {
        if self.complete() { self.teams() } else { self.made() % self.teams() + 1 }
    }

    /// The team on the clock, or `None` when the draft is over.
    pub fn on_clock(&self) -> Option<usize> {
        if self.complete() { None } else { Some(self.owners[self.made()]) }
    }

    pub fn slot_ppg(&self) -> i64 {
        RookieScale::ppg(self.round() as i32, self.pick_in_round() as i32, self.teams() as i32)
    }

    pub fn slot_seasons(&self) -> i32 {
        RookieScale::seasons_for_round(self.round() as i32)
    }

    /// Overall pick numbers still ahead for `team` (including the one on the clock).
    pub fn picks_ahead(&self, team: usize) -> Vec<usize> {
        (self.made()..self.total_picks()).filter(|&i| self.owners[i] == team).map(|i| i + 1).collect()
    }
}

/// What one team wants from the draft, built from its roster holes and its money.
#[derive(Clone, Debug)]
pub struct DraftStrategy {
    /// Extra pull for each position (0 = nothing needed).
    pub need: BTreeMap<Position, f64>,
    pub need_weight: f64,
    pub commute_weight: f64,
    pub label: String,
    pub focus: Vec<Position>,
}

#[derive(Clone, Copy, Debug)]
pub struct DraftTerms {
    pub ppg: i64,
    pub commute: i64,
    pub seasons: i32,
    pub round: usize,
    pub overall: usize,
}

fn starters_avg(x: &Team, pos: Position) -> f64 {
    let k = 1.max(min_active_coverage(pos));
    let mut list: Vec<i32> = x.roster.iter().filter(|p| p.position == pos).map(|p| p.ovr()).collect();
    list.sort_by(|a, b| b.cmp(a));
    if list.is_empty() {
        return 30.0;
    }
    let top: Vec<i32> = list.into_iter().take(k).collect();
    top.iter().sum::<i32>() as f64 / top.len() as f64
}

impl League {
    // ------------------------------------------------------------------ setup

    /// Builds the draft class and pick order. Called when the offseason begins, after the final standings are set.
    pub fn create_draft(&mut self) {
        let mut r = self.rng.fork(self.season as u64 * 6151 + 29);
        // Worst finish picks first: teams that missed the playoffs by record, then semifinal losers, the runner-up, and the
        // champion last. Within a group, worse record first; ties go to the worse point differential, then a coin flip.
        let po = self.playoffs.as_ref();
        let champ = po.and_then(|p| p.champion);
        let finalists: Vec<usize> = po.and_then(|p| p.final_game.as_ref()).map(|g| vec![g.home(), g.away()]).unwrap_or_default();
        let finish = |t: usize| -> i32 {
            if Some(t) == champ {
                3
            } else if finalists.contains(&t) {
                2
            } else if po.map(|p| p.seeds.contains(&t)).unwrap_or(false) {
                1
            } else {
                0
            }
        };
        let n = self.teams.len();
        let coin: Vec<f64> = (0..n).map(|_| r.next_f64()).collect();
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| {
            finish(a).cmp(&finish(b)).then_with(|| {
                let (ra, rb) = (&self.standings.records[a], &self.standings.records[b]);
                ra.pct().total_cmp(&rb.pct()).then_with(|| ra.diff().cmp(&rb.diff())).then_with(|| coin[a].total_cmp(&coin[b]))
            })
        });
        let count = RookieScale::ROUNDS as usize * n + DraftTuning::SPARES;
        let towns: Vec<String> = self.region.all_towns().map(|t| t.name.clone()).collect();
        // Positions are dealt in fixed shares (largest remainder), not rolled, so every spot is always represented.
        let share: Vec<(Position, f64)> = ROSTER_MIX.iter().map(|&(p, c)| (p, (c as f64).max(DraftTuning::MIN_POSITION_WEIGHT))).collect();
        let share_total: f64 = share.iter().map(|s| s.1).sum();
        let exact: Vec<(Position, f64)> = share.iter().map(|&(p, w)| (p, count as f64 * w / share_total)).collect();
        let mut slots: Vec<Position> = Vec::new();
        for &(p, e) in &exact {
            slots.extend(std::iter::repeat_n(p, e.floor() as usize));
        }
        let mut by_remainder: Vec<(Position, f64)> = exact.clone();
        by_remainder.sort_by(|a, b| (b.1 % 1.0).total_cmp(&(a.1 % 1.0)));
        let mut i = 0;
        while slots.len() < count {
            slots.push(by_remainder[i % by_remainder.len()].0);
            i += 1;
        }
        r.shuffle(&mut slots);
        // A share of the class is dealt round-robin across the league's own towns, so every team has local boys to pick.
        let local_count = (count as f64 * DraftTuning::LOCAL_SHARE).round() as usize;
        let mut prospects: Vec<Player> = Vec::new();
        for (i, &pos) in slots.iter().enumerate().take(count) {
            // Every position aims at the same ovr curve (with the odd blue-chip), so no spot owns the top of the board.
            let mut target = DraftTuning::TARGET_MEAN + r.next_gaussian() * DraftTuning::TARGET_SD + DraftTuning::position_offset(pos);
            let star = r.chance(DraftTuning::STAR_CHANCE);
            if star {
                target += r.range(DraftTuning::STAR_BONUS.0, DraftTuning::STAR_BONUS.1) as f64;
            }
            let hometown = if i < local_count { self.teams[i % n].town.name.clone() } else { r.pick(&towns).clone() };
            let age = r.range(DraftTuning::MIN_AGE, DraftTuning::MAX_AGE);
            let mut best: Option<Player> = None;
            for _ in 0..(if star { DraftTuning::CANDIDATES * 3 } else { DraftTuning::CANDIDATES }) {
                let tier = if star { r.weighted(&DraftTuning::STAR_TIERS) } else { r.weighted(&DraftTuning::TIERS) };
                let c = self.tg.players.generate(
                    pos,
                    GenOptions::default().tier(tier).hometown(&hometown).age(age).years_pro(0).acquired(ACQUIRED_FREE_AGENCY), // becomes the draft label if he is picked; otherwise he is on the open market
                );
                if best.as_ref().map(|b| (c.ovr() as f64 - target).abs() < (b.ovr() as f64 - target).abs()).unwrap_or(true) {
                    best = Some(c);
                }
            }
            prospects.push(best.expect("at least one candidate"));
        }
        prospects.sort_by_key(|p| std::cmp::Reverse(p.ovr()));
        self.draft = Some(Draft::new(self.season, order, prospects));
        self.sync_draft_owners();
    }

    /// Applies traded picks to the draft board (the board starts with every team holding its own picks).
    pub fn sync_draft_owners(&mut self) {
        let Some(d) = self.draft.as_mut() else { return };
        for i in 0..d.total_picks() {
            let original = d.order[i % d.teams()];
            let key = crate::trade::PickRef { season: d.season, round: (i / d.teams() + 1) as i32, original_team: original }.key();
            d.owners[i] = self.pick_owners.get(&key).copied().unwrap_or(original);
        }
    }

    // --------------------------------------------------------------- the user

    pub fn user_on_clock(&self) -> bool {
        match (&self.draft, self.user_team) {
            (Some(d), Some(u)) => !d.complete() && d.on_clock() == Some(u),
            _ => false,
        }
    }

    /// Money check for any team: a club that is broke has to pass on its pick.
    pub fn cant_draft_reason(&self, team: usize) -> Option<String> {
        let fin = &self.finances[team];
        if fin.bankrupt {
            return Some("is bankrupt".into());
        }
        if fin.cash < 0 {
            return Some("is broke (cash is below zero)".into());
        }
        if Some(team) != self.user_team {
            let rev = self.rev_per_game(team);
            let t = &self.teams[team];
            let d = self.draft.as_ref().unwrap();
            if ai_in_austerity(rev, fin.cash) && (fin.projected_payroll(t) + d.slot_ppg()) as f64 > ai_payroll_budget(rev, fin.cash) {
                return Some("cannot fit another contract in the budget".into());
            }
        }
        None
    }

    /// What the pick on the clock would cost this team for `p`: fixed slot pay (and years) plus the player's commute.
    pub fn draft_terms(&self, team: usize, p: &Player) -> DraftTerms {
        let d = self.draft.as_ref().unwrap();
        DraftTerms { ppg: d.slot_ppg(), commute: PayScale::commute(town_miles(&p.hometown, &self.teams[team].town.name)), seasons: d.slot_seasons(), round: d.round(), overall: d.overall() }
    }

    /// You draft `player_id` with the pick on the clock. Errors unless you are on the clock and can pay.
    pub fn draft_player(&mut self, player_id: u32) -> Result<DraftPickRecord, String> {
        if !self.user_on_clock() {
            return Err("Not your pick".into());
        }
        let u = self.user();
        if let Some(why) = self.cant_draft_reason(u) {
            return Err(format!("Your club {why}"));
        }
        let p = self.draft.as_ref().unwrap().prospects.iter().find(|x| x.id == player_id).cloned().ok_or_else(|| format!("not available: {player_id}"))?;
        Ok(self.make_pick(u, p))
    }

    /// You pass on the pick on the clock (it is gone for good).
    pub fn pass_pick(&mut self) -> Result<DraftPickRecord, String> {
        if !self.user_on_clock() {
            return Err("Not your pick".into());
        }
        let u = self.user();
        let why = self.cant_draft_reason(u).unwrap_or_else(|| "passed on the pick".into());
        Ok(self.forfeit(u, &why))
    }

    // ------------------------------------------------------------ running it

    /// AI teams pick until it is your turn (or the draft ends). Returns the picks made.
    pub fn sim_draft_to_user(&mut self) -> Vec<DraftPickRecord> {
        let mut out = Vec::new();
        if self.draft.is_none() {
            return out;
        }
        loop {
            let d = self.draft.as_ref().unwrap();
            if d.complete() || d.on_clock() == self.user_team {
                break;
            }
            let team = d.on_clock().unwrap();
            out.push(self.ai_pick(team));
        }
        if self.draft.as_ref().unwrap().complete() {
            self.finish_draft();
        }
        out
    }

    /// Finishes the whole draft. Your picks use your suggested board (or are passed when you are broke).
    pub fn sim_draft_all(&mut self) -> Vec<DraftPickRecord> {
        let mut out = Vec::new();
        if self.draft.is_none() {
            return out;
        }
        while !self.draft.as_ref().unwrap().complete() {
            let team = self.draft.as_ref().unwrap().on_clock().unwrap();
            out.push(self.ai_pick(team));
        }
        self.finish_draft();
        out
    }

    // ----------------------------------------------------------- AI + board

    /// A team's draft plan: which positions it needs most and how much it minds commute costs.
    pub fn draft_strategy_for(&self, team: usize) -> DraftStrategy {
        let t = &self.teams[team];
        let mut need: BTreeMap<Position, f64> = BTreeMap::new();
        let mut holes = 0;
        for &(pos, want) in &ROSTER_MIX {
            let have = t.roster.iter().filter(|p| p.position == pos).count();
            let deficit = (want as i32 - have as i32).max(0);
            let single = matches!(pos, Position::K | Position::P);
            let lg = self.teams.iter().map(|x| starters_avg(x, pos)).sum::<f64>() / self.teams.len() as f64;
            let weak = ((lg - starters_avg(t, pos)) / 12.0).clamp(0.0, 1.5);
            let n = if single { if have == 0 { 1.2 } else { weak * 0.4 } } else { deficit as f64 * 0.55 + weak * 0.6 };
            if deficit > 0 && !single {
                holes += deficit;
            }
            if n > 0.0 {
                need.insert(pos, n.clamp(0.0, 1.8));
            }
        }
        let fin = &self.finances[team];
        let tight = fin.cash < 0 || ai_in_austerity(self.rev_per_game(team), fin.cash);
        let mut entries: Vec<(Position, f64)> = need.iter().filter(|(_, v)| **v >= 0.35).map(|(k, v)| (*k, *v)).collect();
        entries.sort_by(|a, b| b.1.total_cmp(&a.1));
        let focus: Vec<Position> = entries.into_iter().take(3).map(|e| e.0).collect();
        let holey = holes >= DraftTuning::HOLES_DEFICIT;
        let label = if tight { "Watching the budget" } else if holey { "Filling roster holes" } else { "Best player available" };
        DraftStrategy {
            need,
            need_weight: if holey { DraftTuning::NEED_WEIGHT_HOLES } else { DraftTuning::NEED_WEIGHT_STEADY },
            commute_weight: if tight { DraftTuning::COMMUTE_WEIGHT_TIGHT } else { DraftTuning::COMMUTE_WEIGHT_COMFY },
            label: label.to_string(),
            focus,
        }
    }

    /// How much `team` likes `p` right now (higher is better): talent first, then need, youth and commute cost.
    pub fn draft_score(&self, team: usize, p: &Player, s: &DraftStrategy) -> f64 {
        let d = self.draft.as_ref().unwrap();
        let pos = p.position;
        let value = DraftTuning::position_value(pos);
        let youth = (26 - p.age).clamp(-2, 4) as f64 * 0.7;
        let need_mul = 1.0 + s.need.get(&pos).copied().unwrap_or(0.0).clamp(0.0, 1.8) * s.need_weight;
        let commute = PayScale::commute(town_miles(&p.hometown, &self.teams[team].town.name));
        let commute_cost = commute as f64 / 40.0f64.max(d.slot_ppg() as f64) * DraftTuning::COMMUTE_POINTS * s.commute_weight;
        (p.ovr() as f64 + youth) * value * need_mul - commute_cost
    }

    /// The available prospects in the order `team` would take them.
    pub fn draft_board(&self, team: usize) -> Vec<Player> {
        let s = self.draft_strategy_for(team);
        let mut list: Vec<(f64, Player)> = self.draft.as_ref().unwrap().prospects.iter().map(|p| (self.draft_score(team, p, &s), p.clone())).collect();
        list.sort_by(|a, b| b.0.total_cmp(&a.0));
        list.into_iter().map(|x| x.1).collect()
    }

    fn ai_pick(&mut self, team: usize) -> DraftPickRecord {
        if let Some(why) = self.cant_draft_reason(team) {
            return self.forfeit(team, &why);
        }
        let overall = self.draft.as_ref().unwrap().overall();
        let s = self.draft_strategy_for(team);
        let mut r = self.rng.fork(self.season as u64 * 977 + overall as u64);
        let mut best: Option<Player> = None;
        let mut best_score = -1e9;
        for p in &self.draft.as_ref().unwrap().prospects {
            let sc = self.draft_score(team, p, &s) + r.gaussian(0.0, 1.2);
            if sc > best_score {
                best_score = sc;
                best = Some(p.clone());
            }
        }
        if Some(team) == self.user_team {
            // Auto-picking for you: the plain best of your board, no noise.
            best = self.draft_board(team).into_iter().next();
        }
        self.make_pick(team, best.expect("a prospect is available"))
    }

    fn make_pick(&mut self, team: usize, mut p: Player) -> DraftPickRecord {
        let terms = self.draft_terms(team, &p);
        let season = self.season;
        let d = self.draft.as_mut().unwrap();
        let overall = d.overall();
        d.prospects.retain(|x| x.id != p.id);
        p.acquired = acquired_draft(season + 1, overall as i32); // held in the offseason, so it is next season's draft
        let (round, pick_in_round) = (d.round(), d.pick_in_round());
        let t = &mut self.teams[team];
        let contract = Negotiation::contract_for(&p, &t.town, terms.ppg, terms.seasons);
        t.roster.push(p.clone());
        self.finances[team].contracts.insert(p.id, contract);
        if Some(team) == self.user_team && t.active_ids.len() < ACTIVE_ROSTER_SIZE {
            t.active_ids.insert(p.id);
        }
        let rec = DraftPickRecord {
            overall,
            round,
            pick_in_round,
            team,
            forfeited: false,
            reason: None,
            player_id: Some(p.id),
            name: p.name.clone(),
            pos: p.position.label().to_string(),
            ovr: p.ovr(),
            age: p.age,
            ppg: terms.ppg,
            commute: terms.commute,
            seasons: terms.seasons,
        };
        let d = self.draft.as_mut().unwrap();
        d.picks.push(rec.clone());
        if d.complete() {
            self.finish_draft();
        }
        rec
    }

    fn forfeit(&mut self, team: usize, reason: &str) -> DraftPickRecord {
        let d = self.draft.as_mut().unwrap();
        let rec = DraftPickRecord {
            overall: d.overall(),
            round: d.round(),
            pick_in_round: d.pick_in_round(),
            team,
            forfeited: true,
            reason: Some(reason.to_string()),
            player_id: None,
            name: String::new(),
            pos: String::new(),
            ovr: 0,
            age: 0,
            ppg: 0,
            commute: 0,
            seasons: 0,
        };
        d.picks.push(rec.clone());
        if d.complete() {
            self.finish_draft();
        }
        rec
    }

    /// Everyone left unsigned goes to free agency, and the owner hears how the draft went.
    fn finish_draft(&mut self) {
        let d = self.draft.as_mut().unwrap();
        let prefix = format!("{}:", d.season);
        self.pick_owners.retain(|k, _| !k.starts_with(&prefix)); // this draft is done: its picks are spent
        if !d.prospects.is_empty() {
            // Most undrafted players go back to their day jobs; only the best few are left to sign as free agents.
            let mut left = std::mem::take(&mut d.prospects);
            left.sort_by_key(|p| std::cmp::Reverse(p.ovr()));
            self.free_agents.extend(left.into_iter().take(DraftTuning::UNDRAFTED_TO_FREE_AGENCY));
        }
        let Some(u) = self.user_team else { return };
        let d = self.draft.as_ref().unwrap();
        let mine: Vec<&DraftPickRecord> = d.picks.iter().filter(|p| p.team == u).collect();
        let took: Vec<&&DraftPickRecord> = mine.iter().filter(|p| !p.forfeited).collect();
        let mut voice = Rng::new(self.season as u64 * 4093 + 11);
        let t = inbox_text();
        let mut text = format!("{} ", t.pick("draftOver", &mut voice));
        if took.is_empty() {
            text.push_str(t.pick("draftNone", &mut voice));
        } else {
            let list: Vec<String> = took.iter().map(|p| format!("{} ({}, round {})", p.name, p.pos, p.round)).collect();
            text.push_str(&format!("Your class: {}.", list.join(", ")));
        }
        let passed = mine.iter().filter(|p| p.forfeited).count();
        if passed > 0 {
            text.push_str(&format!(" You passed on {passed} pick{}.", if passed == 1 { "" } else { "s" }));
        }
        if self.inbox.iter().any(|i| i.season == self.season && i.title == "Draft results") {
            return;
        }
        self.post_inbox(InboxFeed::Money, InboxKind::Report, "Draft results", &text, PostOpts::week(0));
    }

    pub(crate) fn rev_per_game(&self, i: usize) -> f64 {
        let f = &self.finances[i];
        if f.games.is_empty() {
            (f.town.population as f64).clamp(1000.0, 12000.0)
        } else {
            f.games.iter().map(|g| g.total_revenue()).sum::<i64>() as f64 / f.games.len() as f64
        }
    }

    /// The "Premium Player Draft is coming" inbox note.
    pub(crate) fn write_draft_coming(&mut self) {
        let Some(u) = self.user_team else { return };
        let d = self.draft.as_ref().unwrap();
        let mine = d.picks_ahead(u);
        let first = mine.first().copied().unwrap_or(1);
        let mut r = Rng::new(self.season as u64 * 3121 + 5);
        let text = fill(
            inbox_text().pick("draftComing", &mut r),
            &[("n", d.prospects.len().to_string()), ("rounds", RookieScale::ROUNDS.to_string()), ("pick", first.to_string())],
        );
        self.post_inbox(InboxFeed::Money, InboxKind::Report, "Premium Player Draft", &text, PostOpts::week(0));
    }
}
