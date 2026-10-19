use crate::absence_log::{AbsenceEvent, injury_label};
use crate::ai_strategy::AiStrategy;
use crate::awards::SeasonAwards;
use crate::conduct::ConductGossip;
use crate::fan_favorites::FanNote;
use crate::gazette::{GameDigest, GazetteIssue};
use crate::hall_of_fame::HallEntry;
use crate::inbox::{InboxItem, InboxKind};
use crate::owner_career::{ClubHelp, OwnerSeason};
use crate::playoffs::{PlayoffGame, PlayoffResult, Playoffs};
use crate::schedule::{ScheduleGenerator, ScheduledGame};
use crate::standings::{GameRecord, Standings, TeamRecord};
use crate::text::{fill, inbox_text};
use crate::weather::WeatherModel;
use sp_core::generator::GenOptions;
use sp_core::jobs::{JobTable, tuning as job_tuning};
use sp_core::player::{ACQUIRED_TRY_OUT, Player};
use sp_core::region::{deal_nicknames, is_rivalry};
use sp_core::team::{ROSTER_MIX, pick_active_list};
use sp_core::team_generator::TeamGenerator;
use sp_core::{PlayerGenerator, Position, Region, Rng, Team, Tier, Weather};
use sp_economy::business::{promo_by_id, SponsorMarket};
use sp_economy::{EconomyEngine, Facility, GameInput, LedgerEntry, LedgerKind, ResignPremium, TeamFinance, TownEconomy};
use sp_economy::finance::AttOpts;
use sp_sim::{BoxScoreBuilder, GameConfig, GameResult, GameSim, PlayerStats, Side};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LeaguePhase {
    RegularSeason,
    Semifinals,
    Championship,
    Complete,
    Offseason,
}

