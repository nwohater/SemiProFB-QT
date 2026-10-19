use crate::game_sim::GameResult;
use crate::game_state::{Side, Sides};
use crate::play_event::{DriveSummary, KeyPlay, ScoringPlay};
use crate::stats::PlayerStats;
use serde::{Deserialize, Serialize};
use sp_core::{Attr, Player, Position, Weather};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerLine {
    pub id: u32,
    pub name: String,
    pub pos: Position,
    pub raw: BTreeMap<String, f64>,
    /// Column label -> formatted value, in column order.
    #[serde(rename = "stats")]
    pub display: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamStatLine {
    pub label: String,
    pub home: String,
    pub away: String,
    /// 0..1 for comparison bars.
    pub home_share: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerOfGame {
    pub side: Side,
    pub player: PlayerLine,
    pub summary: String,
}

/// One player's game: a score for ranking and a plain-language stat line.
#[derive(Clone, Debug)]
pub struct Performance {
    pub side: Side,
    pub player: Player,
    pub score: f64,
    pub summary: String,
    pub stats: BTreeMap<String, f64>,
}

/// Player tables by category, each sorted best first.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerTables {
    pub passing: Vec<PlayerLine>,
    pub rushing: Vec<PlayerLine>,
    pub receiving: Vec<PlayerLine>,
    pub offensive_line: Vec<PlayerLine>,
    pub defense: Vec<PlayerLine>,
    pub kicking: Vec<PlayerLine>,
    pub punting: Vec<PlayerLine>,
    pub returns: Vec<PlayerLine>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoxScore {
    pub home_name: String,
    pub away_name: String,
    /// Each side's regular-season record after this game ("4-1"); `None` for playoff games.
    pub home_record: Option<String>,
    pub away_record: Option<String>,
    pub home_score: i32,
    pub away_score: i32,
    pub quarters: Sides<Vec<i32>>,
    pub overtime: bool,
    pub tie: bool,
    pub weather: Weather,
    pub team_stats: Vec<TeamStatLine>,
    pub player_tables: Sides<PlayerTables>,
    pub scoring: Vec<ScoringPlay>,
    pub drives: Vec<DriveSummary>,
    pub key_plays: Vec<KeyPlay>,
    pub player_of_game: Option<PlayerOfGame>,
    pub injury_notes: Vec<String>,
    pub fatigue_notes: Vec<String>,
    pub why: Sides<Vec<String>>,
    /// The same causes written in the third person, naming the teams, for games the owner is not in.
    pub why_named: Sides<Vec<String>>,
    /// Short team names for headings.
    pub home_nick: String,
    pub away_nick: String,
}

impl BoxScore {
    pub fn winner(&self) -> Option<Side> {
        if self.tie {
            None
        } else if self.home_score > self.away_score {
            Some(Side::Home)
        } else {
            Some(Side::Away)
        }
    }
}

fn mmss(secs: i32) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}

fn rating(c: f64, a: f64, yds: f64, td: f64, int: f64) -> f64 {
    if a == 0.0 {
        return 0.0;
    }
    let cl = |x: f64| x.clamp(0.0, 2.375);
    let x1 = cl((c / a - 0.3) * 5.0);
    let x2 = cl((yds / a - 3.0) * 0.25);
    let x3 = cl(td / a * 20.0);
    let x4 = cl(2.375 - int / a * 25.0);
    (x1 + x2 + x3 + x4) / 6.0 * 100.0
}

fn f0(v: f64) -> String {
    format!("{}", v.round() as i64)
}

/// Whole numbers print without a decimal; halves (shared sacks) print as "0.5".
fn whole_or_plain(v: f64) -> String {
    if v == v.round() { f0(v) } else { format!("{v}") }
}

pub struct BoxScoreBuilder;

impl BoxScoreBuilder {
    pub fn build(r: &GameResult) -> BoxScore {
        let tables = Sides::new(Self::tables_for(r, Side::Home), Self::tables_for(r, Side::Away));
        let player_of_game = Self::player_of_game(r);
        BoxScore {
            home_name: r.team_names.home.clone(),
            away_name: r.team_names.away.clone(),
            home_record: None,
            away_record: None,
            home_score: r.home_score,
            away_score: r.away_score,
            quarters: r.score_by_quarter.clone(),
            overtime: r.overtime,
            tie: r.tie,
            weather: r.weather,
            team_stats: Self::team_lines(r),
            player_tables: tables,
            scoring: r.scoring_plays.clone(),
            drives: r.drives.clone(),
            key_plays: r.key_plays.clone(),
            player_of_game,
            injury_notes: Self::injury_notes(r),
            fatigue_notes: Self::fatigue_notes(r),
            why: Sides::new(Self::why(r, Side::Home, false), Self::why(r, Side::Away, false)),
            why_named: Sides::new(Self::why(r, Side::Home, true), Self::why(r, Side::Away, true)),
            home_nick: r.team_nicks.home.clone(),
            away_nick: r.team_nicks.away.clone(),
        }
    }

    // ----------------------------------------------------------- team lines

    fn team_lines(r: &GameResult) -> Vec<TeamStatLine> {
        let (h, a) = (&r.team_stats.home, &r.team_stats.away);
        let num = |label: &str, x: i32, y: i32, lower_better: bool| {
            let tot = x + y;
            let mut share = if tot == 0 { 0.5 } else { x as f64 / tot as f64 };
            if lower_better {
                share = 1.0 - share;
            }
            TeamStatLine { label: label.into(), home: x.to_string(), away: y.to_string(), home_share: share }
        };
        let share = |x: i32, y: i32| if x + y == 0 { 0.5 } else { x as f64 / (x + y) as f64 };
        vec![
            num("Total yards", h.total_yards(), a.total_yards(), false),
            num("Rush yards", h.rush_yards, a.rush_yards, false),
            num("Pass yards", h.pass_yards, a.pass_yards, false),
            num("First downs", h.first_downs, a.first_downs, false),
            TeamStatLine {
                label: "Third down".into(),
                home: format!("{}/{}", h.third_conv, h.third_att),
                away: format!("{}/{}", a.third_conv, a.third_att),
                home_share: share(h.third_conv, a.third_conv),
            },
            num("Turnovers", h.turnovers, a.turnovers, true),
            num("Sacks made", h.sacks_made, a.sacks_made, false),
            TeamStatLine {
                label: "Penalties".into(),
                home: format!("{}-{}", h.penalties, h.penalty_yards),
                away: format!("{}-{}", a.penalties, a.penalty_yards),
                home_share: if h.penalty_yards + a.penalty_yards == 0 { 0.5 } else { 1.0 - h.penalty_yards as f64 / (h.penalty_yards + a.penalty_yards) as f64 },
            },
            TeamStatLine {
                label: "Time of poss.".into(),
                home: mmss(h.seconds_of_possession),
                away: mmss(a.seconds_of_possession),
                home_share: share(h.seconds_of_possession, a.seconds_of_possession),
            },
        ]
    }

    // -------------------------------------------------------- player tables

    fn tables_for(r: &GameResult, side: Side) -> PlayerTables {
        let players = &r.players[side];
        let stats = &r.player_stats[side];
        let mut out = PlayerTables::default();
        let line = |p: &Player, st: &PlayerStats, disp: Vec<(&str, String)>| PlayerLine {
            id: p.id,
            name: p.name.clone(),
            pos: p.position,
            raw: st.to_map(),
            display: disp.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
        };

        for (id, st) in stats {
            let Some(p) = players.get(id) else { continue };
            let g = |k: &str| st.get(k);

            if g("pass_att") > 0.0 || g("sacks_taken") > 0.0 {
                out.passing.push(line(
                    p,
                    st,
                    vec![
                        ("C/A", format!("{}/{}", f0(g("pass_cmp")), f0(g("pass_att")))),
                        ("Yds", f0(g("pass_yds"))),
                        ("TD", f0(g("pass_td"))),
                        ("INT", f0(g("pass_int"))),
                        ("Sck", format!("{}-{}", f0(g("sacks_taken")), f0(g("sack_yds")))),
                        ("Rtg", format!("{:.1}", rating(g("pass_cmp"), g("pass_att"), g("pass_yds"), g("pass_td"), g("pass_int")))),
                    ],
                ));
            }
            if g("rush_att") > 0.0 {
                out.rushing.push(line(
                    p,
                    st,
                    vec![
                        ("Att", f0(g("rush_att"))),
                        ("Yds", f0(g("rush_yds"))),
                        ("Avg", format!("{:.1}", g("rush_yds") / g("rush_att"))),
                        ("TD", f0(g("rush_td"))),
                        ("Lng", f0(g("rush_long"))),
                        ("Fum", f0(g("fumbles"))),
                    ],
                ));
            }
            if g("targets") > 0.0 {
                out.receiving.push(line(
                    p,
                    st,
                    vec![
                        ("Tgt", f0(g("targets"))),
                        ("Rec", f0(g("rec"))),
                        ("Yds", f0(g("rec_yds"))),
                        ("TD", f0(g("rec_td"))),
                        ("Lng", f0(g("rec_long"))),
                        ("Drp", f0(g("drops"))),
                        ("YAC", f0(g("yac"))),
                    ],
                ));
            }
            if matches!(p.position, Position::Ot | Position::Og | Position::C) && g("snaps_off") > 0.0 {
                out.offensive_line.push(line(
                    p,
                    st,
                    vec![
                        ("Snaps", f0(g("snaps_off"))),
                        ("SckAllowed", f0(g("sacks_allowed"))),
                        ("PressAllowed", f0(g("pressures_allowed"))),
                        ("Pancakes", f0(g("pancakes"))),
                        ("Pen", f0(g("penalties"))),
                    ],
                ));
            }
            let tackles = g("tackles_solo") + g("tackles_ast");
            if tackles > 0.0 || g("sacks") > 0.0 || g("pd") > 0.0 || g("int") > 0.0 || g("ff") > 0.0 || g("fr") > 0.0 || g("pressures") > 0.0 {
                out.defense.push(line(
                    p,
                    st,
                    vec![
                        ("Tkl", f0(tackles)),
                        ("Solo", f0(g("tackles_solo"))),
                        ("TFL", whole_or_plain(g("tfl"))),
                        ("Sck", whole_or_plain(g("sacks"))),
                        ("QBH", f0(g("qb_hits"))),
                        ("Prs", f0(g("pressures"))),
                        ("PD", f0(g("pd"))),
                        ("INT", f0(g("int"))),
                        ("FF", f0(g("ff"))),
                        ("FR", f0(g("fr"))),
                    ],
                ));
            }
            if g("fga") > 0.0 || g("xpa") > 0.0 {
                out.kicking.push(line(
                    p,
                    st,
                    vec![
                        ("FG", format!("{}/{}", f0(g("fgm")), f0(g("fga")))),
                        ("Lng", f0(g("fg_long"))),
                        ("XP", format!("{}/{}", f0(g("xpm")), f0(g("xpa")))),
                        ("Pts", f0(g("fgm") * 3.0 + g("xpm"))),
                    ],
                ));
            }
            if g("punts") > 0.0 {
                out.punting.push(line(
                    p,
                    st,
                    vec![
                        ("Punts", f0(g("punts"))),
                        ("Yds", f0(g("punt_yds"))),
                        ("Avg", format!("{:.1}", g("punt_yds") / g("punts"))),
                        ("In20", f0(g("punt_in20"))),
                        ("TB", f0(g("punt_tb"))),
                    ],
                ));
            }
            if g("kr") > 0.0 || g("pr") > 0.0 {
                out.returns.push(line(
                    p,
                    st,
                    vec![
                        ("KR", f0(g("kr"))),
                        ("KRYds", f0(g("kr_yds"))),
                        ("KRTD", f0(g("kr_td"))),
                        ("PR", f0(g("pr"))),
                        ("PRYds", f0(g("pr_yds"))),
                        ("PRTD", f0(g("pr_td"))),
                    ],
                ));
            }
        }
        let key = |l: &PlayerLine, k: &str| l.raw.get(k).copied().unwrap_or(0.0).round() as i64;
        out.passing.sort_by_key(|l| std::cmp::Reverse(key(l, "pass_yds")));
        out.rushing.sort_by_key(|l| std::cmp::Reverse(key(l, "rush_yds")));
        out.receiving.sort_by_key(|l| std::cmp::Reverse(key(l, "rec_yds")));
        out.offensive_line.sort_by_key(|l| std::cmp::Reverse(key(l, "snaps_off")));
        let tk = |l: &PlayerLine| {
            let g = |k: &str| l.raw.get(k).copied().unwrap_or(0.0);
            g("tackles_solo") + g("tackles_ast") * 0.5 + g("sacks") * 3.0 + g("int") * 3.0
        };
        out.defense.sort_by(|a, b| tk(b).total_cmp(&tk(a)));
        out
    }

    // ------------------------------------------------------ player of game

    fn player_of_game(r: &GameResult) -> Option<PlayerOfGame> {
        let mut best: Option<Performance> = None;
        let mut best_score = -1.0;
        for perf in Self::performances(r) {
            // winning team bonus keeps the award with the winner when close
            let s = perf.score + if r.winner() == Some(perf.side) { 1.5 } else { 0.0 };
            if s > best_score {
                best_score = s;
                best = Some(perf);
            }
        }
        let best = best?;
        let line = PlayerLine { id: best.player.id, name: best.player.name.clone(), pos: best.player.position, raw: best.stats.clone(), display: vec![] };
        Some(PlayerOfGame { side: best.side, player: line, summary: best.summary })
    }

    /// Every player's game scored and described, best first. Used for the player of the game and for league news.
    pub fn performances(r: &GameResult) -> Vec<Performance> {
        let mut out = Vec::new();
        for side in Side::ALL {
            let players = &r.players[side];
            for (id, st) in &r.player_stats[side] {
                let Some(p) = players.get(id) else { continue };
                let g = |k: &str| st.get(k);
                let score = g("pass_yds") / 25.0 + g("pass_td") * 4.0 - g("pass_int") * 3.0 + g("rush_yds") / 10.0 + g("rush_td") * 4.0 + g("rec_yds") / 10.0 + g("rec_td") * 4.0
                    + g("sacks") * 3.0
                    + g("int") * 4.0
                    + (g("tackles_solo") + g("tackles_ast") * 0.5) * 0.5
                    + g("tfl")
                    + g("pd") * 0.7
                    + g("ff") * 2.0
                    + g("fgm") * 1.5
                    - g("fumbles_lost") * 3.0
                    + g("kr_td") * 6.0
                    + g("pr_td") * 6.0;
                let mut parts: Vec<String> = Vec::new();
                let td = |k: &str, label: &str| if g(k) > 0.0 { format!(", {} {label}", f0(g(k))) } else { String::new() };
                if g("pass_att") > 0.0 {
                    parts.push(format!("{}/{}, {} yds{}{}", f0(g("pass_cmp")), f0(g("pass_att")), f0(g("pass_yds")), td("pass_td", "TD"), td("pass_int", "INT")));
                }
                if g("rush_att") > 0.0 && p.position != Position::Qb {
                    let n = g("rush_att").round() as i64;
                    parts.push(format!("{} {}, {} yds{}", n, if n == 1 { "carry" } else { "carries" }, f0(g("rush_yds")), td("rush_td", "TD")));
                }
                if g("rec") > 0.0 {
                    let n = g("rec").round() as i64;
                    parts.push(format!("{} {}, {} yds{}", n, if n == 1 { "catch" } else { "catches" }, f0(g("rec_yds")), td("rec_td", "TD")));
                }
                if g("sacks") > 0.0 {
                    parts.push(format!("{} {}", whole_or_plain(g("sacks")), if g("sacks") == 1.0 { "sack" } else { "sacks" }));
                }
                if g("int") > 0.0 {
                    parts.push(format!("{} INT", f0(g("int"))));
                }
                let tk = g("tackles_solo") + g("tackles_ast");
                if tk >= 6.0 {
                    parts.push(format!("{} tackles", f0(tk)));
                }
                if g("fgm") > 0.0 {
                    parts.push(format!("{}/{} FG", f0(g("fgm")), f0(g("fga"))));
                }
                if g("kr_td") > 0.0 {
                    parts.push("kickoff return TD".into());
                }
                if g("pr_td") > 0.0 {
                    parts.push("punt return TD".into());
                }
                let summary = if parts.is_empty() { "Showed up.".to_string() } else { parts.join(", ") };
                out.push(Performance { side, player: p.clone(), score, summary, stats: st.to_map() });
            }
        }
        out.sort_by(|a, b| b.score.total_cmp(&a.score));
        out
    }

    // ----------------------------------------------------------- notes

    fn injury_notes(r: &GameResult) -> Vec<String> {
        r.injuries
            .iter()
            .map(|i| {
                let pos = r.players[i.side].get(&i.player_id).map(|p| p.position.label()).unwrap_or("?");
                let what = match i.severity.as_str() {
                    "shaken" => "shaken up, back next game".to_string(),
                    "minor" => "minor injury, out 1 game".to_string(),
                    "multiWeek" => format!("out {} games", i.games_out),
                    _ => "season-ending injury".to_string(),
                };
                format!("{} ({}, {}): {}", i.name, pos, r.team_names[i.side], what)
            })
            .collect()
    }

    fn fatigue_notes(r: &GameResult) -> Vec<String> {
        let mut out = Vec::new();
        for side in Side::ALL {
            let mut worst: Vec<(&u32, &f64)> = r.end_fatigue[side].iter().filter(|(_, v)| **v > 0.6).collect();
            worst.sort_by(|a, b| b.1.total_cmp(a.1));
            for (id, _) in worst.into_iter().take(2) {
                let p = &r.players[side][id];
                out.push(format!("{} ({}): worn down by the end (EN {}).", p.name, p.position.label(), p.ratings[Attr::Endurance]));
            }
        }
        out
    }

    // ------------------------------------------------------------ why

    /// Up to three causes, from `side`'s perspective: why they won, lost, or tied. Written as "we" and "they" for the owner's
    /// own game, or naming both teams when `named` (someone else's game).
    fn why(r: &GameResult, side: Side, named: bool) -> Vec<String> {
        let us = r.team_nicks[side].as_str();
        let them = r.team_nicks[side.other()].as_str();
        let poss = |n: &str| if n.ends_with('s') { format!("{n}'") } else { format!("{n}'s") };
        let subj = if named { us.to_string() } else { "We".to_string() }; // sentence subject
        let our_poss = if named { poss(us) } else { "Our".to_string() };
        let them_poss = if named { poss(them) } else { "Their".to_string() }; // start of a sentence
        let them_poss_low = if named { poss(them) } else { "their".to_string() };
        let them_subj = if named { them.to_string() } else { "They".to_string() };
        let us_obj = if named { us.to_string() } else { "us".to_string() };
        let me = &r.team_stats[side];
        let op = &r.team_stats[side.other()];
        let my_stats = &r.player_stats[side];
        let my_players = &r.players[side];
        let won = r.winner() == Some(side);
        let lost = r.winner() == Some(side.other());
        let plural = |n: i32| if n == 1 { "" } else { "s" };

        let top_name = |k: &str, pos: Option<&[Position]>| -> Option<String> {
            let mut best: Option<u32> = None;
            let mut bv = 0.0;
            for (id, st) in my_stats {
                let Some(p) = my_players.get(id) else { continue };
                if pos.is_some_and(|ps| !ps.contains(&p.position)) {
                    continue;
                }
                let v = st.get(k);
                if v > bv {
                    bv = v;
                    best = Some(*id);
                }
            }
            best.map(|id| my_players[&id].name.clone())
        };

        // (signed impact for `side`, text if favorable, text if unfavorable)
        let mut factors: Vec<(f64, String, String)> = Vec::new();

        let tov = op.turnovers - me.turnovers;
        if tov != 0 {
            factors.push((
                tov as f64 * 4.0,
                format!("{subj} won the turnover battle: {} giveaway{} to {them_poss_low} {}.", me.turnovers, plural(me.turnovers), op.turnovers),
                format!("Turnovers hurt{}: {} giveaway{} to {them_poss_low} {}.", if named { format!(" {us}") } else { String::new() }, me.turnovers, plural(me.turnovers), op.turnovers),
            ));
        }
        let rush_diff = me.rush_yards - op.rush_yards;
        let rusher = top_name("rush_yds", None);
        factors.push((
            rush_diff as f64 / 22.0,
            format!("The ground game ran {} yards to {them_poss_low} {}{}.", me.rush_yards, op.rush_yards, rusher.as_ref().map(|n| format!(". {n} did most of it")).unwrap_or_default()),
            format!("{them_subj} out-rushed {us_obj} {} to {}.", op.rush_yards, me.rush_yards),
        ));
        let pass_diff = me.pass_yards - op.pass_yards;
        let passer = top_name("pass_yds", None);
        factors.push((
            pass_diff as f64 / 28.0,
            format!(
                "{our_poss} passing game out-gained {} {} to {}{}.",
                if named { poss(them) } else { "theirs".to_string() },
                me.pass_yards,
                op.pass_yards,
                passer.as_ref().map(|n| format!(", led by {n}")).unwrap_or_default()
            ),
            format!("{them_poss} passing game out-gained {} {} to {}.", if named { poss(us) } else { "ours".to_string() }, op.pass_yards, me.pass_yards),
        ));
        let sack_diff = op.sacks_taken - me.sacks_taken;
        if sack_diff != 0 || me.sacks_taken > 0 || op.sacks_taken > 0 {
            let our_rusher = top_name("sacks", None);
            let leaky = top_name("sacks_allowed", Some(&[Position::Ot, Position::Og, Position::C]));
            factors.push((
                sack_diff as f64 * 1.8,
                format!(
                    "{subj} got to the QB {} time{}{} and allowed {}.",
                    op.sacks_taken,
                    plural(op.sacks_taken),
                    our_rusher.as_ref().map(|n| format!(", {n} leading the way")).unwrap_or_default(),
                    me.sacks_taken
                ),
                format!("{our_poss} line allowed {} sack{}{}.", me.sacks_taken, plural(me.sacks_taken), leaky.as_ref().map(|n| format!(", most on {n}")).unwrap_or_default()),
            ));
        }
        let pen_diff = op.penalty_yards - me.penalty_yards;
        factors.push((
            pen_diff as f64 / 18.0,
            format!("Cleaner football: {} penalties for {} yards vs {them_poss_low} {} for {}.", me.penalties, me.penalty_yards, op.penalties, op.penalty_yards),
            format!("Flags piled up: {} penalties for {} yards vs {them_poss_low} {}.", me.penalties, me.penalty_yards, op.penalties),
        ));
        let third_me = if me.third_att == 0 { 0.0 } else { me.third_conv as f64 / me.third_att as f64 };
        let third_op = if op.third_att == 0 { 0.0 } else { op.third_conv as f64 / op.third_att as f64 };
        factors.push((
            (third_me - third_op) * 6.0,
            format!("{subj} kept drives alive on third down ({}/{} vs {}/{}).", me.third_conv, me.third_att, op.third_conv, op.third_att),
            format!("{subj} could not get off the field or move the chains on third down ({}/{} vs {}/{}).", me.third_conv, me.third_att, op.third_conv, op.third_att),
        ));
        let my_miss = me.fg_att - me.fg_made;
        let op_miss = op.fg_att - op.fg_made;
        let fg_miss = my_miss - op_miss;
        if my_miss > 0 || op_miss > 0 {
            factors.push((
                -(fg_miss as f64) * 1.5,
                format!("{them_poss} kicker left points on the field."),
                format!("{subj} left points on the field: {my_miss} missed field goal{}.", plural(my_miss)),
            ));
        }
        for inj in r.injuries.iter().filter(|i| i.side == side) {
            if let Some(p) = my_players.get(&inj.player_id) {
                if p.position == Position::Qb {
                    factors.push((-3.0, String::new(), format!("Starting QB {} went down injured.", p.name)));
                }
            }
        }

        if r.tie {
            return vec!["Dead even: neither side could separate.".to_string()];
        }
        // Winner: biggest positives. Loser: biggest negatives.
        let mut picks: Vec<&(f64, String, String)> = factors.iter().filter(|f| if won { f.0 > 0.4 } else { f.0 < -0.4 }).collect();
        if won {
            picks.sort_by(|a, b| b.0.total_cmp(&a.0));
        } else {
            picks.sort_by(|a, b| a.0.total_cmp(&b.0));
        }
        let texts: Vec<String> = picks.iter().take(3).map(|f| if won { f.1.clone() } else { f.2.clone() }).filter(|t| !t.is_empty()).collect();
        if texts.is_empty() {
            return vec![if lost {
                "No single cause. A close game that bounced the other way.".to_string()
            } else {
                format!("A close game that bounced {} way.", if named { poss(us) } else { "our".to_string() })
            }];
        }
        texts
    }
}
