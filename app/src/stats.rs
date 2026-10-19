//! Stat tables shared by League leaders, Team stats and the box score: which players qualify, which columns show, and how each
//! column is worked out. Ported from the Dart `stat_defs.dart`. The view-model carries both display text and a sort value.

use serde_json::{Value, json};
use sp_core::Position;
use std::collections::BTreeMap;

pub type Stats = BTreeMap<String, f64>;

fn g(m: &Stats, k: &str) -> f64 {
    m.get(k).copied().unwrap_or(0.0)
}
fn div(a: f64, b: f64) -> f64 {
    if b == 0.0 { 0.0 } else { a / b }
}
fn n(v: f64) -> String {
    format!("{}", v.round() as i64)
}
fn d1(v: f64) -> String {
    format!("{v:.1}")
}
fn pct(v: f64) -> String {
    format!("{:.1}", v * 100.0)
}

pub struct Col {
    pub header: &'static str,
    pub width: u32,
    text: Box<dyn Fn(&Stats) -> String>,
    value: Box<dyn Fn(&Stats) -> f64>,
}

fn col(header: &'static str, width: u32, text: impl Fn(&Stats) -> String + 'static, value: impl Fn(&Stats) -> f64 + 'static) -> Col {
    Col { header, width, text: Box::new(text), value: Box::new(value) }
}
fn count(header: &'static str, key: &'static str, width: u32) -> Col {
    col(header, width, move |m| n(g(m, key)), move |m| g(m, key))
}
fn ratio(header: &'static str, a: &'static str, b: &'static str, width: u32, fmt: fn(f64) -> String) -> Col {
    col(header, width, move |m| fmt(div(g(m, a), g(m, b))), move |m| div(g(m, a), g(m, b)))
}
fn games() -> Col {
    count("G", "gp", 40)
}

pub struct Category {
    pub name: &'static str,
    qualifies: Box<dyn Fn(&Stats, Position) -> bool>,
    pub cols: Vec<Col>,
    pub sort_col: usize,
}

impl Category {
    pub fn qualifies(&self, m: &Stats, p: Position) -> bool {
        (self.qualifies)(m, p)
    }
}

fn cat(name: &'static str, qualifies: impl Fn(&Stats, Position) -> bool + 'static, cols: Vec<Col>, sort_col: usize) -> Category {
    Category { name, qualifies: Box::new(qualifies), cols, sort_col }
}

/// NFL passer rating (0 to 158.3).
pub fn passer_rating(m: &Stats) -> f64 {
    let att = g(m, "pass_att");
    if att == 0.0 {
        return 0.0;
    }
    let c = |x: f64| x.clamp(0.0, 2.375);
    let a = c((g(m, "pass_cmp") / att - 0.3) * 5.0);
    let b = c((g(m, "pass_yds") / att - 3.0) * 0.25);
    let cc = c(g(m, "pass_td") / att * 20.0);
    let d = c(2.375 - g(m, "pass_int") / att * 25.0);
    (a + b + cc + d) / 6.0 * 100.0
}