/// Result of advancing one step (a regular-season week or a playoff round).
#[derive(Clone, Debug)]
pub struct WeekResult {
    pub week: i32,
    pub games: Vec<GameRecord>,
    pub phase_after: LeaguePhase,
    /// What happened to your players at their day jobs this week (hurt at work, stuck on a work trip).
    pub work_news: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeasonSummary {
    pub season: i32,
    /// Team index.
    pub champion: usize,
    pub champion_name: String,
    /// "Town W-L-T" by team index.
    pub records: Vec<String>,
    pub playoff_seeds: Vec<usize>,
}

/// A player who retired this offseason, kept whole so his bio can still be opened. `team` is `None` for a free agent.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RetiredEntry {
    pub player: Player,
    pub team: Option<usize>,
    pub abbr: String,
    pub hall_of_fame: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RetiredPlayer {
    pub name: String,
    pub pos: String,
    pub abbr: String,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SeasonRow {
    pub season: i32,
    /// -1 = free agent.
    pub team: i32,
    pub stats: BTreeMap<String, f64>,
    /// Team abbreviation at the time, so a franchise that later folds keeps its label (the slot gets a new team).
    pub abbr: Option<String>,
    /// His team's nickname at the time, for the same reason.
    pub nickname: Option<String>,
}

/// Options for a new league.
pub struct NewLeague {
    /// `None` = a spectator league with no owner.
    pub user_team: Option<usize>,
    pub keep_box_scores: bool,
    pub jobs: Option<JobTable>,
    pub names: Option<sp_core::names::NameBank>,
    pub sizes: Option<sp_core::sizes::SizeTable>,
    /// Owner picks (town id -> nickname), claimed first.
    pub nicknames: HashMap<String, String>,
    pub town_ids: Option<Vec<String>>,
    pub region: Option<&'static Region>,
    /// Use the small built-in name/size/job tables (fast; for tests).
    pub small_tables: bool,
}

impl Default for NewLeague {
    fn default() -> Self {
        Self { user_team: Some(0), keep_box_scores: false, jobs: None, names: None, sizes: None, nicknames: HashMap::new(), town_ids: None, region: None, small_tables: false }
    }
}

/// Two distinct elements of a slice, mutably.
pub fn two_mut<T>(v: &mut [T], a: usize, b: usize) -> (&mut T, &mut T) {
    assert_ne!(a, b);
    if a < b {
        let (l, r) = v.split_at_mut(b);
        (&mut l[a], &mut r[0])
    } else {
        let (l, r) = v.split_at_mut(a);
        (&mut r[0], &mut l[b])
    }
}

/// Stable name hash (the Dart used `String.hashCode`, which is not portable).
fn name_hash(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h & 0x7fff_ffff
}

/// 12-team league (D26-D29): 3 divisions of 4, 14 games, 4-team playoffs. Deterministic by seed.
pub struct League {
    pub rng: Rng,
    pub teams: Vec<Team>,
    pub tg: TeamGenerator,
    pub schedule: Vec<ScheduledGame>,
    /// `None` = no owner (a spectator league).
    pub user_team: Option<usize>,
    /// The seed this league was created with (shown in Settings so a franchise can be shared).
    pub seed: u64,
    pub keep_box_scores: bool,
    pub standings: Standings,
    pub playoffs: Option<Playoffs>,
    pub phase: LeaguePhase,
    /// Next regular-season week to play.
    pub week: i32,
    pub season: i32,
    pub results: Vec<GameRecord>,
    /// Unsigned players (released, expired, folded teams, new tryout entrants).
    pub free_agents: Vec<Player>,
    /// The Premium Player Draft for this offseason (`None` outside the offseason and before it is created).
    pub draft: Option<crate::draft::Draft>,
    /// Traded draft picks: "season:round:originalTeam" -> current owner (a pick not listed belongs to its original team).
    pub pick_owners: BTreeMap<String, usize>,
    /// Players who cannot be traded yet (just signed): player id -> trade clock when the lock ends.
    pub trade_lock: BTreeMap<u32, i32>,
    /// One entry per completed season.
    pub history: Vec<SeasonSummary>,
    /// Per-player season rows (career stats = sum of rows).
    pub season_rows: BTreeMap<u32, Vec<SeasonRow>>,
    /// Who retired (id -> name, position, last team abbreviation), so career leaders can still name them.
    pub retired_players: BTreeMap<u32, RetiredPlayer>,
    /// Retired players worth remembering, ranked by career score; the best are inducted.
    pub hall_of_fame: Vec<HallEntry>,
    /// The user's season-by-season log and gifts to other clubs.
    pub owner_seasons: Vec<OwnerSeason>,
    pub club_help: Vec<ClubHelp>,
    /// Same as `season_rows`, for playoff games only.
    pub playoff_rows: BTreeMap<u32, Vec<SeasonRow>>,
    /// Season a town was last in the league (for replacement picks, D30).
    pub town_last_used: BTreeMap<String, i32>,
    /// Nicknames already used in this franchise (current and folded).
    pub used_nicknames: BTreeSet<String>,
    /// Your last offseason's rating changes (before -> after only, D12).
    pub last_changes: BTreeMap<u32, ([i32; 8], [i32; 8])>,
    pub last_retired: Vec<String>,
    /// Everyone who retired in the latest offseason, for the retirement screen.
    pub last_retired_entries: Vec<RetiredEntry>,
    /// User's players whose contracts expired and need a decision (re-sign or let go).
    pub pending_expired: BTreeSet<u32>,
    /// Expiring players of yours who rode the bench this season and want to play.
    pub wants_to_play: BTreeSet<u32>,
    /// Open tryouts can run once per week; reset whenever the league advances.
    pub tryout_held_this_week: bool,
    /// Surprise bill: one roll per regular season, once cash is over the line.
    pub surprise_bill_attempted: bool,
    /// League-wide fan favorites (player ids). Roughly 12-20 at any time.
    pub fan_favorite_ids: BTreeSet<u32>,
    /// What changed about fan favorites in the last offseason.
    pub fan_news: Vec<FanNote>,
    /// Winners and finalists of each finished season.
    pub awards_history: Vec<SeasonAwards>,
    /// The owner's inbox (oldest first).
    pub inbox: Vec<InboxItem>,
    pub inbox_seq: u32,
    pub region: &'static Region,
    /// Off-field trouble waiting for the next Gazette's Gossip column.
    pub conduct_gossip: Vec<ConductGossip>,
    /// Last cash-trouble rumor stage told per AI team (1 low, 2 very low, 3 negative/bankrupt).
    pub rival_cash_stage: BTreeMap<usize, i32>,
    /// Sponsor news from the last advance (new mid-season offers).
    pub sponsor_news: Vec<String>,
    /// Money for each team (same order as `teams`).
    pub finances: Vec<TeamFinance>,
    /// Games remaining out for each injured player (player id -> games).
    pub injured_games: BTreeMap<u32, i32>,
    /// Why a player is out when it was his day job or conduct (player id -> short reason).
    pub out_reason: BTreeMap<u32, String>,
    /// Every injury and work absence, oldest first, kept for good.
    pub absences: Vec<AbsenceEvent>,
    /// The weekly Gazette: finished issues, this week's game digests, and the last money-trouble stage reported per club.
    pub gazette: Vec<GazetteIssue>,
    pub week_digests: Vec<GameDigest>,
    pub gazette_cash_stage: BTreeMap<usize, i32>,
    /// When on, your game-day list is re-picked after every game, like the AI teams do.
    pub auto_set_roster: bool,
    /// When on, your depth chart is kept automatic: custom orders are cleared before every game.
    pub auto_depth: bool,
    /// Day jobs and their risks. Set when the league is created or loaded.
    pub jobs: JobTable,
    pub(crate) work_news: Vec<String>,
    /// Season stat totals per player id (ids are unique league-wide).
    pub season_stats: BTreeMap<u32, PlayerStats>,
    /// Playoff-only totals for the current season.
    pub playoff_stats: BTreeMap<u32, PlayerStats>,
}

impl League {
    /// New league from a region (California unless the options or town ids say otherwise).
    pub fn new(seed: u64, opts: NewLeague) -> League {
        let reg = opts.region.unwrap_or_else(|| match &opts.town_ids {
            None => Region::california(),
            Some(ids) => Region::of_towns(ids),
        });
        let mut rng = Rng::new(seed);
        // single generator => unique player ids across the league
        let pg = if opts.small_tables {
            PlayerGenerator::with_tables(rng.fork(1), sp_core::names::NameBank::built_in(), sp_core::sizes::SizeTable::built_in(), opts.jobs.clone().unwrap_or_else(JobTable::built_in))
        } else {
            let mut pg = PlayerGenerator::new(rng.fork(1));
            if let Some(j) = &opts.jobs {
                pg.jobs = j.clone();
            }
            if let Some(n) = &opts.names {
                pg.names = n.clone();
            }
            if let Some(z) = &opts.sizes {
                pg.sizes = z.clone();
            }
            pg
        };
        let mut tg = TeamGenerator::new(pg);
        let ids: Vec<String> = opts.town_ids.clone().unwrap_or_else(|| reg.default_league_towns());
        let mut towns: Vec<sp_core::Town> = ids.iter().map(|id| reg.town(id).expect("known town").clone()).collect();
        assert!(
            towns.len() == 12 && (0..12).all(|i| towns[i].division == reg.divisions[i / 4]),
            "the league needs 4 towns per division, in division order"
        );
        // Unique deal from the nickname table; owner picks (town id -> nickname) claimed first.
        towns = deal_nicknames(&towns, &mut Rng::new(seed ^ 0x4E1C_4A1E), &opts.nicknames, Some(&reg.id));
        // Dart fix: `nearbyTowns` was never passed, so every starting player was a local. Draw the other 45% from the division.
        let mut teams: Vec<Team> = towns
            .iter()
            .map(|t| {
                let nearby: Vec<String> = towns.iter().filter(|o| o.division == t.division && o.name != t.name).map(|o| o.name.clone()).collect();
                tg.generate(t, &nearby)
            })
            .collect();
        for t in &mut teams {
            let h = name_hash(&t.town.name);
            t.strategy = AiStrategy::for_team(t, &mut rng.fork(h));
        }
        let schedule = ScheduleGenerator::generate(&mut rng.fork(2));
        let tiebreak = rng.next_int(1 << 20) as u64;
        let standings = Standings::new(teams.len(), ScheduleGenerator::division_of, tiebreak);
        let jobs = tg.players.jobs.clone();
        let mut town_last_used = BTreeMap::new();
        let mut used_nicknames = BTreeSet::new();
        for t in &teams {
            town_last_used.insert(t.town.id.clone(), 1);
            used_nicknames.insert(t.town.nickname.clone());
        }
        let finances: Vec<TeamFinance> = teams.iter().map(|t| EconomyEngine::create(t, &mut rng.fork(t.town.name.len() as u64 * 31 + t.town.population as u64))).collect();
        let mut lg = League {
            rng,
            teams,
            tg,
            schedule,
            user_team: opts.user_team,
            seed,
            keep_box_scores: opts.keep_box_scores,
            standings,
            playoffs: None,
            phase: LeaguePhase::RegularSeason,
            week: 1,
            season: 1,
            results: Vec::new(),
            free_agents: Vec::new(),
            draft: None,
            pick_owners: BTreeMap::new(),
            trade_lock: BTreeMap::new(),
            history: Vec::new(),
            season_rows: BTreeMap::new(),
            retired_players: BTreeMap::new(),
            hall_of_fame: Vec::new(),
            owner_seasons: Vec::new(),
            club_help: Vec::new(),
            playoff_rows: BTreeMap::new(),
            town_last_used,
            used_nicknames,
            last_changes: BTreeMap::new(),
            last_retired: Vec::new(),
            last_retired_entries: Vec::new(),
            pending_expired: BTreeSet::new(),
            wants_to_play: BTreeSet::new(),
            tryout_held_this_week: false,
            surprise_bill_attempted: false,
            fan_favorite_ids: BTreeSet::new(),
            fan_news: Vec::new(),
            awards_history: Vec::new(),
            inbox: Vec::new(),
            inbox_seq: 0,
            region: reg,
            conduct_gossip: Vec::new(),
            rival_cash_stage: BTreeMap::new(),
            sponsor_news: Vec::new(),
            finances,
            injured_games: BTreeMap::new(),
            out_reason: BTreeMap::new(),
            absences: Vec::new(),
            gazette: Vec::new(),
            week_digests: Vec::new(),
            gazette_cash_stage: BTreeMap::new(),
            auto_set_roster: false,
            auto_depth: false,
            jobs,
            work_news: Vec::new(),
            season_stats: BTreeMap::new(),
            playoff_stats: BTreeMap::new(),
        };
        let mut r404 = lg.rng.fork(404);
        lg.seed_fan_favorites(&mut r404);
        if let Some(u) = lg.user_team {
            // Sponsor offers waiting for the owner on day one.
            let town = lg.teams[u].town.clone();
            let offers = SponsorMarket::generate(&town, &mut lg.rng.fork(77), 3, 0, &[], &[]);
            lg.finances[u].offers.extend(offers);
            let mut r78 = lg.rng.fork(78);
            lg.assign_council_goals(&mut r78, None);
            lg.write_inbox_welcome();
            lg.write_council_goals_letter();
        }
        lg
    }

    pub fn division_of(&self, team: usize) -> usize {
        ScheduleGenerator::division_of(team)
    }

    pub fn complete(&self) -> bool {
        self.phase == LeaguePhase::Complete
    }

    /// The owner's team index; panics in a spectator league (owner actions only make sense with an owner).
    pub fn user(&self) -> usize {
        self.user_team.expect("this action needs an owner")
    }

    pub fn me(&self) -> &Team {
        &self.teams[self.user()]
    }

    pub fn my_finance(&self) -> &TeamFinance {
        &self.finances[self.user()]
    }

    pub fn my_finance_mut(&mut self) -> &mut TeamFinance {
        let u = self.user();
        &mut self.finances[u]
    }

    /// Your regular-season record across every season so far (finished seasons from `history`, plus the one in progress).
    pub fn user_career_record(&self) -> TeamRecord {
        let u = self.user();
        let mut r = TeamRecord::default();
        let re = regex::Regex::new(r"(\d+)-(\d+)-(\d+)$").unwrap();
        for h in &self.history {
            if let Some(m) = re.captures(&h.records[u]) {
                r.wins += m[1].parse::<i32>().unwrap();
                r.losses += m[2].parse::<i32>().unwrap();
                r.ties += m[3].parse::<i32>().unwrap();
            }
        }
        if self.history.last().map(|h| h.season) != Some(self.season) {
            let c = &self.standings.records[u];
            r.wins += c.wins;
            r.losses += c.losses;
            r.ties += c.ties;
        }
        r
    }

    /// Your franchise went bankrupt (D3): game over.
    pub fn game_over(&self) -> bool {
        self.user_team.map(|u| self.finances[u].bankrupt).unwrap_or(false)
    }

    /// AI teams that went bankrupt this season (replaced in the offseason).
    pub fn bankrupt_ai_teams(&self) -> Vec<usize> {
        (0..self.teams.len()).filter(|&i| Some(i) != self.user_team && self.finances[i].bankrupt).collect()
    }

    pub fn champion(&self) -> Option<usize> {
        self.playoffs.as_ref().and_then(|p| p.champion)
    }

    pub fn games_in_week(&self, w: i32) -> Vec<ScheduledGame> {
        self.schedule.iter().filter(|g| g.week == w).copied().collect()
    }

    pub fn team_schedule(&self, team: usize) -> Vec<ScheduledGame> {
        let mut v: Vec<ScheduledGame> = self.schedule.iter().filter(|g| g.home == team || g.away == team).copied().collect();
        v.sort_by_key(|g| g.week);
        v
    }

    // --------------------------------------------------------- absence log

    /// One player's history, oldest first.
    pub fn absences_for(&self, player_id: u32) -> Vec<&AbsenceEvent> {
        self.absences.iter().filter(|a| a.player_id == player_id).collect()
    }

    /// Public door to the absence log for other parts of the league (off-field suspensions).
    pub fn record_absence(&mut self, player_id: u32, week: i32, work: bool, what: &str, games: i32) {
        self.absences.push(AbsenceEvent::new(player_id, self.season, week, work, what, games));
    }

    /// He sat one more game; closes his latest open event when the time is served.
    fn serve_absence(&mut self, player_id: u32, done: bool) {
        if let Some(a) = self.absences.iter_mut().rev().find(|a| a.player_id == player_id && a.open) {
            a.missed += 1;
            if done {
                a.open = false;
            }
        }
    }

    /// Re-picks your active list: the best 46 who are healthy, keeping positional minimums.
    pub fn auto_set_active(&mut self) {
        let Some(u) = self.user_team else { return };
        let unavailable: HashSet<u32> = self.injured_games.keys().copied().collect();
        let active = pick_active_list(&self.teams[u], &unavailable);
        self.teams[u].active_ids = active;
    }

    /// What an expiring player of yours adds to his ask, or `None` if `player_id` isn't up for re-signing.
    pub fn resign_premium(&mut self, player_id: u32) -> Option<ResignPremium> {
        let u = self.user();
        if self.teams[u].roster.iter().any(|p| p.id == player_id) {
            self.sync_awards_for(player_id); // quote the ask from the awards on record
        }
        if self.pending_expired.contains(&player_id) {
            Some(ResignPremium { cash: self.finances[u].cash, wants_to_play: self.wants_to_play.contains(&player_id) })
        } else {
            None
        }
    }

    // --------------------------------------------------------------- advance

    /// Plays the next unit: a regular-season week, a playoff round, then completes.
    pub fn advance(&mut self) -> WeekResult {
        self.work_news.clear();
        self.week_digests.clear();
        self.tryout_held_this_week = false;
        self.sponsor_news.clear();
        assert!(!self.game_over(), "Game over: your team went bankrupt");
        self.resolve_open_conduct(); // a letter you ignored counts as "let him sit" before the next game
        let injured_before = self.injured_games.clone();
        let reason_before = self.out_reason.clone();
        let r = self.advance_step();
        self.write_inbox(&r, &injured_before, &reason_before);
        if r.games.first().map(|g| !g.playoff).unwrap_or(false) {
            self.maybe_roll_surprise_bill(r.week);
        }
        if r.phase_after != LeaguePhase::Complete {
            self.roll_conduct(r.week);
        }
        self.write_gazette(r.week, &injured_before);
        self.week_digests.clear();
        r
    }

    fn advance_step(&mut self) -> WeekResult {
        match self.phase {
            LeaguePhase::RegularSeason => self.play_regular_week(),
            LeaguePhase::Semifinals => self.play_playoff_round(false, LeaguePhase::Championship, 15),
            LeaguePhase::Championship => self.play_playoff_round(true, LeaguePhase::Complete, 16),
            LeaguePhase::Complete => panic!("Season complete: call begin_offseason()"),
            LeaguePhase::Offseason => panic!("In offseason: finish it with finish_offseason()"),
        }
    }

    /// Plays everything until the season is complete (used by tools/tests).
    pub fn sim_season(&mut self) {
        while !self.complete() && !self.game_over() && self.phase != LeaguePhase::Offseason {
            self.advance();
        }
    }

    fn play_regular_week(&mut self) -> WeekResult {
        let mut played = Vec::new();
        for g in self.games_in_week(self.week) {
            played.push(self.play_game(g.week, g.home, g.away, false));
        }
        let done = self.week;
        self.week += 1;
        self.mid_season_offers(done);
        if self.week > ScheduleGenerator::WEEKS as i32 {
            self.playoffs = Some(Playoffs::from_standings(&self.standings, 3));
            self.phase = LeaguePhase::Semifinals;
            // The regular season decides the awards (playoff stats are kept apart): lock the winners in now.
            if self.awards_history.last().map(|a| a.season) != Some(self.season) {
                let race = self.award_race(true);
                self.awards_history.push(race);
                self.sync_award_counts();
            }
        }
        WeekResult { week: done, games: played, phase_after: self.phase, work_news: self.work_news.clone() }
    }

    /// Sponsors come knocking during the season too: stale offers lapse, and a new one sometimes shows up
    /// (more often after a win). It stays on the table for two weeks.
    fn mid_season_offers(&mut self, played_week: i32) {
        let Some(u) = self.user_team else { return };
        let week = self.week;
        let lapsed: Vec<String> = self.finances[u].offers.iter().filter(|o| o.expires_week > 0 && o.expires_week < week).map(|o| o.name.clone()).collect();
        self.finances[u].offers.retain(|o| !(o.expires_week > 0 && o.expires_week < week));
        self.write_offer_lapsed(&lapsed);
        let mine = self.games_in_week(played_week).into_iter().find(|g| g.home == u || g.away == u);
        let rec = mine.and_then(|m| self.results.iter().find(|r| !r.playoff && r.week == played_week && r.home == m.home && r.away == m.away));
        let won = rec.map(|r| (r.home == u && r.home_score > r.away_score) || (r.away == u && r.away_score > r.home_score)).unwrap_or(false);
        let mut r = self.rng.fork(self.season as u64 * 977 + played_week as u64 * 31 + 5);
        if week > ScheduleGenerator::WEEKS as i32 || self.finances[u].offers.len() >= 5 || !r.chance(if won { 0.5 } else { 0.3 }) {
            return;
        }
        let fin = &self.finances[u];
        let names: Vec<String> = fin.offers.iter().map(|x| x.name.clone()).chain(fin.sponsors.iter().map(|x| x.name.clone())).collect();
        let clauses: Vec<String> = fin.offers.iter().map(|x| x.clause_id.clone()).chain(fin.sponsors.iter().map(|x| x.clause_id.clone())).collect();
        let town = self.teams[u].town.clone();
        let o = SponsorMarket::generate(&town, &mut r, 1, week + 2, &names, &clauses).remove(0);
        let line = fill(inbox_text().pick("sponsorOfferNews", &mut r), &[("s", o.name.clone()), ("w", (week + 2).to_string())]);
        self.finances[u].offers.push(o);
        self.sponsor_news.push(line);
    }

    fn play_playoff_round(&mut self, is_final: bool, next: LeaguePhase, week_no: i32) -> WeekResult {
        let games: Vec<PlayoffGame> = if is_final { vec![self.playoffs.as_ref().unwrap().final_game.clone().expect("final created")] } else { self.playoffs.as_ref().unwrap().semifinals.clone() };
        let mut played = Vec::new();
        for (i, g) in games.iter().enumerate() {
            let rec = self.play_game(week_no, g.home(), g.away(), true);
            let result = PlayoffResult { home_score: rec.home_score, away_score: rec.away_score };
            let po = self.playoffs.as_mut().unwrap();
            if is_final {
                po.final_game.as_mut().unwrap().result = Some(result);
            } else {
                po.semifinals[i].result = Some(result);
            }
            played.push(rec);
        }
        let po = self.playoffs.as_mut().unwrap();
        if next == LeaguePhase::Championship {
            po.create_final();
        } else {
            po.champion = played[0].winner();
        }
        self.phase = next;
        WeekResult { week: week_no, games: played, phase_after: self.phase, work_news: self.work_news.clone() }
    }

    fn play_game(&mut self, week_no: i32, home: usize, away: usize, playoff: bool) -> GameRecord {
        let hu = self.unavailable(home);
        let au = self.unavailable(away);
        // Players sitting out this game work off one game of their injury.
        for id in hu.iter().chain(au.iter()).copied().collect::<Vec<_>>() {
            let n = self.injured_games[&id];
            self.serve_absence(id, n <= 1);
            if n <= 1 {
                self.injured_games.remove(&id);
                self.out_reason.remove(&id);
            } else {
                self.injured_games.insert(id, n - 1);
            }
        }
        self.prepare_active(home, &hu);
        self.prepare_active(away, &au);
        let gw = WeatherModel::pick(&mut self.rng, &self.teams[home].town, week_no, self.region);
        let sim_rng = self.rng.fork(week_no as u64 * 100 + home as u64 * 10 + away as u64);
        let hu_set: HashSet<u32> = hu.iter().copied().collect();
        let au_set: HashSet<u32> = au.iter().copied().collect();
        let result = GameSim::with_unavailable(
            sim_rng,
            &self.teams[home],
            &self.teams[away],
            GameConfig { weather: gw.weather, wind_arrow: gw.wind_arrow, playoff, keep_events: false },
            &hu_set,
            &au_set,
        )
        .simulate();
        let mut rec = GameRecord::new(week_no, home, away, result.home_score, result.away_score, result.overtime, playoff, gw.weather);
        if self.keep_box_scores {
            rec.box_score = Some(BoxScoreBuilder::build(&result));
        }
        let boost = self.fan_favorite_mult(&self.teams[home], &hu_set);
        self.settle_money(&mut rec, &result, home, away, week_no, playoff, boost);
        let rivalry = is_rivalry(&self.teams[home].town.id, &self.teams[away].town.id);
        let digest = GameDigest::build(&rec, &result, rivalry, self.standings.records[home].clone(), self.standings.records[away].clone());
        self.week_digests.push(digest);
        self.standings.add(&rec);
        if !playoff {
            let label = |t: usize| {
                let r = &self.standings.records[t];
                format!("{}-{}{}", r.wins, r.losses, if r.ties > 0 { format!("-{}", r.ties) } else { String::new() })
            };
            let (hl, al) = (label(home), label(away));
            if let Some(b) = rec.box_score.as_mut() {
                b.home_record = Some(hl);
                b.away_record = Some(al);
            }
        }
        self.results.push(rec.clone());
        self.accumulate(&result, week_no, playoff);
        if week_no < 16 {
            self.roll_job_risks(home, week_no);
            self.roll_job_risks(away, week_no);
        }
        if self.auto_set_roster {
            self.auto_set_active(); // after injuries and work absences are known, so you see the next lineup already set
        }
        rec
    }

    /// After a game: each healthy player may get hurt at his day job, or be kept away by work travel (misses the next game).
    fn roll_job_risks(&mut self, team: usize, week_no: i32) {
        let roster: Vec<(u32, String, Position, String)> = self.teams[team].roster.iter().map(|p| (p.id, p.name.clone(), p.position, p.day_job.clone())).collect();
        for (id, name, pos, day_job) in roster {
            if self.injured_games.get(&id).copied().unwrap_or(0) > 0 {
                continue;
            }
            let job = self.jobs.lookup(&day_job);
            let job_l = job.name.to_lowercase();
            if self.rng.chance(job.injury_risk as f64 * job_tuning::INJURY_PER_POINT) {
                let weights: Vec<(usize, f64)> = job_tuning::SEVERITY.iter().enumerate().map(|(i, s)| (i, s.weight)).collect();
                let s = &job_tuning::SEVERITY[self.rng.weighted(&weights)];
                let games = self.rng.range(s.min_games, s.max_games);
                self.injured_games.insert(id, games);
                let why = format!("Hurt at work ({job_l})");
                self.out_reason.insert(id, why.clone());
                self.record_absence(id, week_no, true, &why, games);
                if Some(team) == self.user_team {
                    self.work_news.push(format!("{} {} got hurt at his {job_l} job and is out {games} game{}.", pos.label(), name, if games == 1 { "" } else { "s" }));
                }
            } else if self.rng.chance(job.travel_risk as f64 * job_tuning::TRAVEL_MISS_PER_POINT) {
                self.injured_games.insert(id, 1);
                let why = format!("Stuck on a work trip ({job_l})");
                self.out_reason.insert(id, why.clone());
                self.record_absence(id, week_no, true, &why, 1);
                if Some(team) == self.user_team {
                    self.work_news.push(format!("{} {} is stuck on a work trip and misses the next game.", pos.label(), name));
                }
            }
        }
    }

    fn unavailable(&self, team: usize) -> BTreeSet<u32> {
        self.teams[team].roster.iter().filter(|p| self.injured_games.get(&p.id).copied().unwrap_or(0) > 0).map(|p| p.id).collect()
    }

    /// AI teams re-pick the best 46 each game; the user's team keeps its list minus injured players.
    fn prepare_active(&mut self, team: usize, unavailable: &BTreeSet<u32>) {
        if Some(team) == self.user_team {
            if self.auto_depth {
                self.teams[team].auto_sort_all();
            }
            if self.auto_set_roster {
                self.auto_set_active();
            } else {
                for id in unavailable {
                    self.teams[team].active_ids.remove(id);
                }
            }
            return;
        }
        let un: HashSet<u32> = unavailable.iter().copied().collect();
        let active = pick_active_list(&self.teams[team], &un);
        self.teams[team].active_ids = active;
    }

    fn accumulate(&mut self, r: &GameResult, week_no: i32, playoff: bool) {
        for side in Side::ALL {
            let into = if playoff { &mut self.playoff_stats } else { &mut self.season_stats };
            for (id, st) in &r.player_stats[side] {
                let dst = into.entry(*id).or_default();
                for (k, v) in st.to_map() {
                    if k.ends_with("_long") || k == "fg_long" {
                        dst.set_max(&k, v);
                    } else {
                        dst.add(&k, v);
                    }
                }
                // Anyone with a recorded stat played in the game, snaps or not (a carrier can be off the counted unit).
                if !st.is_empty() {
                    dst.inc("gp");
                }
            }
            for inj in r.injuries.iter().filter(|i| i.side == side) {
                if inj.games_out > 0 {
                    *self.injured_games.entry(inj.player_id).or_insert(0) += inj.games_out;
                    self.record_absence(inj.player_id, week_no, false, injury_label(&inj.severity), inj.games_out);
                    self.out_reason.remove(&inj.player_id);
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn settle_money(&mut self, rec: &mut GameRecord, result: &GameResult, home: usize, away: usize, week_no: i32, playoff: bool, home_boost: f64) {
        let pct = |lg: &League, t: usize| {
            let r = &lg.standings.records[t];
            if r.games() == 0 { 0.5 } else { r.pct() }
        };
        let sev = |s: Side| -> Vec<String> { result.injuries.iter().filter(|i| i.side == s).map(|i| i.severity.clone()).collect() };
        let tie = rec.tie();
        let rivalry = is_rivalry(&self.teams[home].town.id, &self.teams[away].town.id);
        let margin = (rec.home_score - rec.away_score).abs();
        let home_won = rec.home_score > rec.away_score;
        let mut hi = GameInput::new(week_no, playoff, home_won, tie, margin, rec.weather, rivalry, pct(self, away));
        hi.injury_severities = sev(Side::Home);
        hi.attendance_boost = home_boost;
        let mut ai = GameInput::new(week_no, playoff, home_won, tie, margin, rec.weather, rivalry, pct(self, home));
        ai.injury_severities = sev(Side::Away);
        let (hfin, afin) = two_mut(&mut self.finances, home, away);
        let (hf, af) = EconomyEngine::settle(&mut self.rng, &self.teams[home], &self.teams[away], hfin, afin, &hi, &ai, self.season);
        rec.home_finance = Some(hf);
        rec.away_finance = Some(af);
    }
}

/// The owner's levers on the finance side (user team only).
impl League {
    /// Moves one of your players to a new listed position (rating-based OVR is unchanged; only the label and depth slot move).
    pub fn change_position(&mut self, player_id: u32, pos: Position) -> bool {
        let u = self.user();
        let t = &mut self.teams[u];
        let Some(i) = t.roster.iter().position(|p| p.id == player_id) else { return false };
        if !t.roster[i].position.can_switch_to().contains(&pos) {
            return false;
        }
        let old = t.roster[i].position;
        t.roster[i].position = pos;
        if let Some(d) = t.depth.get_mut(&old) {
            d.retain(|id| *id != player_id);
        }
        true
    }

    /// Ticket price, clamped to a sane range.
    pub fn set_ticket_price(&mut self, price: f64) {
        self.my_finance_mut().ticket_price = price.clamp(1.0, 25.0);
    }

    fn projection_mult(&self) -> f64 {
        let f = self.my_finance();
        let unavailable: HashSet<u32> = self.injured_games.keys().copied().collect();
        f.lights_mult() * f.sponsor_att_mult() * self.fan_favorite_mult(self.me(), &unavailable)
    }

    /// Expected crowd at `price` for the next home game (mean of the price curve; game day still rolls noise).
    pub fn project_attendance(&self, price: f64, weather: Weather, opponent_win_pct: f64) -> i64 {
        let f = self.my_finance();
        TownEconomy::attendance(&f.town, f.fan_interest, price, &AttOpts { opponent_win_pct, weather, capacity: Some(f.capacity()), extra_mult: self.projection_mult(), ..Default::default() })
    }

    /// Low / typical / high crowd at `price`. Quiet nights (`promo` false) use the wider swing.
    pub fn project_attendance_band(&self, price: f64, weather: Weather, opponent_win_pct: f64, promo: bool) -> sp_economy::finance::AttendanceBand {
        let f = self.my_finance();
        TownEconomy::attendance_band(&f.town, f.fan_interest, price, &AttOpts { opponent_win_pct, weather, capacity: Some(f.capacity()), promo, extra_mult: self.projection_mult(), ..Default::default() })
    }

    /// Plan a promo night for one of your home games that has not been played yet.
    pub fn schedule_promo(&mut self, week_no: i32, promo_id: &str) -> bool {
        let Some(promo) = promo_by_id(promo_id) else { return false };
        let u = self.user();
        let is_home = self.schedule.iter().any(|g| g.week == week_no && g.home == u);
        if !is_home || self.phase != LeaguePhase::RegularSeason || week_no < self.week {
            return false;
        }
        let cost = promo.cost(&self.teams[u].town);
        if self.finances[u].cash < cost {
            return false;
        }
        self.finances[u].promos.insert(week_no, promo_id.to_string());
        true
    }

    /// Only promos for games not yet played can be cancelled.
    pub fn cancel_promo(&mut self, week_no: i32) -> bool {
        if self.phase != LeaguePhase::RegularSeason || week_no < self.week {
            return false;
        }
        self.my_finance_mut().promos.remove(&week_no).is_some()
    }

    pub const TRYOUT_COST: i64 = 400;

    /// Open tryouts: for a small fee, a handful of local walk-ons show up and join the free-agent pool.
    /// Pass a `position` to look for a specific job (a punter, a lineman); otherwise a mix turns up with at least one of every position.
    pub fn hold_tryouts(&mut self, position: Option<Position>, count: usize) -> Vec<Player> {
        let u = self.user();
        if self.finances[u].cash < Self::TRYOUT_COST || self.game_over() || self.tryout_held_this_week {
            return Vec::new();
        }
        self.tryout_held_this_week = true;
        let (season, week) = (self.season, self.week);
        let f = &mut self.finances[u];
        f.cash -= Self::TRYOUT_COST;
        f.ledger.push(LedgerEntry::new(season, week, LedgerKind::Other, -Self::TRYOUT_COST, "open tryouts"));
        let mut r = self.rng.fork(season as u64 * 131 + week as u64 * 7 + self.free_agents.len() as u64);
        let mix_total: usize = ROSTER_MIX.iter().map(|m| m.1).sum();
        let weights: Vec<(Position, f64)> = ROSTER_MIX.iter().map(|&(p, n)| (p, n as f64 / mix_total as f64)).collect();
        let town = self.teams[u].town.clone();
        let nearby: Vec<String> = self.region.all_towns().filter(|t| t.division == town.division).map(|t| t.name.clone()).collect();
        let mut made = Vec::new();
        // A mixed turnout always includes at least one player at every position, so nobody gets stuck short at a spot.
        let total = if position.is_none() { count.max(Position::ALL.len()) } else { count };
        for i in 0..total {
            let pos = position.unwrap_or_else(|| if i < Position::ALL.len() { Position::ALL[i] } else { r.weighted(&weights) });
            let tier = r.weighted(&[(Tier::WalkOn, 60.0), (Tier::LocalAmateur, 40.0)]);
            let home = if r.chance(0.75) { town.name.clone() } else { r.pick(&nearby).clone() };
            let age = r.range(19, 30);
            made.push(self.tg.players.generate(pos, GenOptions::default().tier(tier).hometown(&home).age(age).years_pro(0).acquired(ACQUIRED_TRY_OUT)));
        }
        self.free_agents.extend(made.iter().cloned());
        made
    }

    /// How the locker room sees your team: before the first game of a season everybody is 0.500.
    pub fn my_win_pct(&self) -> f64 {
        let r = &self.standings.records[self.user()];
        if r.games() == 0 { 0.5 } else { r.pct() }
    }

    pub fn upgrade_facility(&mut self, f: Facility) -> bool {
        let (week, season) = (self.week, self.season);
        self.my_finance_mut().upgrade(f, week, season)
    }

    /// Remove one level of a facility to save its upkeep. No refund.
    pub fn downgrade_facility(&mut self, f: Facility) -> bool {
        self.my_finance_mut().downgrade(f)
    }

    pub fn take_loan(&mut self, amount: i64) -> bool {
        let (week, season) = (self.week, self.season);
        self.my_finance_mut().take_loan(amount, week, season)
    }

    pub fn repay_loan(&mut self, amount: i64) -> bool {
        let (week, season) = (self.week, self.season);
        self.my_finance_mut().repay_loan(amount, week, season)
    }

    /// An inbox message pointer used by tests and UI.
    pub fn inbox_of_kind(&self, kind: InboxKind) -> Vec<&InboxItem> {
        self.inbox.iter().filter(|m| m.kind == kind).collect()
    }
}
