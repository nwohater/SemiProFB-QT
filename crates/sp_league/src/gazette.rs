use crate::league::League;
use crate::schedule::ScheduleGenerator;
use crate::standings::{GameRecord, TeamRecord};
use crate::text::{fill, gazette_text};
use serde::{Deserialize, Serialize};
use sp_core::Rng;
use sp_sim::box_score::Performance;
use sp_sim::play_event::InjuryRecord;
use sp_sim::{BoxScoreBuilder, GameResult, Side};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Sections of an issue, in page order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GazetteSection {
    Front,
    Briefs,
    Scores,
    Spotlight,
    Absences,
    Money,
    Gossip,
    Standings,
}

/// A finished game, so the scoreboard can show both clubs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GameLine {
    pub away: usize,
    pub away_score: i32,
    pub home: usize,
    pub home_score: i32,
    /// Rivalry | Semifinal | Championship | OT | '' (can combine, e.g. 'Rivalry · OT')
    pub tag: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GazetteStory {
    pub section: GazetteSection,
    pub headline: String,
    pub body: String,
    /// Team index the story is about, or -1.
    pub team: i32,
    /// Player the story is about, or -1.
    pub player_id: i64,
    /// Story type, which is also the name of its picture in assets/artwork/stories ('' = none).
    pub kind: String,
    /// Set on scoreboard stories.
    pub game: Option<GameLine>,
}

impl GazetteStory {
    fn new(section: GazetteSection, headline: String, body: String) -> Self {
        Self { section, headline, body, team: -1, player_id: -1, kind: String::new(), game: None }
    }

