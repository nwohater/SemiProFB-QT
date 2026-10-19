use crate::ai_business::{ai_adjust_ticket_price, ai_in_austerity, ai_payroll_budget};
use crate::ai_strategy::AiStrategy;
use crate::hall_of_fame::HallEntry;
use crate::league::{League, LeaguePhase, RetiredEntry, RetiredPlayer, SeasonRow, SeasonSummary};
use crate::schedule::ScheduleGenerator;
use crate::standings::Standings;
use crate::trade::TradeTuning;
use sp_core::generator::GenOptions;
use sp_core::player::{ACQUIRED_FREE_AGENCY, Player};
use sp_core::progression::{PlayerChange, advance, retire_chance};
use sp_core::region::{mascot_table, pick_unused_nickname};
use sp_core::team::{ACTIVE_ROSTER_SIZE, ROSTER_MIX, min_active_coverage, pick_active_list};
use sp_core::{Position, Rng, Tier};
use sp_economy::finance::EconTuning;
use sp_economy::{EconomyEngine, LedgerEntry, LedgerKind, Negotiation, OfferOutcome, OfferResponse, PayScale};
use std::collections::{BTreeMap, HashMap, HashSet};

/// Starting slots per position on the field (sim formations), used to tell a starter from a backup.
fn starter_slots(p: Position) -> usize {
    use Position::*;
    match p {
        Qb | Rb | Fb | Te | C | Mlb | K | P => 1,
        Wr => 3,
        Ot | Og | De | Dt | Olb | Cb | S => 2,
    }
}

/// An established player (ovr 50+) who was undressed or behind the starters at his spot all year.
fn sat_on_bench(t: &sp_core::Team, p: &Player) -> bool {
    if p.ovr() < EconTuning::BENCHED_MIN_OVR {
        return false;
    }
    if !t.active_ids.contains(&p.id) {
        return true;
    }
    let rank = t.depth_order(p.position).iter().position(|x| x.id == p.id).unwrap_or(usize::MAX);
    rank >= starter_slots(p.position)
}

/// What the player sees at the start of the offseason (progression report, retirements, contract decisions).
#[derive(Clone, Debug)]
pub struct OffseasonSummary {
    pub season: i32,
    /// Team index.
    pub champion: usize,
    /// Before -> after only (D12).
    pub user_changes: Vec<PlayerChange>,
    pub user_retired: Vec<Player>,
    /// User's player ids needing a decision.
    pub expiring: Vec<u32>,
    pub free_agent_count: usize,
}

#[derive(Clone, Debug)]
pub struct OffseasonResult {
    pub new_season: i32,
    /// (folded team, replacement team)
    pub replaced: Vec<(String, String)>,
    /// Fewer than 46 available after cuts.
    pub user_roster_short: bool,
    /// Walk-ons found at the diner when your roster could not field a team.
    pub auto_signed: Vec<Player>,
}

/// Share of a folded team's roster (best first) that carries over to its replacement club.
pub const FOLDED_KEEP_SHARE: f64 = 0.5;

impl League {
    // ------------------------------------------------------------ begin

