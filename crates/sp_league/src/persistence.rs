//! Save and load a whole franchise as JSON. Box scores are kept for your own games only; ledgers are trimmed.

use crate::absence_log::AbsenceEvent;
use crate::awards::SeasonAwards;
use crate::conduct::ConductGossip;
use crate::draft::Draft;
use crate::fan_favorites::FanNote;
use crate::gazette::GazetteIssue;
use crate::hall_of_fame::HallEntry;
use crate::inbox::InboxItem;
use crate::league::{League, LeaguePhase, RetiredEntry, RetiredPlayer, SeasonRow, SeasonSummary};
use crate::owner_career::{ClubHelp, OwnerSeason};
use crate::playoffs::Playoffs;
use crate::schedule::{ScheduleGenerator, ScheduledGame};
use crate::standings::{GameRecord, Standings};
use serde::{Deserialize, Serialize};
use sp_core::jobs::JobTable;
use sp_core::team_generator::TeamGenerator;
use sp_core::{Player, PlayerGenerator, Region, Rng, Team};
use sp_economy::TeamFinance;
use sp_sim::PlayerStats;
use std::collections::{BTreeMap, BTreeSet, HashSet};

pub const SAVE_VERSION: u32 = 1;

/// How many ledger rows each club keeps in a save.
pub const LEDGER_KEEP: usize = 400;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveData {
    v: u32,
    rng: Rng,
    gen_rng: Rng,
    next_id: u32,
    user_team: Option<usize>,
    seed: u64,
    phase: LeaguePhase,
    week: i32,
    season: i32,
    tiebreak: u64,
    teams: Vec<Team>,
    finances: Vec<TeamFinance>,
    schedule: Vec<ScheduledGame>,
    results: Vec<GameRecord>,
    keep_box: bool,
    auto_set: bool,
    #[serde(default)]
    auto_depth: bool,
    playoffs: Option<Playoffs>,
    absences: Vec<AbsenceEvent>,
    injured: BTreeMap<u32, i32>,
    out_why: BTreeMap<u32, String>,
    stats: BTreeMap<u32, BTreeMap<String, f64>>,
    pstats: BTreeMap<u32, BTreeMap<String, f64>>,
    prows: BTreeMap<u32, Vec<SeasonRow>>,
    pick_owners: BTreeMap<String, usize>,
    trade_lock: BTreeMap<u32, i32>,
    fa: Vec<Player>,
    draft: Option<Draft>,
    history: Vec<SeasonSummary>,
    rows: BTreeMap<u32, Vec<SeasonRow>>,
    town_used: BTreeMap<String, i32>,
    nicks_used: BTreeSet<String>,
    pending: BTreeSet<u32>,
    wants_play: BTreeSet<u32>,
    tryout: bool,
    bill_tried: bool,
    favs: BTreeSet<u32>,
    fan_news: Vec<FanNote>,
    changes: BTreeMap<u32, ([i32; 8], [i32; 8])>,
    retired: Vec<String>,
    retired_now: Vec<RetiredEntry>,
    hof: Vec<HallEntry>,
    owner_seasons: Vec<OwnerSeason>,
    club_help: Vec<ClubHelp>,
    retired_all: BTreeMap<u32, RetiredPlayer>,
    awards: Vec<SeasonAwards>,
    rival_cash: BTreeMap<usize, i32>,
    gazette_cash: BTreeMap<usize, i32>,
    conduct_gossip: Vec<ConductGossip>,
    region: String,
    jobs: Vec<(String, i32, i32)>,
    gazette: Vec<GazetteIssue>,
    inbox: Vec<InboxItem>,
    inbox_seq: u32,
}

pub struct LeagueSave;

impl LeagueSave {
    pub fn encode(lg: &League) -> String {
        serde_json::to_string(&Self::to_data(lg)).expect("league serializes")
    }

    pub fn decode(s: &str) -> Result<League, String> {
        let data: SaveData = serde_json::from_str(s).map_err(|e| format!("bad save: {e}"))?;
        Self::from_data(data)
    }

