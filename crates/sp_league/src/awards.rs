use crate::league::League;
use serde::{Deserialize, Serialize};
use sp_core::{Player, Position, Rng};
use std::collections::BTreeMap;

/// Tuning for the season awards. Scores are fantasy-style point totals scaled by position so one position cannot sweep.
pub struct AwardTuning;

impl AwardTuning {
    /// In the league's first season everyone is a rookie, so Rookie of the Year only counts players this age or younger.
    pub const FOUNDING_ROOKIE_MAX_AGE: i32 = 23;

    /// Offense: yards and touchdowns, minus giveaways.
    pub const PASS_YD: f64 = 0.04;
    pub const PASS_TD: f64 = 4.0;
    pub const PASS_INT: f64 = -2.0;
    pub const RUSH_YD: f64 = 0.1;
    pub const RUSH_TD: f64 = 7.0;
    pub const REC_YD: f64 = 0.1;
    pub const REC_TD: f64 = 7.0;
    pub const RECEPTION: f64 = 0.25;
    pub const FUMBLE_LOST: f64 = -2.0;

    /// Defense: stops, pressure and takeaways.
    pub const TACKLE: f64 = 0.9;
    pub const SACK: f64 = 6.0;
    pub const TFL: f64 = 2.0;
    pub const INTERCEPTION: f64 = 9.0;
    pub const PASS_DEFENDED: f64 = 1.2;
    pub const FORCED_FUMBLE: f64 = 4.0;
    pub const RECOVERY: f64 = 2.0;
    pub const DEF_TD: f64 = 6.0;
    pub const QB_HIT: f64 = 0.5;

    /// Trench Monster (offensive line): pancake blocks, minus the sacks and pressures that got through.
    pub const PANCAKE: f64 = 1.0;
    pub const SACK_ALLOWED: f64 = -5.0;
    pub const PRESSURE_ALLOWED: f64 = -0.5;

    /// Quarterbacks pile up bigger numbers than anyone else, so their score counts for less.
    pub fn position_mult(p: Position) -> f64 {
        use Position::*;
        match p {
            Qb => 0.92,
            Rb | Fb => 0.76,
            Wr => 1.0,
            Te => 0.8,
            De => 0.9,
            Dt => 0.95,
            Mlb | Olb => 1.0,
            Cb => 1.15,
            S => 1.05,
            _ => 1.0,
        }
    }

    /// Extra MVP-only discount for pass rushers (their sack points stack up fast next to a 2,000-yard back), so DPOY is unaffected.
    pub fn mvp_position_mult(p: Position) -> f64 {
        use Position::*;
        match p {
            De | Dt => 1.2,
            Mlb | Olb | Cb | S => 1.05,
            _ => 1.0,
        }
    }

    /// A winning team helps an MVP case: score x (base + span x win pct).
    pub const MVP_TEAM_BASE: f64 = 0.85;
    pub const MVP_TEAM_SPAN: f64 = 0.3;

    /// A player needs at least this share of his team's games.
    pub const MIN_GAME_SHARE: f64 = 0.5;
    pub const FINALISTS: usize = 3;

    /// Voters are human: each player's season score is nudged up or down by up to this share, the same for every award he is in.
    pub const VOTER_NOISE: f64 = 0.08;
}

pub fn is_offense(p: Position) -> bool {
    use Position::*;
    matches!(p, Qb | Rb | Fb | Wr | Te)
}

pub fn is_defense(p: Position) -> bool {
    use Position::*;
    matches!(p, De | Dt | Mlb | Olb | Cb | S)
}