    /// Season recap -> progression -> retirements -> contract rollover. Call once after the title game.
    pub fn begin_offseason(&mut self) -> OffseasonSummary {
        self.tryout_held_this_week = false;
        assert!(self.complete(), "Season is not complete");
        for f in &mut self.finances {
            f.promos.clear(); // last season's promo plans are over; pick again next season
        }
        let mut r = self.rng.fork(self.season as u64 * 7919);
        // Awards come first: they read this season's stats and who is still a rookie, before the books roll over.
        // Normally locked in when the regular season ended; this covers a save from before that.
        if self.awards_history.last().map(|a| a.season) != Some(self.season) {
            let race = self.award_race(true);
            self.awards_history.push(race);
        }
        self.sync_award_counts();
        let awards = self.awards_history.last().unwrap().clone();

        // Archive the season.
        let abbr_of = |lg: &League, t: Option<usize>| t.map(|t| lg.teams[t].town.abbr());
        let nick_of = |lg: &League, t: Option<usize>| t.map(|t| lg.teams[t].town.nickname.clone());
        let mut team_of: HashMap<u32, usize> = HashMap::new();
        for (i, t) in self.teams.iter().enumerate() {
            for p in &t.roster {
                team_of.insert(p.id, i);
            }
        }
        let season = self.season;
        let stats: Vec<(u32, std::collections::BTreeMap<String, f64>)> = self.season_stats.iter().map(|(id, s)| (*id, s.to_map())).collect();
        for (id, map) in stats {
            let t = team_of.get(&id).copied();
            let row = SeasonRow { season, team: t.map(|x| x as i32).unwrap_or(-1), stats: map, abbr: abbr_of(self, t), nickname: nick_of(self, t) };
            self.season_rows.entry(id).or_default().push(row);
        }
        // Rostered players who never got in still get a line (zero games), so a bench year shows on his bio.
        for ti in 0..self.teams.len() {
            let ids: Vec<u32> = self.teams[ti].roster.iter().map(|p| p.id).collect();
            for id in ids {
                if self.season_stats.contains_key(&id) {
                    continue;
                }
                let row = SeasonRow { season, team: ti as i32, stats: BTreeMap::from([("gp".to_string(), 0.0)]), abbr: abbr_of(self, Some(ti)), nickname: nick_of(self, Some(ti)) };
                self.season_rows.entry(id).or_default().push(row);
            }
        }
        let pstats: Vec<(u32, std::collections::BTreeMap<String, f64>)> = self.playoff_stats.iter().map(|(id, s)| (*id, s.to_map())).collect();
        for (id, map) in pstats {
            let t = team_of.get(&id).copied();
            let row = SeasonRow { season, team: t.map(|x| x as i32).unwrap_or(-1), stats: map, abbr: abbr_of(self, t), nickname: nick_of(self, t) };
            self.playoff_rows.entry(id).or_default().push(row);
        }
        let champ = self.playoffs.as_ref().unwrap().champion.expect("a champion");
        self.log_owner_season();
        let records: Vec<String> = (0..self.teams.len()).map(|i| format!("{} {}", self.teams[i].town.name, self.standings.records[i])).collect();
        self.history.push(SeasonSummary { season, champion: champ, champion_name: self.teams[champ].town.team_name(), records, playoff_seeds: self.playoffs.as_ref().unwrap().seeds.clone() });

        // The Town Council judges the season before the books roll over.
        self.file_council_report();

        // Fans: title run and playoff bumps.
        let seeds = self.playoffs.as_ref().unwrap().seeds.clone();
        for i in 0..self.teams.len() {
            if i == champ {
                self.finances[i].fan_interest = (self.finances[i].fan_interest + 10.0).clamp(0.0, 100.0);
            } else if seeds.contains(&i) {
                self.finances[i].fan_interest = (self.finances[i].fan_interest + 4.0).clamp(0.0, 100.0);
            }
        }

        let mut user_changes: Vec<PlayerChange> = Vec::new();
        let mut user_retired: Vec<Player> = Vec::new();
        self.last_retired_entries.clear();
        let mut inductees: Vec<HallEntry> = Vec::new();

        for ti in 0..self.teams.len() {
            let roster = std::mem::take(&mut self.teams[ti].roster);
            let mut keep: Vec<Player> = Vec::new();
            for p in roster {
                let season_ending = self.injured_games.get(&p.id).copied().unwrap_or(0) >= 14;
                if r.chance(retire_chance(&p, season_ending)) {
                    self.finances[ti].contracts.remove(&p.id);
                    self.teams[ti].active_ids.remove(&p.id);
                    let (abbr, nick) = (self.teams[ti].town.abbr(), self.teams[ti].town.nickname.clone());
                    self.retired_players.insert(p.id, RetiredPlayer { name: p.name.clone(), pos: p.position.label().to_string(), abbr: abbr.clone() });
                    let hof = self.record_retirement(&p, &abbr, Some(&nick));
                    if let Some(h) = &hof {
                        inductees.push(h.clone());
                    }
                    self.last_retired_entries.push(RetiredEntry { player: p.clone(), team: Some(ti), abbr, hall_of_fame: hof.is_some() });
                    if Some(ti) == self.user_team {
                        user_retired.push(p);
                    }
                    continue;
                }
                let snaps = self.season_stats.get(&p.id).map(|s| s.get("snaps_off") + s.get("snaps_def")).unwrap_or(0.0);
                let change = advance(&p, &mut r, snaps.round() as i32, self.finances[ti].training_mult());
                keep.push(change.after.clone());
                if Some(ti) == self.user_team {
                    user_changes.push(change);
                }
            }
            self.teams[ti].roster = keep;
        }

        // Free agents age too; the weakest old ones retire.
        let last_abbr = |lg: &League, id: u32| lg.season_rows.get(&id).and_then(|rows| rows.last()).and_then(|r| r.abbr.clone()).unwrap_or_else(|| "–".to_string());
        let fas = std::mem::take(&mut self.free_agents);
        let mut fa_keep: Vec<Player> = Vec::new();
        for p in fas {
            if r.chance(retire_chance(&p, false) + if p.ovr() < 40 { 0.05 } else { 0.0 }) {
                let abbr = last_abbr(self, p.id);
                self.retired_players.insert(p.id, RetiredPlayer { name: p.name.clone(), pos: p.position.label().to_string(), abbr: abbr.clone() });
                let nick = self.season_rows.get(&p.id).and_then(|rows| rows.iter().rev().find(|r| r.team >= 0)).and_then(|r| r.nickname.clone());
                let hof = self.record_retirement(&p, &abbr, nick.as_deref());
                if let Some(h) = &hof {
                    inductees.push(h.clone());
                }
                self.last_retired_entries.push(RetiredEntry { player: p.clone(), team: None, abbr, hall_of_fame: hof.is_some() });
                continue;
            }
            fa_keep.push(advance(&p, &mut r, 0, 1.0).after);
        }
        self.free_agents = fa_keep;

        let mut r61 = r.fork(61);
        self.roll_fan_favorites(&mut r61);

        // Injuries mostly heal over the offseason.
        let ids: Vec<u32> = self.injured_games.keys().copied().collect();
        for id in ids {
            let left = self.injured_games[&id] - 14;
            if left <= 0 {
                self.injured_games.remove(&id);
                self.out_reason.remove(&id);
                for a in &mut self.absences {
                    if a.player_id == id {
                        a.open = false; // healed over the winter
                    }
                }
            } else {
                self.injured_games.insert(id, left);
            }
        }

        // Contracts roll down a year. AI decides on expiring deals; the user is asked.
        self.pending_expired.clear();
        self.wants_to_play.clear();
        for ti in 0..self.teams.len() {
            let contract_ids: Vec<u32> = self.finances[ti].contracts.keys().copied().collect();
            for cid in contract_ids {
                let left = {
                    let c = self.finances[ti].contracts.get_mut(&cid).unwrap();
                    c.seasons_left -= 1;
                    c.seasons_left
                };
                if left > 0 {
                    continue;
                }
                let Some(p) = self.teams[ti].roster.iter().find(|p| p.id == cid).cloned() else {
                    self.finances[ti].contracts.remove(&cid);
                    continue;
                };
                if Some(ti) == self.user_team {
                    self.pending_expired.insert(p.id);
                    if sat_on_bench(&self.teams[ti], &p) {
                        self.wants_to_play.insert(p.id);
                    }
                    continue;
                }
                let town = self.teams[ti].town.clone();
                let budget_ok = self.finances[ti].cash > -20_000;
                if budget_ok && self.is_hall_bound(&p) {
                    // A legend stays until he retires: a fresh deal at his asking price, no age discount.
                    let seasons = r.weighted(&[(1, 50.0), (2, 50.0)]);
                    let ppg = (PayScale::ask(&p, &town, seasons) as f64 * r.gaussian(1.0, 0.04)).round() as i64;
                    self.finances[ti].contracts.insert(p.id, Negotiation::contract_for(&p, &town, ppg, seasons));
                    continue;
                }
                let p_re = (if p.ovr() >= 50 { 0.9 } else { 0.55 }) * (if budget_ok { 1.0 } else { 0.5 }) * (if p.age >= 33 { 0.45 } else if p.age >= 31 { 0.75 } else { 1.0 });
                if r.chance(p_re) {
                    let seasons = r.weighted(&[(1, 30.0), (2, 35.0), (3, 35.0)]);
                    let ppg = (PayScale::ask(&p, &town, seasons) as f64 * r.gaussian(1.0, 0.04)).round() as i64;
                    self.finances[ti].contracts.insert(p.id, Negotiation::contract_for(&p, &town, ppg, seasons));
                } else {
                    self.to_free_agency(ti, &p);
                }
            }
        }

        self.last_changes = user_changes.iter().map(|c| (c.before.id, (c.before.ratings.to_array(), c.after.ratings.to_array()))).collect();
        self.last_retired = user_retired.iter().map(|p| format!("{} ({}, {})", p.name, p.position.label(), p.age)).collect();

        self.generate_entrants(&mut r, 90);
        // Cap the pool so it doesn't grow forever: keep the best 220.
        // A fan-favorite free agent stays even past the cut. The inbox tells the owner he is available,
        // and a young local hero is often not among the 220 highest keep-scores.
        if self.free_agents.len() > 220 {
            // Keep the most useful, not just the highest rated: youth has upside, old players are on the way out.
            let keep = |p: &Player| p.ovr() as f64 + if p.age < 25 { (25 - p.age) as f64 * 2.0 } else { 0.0 } - if p.age > 30 { (p.age - 30) as f64 * 2.0 } else { 0.0 };
            self.free_agents.sort_by(|a, b| keep(b).total_cmp(&keep(a)));
            let spared: Vec<Player> = self.free_agents[220..].iter().filter(|p| self.fan_favorite_ids.contains(&p.id)).cloned().collect();
            self.free_agents.truncate(220);
            self.free_agents.extend(spared);
        }

        self.phase = LeaguePhase::Offseason;
        self.create_draft();
        self.write_draft_coming();
        self.write_inbox_awards(&awards);
        self.write_inbox_hall_of_fame(&mut inductees);
        self.write_inbox_offseason(false);
        OffseasonSummary { season, champion: champ, user_changes, user_retired, expiring: self.pending_expired.iter().copied().collect(), free_agent_count: self.free_agents.len() }
    }