    fn as_brief(&self) -> Self {
        Self { section: GazetteSection::Briefs, ..self.clone() }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GazetteIssue {
    pub season: i32,
    pub week: i32,
    pub label: String,
    pub stories: Vec<GazetteStory>,
    /// Running issue number across the whole game (old issues are dropped, the number keeps climbing).
    pub number: i32,
}

impl GazetteIssue {
    pub fn in_section(&self, s: GazetteSection) -> Vec<&GazetteStory> {
        self.stories.iter().filter(|st| st.section == s).collect()
    }

    pub fn lead(&self) -> Option<&GazetteStory> {
        self.in_section(GazetteSection::Front).into_iter().next()
    }
}

/// Every story type that has a picture in assets/artwork/stories.
pub const GAZETTE_STORY_KINDS: [&str; 13] = ["champion", "semifinal", "rivalry", "rivalry_ot", "upset", "star", "shutout", "overtime", "blowout", "hot_streak", "cold_streak", "broke", "quiet"];

/// Pictures per story type: the first is `kind.webp`, the others `kind_2.webp`, `kind_3.webp` and so on.
pub const GAZETTE_ART_VARIANTS: usize = 5;

pub fn gazette_art_file(kind: &str, variant: usize) -> String {
    if variant <= 1 { kind.to_string() } else { format!("{kind}_{variant}") }
}

/// Which picture variant (1..=`GAZETTE_ART_VARIANTS`) a week's lead story gets. Its own Rng, so the sim and the text are untouched.
pub fn gazette_variant_for(season: i32, week_no: i32) -> usize {
    1 + Rng::new(season as u64 * 3571 + week_no as u64 * 17 + 5).next_int(GAZETTE_ART_VARIANTS)
}

/// Tuning for what makes the front page.
pub struct GazetteTuning;

impl GazetteTuning {
    pub const MAX_ISSUES: usize = 40;
    pub const CHAMPION: f64 = 100.0;
    pub const SEMIFINAL_BONUS: f64 = 40.0;
    pub const RIVALRY: f64 = 62.0;
    pub const UPSET: f64 = 52.0;
    pub const UPSET_PCT_GAP: f64 = 0.25;
    pub const STAR: f64 = 30.0;
    pub const STAR_MIN_SCORE: f64 = 16.0;
    pub const SHUTOUT: f64 = 44.0;
    pub const OVERTIME: f64 = 46.0;
    pub const BLOWOUT: f64 = 42.0;
    pub const BLOWOUT_MARGIN: i32 = 28;
    pub const STREAK: f64 = 47.0;
    pub const STREAK_LENGTH: i32 = 4;
    /// Early in the year a 2- or 3-game run still makes the briefs (it never beats a real lead).
    pub const MINOR_STREAK_LENGTH: i32 = 2;
    pub const MINOR_STREAK: f64 = 28.0;
    /// A comfortable win that is not quite a blowout still makes the briefs.
    pub const BIG_WIN_MARGIN: i32 = 17;
    pub const BIG_WIN: f64 = 30.0;
    pub const BROKE: f64 = 53.0;
    pub const SPOTLIGHT_MIN_SCORE: f64 = 12.0;
    pub const SPOTLIGHT_COUNT: usize = 3;
    pub const ABSENCES_MIN_OVR: i32 = 48;
    pub const ABSENCES_MAX: usize = 4;
    pub const GOSSIP_MAX: usize = 4;
    pub const MIN_GAMES_FOR_UPSET: i32 = 3;
    /// Rotation: a kind that led one of the last `RECENT_LEADS` issues loses `REPEAT_PENALTY` per time, so a shutout or a
    /// broke club can take the front page after a run of upsets. Title-game and semifinal stories are never held back.
    pub const RECENT_LEADS: usize = 4;
    pub const REPEAT_PENALTY: f64 = 14.0;
}

/// What happened in one game, kept for the week's issue and then dropped.
#[derive(Clone, Debug)]
pub struct GameDigest {
    pub rec: GameRecord,
    pub rivalry: bool,
    pub home_pre_pct: f64,
    pub away_pre_pct: f64,
    pub home_pre_games: i32,
    pub away_pre_games: i32,
    /// Best few players of the game.
    pub top: Vec<Performance>,
    pub injuries: BTreeMap<u32, InjuryRecord>,
}

impl GameDigest {
    pub fn build(rec: &GameRecord, result: &GameResult, rivalry: bool, home_pre: TeamRecord, away_pre: TeamRecord) -> GameDigest {
        GameDigest {
            rec: rec.clone_light(),
            rivalry,
            home_pre_pct: home_pre.pct(),
            away_pre_pct: away_pre.pct(),
            home_pre_games: home_pre.games(),
            away_pre_games: away_pre.games(),
            top: BoxScoreBuilder::performances(result).into_iter().take(5).collect(),
            injuries: result.injuries.iter().map(|i| (i.player_id, i.clone())).collect(),
        }
    }
}

/// Picks a line from a pool, avoiding lines already used in this issue, so two stories of one kind never read alike.
struct FreshPicker {
    rng: Rng,
    used: BTreeSet<String>,
}

impl FreshPicker {
    fn new(rng: Rng) -> Self {
        Self { rng, used: BTreeSet::new() }
    }

    fn pick(&mut self, items: &[String]) -> String {
        let fresh: Vec<&String> = items.iter().filter(|t| !self.used.contains(*t)).collect();
        let t = if fresh.is_empty() { self.rng.pick(items).clone() } else { (*self.rng.pick(&fresh)).clone() };
        self.used.insert(t.clone());
        t
    }
}

struct Cand {
    score: f64,
    story: GazetteStory,
}

/// A lead's picture kind without the variant number, with overtime rivalries counted as rivalries.
pub fn gazette_kind_family(kind: &str) -> String {
    let re = regex::Regex::new(r"_\d$").unwrap();
    re.replace(kind, "").replacen("rivalry_ot", "rivalry", 1)
}

fn a_job(job: &str) -> String {
    if job.chars().next().map(|c| "aeiouAEIOU".contains(c)).unwrap_or(false) { format!("an {job}") } else { format!("a {job}") }
}

fn cap_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => format!("{}{}", f.to_uppercase(), c.as_str()),
        None => String::new(),
    }
}

fn dollars(v: i64) -> String {
    format!("${}", crate::text::commas(v))
}

fn strip_period(s: &str) -> String {
    s.strip_suffix('.').unwrap_or(s).to_string()
}

struct PInfo {
    name: String,
    pos: &'static str,
    day_job: String,
    #[allow(dead_code)]
    hometown: String,
    ovr: i32,
    team: usize,
}