pub fn categories() -> Vec<Category> {
    vec![
        cat("Passing", |m, _| g(m, "pass_att") > 0.0, vec![
            count("Cmp", "pass_cmp", 54), count("Att", "pass_att", 54),
            ratio("Cmp%", "pass_cmp", "pass_att", 54, pct), count("Yds", "pass_yds", 54),
            ratio("Y/A", "pass_yds", "pass_att", 54, d1), count("TD", "pass_td", 54), count("INT", "pass_int", 54),
            count("Sck", "sacks_taken", 54), count("Lng", "pass_long", 54),
            col("Rtg", 54, |m| d1(passer_rating(m)), passer_rating), games(),
        ], 3),
        cat("Rushing", |m, _| g(m, "rush_att") > 0.0, vec![
            count("Att", "rush_att", 54), count("Yds", "rush_yds", 54), ratio("YPC", "rush_yds", "rush_att", 54, d1),
            count("TD", "rush_td", 54), count("Lng", "rush_long", 54), count("1D", "rush_fd", 54), count("Fum", "fumbles", 54),
            ratio("Yds/G", "rush_yds", "gp", 58, d1), games(),
        ], 1),
        cat("Receiving", |m, _| g(m, "targets") > 0.0 || g(m, "rec") > 0.0, vec![
            count("Rec", "rec", 54), count("Tgt", "targets", 54), count("Yds", "rec_yds", 54),
            ratio("Y/R", "rec_yds", "rec", 54, d1), ratio("Y/T", "rec_yds", "targets", 54, d1), ratio("Ctch%", "rec", "targets", 58, pct),
            count("TD", "rec_td", 54), count("Lng", "rec_long", 54), count("YAC", "yac", 54), count("Drp", "drops", 54), games(),
        ], 2),
        cat("Defense", |m, _| g(m, "tackles_solo") + g(m, "tackles_ast") + g(m, "sacks") + g(m, "int") + g(m, "pd") > 0.0, vec![
            col("Tkl", 54, |m| n(g(m, "tackles_solo") + g(m, "tackles_ast")), |m| g(m, "tackles_solo") + g(m, "tackles_ast")),
            count("Solo", "tackles_solo", 54),
            col("TFL", 54, |m| d1(g(m, "tfl")), |m| g(m, "tfl")),
            col("Sck", 54, |m| d1(g(m, "sacks")), |m| g(m, "sacks")),
            count("QBH", "qb_hits", 54), count("INT", "int", 54), count("PD", "pd", 54), count("FF", "ff", 54), count("FR", "fr", 54), games(),
        ], 0),
        cat("Kicking", |m, _| g(m, "fga") + g(m, "xpa") > 0.0, vec![
            count("FGM", "fgm", 54), count("FGA", "fga", 54), ratio("FG%", "fgm", "fga", 54, pct), count("Lng", "fg_long", 54),
            count("XPM", "xpm", 54), count("XPA", "xpa", 54),
            col("Pts", 54, |m| n(g(m, "fgm") * 3.0 + g(m, "xpm")), |m| g(m, "fgm") * 3.0 + g(m, "xpm")),
        ], 0),
        cat("Punting", |m, _| g(m, "punts") > 0.0, vec![
            count("Punts", "punts", 58), count("Yds", "punt_yds", 54), ratio("Avg", "punt_yds", "punts", 54, d1),
            count("In20", "punt_in20", 54), count("TB", "punt_tb", 54),
        ], 0),
        cat("Kick returns", |m, _| g(m, "kr") > 0.0, vec![
            count("Ret", "kr", 54), count("Yds", "kr_yds", 54), ratio("Avg", "kr_yds", "kr", 54, d1), count("Lng", "kr_long", 54), count("TD", "kr_td", 54), games(),
        ], 0),
        cat("Punt returns", |m, _| g(m, "pr") > 0.0, vec![
            count("Ret", "pr", 54), count("Yds", "pr_yds", 54), ratio("Avg", "pr_yds", "pr", 54, d1), count("Lng", "pr_long", 54), count("TD", "pr_td", 54), games(),
        ], 0),
    ]
}

/// Offensive-line table (team page and box score).
pub fn ol_category() -> Category {
    cat("Offensive line", |m, p| matches!(p, Position::Ot | Position::Og | Position::C) && g(m, "snaps_off") > 0.0, vec![
        count("Snaps", "snaps_off", 58), count("SckAlw", "sacks_allowed", 62), count("PrsAlw", "pressures_allowed", 62),
        count("Pancks", "pancakes", 62), count("Pen", "penalties", 54), games(),
    ], 0)
}

pub fn by_name(name: &str) -> Category {
    if name == "Offensive line" {
        return ol_category();
    }
    categories().into_iter().find(|c| c.name == name).unwrap_or_else(|| categories().remove(0))
}

pub struct RowIn {
    pub id: u32,
    pub name: String,
    pub team: String,
    pub pos: String,
    pub mine: bool,
    pub stats: Stats,
}

/// A table as JSON: columns, and rows of `{text, value}` cells. `drop` removes columns by header (the box score has no
/// games-played or per-game columns).
pub fn table(c: &Category, rows: &[RowIn], drop: impl Fn(&str) -> bool) -> Value {
    let keep: Vec<usize> = (0..c.cols.len()).filter(|&i| !drop(c.cols[i].header)).collect();
    let cols: Vec<Value> = keep.iter().map(|&i| json!({ "header": c.cols[i].header, "width": c.cols[i].width })).collect();
    let sort_col = keep.iter().position(|&i| i == c.sort_col).unwrap_or(0);
    let out: Vec<Value> = rows
        .iter()
        .map(|r| {
            let cells: Vec<Value> = keep.iter().map(|&i| json!({ "t": (c.cols[i].text)(&r.stats), "v": (c.cols[i].value)(&r.stats) })).collect();
            json!({ "id": r.id, "name": r.name, "team": r.team, "pos": r.pos, "mine": r.mine, "cells": cells })
        })
        .collect();
    json!({ "name": c.name, "cols": cols, "sortCol": sort_col, "rows": out })
}

// ------------------------------------------------------------------ player bio columns

type Fmt = Box<dyn Fn(&Stats) -> String>;

fn pn(key: &'static str) -> Fmt {
    Box::new(move |m| n(g(m, key)))
}
fn pavg(yds: &'static str, cnt: &'static str) -> Fmt {
    Box::new(move |m| if g(m, cnt) == 0.0 { "–".to_string() } else { d1(g(m, yds) / g(m, cnt)) })
}
fn pratio(a: &'static str, b: &'static str) -> Fmt {
    Box::new(move |m| format!("{}/{}", n(g(m, a)), n(g(m, b))))
}