    // ------------------------------------------------------ user actions

    fn find_player_in(&self, id: u32) -> Option<Player> {
        self.free_agents.iter().find(|p| p.id == id).cloned().or_else(|| self.me().roster.iter().find(|p| p.id == id).cloned())
    }

    /// Ask a free agent or one of your expiring players: returns their response (no side effects).
    pub fn negotiate(&mut self, player_id: u32, ppg: i64, seasons: i32, signing_bonus: i64, round: i32) -> OfferResponse {
        self.sync_awards_for(player_id);
        let p = self.find_player_in(player_id).unwrap_or_else(|| panic!("unknown player {player_id}"));
        let win = self.my_win_pct();
        let resign = self.resign_premium(player_id);
        Negotiation::respond(&p, &self.me().town, ppg, seasons, signing_bonus, round, win, resign.as_ref())
    }

    /// Finalize a deal (re-sign or free-agent signing, any time of year). Fails unless the player would accept (round 2 tolerance).
    pub fn sign_player(&mut self, player_id: u32, ppg: i64, seasons: i32, signing_bonus: i64) -> bool {
        assert!(!self.game_over(), "Game over");
        let resp = self.negotiate(player_id, ppg, seasons, signing_bonus, Negotiation::MAX_ROUNDS);
        if resp.outcome != OfferOutcome::Accept {
            return false;
        }
        let u = self.user();
        let fa = self.free_agents.iter().position(|p| p.id == player_id);
        let player: Player;
        if let Some(i) = fa {
            player = self.free_agents.remove(i);
            self.teams[u].roster.push(player.clone());
            // A new signing dresses right away if there is room on the 46-man list.
            if self.teams[u].active_ids.len() < ACTIVE_ROSTER_SIZE {
                self.teams[u].active_ids.insert(player.id);
            }
        } else if self.pending_expired.contains(&player_id) {
            player = self.teams[u].roster.iter().find(|p| p.id == player_id).cloned().expect("expiring player is on the roster");
            self.pending_expired.remove(&player_id);
            self.wants_to_play.remove(&player_id);
        } else {
            return false;
        }
        let contract = Negotiation::contract_for(&player, &self.teams[u].town, ppg, seasons);
        self.finances[u].contracts.insert(player_id, contract);
        if fa.is_some() {
            let clock = self.trade_clock();
            self.trade_lock.insert(player_id, clock + TradeTuning::LOCK_AFTER_SIGNING); // no sign-and-flip
        }
        if signing_bonus > 0 {
            let season = self.season;
            let f = &mut self.finances[u];
            f.cash -= signing_bonus;
            f.ledger.push(LedgerEntry::new(season, 0, LedgerKind::Other, -signing_bonus, &format!("signing bonus {}", player.name)));
        }
        true
    }