    fn to_data(lg: &League) -> SaveData {
        // Box scores are kept for your own games only, to keep saves small.
        let results: Vec<GameRecord> = lg
            .results
            .iter()
            .map(|g| {
                let mut g = g.clone();
                if !(Some(g.home) == lg.user_team || Some(g.away) == lg.user_team) {
                    g.box_score = None;
                }
                g
            })
            .collect();
        SaveData {
            v: SAVE_VERSION,
            rng: lg.rng.clone(),
            gen_rng: lg.tg.players.rng.clone(),
            next_id: lg.tg.players.next_id,
            user_team: lg.user_team,
            seed: lg.seed,
            phase: lg.phase,
            week: lg.week,
            season: lg.season,
            tiebreak: lg.standings.tiebreak_seed,
            teams: lg.teams.clone(),
            finances: lg.finances.iter().map(|f| f.for_save(LEDGER_KEEP)).collect(),
            schedule: lg.schedule.clone(),
            results,
            keep_box: lg.keep_box_scores,
            auto_set: lg.auto_set_roster,
            auto_depth: lg.auto_depth,
            playoffs: lg.playoffs.clone(),
            absences: lg.absences.clone(),
            injured: lg.injured_games.clone(),
            out_why: lg.out_reason.iter().filter(|(k, _)| lg.injured_games.contains_key(k)).map(|(k, v)| (*k, v.clone())).collect(),
            stats: lg.season_stats.iter().map(|(k, v)| (*k, v.to_map())).collect(),
            pstats: lg.playoff_stats.iter().map(|(k, v)| (*k, v.to_map())).collect(),
            prows: lg.playoff_rows.clone(),
            pick_owners: lg.pick_owners.clone(),
            trade_lock: lg.trade_lock.clone(),
            fa: lg.free_agents.clone(),
            draft: lg.draft.clone(),
            history: lg.history.clone(),
            rows: lg.season_rows.clone(),
            town_used: lg.town_last_used.clone(),
            nicks_used: lg.used_nicknames.clone(),
            pending: lg.pending_expired.clone(),
            wants_play: lg.wants_to_play.clone(),
            tryout: lg.tryout_held_this_week,
            bill_tried: lg.surprise_bill_attempted,
            favs: lg.fan_favorite_ids.clone(),
            fan_news: lg.fan_news.clone(),
            changes: lg.last_changes.clone(),
            retired: lg.last_retired.clone(),
            retired_now: lg.last_retired_entries.clone(),
            hof: lg.hall_of_fame.clone(),
            owner_seasons: lg.owner_seasons.clone(),
            club_help: lg.club_help.clone(),
            retired_all: lg.retired_players.clone(),
            awards: lg.awards_history.clone(),
            rival_cash: lg.rival_cash_stage.clone(),
            gazette_cash: lg.gazette_cash_stage.clone(),
            conduct_gossip: lg.conduct_gossip.clone(),
            region: lg.region.id.clone(),
            jobs: lg.jobs.rows(),
            gazette: lg.gazette.clone(),
            inbox: lg.inbox.clone(),
            inbox_seq: lg.inbox_seq,
        }
    }

    fn from_data(d: SaveData) -> Result<League, String> {
        if d.v != SAVE_VERSION {
            return Err(format!("Unsupported save version {}", d.v));
        }
        let region: &'static Region = Region::by_id(Some(&d.region));
        let mut tg = TeamGenerator::new(PlayerGenerator::new(d.gen_rng.clone()));
        tg.players.next_id = d.next_id;
        tg.players.jobs = JobTable::from_rows(&d.jobs);
        let jobs: JobTable = tg.players.jobs.clone();
        let mut standings = Standings::new(d.teams.len(), ScheduleGenerator::division_of, d.tiebreak);
        for r in &d.results {
            standings.add(r);
        }
        let _ = HashSet::<u32>::new();
        Ok(League {
            rng: d.rng,
            teams: d.teams,
            tg,
            schedule: d.schedule,
            user_team: d.user_team,
            seed: d.seed,
            keep_box_scores: d.keep_box,
            standings,
            playoffs: d.playoffs,
            phase: d.phase,
            week: d.week,
            season: d.season,
            results: d.results,
            free_agents: d.fa,
            draft: d.draft,
            pick_owners: d.pick_owners,
            trade_lock: d.trade_lock,
            history: d.history,
            season_rows: d.rows,
            retired_players: d.retired_all,
            hall_of_fame: d.hof,
            owner_seasons: d.owner_seasons,
            club_help: d.club_help,
            playoff_rows: d.prows,
            town_last_used: d.town_used,
            used_nicknames: d.nicks_used,
            last_changes: d.changes,
            last_retired: d.retired,
            last_retired_entries: d.retired_now,
            pending_expired: d.pending,
            wants_to_play: d.wants_play,
            tryout_held_this_week: d.tryout,
            surprise_bill_attempted: d.bill_tried,
            fan_favorite_ids: d.favs,
            fan_news: d.fan_news,
            awards_history: d.awards,
            inbox: d.inbox,
            inbox_seq: d.inbox_seq,
            region,
            conduct_gossip: d.conduct_gossip,
            rival_cash_stage: d.rival_cash,
            sponsor_news: Vec::new(),
            finances: d.finances,
            injured_games: d.injured,
            out_reason: d.out_why,
            absences: d.absences,
            gazette: d.gazette,
            week_digests: Vec::new(),
            gazette_cash_stage: d.gazette_cash,
            auto_set_roster: d.auto_set,
            auto_depth: d.auto_depth,
            jobs,
            work_news: Vec::new(),
            season_stats: d.stats.into_iter().map(|(k, v)| (k, PlayerStats::from_map(v))).collect(),
            playoff_stats: d.pstats.into_iter().map(|(k, v)| (k, PlayerStats::from_map(v))).collect(),
        })
    }
}