fn base_columns(pos: Position) -> Vec<(&'static str, Fmt)> {
    use Position::*;
    let games: (&'static str, Fmt) = ("G", pn("gp"));
    match pos {
        Qb => vec![games, ("C/A", pratio("pass_cmp", "pass_att")), ("PaYds", pn("pass_yds")), ("PaTD", pn("pass_td")), ("INT", pn("pass_int")), ("Sck", pn("sacks_taken"))],
        Rb | Fb => vec![games, ("Att", pn("rush_att")), ("RuYds", pn("rush_yds")), ("Avg", pavg("rush_yds", "rush_att")), ("RuTD", pn("rush_td")), ("Rec", pn("rec"))],
        Wr | Te => vec![games, ("Tgt", pn("targets")), ("Rec", pn("rec")), ("ReYds", pn("rec_yds")), ("ReTD", pn("rec_td")), ("Drp", pn("drops"))],
        Ot | Og | C => vec![games, ("Snaps", pn("snaps_off")), ("SckAlw", pn("sacks_allowed")), ("Pancks", pn("pancakes")), ("Pen", pn("penalties"))],
        K => vec![games, ("FG", pratio("fgm", "fga")), ("Lng", pn("fg_long")), ("XP", pratio("xpm", "xpa"))],
        P => vec![games, ("Punts", pn("punts")), ("Avg", pavg("punt_yds", "punts")), ("In20", pn("punt_in20"))],
        _ => vec![games, ("Tkl", Box::new(|m| n(g(m, "tackles_solo") + g(m, "tackles_ast")))), ("TFL", pn("tfl")), ("Sck", Box::new(|m| format!("{}", g(m, "sacks")))), ("PD", pn("pd")), ("INT", pn("int"))],
    }
}

/// Primary columns for the position plus secondary skills (QBs also run, RBs also catch...). `career` decides what shows for the optional groups.
pub fn player_columns(pos: Position, career: &Stats) -> Vec<(&'static str, Fmt)> {
    use Position::*;
    let has = |ks: &[&str]| ks.iter().any(|k| g(career, k) > 0.0);
    let rush = || -> Vec<(&'static str, Fmt)> { vec![("Rush", pn("rush_att")), ("RuYds", pn("rush_yds")), ("RuTD", pn("rush_td"))] };
    let rec = || -> Vec<(&'static str, Fmt)> { vec![("Rec", pn("rec")), ("ReYds", pn("rec_yds")), ("ReTD", pn("rec_td"))] };
    let pass = || -> Vec<(&'static str, Fmt)> { vec![("C/A", pratio("pass_cmp", "pass_att")), ("PaYds", pn("pass_yds")), ("PaTD", pn("pass_td")), ("INT", pn("pass_int"))] };
    let kick = || -> Vec<(&'static str, Fmt)> { vec![("FG", pratio("fgm", "fga")), ("Lng", pn("fg_long")), ("XP", pratio("xpm", "xpa"))] };
    let punt = || -> Vec<(&'static str, Fmt)> { vec![("Punts", pn("punts")), ("PuAvg", pavg("punt_yds", "punts")), ("In20", pn("punt_in20"))] };
    let mut returns: Vec<(&'static str, Fmt)> = Vec::new();
    if has(&["kr"]) {
        returns.extend([("KR", pn("kr")), ("KRYds", pn("kr_yds")), ("KRAvg", pavg("kr_yds", "kr")), ("KRLng", pn("kr_long")), ("KRTD", pn("kr_td"))]);
    }
    if has(&["pr"]) {
        returns.extend([("PR", pn("pr")), ("PRYds", pn("pr_yds")), ("PRAvg", pavg("pr_yds", "pr")), ("PRLng", pn("pr_long")), ("PRTD", pn("pr_td"))]);
    }
    let kicking = if has(&["fga", "xpa"]) { kick() } else { vec![] };
    let punting = if has(&["punts"]) { punt() } else { vec![] };
    let mut out = base_columns(pos);
    match pos {
        K => { out.extend(punting); }
        P => { out.extend(kicking); }
        Qb => {
            out.extend(rush());
            if has(&["rec", "targets"]) { out.extend(rec()); }
        }
        Rb | Fb => {
            out.extend(rec().into_iter().skip(1));
            if has(&["pass_att"]) { out.extend(pass()); }
        }
        Wr | Te => {
            if has(&["rush_att"]) { out.extend(rush()); }
            if has(&["pass_att"]) { out.extend(pass()); }
        }
        _ => {
            if has(&["rush_att"]) { out.extend(rush()); }
            if has(&["rec"]) { out.extend(rec()); }
            out.extend(kicking);
            out.extend(punting);
        }
    }
    out.extend(returns);
    out
}