    /// Release a player, paying a % of what's left on the deal (D25).
    pub fn release_player(&mut self, player_id: u32) -> i64 {
        let u = self.user();
        let p = self.teams[u].roster.iter().find(|x| x.id == player_id).cloned().expect("player on the roster");
        let c = self.finances[u].contracts.get(&player_id);
        let cost = if self.pending_expired.contains(&player_id) || c.is_none() { 0 } else { c.unwrap().release_cost() };
        let season = self.season;
        self.finances[u].cash -= cost;
        if cost > 0 {
            self.finances[u].ledger.push(LedgerEntry::new(season, 0, LedgerKind::Other, -cost, &format!("release {}", p.name)));
        }
        self.pending_expired.remove(&player_id);
        self.to_free_agency(u, &p);
        cost
    }

    // ----------------------------------------------------------- finish

    /// AI roster moves, bankrupt-team replacement, new schedule, fresh season. Unsigned expiring user players walk.
    pub fn finish_offseason(&mut self) -> OffseasonResult {
        self.tryout_held_this_week = false;
        assert!(self.phase == LeaguePhase::Offseason, "Not in the offseason");
        let mut r = self.rng.fork(self.season as u64 * 104729);

        // Any picks still on the board are made for you (best available for your team, or passed if you are broke).
        if self.draft.as_ref().map(|d| !d.complete()).unwrap_or(false) {
            self.sim_draft_all();
        }

        // Expiring players you did not re-sign leave.
        if let Some(u) = self.user_team {
            for id in self.pending_expired.iter().copied().collect::<Vec<_>>() {
                if let Some(p) = self.teams[u].roster.iter().find(|x| x.id == id).cloned() {
                    self.to_free_agency(u, &p);
                }
            }
        }
        self.pending_expired.clear();

        // Revenue per game from the finished season (before books roll) for AI budgets.
        let rev_per_game: Vec<f64> = (0..self.finances.len()).map(|i| self.rev_per_game(i)).collect();

        // Bankrupt AI teams fold and are replaced by a town from the same division (D3, D30).
        let mut replaced: Vec<(String, String)> = Vec::new();
        for i in self.bankrupt_ai_teams() {
            replaced.push(self.replace_team(i, &mut r));
            self.rival_cash_stage.remove(&i);
        }

        // AI teams refill their rosters within budget.
        for i in 0..self.teams.len() {
            if Some(i) == self.user_team {
                continue;
            }
            if replaced.iter().any(|e| e.1 == self.teams[i].town.team_name()) {
                continue; // fresh team already full
            }
            let (rev, cash) = (rev_per_game[i], self.finances[i].cash);
            self.ai_fill(i, ai_payroll_budget(rev, cash), &mut r, ai_in_austerity(rev, cash));
        }

        // New season.
        let sponsors_before = if self.user_team.is_some() { self.snapshot_sponsors() } else { Vec::new() };
        let user_win_pct = self.user_team.map(|u| self.standings.records[u].pct()).unwrap_or(0.0);
        for i in 0..self.finances.len() {
            if Some(i) != self.user_team && !self.finances[i].bankrupt {
                let mut ri = r.fork(7000 + i as u64);
                ai_adjust_ticket_price(&mut self.finances[i], &mut ri);
            }
        }
        self.season += 1;
        for i in 0..self.finances.len() {
            let pct = self.standings.records[i].pct();
            self.finances[i].roll_season(&mut r, pct, Some(i) != self.user_team);
        }
        for i in 0..self.teams.len() {
            self.town_last_used.insert(self.teams[i].town.id.clone(), self.season);
            if Some(i) != self.user_team {
                let mut ri = r.fork(i as u64 + self.season as u64);
                self.teams[i].strategy = AiStrategy::for_team(&self.teams[i], &mut ri);
            }
        }
        self.schedule = ScheduleGenerator::generate(&mut r.fork(3));
        self.standings = Standings::new(self.teams.len(), ScheduleGenerator::division_of, r.next_int(1 << 20) as u64);
        self.playoffs = None;
        self.results.clear();
        self.season_stats.clear();
        self.playoff_stats.clear();
        self.week = 1;
        self.phase = LeaguePhase::RegularSeason;

        // AI owners with spare cash fix up the stadium and book promo nights.
        for i in 0..self.teams.len() {
            if Some(i) == self.user_team || replaced.iter().any(|e| e.1 == self.teams[i].town.team_name()) {
                continue;
            }
            let mut ri = r.fork(5000 + i as u64);
            self.ai_spend_surplus(i, rev_per_game[i], &mut ri);
        }

        // Top up the user's active list with the best available if cuts/retirements left holes.
        let mut auto: Vec<Player> = Vec::new();
        if let Some(u) = self.user_team {
            let ids: HashSet<u32> = self.teams[u].roster.iter().map(|p| p.id).collect();
            self.teams[u].active_ids.retain(|id| ids.contains(id));
            if self.teams[u].active_ids.len() < ACTIVE_ROSTER_SIZE {
                let best = pick_active_list(&self.teams[u], &HashSet::new());
                let mut sorted: Vec<u32> = best.into_iter().collect();
                sorted.sort();
                for id in sorted {
                    if self.teams[u].active_ids.len() >= ACTIVE_ROSTER_SIZE {
                        break;
                    }
                    self.teams[u].active_ids.insert(id);
                }
            }
            auto = self.ensure_minimum(u);
            let ids: HashSet<u32> = self.teams[u].roster.iter().map(|p| p.id).collect();
            self.teams[u].active_ids.retain(|id| ids.contains(id));
            let best = pick_active_list(&self.teams[u], &HashSet::new());
            let mut sorted: Vec<u32> = best.into_iter().collect();
            sorted.sort();
            for id in sorted {
                if self.teams[u].active_ids.len() >= ACTIVE_ROSTER_SIZE {
                    break;
                }
                self.teams[u].active_ids.insert(id);
            }
        }
        self.write_sponsor_rollover(&sponsors_before, user_win_pct);
        let mut r6000 = r.fork(6000);
        self.assign_council_goals(&mut r6000, Some(user_win_pct));
        self.write_inbox_offseason(true);
        self.write_council_goals_letter();
        self.surprise_bill_attempted = false;
        let short = self.user_team.map(|u| self.teams[u].roster.len() < ACTIVE_ROSTER_SIZE).unwrap_or(false);
        OffseasonResult { new_season: self.season, replaced, user_roster_short: short, auto_signed: auto }
    }