type Vars = Vec<(&'static str, String)>;

impl League {
    /// Builds this week's issue from the games just played. Own Rng, so game sims are unaffected.
    pub fn write_gazette(&mut self, week_no: i32, injured_before: &BTreeMap<u32, i32>) {
        let Some(u) = self.user_team else { return };
        if self.week_digests.is_empty() {
            return;
        }
        let gt = gazette_text();
        let mut nr = FreshPicker::new(Rng::new(self.season as u64 * 6151 + week_no as u64 * 131 + 9));
        let names: Vec<String> = self.teams.iter().map(|t| t.town.team_name()).collect();
        let towns: Vec<String> = self.teams.iter().map(|t| t.town.name.clone()).collect();
        let name = |i: usize| names[i].clone();
        let town = |i: usize| towns[i].clone();
        let mut pinfo: HashMap<u32, PInfo> = HashMap::new();
        for (ti, t) in self.teams.iter().enumerate() {
            for p in &t.roster {
                pinfo.insert(p.id, PInfo { name: p.name.clone(), pos: p.position.label(), day_job: p.day_job.clone(), hometown: p.hometown.clone(), ovr: p.ovr(), team: ti });
            }
        }
        let weeks = ScheduleGenerator::WEEKS as i32;
        let label = if week_no <= weeks { format!("Week {week_no}") } else if week_no == weeks + 1 { "Semifinals".to_string() } else { "Championship".to_string() };
        let mut cands: Vec<Cand> = Vec::new();
        let mut scores: Vec<GazetteStory> = Vec::new();
        let mut spotlight_pool: Vec<(Performance, usize)> = Vec::new(); // performance, digest index

        let digests = self.week_digests.clone();
        let team_idx = |d: &GameDigest, s: Side| if s == Side::Home { d.rec.home } else { d.rec.away };

        for (di, d) in digests.iter().enumerate() {
            let rec = &d.rec;
            let tie = rec.tie();
            let w = if tie { rec.home } else { rec.winner().unwrap() };
            let l = if w == rec.home { rec.away } else { rec.home };
            let ws = if w == rec.home { rec.home_score } else { rec.away_score };
            let ls = if w == rec.home { rec.away_score } else { rec.home_score };
            let base: Vars = vec![("w", name(w)), ("l", name(l)), ("ws", ws.to_string()), ("ls", ls.to_string()), ("town", town(w))];
            let pct = |team: usize| if team == rec.home { d.home_pre_pct } else { d.away_pre_pct };
            let pre_games = |team: usize| if team == rec.home { d.home_pre_games } else { d.away_pre_games };
            let star = d.top.first();
            let star_team = star.map(|s| team_idx(d, s.side)).unwrap_or(w);
            let final_week = weeks + 2;

            // ---- scoreboard line
            let tag = if rec.playoff { if rec.week == final_week { "Championship · " } else { "Semifinal · " } } else if d.rivalry { "Rivalry · " } else { "" };
            let head = if tie {
                format!("{} {}, {} {} (tie)", name(rec.home), rec.home_score, name(rec.away), rec.away_score)
            } else {
                format!("{tag}{} {ws}, {} {ls}{}", name(w), name(l), if rec.overtime { " (OT)" } else { "" })
            };
            let score_head = if tie && !tag.is_empty() { format!("{tag}{head}") } else { head };
            let mut body = String::new();
            if let Some(star) = star {
                let pool = if tie { "scoreBodyTie" } else if star_team == w { "scoreBody" } else { "scoreBodyLoss" };
                let mut v = base.clone();
                v.extend([("p", star.player.name.clone()), ("pos", star.player.position.label().to_string()), ("pteam", name(star_team)), ("line", strip_period(&star.summary))]);
                body = fill(&nr.pick(gt.list(pool)), &v);
            }
            let mut tags: Vec<&str> = Vec::new();
            if rec.playoff {
                tags.push(if rec.week == final_week { "Championship" } else { "Semifinal" });
            } else if d.rivalry {
                tags.push("Rivalry");
            }
            if rec.overtime {
                tags.push("OT");
            }
            if tie {
                tags.push("Tie");
            }
            let mut sc = GazetteStory::new(GazetteSection::Scores, score_head, body);
            sc.team = w as i32;
            sc.player_id = star.map(|s| s.player.id as i64).unwrap_or(-1);
            sc.game = Some(GameLine { away: rec.away, away_score: rec.away_score, home: rec.home, home_score: rec.home_score, tag: tags.join(" · ") });
            scores.push(sc);

            for perf in &d.top {
                spotlight_pool.push((perf.clone(), di));
            }

            if tie {
                continue;
            }

            // ---- front-page candidates from this game
            let playoff_bonus = if rec.playoff { GazetteTuning::SEMIFINAL_BONUS } else { 0.0 };
            let game_line = GameLine { away: rec.away, away_score: rec.away_score, home: rec.home, home_score: rec.home_score, tag: String::new() };
            let story = |nr: &mut FreshPicker, kind: &str, heads: &str, bodies: &str| -> GazetteStory {
                let h = fill(&nr.pick(gt.list(heads)), &base);
                let b = fill(&nr.pick(gt.list(bodies)), &base);
                let mut s = GazetteStory::new(GazetteSection::Front, h, b);
                s.team = w as i32;
                s.kind = kind.to_string();
                s.game = Some(game_line.clone());
                s
            };

            if rec.playoff && rec.week == final_week {
                cands.push(Cand { score: GazetteTuning::CHAMPION, story: story(&mut nr, "champion", "champHead", "champBody") });
            } else if rec.playoff {
                cands.push(Cand { score: GazetteTuning::RIVALRY - 4.0 + (ws - ls).abs() as f64 * -0.3 + playoff_bonus * 0.4, story: story(&mut nr, "semifinal", "semiHead", "semiBody") });
            }
            if d.rivalry {
                let ot = rec.overtime;
                let s = if ot { story(&mut nr, "rivalry_ot", "rivalryOtHead", "rivalryOtBody") } else { story(&mut nr, "rivalry", "rivalryHead", "rivalryBody") };
                cands.push(Cand { score: GazetteTuning::RIVALRY + if ot { 6.0 } else { 0.0 } + playoff_bonus * 0.3 - (ws - ls) as f64 * 0.2, story: s });
            }
            if pre_games(w) >= GazetteTuning::MIN_GAMES_FOR_UPSET && pre_games(l) >= GazetteTuning::MIN_GAMES_FOR_UPSET && pct(l) - pct(w) >= GazetteTuning::UPSET_PCT_GAP {
                cands.push(Cand { score: GazetteTuning::UPSET + (pct(l) - pct(w)) * 30.0, story: story(&mut nr, "upset", "upsetHead", "upsetBody") });
            }
            if ls == 0 {
                cands.push(Cand { score: GazetteTuning::SHUTOUT, story: story(&mut nr, "shutout", "shutoutHead", "shutoutBody") });
            }
            if rec.overtime && !d.rivalry {
                cands.push(Cand { score: GazetteTuning::OVERTIME, story: story(&mut nr, "overtime", "otHead", "otBody") });
            }
            if ws - ls >= GazetteTuning::BLOWOUT_MARGIN {
                cands.push(Cand { score: GazetteTuning::BLOWOUT + (ws - ls - GazetteTuning::BLOWOUT_MARGIN) as f64 * 0.3, story: story(&mut nr, "blowout", "blowoutHead", "blowoutBody") });
            } else if ws - ls >= GazetteTuning::BIG_WIN_MARGIN {
                cands.push(Cand { score: GazetteTuning::BIG_WIN + (ws - ls) as f64 * 0.2, story: story(&mut nr, "blowout", "blowoutHead", "blowoutBody") });
            }
            if let Some(star) = star {
                if star.score >= GazetteTuning::STAR_MIN_SCORE {
                    let p = &star.player;
                    let ajob = a_job(&p.day_job.to_lowercase());
                    let head = fill(&nr.pick(gt.list("starHead")), &[("p", p.name.clone()), ("t", name(star_team))]);
                    let mut v = base.clone();
                    v.extend([
                        ("p", p.name.clone()),
                        ("pos", p.position.label().to_string()),
                        ("job", p.day_job.to_lowercase()),
                        ("ajob", ajob.clone()),
                        ("ajob_cap", cap_first(&ajob)),
                        ("line", strip_period(&star.summary)),
                        ("t", name(star_team)),
                    ]);
                    let body = fill(&nr.pick(gt.list("starBody")), &v);
                    let mut st = GazetteStory::new(GazetteSection::Front, head, body);
                    st.team = star_team as i32;
                    st.player_id = p.id as i64;
                    st.kind = "star".into();
                    st.game = Some(game_line.clone());
                    cands.push(Cand { score: GazetteTuning::STAR + star.score, story: st });
                }
            }
        }

        // ---- streaks
        if week_no <= weeks {
            for i in 0..self.teams.len() {
                let s = self.standings.records[i].streak;
                if s.abs() < GazetteTuning::MINOR_STREAK_LENGTH {
                    continue;
                }
                let minor = s.abs() < GazetteTuning::STREAK_LENGTH;
                let v: Vars = vec![("t", name(i)), ("n", s.abs().to_string()), ("town", town(i))];
                let hot = s > 0;
                let head = fill(&nr.pick(gt.list(if hot { "hotHead" } else { "coldHead" })), &v);
                let body = fill(&nr.pick(gt.list(if hot { "hotBody" } else { "coldBody" })), &v);
                let mut st = GazetteStory::new(GazetteSection::Front, head, body);
                st.team = i as i32;
                st.kind = if hot { "hot_streak" } else { "cold_streak" }.into();
                cands.push(Cand { score: if minor { GazetteTuning::MINOR_STREAK + s.abs() as f64 * 3.0 } else { GazetteTuning::STREAK + s.abs() as f64 * 1.5 }, story: st });
            }
        }

        // ---- clubs in money trouble (other teams only)
        let mut stage_of: BTreeMap<usize, i32> = BTreeMap::new();
        let mut new_stages: Vec<(usize, i32)> = Vec::new();
        for i in 0..self.teams.len() {
            if i == u {
                continue;
            }
            let f = &self.finances[i];
            let cash = f.cash;
            let stage = if f.bankrupt || cash < 0 { 3 } else if cash < 1000 { 2 } else if cash < 3000 { 1 } else { 0 };
            stage_of.insert(i, stage);
            if stage >= 2 && self.gazette_cash_stage.get(&i).copied().unwrap_or(0) < stage {
                let v: Vars = vec![("t", name(i)), ("n", if cash < 0 { format!("-{}", dollars(-cash)) } else { dollars(cash) }), ("town", town(i))];
                let head = fill(&nr.pick(gt.list("brokeHead")), &v);
                let body = fill(&nr.pick(gt.list("brokeBody")), &v);
                let mut st = GazetteStory::new(GazetteSection::Front, head, body);
                st.team = i as i32;
                st.kind = "broke".into();
                cands.push(Cand { score: GazetteTuning::BROKE + stage as f64, story: st });
            }
            new_stages.push((i, stage));
        }
        for (i, s) in new_stages {
            self.gazette_cash_stage.insert(i, s);
        }

        // ---- pick the front page and the "also" briefs
        let recent: Vec<String> = self.gazette.iter().rev().take(GazetteTuning::RECENT_LEADS).filter_map(|i| i.lead().filter(|l| !l.kind.is_empty()).map(|l| gazette_kind_family(&l.kind))).collect();
        let rotated = |c: &Cand| {
            let kind = gazette_kind_family(&c.story.kind);
            if kind == "champion" || kind == "semifinal" {
                return c.score;
            }
            c.score - GazetteTuning::REPEAT_PENALTY * recent.iter().filter(|k| **k == kind).count() as f64
        };
        cands.sort_by(|a, b| rotated(b).total_cmp(&rotated(a)));
        let mut stories: Vec<GazetteStory> = Vec::new();
        let mut used_players: BTreeSet<i64> = BTreeSet::new();
        if cands.is_empty() {
            let mut q = GazetteStory::new(GazetteSection::Front, nr.pick(gt.list("quietHead")), nr.pick(gt.list("quietBody")));
            q.kind = "quiet".into();
            stories.push(q);
        } else {
            let lead = cands[0].story.clone();
            if lead.player_id >= 0 {
                used_players.insert(lead.player_id);
            }
            let mut seen_heads: BTreeSet<String> = BTreeSet::from([lead.headline.clone()]);
            stories.push(lead.clone());
            for c in cands.iter().skip(1) {
                if stories.len() >= 3 {
                    break;
                }
                if c.story.kind == "star" {
                    continue; // player stories live in the Player spotlight section
                }
                if c.story.team == lead.team && c.story.player_id == lead.player_id {
                    continue;
                }
                if !seen_heads.insert(c.story.headline.clone()) {
                    continue;
                }
                stories.push(c.story.as_brief());
            }
        }
        stories.extend(scores);

        // ---- player spotlight
        spotlight_pool.sort_by(|a, b| b.0.score.total_cmp(&a.0.score));
        let mut shown = 0;
        for (perf, di) in &spotlight_pool {
            if shown >= GazetteTuning::SPOTLIGHT_COUNT || perf.score < GazetteTuning::SPOTLIGHT_MIN_SCORE {
                break;
            }
            if used_players.contains(&(perf.player.id as i64)) {
                continue;
            }
            let p = &perf.player;
            let t = team_idx(&digests[*di], perf.side);
            let ajob = a_job(&p.day_job.to_lowercase());
            let v: Vars = vec![
                ("p", p.name.clone()),
                ("pos", p.position.label().to_string()),
                ("t", name(t)),
                ("job", p.day_job.to_lowercase()),
                ("ajob", ajob.clone()),
                ("ajob_cap", cap_first(&ajob)),
                ("home", p.hometown.clone()),
                ("line", strip_period(&perf.summary)),
            ];
            let mut st = GazetteStory::new(GazetteSection::Spotlight, fill(&nr.pick(gt.list("spotHead")), &v), fill(&nr.pick(gt.list("spotBody")), &v));
            st.team = t as i32;
            st.player_id = p.id as i64;
            stories.push(st);
            used_players.insert(p.id as i64);
            shown += 1;
        }

        // ---- who is missing, and why (other teams only)
        let mut digest_injury: BTreeMap<u32, (InjuryRecord, usize)> = BTreeMap::new();
        for (di, d) in digests.iter().enumerate() {
            for (id, inj) in &d.injuries {
                digest_injury.insert(*id, (inj.clone(), di));
            }
        }
        let mut missing: Vec<(u32, usize, i32)> = Vec::new(); // player, team, games out
        for (id, games) in &self.injured_games {
            if injured_before.contains_key(id) {
                continue;
            }
            let Some(pi) = pinfo.get(id) else { continue };
            if self.out_reason.get(id).map(|r| r.starts_with("Suspended")).unwrap_or(false) {
                continue; // told in the Gossip column
            }
            if pi.team == u || pi.ovr < GazetteTuning::ABSENCES_MIN_OVR {
                continue;
            }
            missing.push((*id, pi.team, *games));
        }
        missing.sort_by_key(|(id, _, _)| std::cmp::Reverse(pinfo[id].ovr));
        for (id, t, g) in missing.into_iter().take(GazetteTuning::ABSENCES_MAX) {
            let pi = &pinfo[&id];
            let reason = self.out_reason.get(&id).cloned();
            let inj = digest_injury.get(&id);
            let opp = inj.map(|(_, di)| { let r = &digests[*di].rec; name(if r.home == t { r.away } else { r.home }) }).unwrap_or_default();
            let ajob = a_job(&pi.day_job.to_lowercase());
            let v: Vars = vec![
                ("p", pi.name.clone()),
                ("pos", pi.pos.to_string()),
                ("t", name(t)),
                ("town", town(t)),
                ("g", g.to_string()),
                ("job", pi.day_job.to_lowercase()),
                ("ajob", ajob.clone()),
                ("ajob_cap", cap_first(&ajob)),
                ("l", if opp.is_empty() { "last".to_string() } else { opp }),
            ];
            let pool = match &reason {
                None => {
                    if inj.map(|(i, _)| i.severity == "seasonEnding").unwrap_or(false) {
                        "gameInjurySeason"
                    } else if g <= 1 {
                        "gameInjuryShort"
                    } else {
                        "gameInjuryLong"
                    }
                }
                Some(r) if r.starts_with("Stuck") => "workTrip",
                Some(_) => "workInjury",
            };
            let mut st = GazetteStory::new(GazetteSection::Absences, format!("{} {}, {}", pi.pos, pi.name, name(t)), fill(&nr.pick(gt.list(pool)), &v));
            st.team = t as i32;
            st.player_id = id as i64;
            stories.push(st);
        }

        // ---- business page
        let mut tight: Vec<(usize, i32)> = stage_of.iter().filter(|(_, s)| **s >= 1).map(|(i, s)| (*i, *s)).collect();
        tight.sort_by_key(|(i, _)| self.finances[*i].cash);
        // One story per club so the page can show each helmet.
        for (i, stage) in tight.into_iter().take(4) {
            let cash = self.finances[i].cash;
            let note = if stage >= 3 { nr.pick(gt.list("moneyDoomed")) } else { nr.pick(gt.list("moneyNotes")) };
            let line = fill(&gt.list("moneyLine")[0], &[("t", name(i)), ("n", if cash < 0 { format!("-{}", dollars(-cash)) } else { dollars(cash) }), ("note", note)]);
            let tail = line.find(": ").map(|k| line[k + 2..].to_string()).unwrap_or(line.clone());
            let mut st = GazetteStory::new(GazetteSection::Money, format!("{} on thin ice", name(i)), tail);
            st.team = i as i32;
            stories.push(st);
        }

        // ---- gossip: off-field trouble and who did or did not pay the lawyers
        let mut gossip_rng = Rng::new(self.season as u64 * 4513 + week_no as u64 * 19 + 3);
        let mut gossip = self.conduct_gossip.clone();
        Rng::new(self.season as u64 * 31 + week_no as u64).shuffle(&mut gossip);
        for gs in gossip.iter().take(GazetteTuning::GOSSIP_MAX) {
            let Some(pi) = pinfo.get(&gs.player_id) else { continue };
            let v: Vars = vec![
                ("p", pi.name.clone()),
                ("pos", pi.pos.to_string()),
                ("t", name(gs.team)),
                ("town", town(gs.team)),
                ("amt", dollars(gs.cost)),
                ("gm", if gs.games == 1 { "1 game".to_string() } else { format!("{} games", gs.games) }),
                ("title", gs.title.clone()),
            ];
            let (heads, tails) = if gs.paid { ("gossipPaidHead", "gossipPaidTail") } else { ("gossipUnpaidHead", "gossipUnpaidTail") };
            let head = fill(gossip_rng.pick(gt.list(heads)), &v);
            let tail = fill(gossip_rng.pick(gt.list(tails)), &v);
            let mut st = GazetteStory::new(GazetteSection::Gossip, head, format!("{} {}", gs.body, tail));
            st.team = gs.team as i32;
            st.player_id = gs.player_id as i64;
            stories.push(st);
        }
        self.conduct_gossip.clear();

        // ---- standings and leaders (regular season)
        if week_no >= 2 && week_no <= weeks {
            let mut lines = Vec::new();
            for d in 0..3 {
                let order = self.standings.division_order(d);
                let Some(&first) = order.first() else { continue };
                let dn = self.region.divisions.get(d).cloned().unwrap_or_else(|| format!("Division {}", d + 1));
                lines.push(format!("{dn}: {} {}", name(first), self.standings.records[first]));
            }
            if !lines.is_empty() {
                stories.push(GazetteStory::new(GazetteSection::Standings, "Division leaders".into(), lines.join("\n")));
            }
            let mut leaders = Vec::new();
            let mut leader = |label: &str, key: &str, decimal: bool| {
                let mut rows: Vec<(u32, f64)> = self.season_stats.iter().filter(|(id, _)| pinfo.contains_key(id)).map(|(id, st)| (*id, st.get(key))).filter(|r| r.1 > 0.0).collect();
                rows.sort_by(|a, b| b.1.total_cmp(&a.1));
                let Some(top) = rows.first() else { return };
                let pi = &pinfo[&top.0];
                leaders.push(format!("{label}: {} ({}) {}", pi.name, name(pi.team), if decimal { format!("{:.1}", top.1) } else { (top.1.round() as i64).to_string() }));
            };
            leader("Passing yards", "pass_yds", false);
            leader("Rushing yards", "rush_yds", false);
            leader("Receiving yards", "rec_yds", false);
            leader("Sacks", "sacks", true);
            if !leaders.is_empty() {
                stories.push(GazetteStory::new(GazetteSection::Standings, "League leaders".into(), leaders.join("\n")));
            }
        }

        // The lead and the "also in the news" briefs get a picture of their story type, picked at random (each brief a
        // different variant from the lead). Own Rng, so the text above is unchanged.
        let variant = gazette_variant_for(self.season, week_no);
        let mut brief_no = 0;
        for st in stories.iter_mut() {
            let lead = st.section == GazetteSection::Front;
            let brief = st.section == GazetteSection::Briefs;
            let v = if brief {
                brief_no += 1;
                (variant + brief_no - 1) % GAZETTE_ART_VARIANTS + 1
            } else {
                variant
            };
            st.kind = if (lead || brief) && !st.kind.is_empty() { gazette_art_file(&st.kind, v) } else { String::new() };
        }

        // Old saves have no numbers: count back from the newest issue's position so the numbering carries on.
        let prev = match self.gazette.last() {
            None => 0,
            Some(l) => if l.number > 0 { l.number } else { self.gazette.len() as i32 },
        };
        self.gazette.push(GazetteIssue { season: self.season, week: week_no, label, stories, number: prev + 1 });
        if self.gazette.len() > GazetteTuning::MAX_ISSUES {
            let cut = self.gazette.len() - GazetteTuning::MAX_ISSUES;
            self.gazette.drain(0..cut);
        }
    }
}
