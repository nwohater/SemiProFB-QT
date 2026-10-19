use crate::awards::{defense_score, is_defense, is_offense, is_oline, offense_score};
use crate::inbox::{InboxFeed, InboxKind, PostOpts};
use crate::league::League;
use crate::text::commas;
use serde::{Deserialize, Serialize};
use sp_core::{Player, Position};
use std::collections::{BTreeMap, HashMap};

/// Tuning for the Hall of Fame. A career is scored once, when the player retires; the bar for induction is absolute,
/// so the Hall does not shrink or swell with the size of the league.
pub struct HofTuning;

impl HofTuning {
    /// Career points per raw production point, by position. Set so a top-three season at any position is worth about 10 before the
    /// baseline (raw units differ wildly: a guard racks up pancakes, a kicker a few dozen points).
    pub fn scale(p: Position) -> f64 {
        use Position::*;
        match p {
            Qb => 0.049,
            Rb => 0.042,
            Fb => 0.2,
            Wr => 0.049,
            Te => 0.096,
            Ot => 0.03,
            Og => 0.032,
            C => 0.045,
            De => 0.0625,
            Dt => 0.072,
            Olb => 0.076,
            Mlb => 0.056,
            Cb => 0.08,
            S => 0.072,
            K => 0.125,
            P => 0.027,
        }
    }

    /// Share of the induction score at which a player counts as an obvious Hall of Famer.
    pub const HALL_BOUND_SHARE: f64 = 0.85;
    /// A season must beat this to count: replacement-level play earns nothing, so long careers of mediocrity don't pile up.
    pub const BASELINE: f64 = 4.0;
    /// Playoff production counts a little extra: big games.
    pub const PLAYOFF_MULT: f64 = 1.25;
    /// A season with at least this many games played counts toward longevity.
    pub const STARTER_GAMES: f64 = 7.0;
    pub const PER_SEASON: f64 = 1.5;
    /// A title for a player who played in the playoffs that year.
    pub const PER_TITLE: f64 = 6.0;

    pub fn award_points(id: &str) -> f64 {
        match id {
            "mvp" => 45.0,
            "opoy" | "dpoy" | "ol" => 28.0,
            "roy" => 10.0,
            _ => 0.0,
        }
    }

    /// Careers shorter than this never get in, whatever the numbers.
    pub const MIN_SEASONS: usize = 5;
    /// Career points needed for induction.
    pub const INDUCT: f64 = 85.0;
    /// Retirees below this are not kept at all (keeps the ranking short).
    pub const KEEP: f64 = 18.0;
}

fn g(s: &BTreeMap<String, f64>, k: &str) -> f64 {
    s.get(k).copied().unwrap_or(0.0)
}