    // ---------------------------------------------------------------- helpers

    pub(crate) fn to_free_agency(&mut self, team: usize, p: &Player) {
        self.teams[team].roster.retain(|x| x.id != p.id);
        self.teams[team].active_ids.remove(&p.id);
        self.finances[team].contracts.remove(&p.id);
        let mut q = p.clone();
        q.acquired = ACQUIRED_FREE_AGENCY.to_string(); // back on the market: his next team gets him through free agency
        self.free_agents.push(q);
    }

    /// Guarantees a team can field the minimum positional coverage by signing the cheapest free agents for any hole
    /// (one-year deals, so they cost more per game). Returns who was signed.
    fn ensure_minimum(&mut self, i: usize) -> Vec<Player> {
        let mut signed = Vec::new();
        for &pos in &Position::ALL {
            let want = min_active_coverage(pos);
            let mut have = self.teams[i].roster.iter().filter(|p| p.position == pos).count();
            while have < want {
                let town = self.teams[i].town.clone();
                let mut cands: Vec<usize> = (0..self.free_agents.len()).filter(|&k| self.free_agents[k].position == pos).collect();
                cands.sort_by_key(|&k| PayScale::ask(&self.free_agents[k], &town, 1));
                let p = match cands.first() {
                    Some(&k) => self.free_agents.remove(k),
                    // Nobody on the market at this spot: a walk-on turns up (the Dart left the hole).
                    None => self.tg.players.generate(pos, GenOptions::default().tier(Tier::WalkOn).hometown(&town.name).years_pro(0).acquired(ACQUIRED_FREE_AGENCY)),
                };
                self.teams[i].roster.push(p.clone());
                let ppg = PayScale::ask(&p, &town, 1);
                self.finances[i].contracts.insert(p.id, Negotiation::contract_for(&p, &town, ppg, 1));
                signed.push(p);
                have += 1;
            }
        }
        signed
    }