pub fn is_oline(p: Position) -> bool {
    use Position::*;
    matches!(p, Ot | Og | C)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AwardFinalist {
    pub player_id: u32,
    pub name: String,
    pub pos: String,
    /// Team name at the time.
    pub team: String,
    /// Key stats in words.
    pub line: String,
    pub score: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AwardCategory {
    /// mvp | opoy | dpoy | ol | roy
    pub id: String,
    pub title: String,
    pub blurb: String,
    /// Best first, up to three.
    pub finalists: Vec<AwardFinalist>,
}

impl AwardCategory {
    pub fn winner(&self) -> Option<&AwardFinalist> {
        self.finalists.first()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeasonAwards {
    pub season: i32,
    /// False = a race that is still on.
    pub finalized: bool,
    /// Regular-season games each team has played (for the race header).
    pub games_played: i32,
    pub categories: Vec<AwardCategory>,
}

impl SeasonAwards {
    pub fn category(&self, id: &str) -> Option<&AwardCategory> {
        self.categories.iter().find(|c| c.id == id)
    }
}

/// One award a player has won, for his bio.
#[derive(Clone, Debug, PartialEq)]
pub struct AwardWin {
    pub season: i32,
    pub id: String,
    pub title: String,
    /// Team name at the time.
    pub team: String,
}

/// A steady per-player, per-season nudge in 1 +/- `VOTER_NOISE`, so the race does not flicker between looks but varies from year to year.
fn voter_nudge(league_seed: u64, season: i32, player_id: u32) -> f64 {
    let seed = league_seed.wrapping_mul(1_000_003) ^ (season as u64).wrapping_mul(7919) ^ (player_id as u64).wrapping_mul(104_729);
    1.0 + AwardTuning::VOTER_NOISE * (2.0 * Rng::new(seed).next_f64() - 1.0)
}

fn g(s: &BTreeMap<String, f64>, k: &str) -> f64 {
    s.get(k).copied().unwrap_or(0.0)
}

pub fn offense_score(s: &BTreeMap<String, f64>) -> f64 {
    use AwardTuning as T;
    g(s, "pass_yds") * T::PASS_YD + g(s, "pass_td") * T::PASS_TD + g(s, "pass_int") * T::PASS_INT + g(s, "rush_yds") * T::RUSH_YD + g(s, "rush_td") * T::RUSH_TD
        + g(s, "rec_yds") * T::REC_YD + g(s, "rec_td") * T::REC_TD + g(s, "rec") * T::RECEPTION + g(s, "fumbles_lost") * T::FUMBLE_LOST
}

pub fn defense_score(s: &BTreeMap<String, f64>) -> f64 {
    use AwardTuning as T;
    (g(s, "tackles_solo") + g(s, "tackles_ast")) * T::TACKLE + g(s, "sacks") * T::SACK + g(s, "tfl") * T::TFL + g(s, "int") * T::INTERCEPTION + g(s, "pd") * T::PASS_DEFENDED
        + g(s, "ff") * T::FORCED_FUMBLE + g(s, "fr") * T::RECOVERY + g(s, "int_td") * T::DEF_TD + g(s, "qb_hits") * T::QB_HIT
}

pub fn trench_score(s: &BTreeMap<String, f64>) -> f64 {
    use AwardTuning as T;
    g(s, "pancakes") * T::PANCAKE + g(s, "sacks_allowed") * T::SACK_ALLOWED + g(s, "pressures_allowed") * T::PRESSURE_ALLOWED
}

fn n(v: f64) -> String {
    crate::text::commas(v.round() as i64)
}

fn sacks_text(s: &BTreeMap<String, f64>) -> String {
    let v = g(s, "sacks");
    if v == v.round() { format!("{v:.0}") } else { format!("{v:.1}") }
}

fn stat_line(pos: Position, s: &BTreeMap<String, f64>) -> String {
    if is_defense(pos) {
        let tkl = g(s, "tackles_solo") + g(s, "tackles_ast");
        return format!("{} tkl, {} sck, {} INT{}", n(tkl), sacks_text(s), n(g(s, "int")), if g(s, "pd") > 0.0 { format!(", {} PD", n(g(s, "pd"))) } else { String::new() });
    }
    if is_oline(pos) {
        let sa = g(s, "sacks_allowed");
        return format!("{} pancakes, {} sack{} allowed, {} pressures allowed", n(g(s, "pancakes")), n(sa), if sa == 1.0 { "" } else { "s" }, n(g(s, "pressures_allowed")));
    }
    match pos {
        Position::Qb => format!(
            "{} yds, {} TD, {} INT{}",
            n(g(s, "pass_yds")),
            n(g(s, "pass_td")),
            n(g(s, "pass_int")),
            if g(s, "rush_yds") >= 100.0 { format!(", {} rush yds", n(g(s, "rush_yds"))) } else { String::new() }
        ),
        Position::Rb | Position::Fb => format!("{} rush yds, {} TD, {} rec", n(g(s, "rush_yds")), n(g(s, "rush_td") + g(s, "rec_td")), n(g(s, "rec"))),
        _ => format!("{} rec, {} yds, {} TD", n(g(s, "rec")), n(g(s, "rec_yds")), n(g(s, "rec_td"))),
    }
}

const TITLES: [(&str, &str, &str); 5] = [
    ("mvp", "Most Valuable Player", "Best season in the league, helped by a winning team."),
    ("opoy", "Offensive Player of the Year", "Quarterbacks, backs, receivers and tight ends."),
    ("dpoy", "Defensive Player of the Year", "Linemen, linebackers and defensive backs."),
    ("ol", "Trench Monster", "Best offensive lineman: pancake blocks, minus sacks and pressures allowed."),
    ("roy", "Rookie of the Year", "First-year players: draftees and other rookies. In the league's first season, players 23 and under."),
];

#[derive(Clone)]
struct Cand {
    id: u32,
    team: usize,
    value: f64,
    off: bool,
    position: Position,
    is_rookie: bool,
    age: i32,
}

impl League {
    /// Refreshes one player's award count from the history. Called when a price is quoted, so the ask never relies on a stale copy.
    pub fn sync_awards_for(&mut self, player_id: u32) {
        let n = self.awards_won(player_id).len() as i32;
        for t in &mut self.teams {
            for p in &mut t.roster {
                if p.id == player_id {
                    p.awards = n;
                }
            }
        }
        for p in &mut self.free_agents {
            if p.id == player_id {
                p.awards = n;
            }
        }
    }

    /// Copies each player's award count onto him so his pay ask reflects it. Call whenever `awards_history` changes.
    pub fn sync_award_counts(&mut self) {
        let mut counts: BTreeMap<u32, i32> = BTreeMap::new();
        for a in &self.awards_history {
            for c in &a.categories {
                if let Some(w) = c.winner() {
                    *counts.entry(w.player_id).or_insert(0) += 1;
                }
            }
        }
        for t in &mut self.teams {
            for p in &mut t.roster {
                p.awards = counts.get(&p.id).copied().unwrap_or(0);
            }
        }
        for p in &mut self.free_agents {
            p.awards = counts.get(&p.id).copied().unwrap_or(0);
        }
    }

    /// Every award `player_id` has won, oldest first.
    pub fn awards_won(&self, player_id: u32) -> Vec<AwardWin> {
        let mut out = Vec::new();
        for a in &self.awards_history {
            for c in &a.categories {
                if let Some(w) = c.winner() {
                    if w.player_id == player_id {
                        out.push(AwardWin { season: a.season, id: c.id.clone(), title: c.title.clone(), team: w.team.clone() });
                    }
                }
            }
        }
        out
    }

    /// The awards as they stand right now (a race until the season ends). Uses regular-season numbers only.
    pub fn award_race(&self, finalized: bool) -> SeasonAwards {
        let mut cands: Vec<Cand> = Vec::new();
        let mut linemen: Vec<Cand> = Vec::new();
        let mut played = 0;
        for (t, team) in self.teams.iter().enumerate() {
            let games = self.standings.records[t].games();
            if games > played {
                played = games;
            }
            for p in &team.roster {
                let (in_off, in_def, in_line) = (is_offense(p.position), is_defense(p.position), is_oline(p.position));
                if !in_off && !in_def && !in_line {
                    continue;
                }
                let Some(stats) = self.season_stats.get(&p.id) else { continue };
                let s = stats.to_map();
                if g(&s, "gp") < ((games as f64 * AwardTuning::MIN_GAME_SHARE).ceil() as i64).clamp(1, 99) as f64 {
                    continue;
                }
                let base = Cand { id: p.id, team: t, value: 0.0, off: true, position: p.position, is_rookie: p.is_rookie(), age: p.age };
                if in_line {
                    let v = trench_score(&s) * voter_nudge(self.seed, self.season, p.id);
                    if v > 0.0 {
                        linemen.push(Cand { value: v, ..base });
                    }
                    continue;
                }
                let raw = if in_off { offense_score(&s) } else { defense_score(&s) };
                let v = raw * AwardTuning::position_mult(p.position) * voter_nudge(self.seed, self.season, p.id);
                if v > 0.0 {
                    cands.push(Cand { value: v, off: in_off, ..base });
                }
            }
        }
        let empty = |c: &(&str, &str, &str)| AwardCategory { id: c.0.into(), title: c.1.into(), blurb: c.2.into(), finalists: vec![] };
        if cands.is_empty() && linemen.is_empty() {
            return SeasonAwards { season: self.season, finalized, games_played: played, categories: TITLES.iter().map(empty).collect() };
        }
        let top_off = cands.iter().filter(|c| c.off).fold(1.0, |m, c| if c.value > m { c.value } else { m });
        let top_def = cands.iter().filter(|c| !c.off).fold(1.0, |m, c| if c.value > m { c.value } else { m });
        let top_line = linemen.iter().fold(1.0, |m, c| if c.value > m { c.value } else { m });
        let mvp_value = |c: &Cand| c.value * AwardTuning::mvp_position_mult(c.position);
        let top_all = cands.iter().fold(1.0, |m, c| if mvp_value(c) > m { mvp_value(c) } else { m });
        let norm = |c: &Cand| c.value / if c.off { top_off } else { top_def };
        // The MVP weighs both sides of the ball on one scale, so a dominant back is not just "1.0" next to a dominant end.
        let norm_all = |c: &Cand| mvp_value(c) / top_all;

        let fin = |c: &Cand, score: f64| -> AwardFinalist {
            let p = self.teams[c.team].roster.iter().find(|p| p.id == c.id).expect("candidate on roster");
            AwardFinalist {
                player_id: c.id,
                name: p.name.clone(),
                pos: c.position.label().to_string(),
                team: self.teams[c.team].town.team_name(),
                line: stat_line(c.position, &self.season_stats[&c.id].to_map()),
                score,
            }
        };
        let top = |pool: Vec<&Cand>, score: &dyn Fn(&Cand) -> f64| -> Vec<AwardFinalist> {
            let mut l: Vec<(&Cand, f64)> = pool.into_iter().map(|c| (c, score(c))).collect();
            l.sort_by(|a, b| b.1.total_cmp(&a.1));
            l.into_iter().take(AwardTuning::FINALISTS).map(|(c, s)| fin(c, s)).collect()
        };
        let team_mult = |t: usize| {
            let r = &self.standings.records[t];
            AwardTuning::MVP_TEAM_BASE + AwardTuning::MVP_TEAM_SPAN * if r.games() == 0 { 0.5 } else { r.pct() }
        };
        let rookies: Vec<&Cand> = cands.iter().filter(|c| c.is_rookie && (self.season > 1 || c.age <= AwardTuning::FOUNDING_ROOKIE_MAX_AGE)).collect();
        let cat = |i: usize, finalists: Vec<AwardFinalist>| AwardCategory { id: TITLES[i].0.into(), title: TITLES[i].1.into(), blurb: TITLES[i].2.into(), finalists };
        SeasonAwards {
            season: self.season,
            finalized,
            games_played: played,
            categories: vec![
                cat(0, top(cands.iter().collect(), &|c| norm_all(c) * team_mult(c.team))),
                cat(1, top(cands.iter().filter(|c| c.off).collect(), &norm)),
                cat(2, top(cands.iter().filter(|c| !c.off).collect(), &norm)),
                cat(3, top(linemen.iter().collect(), &|c| c.value / top_line)),
                cat(4, top(rookies, &norm)),
            ],
        }
    }
}

#[allow(dead_code)]
fn _keep(_: &Player) {}