/// Production in one stat line, in career points above replacement level.
pub fn season_production(pos: Position, s: &BTreeMap<String, f64>, mult: f64) -> f64 {
    let raw = match pos {
        Position::K => g(s, "fgm") * 3.0 + g(s, "xpm") - (g(s, "fga") - g(s, "fgm")) * 1.5 - (g(s, "xpa") - g(s, "xpm")) * 1.5,
        Position::P => {
            if g(s, "punts") == 0.0 {
                0.0
            } else {
                g(s, "punts") * 0.6 + (g(s, "punt_yds") / g(s, "punts") - 36.0).clamp(0.0, 20.0) * g(s, "punts") * 0.25
            }
        }
        p if is_oline(p) => g(s, "pancakes") * 1.5 - g(s, "sacks_allowed") * 2.0 + g(s, "gp") * 2.0,
        p if is_offense(p) => offense_score(s),
        p if is_defense(p) => defense_score(s),
        _ => 0.0,
    };
    (raw * HofTuning::scale(pos) * mult - HofTuning::BASELINE).clamp(0.0, 99.0)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HallEntry {
    pub player_id: u32,
    pub name: String,
    pub pos: String,
    /// Last team.
    pub abbr: String,
    pub first_season: i32,
    pub last_season: i32,
    pub seasons: usize,
    pub titles: i32,
    /// "season:id"
    pub awards: Vec<String>,
    pub score: f64,
    /// Career numbers in words.
    pub line: String,
    pub inducted: bool,
    /// Last team's nickname, kept because a folded team's slot gets a new club.
    pub nickname: Option<String>,
    /// The jersey number he last wore.
    pub number: Option<i32>,
}

impl HallEntry {
    /// Awards of one kind, e.g. 2 MVPs.
    pub fn count(&self, id: &str) -> usize {
        self.awards.iter().filter(|a| a.ends_with(&format!(":{id}"))).count()
    }
}

impl League {
    /// Seasons this player was on the league champion, as (season, team abbreviation). He counts if he finished the
    /// season on the title team and either played in the playoffs or played in the regular season: a player who sat out
    /// the final (hurt, inactive, never got in) still gets the ring.
    pub fn titles_won(&self, player_id: u32) -> Vec<(i32, String)> {
        let champ_of: HashMap<i32, usize> = self.history.iter().map(|h| (h.season, h.champion)).collect();
        let mut rows: BTreeMap<i32, &crate::league::SeasonRow> = BTreeMap::new();
        for r in self.season_rows.get(&player_id).into_iter().flatten() {
            rows.insert(r.season, r);
        }
        for r in self.playoff_rows.get(&player_id).into_iter().flatten() {
            rows.insert(r.season, r); // the playoff row wins: it is his team at the end
        }
        rows.values()
            .filter(|r| r.team >= 0 && champ_of.get(&r.season) == Some(&(r.team as usize)))
            .map(|r| (r.season, r.abbr.clone().unwrap_or_else(|| self.teams[r.team as usize].town.abbr())))
            .collect()
    }

    /// Seasons with at least one game; a bench year (a line with zero games) does not count toward the Hall.
    fn seasons_played(&self, id: u32) -> usize {
        self.season_rows.get(&id).map(|rows| rows.iter().filter(|r| r.stats.get("gp").copied().unwrap_or(0.0) > 0.0).count()).unwrap_or(0)
    }

    /// A veteran whose career already looks Hall-bound: enough seasons played and most of the induction score banked.
    /// AI clubs keep these players until they retire when they can pay them.
    pub fn is_hall_bound(&self, p: &Player) -> bool {
        if self.seasons_played(p.id) < HofTuning::MIN_SEASONS {
            return false;
        }
        self.score_career(p, "", None).score >= HofTuning::INDUCT * HofTuning::HALL_BOUND_SHARE
    }

    /// Scores a finished career: regular-season and playoff production, years as a regular, titles, and awards.
    pub fn score_career(&self, p: &Player, last_abbr: &str, last_nickname: Option<&str>) -> HallEntry {
        let empty = Vec::new();
        let rows = self.season_rows.get(&p.id).unwrap_or(&empty);
        let po: HashMap<i32, &crate::league::SeasonRow> = self.playoff_rows.get(&p.id).into_iter().flatten().map(|r| (r.season, r)).collect();
        let champ_of: HashMap<i32, usize> = self.history.iter().map(|h| (h.season, h.champion)).collect();
        let mut score = 0.0;
        let mut titles = 0;
        let mut career: BTreeMap<String, f64> = BTreeMap::new();
        for r in rows {
            score += season_production(p.position, &r.stats, 1.0);
            if g(&r.stats, "gp") >= HofTuning::STARTER_GAMES {
                score += HofTuning::PER_SEASON;
            }
            for (k, v) in &r.stats {
                let cur = career.get(k).copied().unwrap_or(0.0);
                career.insert(k.clone(), if k.ends_with("_long") { cur.max(*v) } else { cur + v });
            }
            if let Some(pr) = po.get(&r.season) {
                score += season_production(p.position, &pr.stats, 0.5) * HofTuning::PLAYOFF_MULT;
            }
            let team = po.get(&r.season).map(|x| x.team).unwrap_or(r.team);
            if r.team >= 0 && champ_of.get(&r.season).map(|&c| c as i32) == Some(team) {
                titles += 1;
                score += HofTuning::PER_TITLE;
            }
        }
        let won = self.awards_won(p.id);
        for w in &won {
            score += HofTuning::award_points(&w.id);
        }
        let first = rows.first().map(|r| r.season).unwrap_or(self.season);
        let last = rows.last().map(|r| r.season).unwrap_or(self.season);
        let inducted = self.seasons_played(p.id) >= HofTuning::MIN_SEASONS && score >= HofTuning::INDUCT;
        HallEntry {
            player_id: p.id,
            name: p.name.clone(),
            pos: p.position.label().to_string(),
            abbr: last_abbr.to_string(),
            first_season: first,
            last_season: last,
            seasons: rows.len(),
            titles,
            awards: won.iter().map(|w| format!("{}:{}", w.season, w.id)).collect(),
            score,
            line: career_line(p.position, &career),
            inducted,
            nickname: last_nickname.map(|s| s.to_string()),
            number: p.jersey,
        }
    }

    /// A retiring player's career goes on the books; returns the entry if he made the Hall.
    pub fn record_retirement(&mut self, p: &Player, last_abbr: &str, last_nickname: Option<&str>) -> Option<HallEntry> {
        let e = self.score_career(p, last_abbr, last_nickname);
        if e.score < HofTuning::KEEP {
            return None;
        }
        self.hall_of_fame.push(e.clone());
        if e.inducted { Some(e) } else { None }
    }

    /// Everyone inducted, newest class first, best first within a class.
    pub fn inductees(&self) -> Vec<&HallEntry> {
        let mut v: Vec<&HallEntry> = self.hall_of_fame.iter().filter(|e| e.inducted).collect();
        v.sort_by(|a, b| b.last_season.cmp(&a.last_season).then_with(|| b.score.total_cmp(&a.score)));
        v
    }

    /// Retired players who fell short, best first: the debate list.
    pub fn near_misses(&self) -> Vec<&HallEntry> {
        let mut v: Vec<&HallEntry> = self.hall_of_fame.iter().filter(|e| !e.inducted).collect();
        v.sort_by(|a, b| b.score.total_cmp(&a.score));
        v
    }

    /// Posts the induction announcement for the class that retired this offseason.
    pub fn write_inbox_hall_of_fame(&mut self, cls: &mut [HallEntry]) {
        if self.user_team.is_none() || cls.is_empty() {
            return;
        }
        cls.sort_by(|a, b| b.score.total_cmp(&a.score));
        let lines: Vec<String> = cls
            .iter()
            .map(|e| format!("{} ({}, {}), {} seasons: {}{}.", e.name, e.pos, e.abbr, e.seasons, e.line, if e.titles > 0 { format!(", {} title{}", e.titles, if e.titles == 1 { "" } else { "s" }) } else { String::new() }))
            .collect();
        self.post_inbox(InboxFeed::Money, InboxKind::Award, &format!("Hall of Fame: class of {}", cls[0].last_season), &format!("{}\nSee League > Hall of Fame.", lines.join("\n")), PostOpts::week(0));
    }
}

fn career_line(pos: Position, s: &BTreeMap<String, f64>) -> String {
    let n = |k: &str| commas(g(s, k).round() as i64);
    if pos == Position::K {
        return format!("{}/{} FG, {} XP", n("fgm"), n("fga"), n("xpm"));
    }
    if pos == Position::P {
        return format!("{} punts, {} avg", n("punts"), if g(s, "punts") == 0.0 { "0".to_string() } else { format!("{:.1}", g(s, "punt_yds") / g(s, "punts")) });
    }
    if is_oline(pos) {
        return format!("{} games, {} pancakes", n("gp"), n("pancakes"));
    }
    if is_defense(pos) {
        let tkl = g(s, "tackles_solo") + g(s, "tackles_ast");
        let sck = g(s, "sacks");
        return format!("{} tkl, {} sck, {} INT", tkl.round() as i64, if sck == sck.round() { format!("{sck:.0}") } else { format!("{sck:.1}") }, n("int"));
    }
    match pos {
        Position::Qb => format!("{} yds, {} TD, {} INT", n("pass_yds"), n("pass_td"), n("pass_int")),
        Position::Rb | Position::Fb => format!("{} rush yds, {} TD", n("rush_yds"), (g(s, "rush_td") + g(s, "rec_td")).round() as i64),
        _ => format!("{} rec, {} yds, {} TD", n("rec"), n("rec_yds"), n("rec_td")),
    }
}