    fn generate_entrants(&mut self, r: &mut Rng, n: usize) {
        let towns: Vec<String> = self.region.all_towns().map(|t| t.name.clone()).collect();
        let total: usize = ROSTER_MIX.iter().map(|m| m.1).sum();
        let weights: Vec<(Position, f64)> = ROSTER_MIX.iter().map(|&(p, c)| (p, c as f64 / total as f64)).collect();
        for _ in 0..n {
            let pos = r.weighted(&weights);
            // The open market is mostly guys with day jobs; a real starter is rare (tryouts are where you gamble on finding one).
            let tier = r.weighted(&[(Tier::WalkOn, 70.0), (Tier::LocalAmateur, 29.0), (Tier::Regular, 1.0)]);
            let home = r.pick(&towns).clone();
            let mut opts = GenOptions::default().tier(tier).hometown(&home).years_pro(0).acquired(ACQUIRED_FREE_AGENCY); // no pro history: a rookie whatever his age
            if r.chance(0.8) {
                opts = opts.age(r.range(20, 27));
            }
            let p = self.tg.players.generate(pos, opts);
            self.free_agents.push(p);
        }
    }

    fn new_nickname(&mut self, r: &mut Rng) -> String {
        let in_use: std::collections::HashSet<String> = self.teams.iter().map(|t| t.town.nickname.clone()).collect();
        let used_ever: std::collections::HashSet<String> = self.used_nicknames.iter().cloned().collect();
        let nick = pick_unused_nickname(r, &used_ever, &in_use, Some(&self.region.id));
        self.used_nicknames.insert(nick.clone());
        nick
    }

    fn new_mascot(&self, r: &mut Rng) -> String {
        let used: HashSet<&String> = self.teams.iter().map(|t| &t.town.mascot).collect();
        let free: Vec<&String> = mascot_table().iter().filter(|n| !used.contains(n)).collect();
        if free.is_empty() { r.pick(mascot_table()).clone() } else { (*r.pick(&free)).clone() }
    }

    /// Fold team `i` and bring in a new one from the same division (reusing towns when the pool runs out).
    fn replace_team(&mut self, i: usize, r: &mut Rng) -> (String, String) {
        let old = self.teams[i].clone();
        let division = self.region.divisions[i / 4].clone();
        let current: HashSet<String> = self.teams.iter().map(|t| t.town.id.clone()).collect();
        let mut candidates: Vec<&sp_core::Town> = self.region.all_towns().filter(|t| t.division == division && !current.contains(&t.id)).collect();
        candidates.sort_by_key(|t| self.town_last_used.get(&t.id).copied().unwrap_or(-1));
        let best_use = self.town_last_used.get(&candidates[0].id).copied().unwrap_or(-1);
        let pool: Vec<&&sp_core::Town> = candidates.iter().filter(|t| self.town_last_used.get(&t.id).copied().unwrap_or(-1) == best_use).collect();
        let picked = (**r.pick(&pool)).clone();
        let nick = self.new_nickname(r);
        let mascot = if self.town_last_used.contains_key(&picked.id) { self.new_mascot(r) } else { picked.mascot.clone() };
        let mut town = picked;
        town.nickname = nick;
        town.mascot = mascot;
        self.town_last_used.insert(old.town.id.clone(), self.season);
        let nearby: Vec<String> = self.region.all_towns().filter(|t| t.division == division).map(|t| t.name.clone()).collect();
        let mut team = self.tg.generate(&town, &nearby);
        // The best of the old roster stay on with the new club (each takes the place of a weaker player at his
        // position); everyone else goes to free agency.
        let mut by_ovr: Vec<&Player> = old.roster.iter().collect();
        by_ovr.sort_by_key(|p| std::cmp::Reverse(p.ovr()));
        let keep_n = (by_ovr.len() as f64 * FOLDED_KEEP_SHARE).round() as usize;
        let mut kept: Vec<u32> = Vec::new();
        for p in by_ovr.into_iter().take(keep_n) {
            let mut weaker: Vec<usize> = (0..team.roster.len()).filter(|&k| team.roster[k].position == p.position && team.roster[k].ovr() < p.ovr()).collect();
            weaker.sort_by_key(|&k| team.roster[k].ovr());
            let Some(&k) = weaker.first() else { continue };
            team.roster.remove(k);
            team.roster.push(p.clone());
            kept.push(p.id);
        }
        for p in &old.roster {
            if !kept.contains(&p.id) {
                let mut q = p.clone();
                q.acquired = ACQUIRED_FREE_AGENCY.to_string();
                self.free_agents.push(q);
            }
        }
        team.active_ids = pick_active_list(&team, &HashSet::new());
        team.strategy = AiStrategy::for_team(&team, &mut r.fork(i as u64));
        let names = (old.town.team_name(), team.town.team_name());
        let mut fin = EconomyEngine::create(&team, &mut r.fork(i as u64 + 1000));
        fin.cash = (fin.cash as f64 * 0.6).round() as i64;
        fin.fan_interest = 35.0;
        self.teams[i] = team;
        self.finances[i] = fin;
        names
    }

    /// Refill an AI roster to the position mix within a per-game payroll `budget`.
    fn ai_fill(&mut self, i: usize, budget: f64, r: &mut Rng, austerity: bool) {
        let active = pick_active_list(&self.teams[i], &HashSet::new());
        self.teams[i].active_ids = active;
        let payroll = |lg: &League| lg.finances[i].projected_payroll(&lg.teams[i]) as f64;

        // Aging veterans are moved along to keep the league's age profile healthy.
        for p in self.teams[i].roster.clone() {
            if (p.age >= 33 || (p.age >= 31 && p.ovr() < 50)) && r.chance(0.7) && !(payroll(self) <= budget && self.is_hall_bound(&p)) {
                self.to_free_agency(i, &p);
            }
        }

        // Release surplus depth first (lowest value players beyond the mix + 2).
        for &(pos, want) in &ROSTER_MIX {
            let mut at: Vec<Player> = self.teams[i].roster.iter().filter(|p| p.position == pos).cloned().collect();
            at.sort_by_key(|p| std::cmp::Reverse(p.ovr()));
            let start = want + if austerity { 0 } else { 2 };
            for p in at.iter().skip(start) {
                if payroll(self) <= budget && self.is_hall_bound(p) {
                    continue; // a legend is never surplus
                }
                self.to_free_agency(i, p);
            }
        }

        for &(pos, want) in &ROSTER_MIX {
            let mut have = self.teams[i].roster.iter().filter(|p| p.position == pos).count();
            while have < want {
                let town = self.teams[i].town.clone();
                let pool: Vec<usize> = (0..self.free_agents.len()).filter(|&k| self.free_agents[k].position == pos).collect();
                if pool.is_empty() {
                    // Nobody on the market at this spot: a walk-on turns up (the Dart signed a random player from another
                    // position here, which left the hole and could leave a club without a kicker).
                    let walk_on = self.tg.players.generate(pos, GenOptions::default().tier(Tier::WalkOn).hometown(&town.name).years_pro(0).acquired(ACQUIRED_FREE_AGENCY));
                    self.free_agents.push(walk_on);
                    continue;
                }
                let mut best: Option<usize> = None;
                let mut best_score = -1e9;
                let pay_now = payroll(self);
                for &k in &pool {
                    let p = &self.free_agents[k];
                    let ask = PayScale::ask(p, &town, 3);
                    let over = if pay_now + PayScale::retainer(ask) as f64 > budget { 1.0 } else { 0.0 };
                    let score = p.ovr_at(pos) as f64 - ask as f64 / 20.0 - over * 25.0 - if p.age > 27 { (p.age - 27) as f64 * 1.5 } else { 0.0 } + if p.age < 25 { (25 - p.age) as f64 * 2.0 } else { 0.0 }; // youth has upside
                    if score > best_score {
                        best_score = score;
                        best = Some(k);
                    }
                }
                let Some(k) = best else { break };
                let p = self.free_agents.remove(k);
                let seasons = r.weighted(&[(1, 30.0), (2, 35.0), (3, 35.0)]);
                let ppg = (PayScale::ask(&p, &town, seasons) as f64 * r.gaussian(1.0, 0.04)).round() as i64;
                let contract = Negotiation::contract_for(&p, &town, ppg, seasons);
                self.teams[i].roster.push(p.clone());
                self.finances[i].contracts.insert(p.id, contract);
                have += 1;
            }
        }
        let active = pick_active_list(&self.teams[i], &HashSet::new());
        self.teams[i].active_ids = active;
    }
}
