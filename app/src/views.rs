//! View-models: plain JSON the QML screens render. Everything here only reads the league.

use serde_json::{Value, json};
use sp_core::geo::town_miles;
use sp_core::region::is_rivalry;
use sp_core::team::min_active_coverage;
use sp_core::{Attr, Player, Position, Region, Town};
use sp_league::insights::Insights;
use sp_league::hall_of_fame::HofTuning;
use sp_league::inbox::InboxFeed;
use sp_league::standings::GameRecord;
use sp_league::{League, LeaguePhase, WeekResult};

pub fn record_of(lg: &League, t: usize) -> String {
    let r = &lg.standings.records[t];
    format!("{}-{}{}", r.wins, r.losses, if r.ties > 0 { format!("-{}", r.ties) } else { String::new() })
}

/// "Dominic Blackey" -> "D. Blackey".
pub fn short_name(name: &str) -> String {
    match name.find(' ') {
        Some(i) if i > 0 => format!("{}. {}", &name[..1], &name[i + 1..]),
        _ => name.to_string(),
    }
}

pub fn helmet_slug(nickname: &str) -> String {
    nickname.to_lowercase().replace(' ', "-")
}

pub fn money(v: i64) -> String {
    sp_league::text::money(v)
}

pub struct NextGame {
    pub week: i32,
    pub home: bool,
    pub opponent: usize,
    pub playoff: bool,
}

pub fn next_game(lg: &League) -> Option<NextGame> {
    let u = lg.user_team?;
    match lg.phase {
        LeaguePhase::RegularSeason => lg.games_in_week(lg.week).into_iter().find(|g| g.home == u || g.away == u).map(|g| NextGame { week: g.week, home: g.home == u, opponent: if g.home == u { g.away } else { g.home }, playoff: false }),
        LeaguePhase::Semifinals => lg.playoffs.as_ref()?.semifinals.iter().find(|g| g.home() == u || g.away() == u).map(|g| NextGame { week: 15, home: g.home() == u, opponent: if g.home() == u { g.away() } else { g.home() }, playoff: true }),
        LeaguePhase::Championship => lg.playoffs.as_ref()?.final_game.as_ref().filter(|g| g.home() == u || g.away() == u).map(|g| NextGame { week: 16, home: g.home() == u, opponent: if g.home() == u { g.away() } else { g.home() }, playoff: true }),
        _ => None,
    }
}

pub fn last_game(lg: &League) -> Option<&GameRecord> {
    let u = lg.user_team?;
    lg.results.iter().rev().find(|g| g.home == u || g.away == u)
}

fn team_ref(lg: &League, t: usize) -> Value {
    let town = &lg.teams[t].town;
    json!({ "index": t, "name": town.team_name(), "town": town.name, "nick": town.nickname, "abbr": town.abbr(), "helmet": helmet_slug(&town.nickname), "record": record_of(lg, t) })
}

fn phase_label(lg: &League) -> &'static str {
    match lg.phase {
        LeaguePhase::RegularSeason => "Regular season",
        LeaguePhase::Semifinals => "Semifinals",
        LeaguePhase::Championship => "Championship",
        LeaguePhase::Complete => "Season complete",
        LeaguePhase::Offseason => "Offseason",
    }
}

/// The persistent header on every screen.
pub fn header(lg: &League) -> Value {
    let u = lg.user();
    let f = lg.my_finance();
    let week_text = match lg.phase {
        LeaguePhase::RegularSeason => format!("Week {} of 14", lg.week.min(14)),
        LeaguePhase::Semifinals => "Semifinals".to_string(),
        LeaguePhase::Championship => "Championship".to_string(),
        LeaguePhase::Complete => "Season complete".to_string(),
        LeaguePhase::Offseason => "Offseason".to_string(),
    };
    json!({
        "team": team_ref(lg, u),
        "season": lg.season,
        "phase": format!("{:?}", lg.phase),
        "phaseLabel": phase_label(lg),
        "weekText": week_text,
        "cash": f.cash,
        "cashText": money(f.cash),
        "unread": lg.unread_inbox(None),
        "gameOver": lg.game_over(),
    })
}

fn division_rows(lg: &League, div: usize) -> Value {
    let order = lg.standings.division_order(div);
    Value::Array(
        order
            .iter()
            .enumerate()
            .map(|(i, &t)| {
                let r = &lg.standings.records[t];
                json!({ "rank": i + 1, "team": team_ref(lg, t), "isMe": Some(t) == lg.user_team, "record": record_of(lg, t), "pf": r.points_for, "pa": r.points_against, "diff": r.diff(), "streak": r.streak })
            })
            .collect(),
    )
}

fn runway(lg: &League) -> String {
    let f = lg.my_finance();
    if f.games.is_empty() {
        return "–".into();
    }
    let avg: f64 = f.games.iter().map(|g| g.net()).sum::<i64>() as f64 / f.games.len() as f64;
    if avg >= 0.0 {
        return "Healthy".into();
    }
    // A tiny average loss gives an absurd count ("~15270 games"); past two seasons of games it is not a worry.
    let games = (f.cash as f64 / -avg).floor() as i64;
    if games >= 28 { "Healthy".into() } else { format!("~{games} games") }
}

/// Position groups the roster screen filters by (label, positions).
pub fn position_groups() -> Vec<(&'static str, Vec<Position>)> {
    use Position::*;
    vec![
        ("QB", vec![Qb]),
        ("RB", vec![Rb, Fb]),
        ("WR", vec![Wr]),
        ("TE", vec![Te]),
        ("OL", vec![Ot, Og, C]),
        ("DL", vec![De, Dt]),
        ("LB", vec![Mlb, Olb]),
        ("DB", vec![Cb, S]),
        ("K", vec![K]),
        ("P", vec![P]),
    ]
}

/// Position groups short of their minimum active coverage.
fn gaps(lg: &League) -> Vec<String> {
    let me = lg.me();
    let mut out = Vec::new();
    for (label, ps) in position_groups() {
        let have = me.active().filter(|p| ps.contains(&p.position)).count();
        let need: usize = ps.iter().map(|&p| min_active_coverage(p)).sum();
        if have < need {
            out.push(format!("{label}: {have} dressed, need {need}"));
        }
    }
    out
}

pub fn home(lg: &League) -> Value {
    let u = lg.user();
    let me = lg.me();
    let f = lg.my_finance();
    let div = lg.division_of(u);
    let ng = next_game(lg);
    let next = ng.as_ref().map(|n| {
        let opp = &lg.teams[n.opponent];
        let rep = Insights::opponent(opp);
        let rivalry = is_rivalry(&me.town.id, &opp.town.id);
        json!({
            "week": n.week, "home": n.home, "playoff": n.playoff, "rivalry": rivalry,
            "opponent": team_ref(lg, n.opponent), "quirk": opp.town.quirk,
            "summary": rep.summary, "tips": rep.tips, "chips": rep.chips,
            "matchups": Insights::matchups(me, opp).iter().map(|m| json!({"label": m.label, "ours": m.ours, "theirs": m.theirs, "fit": m.fit.label()})).collect::<Vec<_>>(),
        })
    });
    let last = last_game(lg).map(|g| {
        let home = g.home == u;
        let (my, their) = if home { (g.home_score, g.away_score) } else { (g.away_score, g.home_score) };
        let opp = if home { g.away } else { g.home };
        let gf = if home { g.home_finance.as_ref() } else { g.away_finance.as_ref() };
        json!({ "week": g.week, "home": home, "my": my, "their": their, "result": if my > their { "W" } else if my < their { "L" } else { "T" }, "ot": g.overtime, "opponent": team_ref(lg, opp), "net": gf.map(|x| x.net()), "attendance": gf.map(|x| x.attendance), "capacity": gf.map(|x| x.capacity), "playoff": g.playoff,
            "box": lg.results.iter().rposition(|x| x.week == g.week && x.home == g.home && x.away == g.away && x.box_score.is_some()) })
    });
    let out: Vec<Value> = me
        .roster
        .iter()
        .filter(|p| lg.injured_games.contains_key(&p.id) && me.active_ids.contains(&p.id))
        .map(|p| json!({ "id": p.id, "name": p.name, "pos": p.position.label(), "games": lg.injured_games[&p.id], "reason": lg.out_reason.get(&p.id) }))
        .collect();
    let rank = lg.standings.division_order(div).iter().position(|&t| t == u).unwrap_or(0) + 1;
    let made = lg.playoffs.as_ref().map(|p| p.seeds.contains(&u)).unwrap_or(false);
    json!({
        "header": header(lg),
        "next": next,
        "last": last,
        "divisionName": lg.region.divisions.get(div).cloned().unwrap_or_default(),
        "division": division_rows(lg, div),
        "rank": rank,
        "madePlayoffs": made,
        "champion": lg.champion().map(|c| team_ref(lg, c)),
        "iWon": lg.champion() == Some(u),
        "money": { "cash": f.cash, "cashText": money(f.cash), "lastNet": f.games.last().map(|g| g.net()), "runway": runway(lg) },
        "alerts": { "out": out, "gaps": gaps(lg), "expiring": lg.pending_expired.len(), "autoSet": lg.auto_set_roster },
        "phase": format!("{:?}", lg.phase),
        "draftOpen": lg.draft.as_ref().map(|d| !d.complete()).unwrap_or(false),
    })
}

fn player_row(lg: &League, p: &Player) -> Value {
    let me = lg.me();
    let fin = lg.my_finance();
    let c = fin.contracts.get(&p.id);
    let status = if let Some(g) = lg.injured_games.get(&p.id) {
        format!("OUT {g} gm")
    } else if lg.pending_expired.contains(&p.id) {
        "Expiring".to_string()
    } else {
        "–".to_string()
    };
    json!({
        "id": p.id, "name": p.name, "short": short_name(&p.name), "pos": p.position.label(), "ovr": p.ovr(), "age": p.age,
        "ht": format!("{}'{}\"", p.height_in / 12, p.height_in % 12), "htIn": p.height_in, "wt": p.weight_lb,
        "ratings": Attr::ALL.iter().map(|&a| p.ratings[a]).collect::<Vec<_>>(),
        "ppg": c.map(|c| c.ppg), "years": c.map(|c| c.seasons_left), "status": status,
        "active": me.active_ids.contains(&p.id), "out": lg.injured_games.contains_key(&p.id),
        "local": town_miles(&p.hometown, &me.town.name) <= 1.0, "hometown": p.hometown, "job": p.day_job,
        "number": me.numbers.get(&p.id).copied().or(p.jersey), "acquired": p.acquired, "yearsPro": p.years_pro, "yearsLabel": p.years_label(),
        "awards": p.awards, "fan": lg.fan_favorite_ids.contains(&p.id),
    })
}

pub fn roster(lg: &League) -> Value {
    let me = lg.me();
    let fin = lg.my_finance();
    let rows: Vec<Value> = me.roster.iter().map(|p| player_row(lg, p)).collect();
    let payroll = fin.projected_payroll(me);
    let groups: Vec<Value> = position_groups().iter().map(|(label, ps)| json!({ "label": label, "have": me.active().filter(|p| ps.contains(&p.position)).count(), "need": ps.iter().map(|&p| min_active_coverage(p)).sum::<usize>() })).collect();
    json!({
        "header": header(lg),
        "rows": rows,
        "attrs": Attr::ALL.iter().map(|a| a.code()).collect::<Vec<_>>(),
        "total": me.roster.len(),
        "active": me.active_ids.len(),
        "activeMax": sp_core::team::ACTIVE_ROSTER_SIZE,
        "payroll": payroll, "payrollText": money(payroll),
        "gaps": gaps(lg),
        "groups": groups,
        "autoSet": lg.auto_set_roster,
    })
}

pub fn standings(lg: &League) -> Value {
    let u = lg.user_team;
    let divisions: Vec<Value> = (0..3).map(|d| json!({ "name": lg.region.divisions.get(d).cloned().unwrap_or_default(), "rows": division_rows(lg, d) })).collect();
    let seeds: Vec<Value> = lg.playoffs.as_ref().map(|p| p.seeds.iter().enumerate().map(|(i, &t)| json!({ "seed": i + 1, "team": team_ref(lg, t), "isMe": Some(t) == u })).collect()).unwrap_or_default();
    let recent: Vec<Value> = lg.results.iter().rev().take(12).map(|g| json!({ "week": g.week, "home": team_ref(lg, g.home), "away": team_ref(lg, g.away), "hs": g.home_score, "as": g.away_score, "ot": g.overtime, "playoff": g.playoff, "mine": Some(g.home) == u || Some(g.away) == u })).collect();
    let games_left: Vec<Value> = if lg.phase == LeaguePhase::RegularSeason {
        lg.games_in_week(lg.week).iter().map(|g| json!({ "week": g.week, "home": team_ref(lg, g.home), "away": team_ref(lg, g.away), "mine": Some(g.home) == u || Some(g.away) == u })).collect()
    } else {
        vec![]
    };
    json!({ "header": header(lg), "divisions": divisions, "seeds": seeds, "recent": recent, "thisWeek": games_left, "history": lg.history.iter().rev().map(|h| json!({"season": h.season, "champion": h.champion_name})).collect::<Vec<_>>() })
}

pub fn inbox(lg: &League) -> Value {
    let items: Vec<Value> = lg
        .inbox
        .iter()
        .rev()
        .map(|i| {
            json!({
                "id": i.id, "season": i.season, "week": i.week, "feed": if i.feed == InboxFeed::Money { "money" } else { "travel" }, "kind": format!("{:?}", i.kind),
                "title": i.title, "text": i.text, "from": i.from, "unread": i.unread, "playerId": i.player_id,
                "pleaOpen": i.plea_team.is_some() && i.plea_ask > 0 && i.plea_state == sp_league::inbox::PleaState::Open, "pleaAsk": i.plea_ask,
                "pleaState": format!("{:?}", i.plea_state), "pleaTeam": i.plea_team.map(|t| lg.teams[t].town.team_name()),
                "conductOpen": i.conduct_state == sp_league::inbox::ConductState::Open && i.conduct_cost > 0, "conductCost": i.conduct_cost, "conductGames": i.conduct_games,
                "conductState": format!("{:?}", i.conduct_state), "isConduct": i.kind == sp_league::inbox::InboxKind::Conduct,
            })
        })
        .collect();
    json!({ "header": header(lg), "cash": lg.user_team.map(|u| lg.finances[u].cash).unwrap_or(0), "items": items, "unreadTotal": lg.unread_inbox(None), "unreadMoney": lg.unread_inbox(Some(InboxFeed::Money)), "unreadTravel": lg.unread_inbox(Some(InboxFeed::Travel)) })
}

pub fn week_result(lg: &League, r: &WeekResult) -> Value {
    let u = lg.user_team;
    let games: Vec<Value> = r
        .games
        .iter()
        .map(|g| {
            let mine = Some(g.home) == u || Some(g.away) == u;
            json!({ "home": team_ref(lg, g.home), "away": team_ref(lg, g.away), "hs": g.home_score, "as": g.away_score, "ot": g.overtime, "mine": mine, "playoff": g.playoff, "weather": format!("{:?}", g.weather),
                    "box": lg.results.iter().rposition(|x| x.week == g.week && x.home == g.home && x.away == g.away && x.box_score.is_some()) })
        })
        .collect();
    let label = if r.week <= 14 { format!("Week {} results", r.week) } else if r.week == 15 { "Semifinal results".to_string() } else { "Championship result".to_string() };
    json!({ "week": r.week, "label": label, "games": games, "workNews": r.work_news, "phaseAfter": format!("{:?}", r.phase_after), "header": header(lg) })
}

pub fn offseason(lg: &mut League) -> Value {
    use sp_economy::{LedgerKind, Negotiation, PayScale};
    let ids: Vec<u32> = lg.pending_expired.iter().copied().collect();
    for &id in &ids {
        lg.sync_awards_for(id);
    }
    let prems: std::collections::BTreeMap<u32, Option<sp_economy::ResignPremium>> = ids.iter().map(|&id| (id, lg.resign_premium(id))).collect();
    let lg = &*lg;
    let u = lg.user();
    let me = lg.me();
    let fin = lg.my_finance();
    let win = lg.standings.records[u].pct();

    // Live budget: players still waiting on a deal are not counted.
    let (mut payroll, mut signed) = (0i64, 0usize);
    for p in &me.roster {
        let Some(c) = fin.contracts.get(&p.id) else { continue };
        if lg.pending_expired.contains(&p.id) {
            continue;
        }
        signed += 1;
        payroll += if me.active_ids.contains(&p.id) { c.active_pay() } else { PayScale::retainer(c.ppg) };
    }
    let n = fin.games.len() as i64;
    let avg = |f: &dyn Fn(&sp_economy::GameFinance) -> i64| if n == 0 { 0 } else { (fin.games.iter().map(f).sum::<i64>() as f64 / n as f64).round() as i64 };
    let rev = avg(&|g| g.total_revenue());
    let other = avg(&|g| g.total_expenses() - g.expenses.get(&LedgerKind::PlayerPay).copied().unwrap_or(0) - g.expenses.get(&LedgerKind::Retainers).copied().unwrap_or(0));
    let net = rev - other - payroll;

    let mut waiting: Vec<Value> = me
        .roster
        .iter()
        .filter(|p| lg.pending_expired.contains(&p.id))
        .map(|p| {
            // Asks reflect the new ratings, so this needs the same premium the negotiation sheet uses.
            let prem = prems.get(&p.id).and_then(|x| x.as_ref());
            let ask = Negotiation::target(p, &me.town, 3, win, prem);
            json!({ "id": p.id, "name": p.name, "pos": p.position.label(), "ovr": p.ovr(), "age": p.age, "ask": ask, "askText": money(ask), "wantsToPlay": prem.map(|r| r.wants_to_play).unwrap_or(false) })
        })
        .collect();
    waiting.sort_by(|a, b| b["ovr"].as_i64().cmp(&a["ovr"].as_i64()));

    let mut changes: Vec<Value> = me
        .roster
        .iter()
        .filter_map(|p| {
            lg.last_changes.get(&p.id).map(|(b, a)| {
                let before = sp_core::ovr::overall(&sp_core::Ratings::from_slice(&b[..]), p.position);
                let delta = p.ovr() - before;
                json!({
                    "id": p.id, "name": p.name, "pos": p.position.label(), "age": p.age, "ovr": p.ovr(), "before": before, "delta": delta,
                    "attrs": Attr::ALL.iter().map(|&at| json!({ "code": at.code(), "from": b[at.index()], "to": a[at.index()] })).collect::<Vec<_>>(),
                })
            })
        })
        .collect();
    changes.sort_by(|a, b| b["delta"].as_i64().cmp(&a["delta"].as_i64()));

    let d = lg.draft.as_ref();
    json!({
        "header": header(lg),
        "season": lg.season,
        "retired": lg.last_retired,
        "fanNews": lg.fan_news.iter().map(|n| n.text.clone()).collect::<Vec<_>>(),
        "expiring": lg.pending_expired.len(),
        "waiting": waiting,
        "freeAgents": lg.free_agents.len(),
        "changes": changes,
        "draftPending": d.map(|d| !d.complete()).unwrap_or(false),
        "champion": lg.history.last().map(|h| h.champion_name.clone()),
        "budget": {
            "cash": fin.cash, "cashText": money(fin.cash), "payroll": payroll, "payrollText": money(payroll), "signed": signed, "active": me.active_ids.len(),
            "known": n > 0, "revenue": money(rev), "otherCosts": money(other), "net": net, "netText": money(net.abs()), "projection": money(net.abs() * 14), "projectionPositive": net >= 0,
        },
    })
}

pub fn draft(lg: &League) -> Value {
    let Some(d) = lg.draft.as_ref() else { return json!({ "none": true }) };
    let u = lg.user();
    let me = lg.me();
    let strat = lg.draft_strategy_for(u);
    let broke = lg.cant_draft_reason(u);
    let on_clock = lg.user_on_clock();
    let best: Vec<u32> = lg.draft_board(u).iter().take(3).map(|p| p.id).collect();
    let prospects: Vec<Value> = d
        .prospects
        .iter()
        .map(|p| {
            let commute = sp_economy::PayScale::commute(town_miles(&p.hometown, &me.town.name));
            json!({
                "id": p.id, "name": p.name, "pos": p.position.label(), "ovr": p.ovr(), "age": p.age, "size": p.size_label(), "hometown": p.hometown,
                "commute": commute, "commuteText": money(commute), "pay": d.slot_ppg() + commute, "payText": money(d.slot_ppg() + commute), "local": commute == 0,
                "best": best.first() == Some(&p.id), "top3": best.contains(&p.id),
            })
        })
        .collect();
    let picks: Vec<Value> = d
        .picks
        .iter()
        .map(|p| json!({
            "overall": p.overall, "round": p.round, "team": team_ref(lg, p.team), "mine": p.team == u, "forfeited": p.forfeited, "reason": p.reason, "playerId": p.player_id,
            "name": p.name, "pos": p.pos, "ovr": p.ovr, "age": p.age, "payText": money(p.ppg), "commuteText": if p.commute > 0 { money(p.commute) } else { String::new() },
        }))
        .collect();
    let order: Vec<Value> = d.order.iter().enumerate().map(|(i, &t)| json!({ "pick": i + 1, "team": team_ref(lg, d.owners[i]), "from": if d.owners[i] != t { Some(lg.teams[t].town.name.clone()) } else { None }, "mine": d.owners[i] == u, "record": record_of(lg, t) })).collect();
    json!({
        "none": false, "season": d.season, "complete": d.complete(), "made": d.made(), "total": d.total_picks(), "round": d.round(), "overall": d.overall(),
        "slotPpg": d.slot_ppg(), "slotPpgText": money(d.slot_ppg()), "slotSeasons": d.slot_seasons(),
        "onClock": on_clock, "clockTeam": d.on_clock().map(|t| team_ref(lg, t)),
        "mine": d.picks_ahead(u), "strategy": strat.label, "focus": strat.focus.iter().map(|p| p.label()).collect::<Vec<_>>(),
        "broke": broke, "canDraft": on_clock && lg.cant_draft_reason(u).is_none(),
        "prospects": prospects, "picks": picks, "order": order,
        "passed": d.picks.iter().filter(|p| p.forfeited).count(),
    })
}

// -------------------------------------------------------------------- new game

fn town_json(t: &Town) -> Value {
    let score = t.population as f64 / 100_000.0 * 0.6 + t.loyalty as f64 + t.wealth as f64 * 0.5 + t.turnout as f64 * 0.6 + t.biz as f64 * 0.4;
    let difficulty = if score >= 17.0 { "Easy" } else if score >= 12.5 { "Medium" } else { "Hard" };
    json!({
        "id": t.id, "name": t.name, "division": t.division, "nickname": t.nickname, "helmet": helmet_slug(&t.nickname), "mascot": t.mascot, "quirk": t.quirk,
        "population": t.population, "wealth": t.wealth, "turnout": t.turnout, "loyalty": t.loyalty, "culture": t.culture, "biz": t.biz, "capacity": t.capacity,
        "difficulty": difficulty, "abbr": t.abbr(),
    })
}

pub fn regions() -> Value {
    Value::Array(Region::all().iter().map(|r| json!({ "id": r.id, "name": r.name, "divisions": r.divisions, "cold": r.cold })).collect())
}

/// A fresh league of 12 towns (4 per division) with a unique nickname each.
pub fn deal(region_id: &str, seed: u64) -> Value {
    let r = Region::by_id(Some(region_id));
    let mut rng = sp_core::Rng::new(seed);
    let ids = r.deal_league_towns(&mut rng);
    let towns: Vec<Town> = ids.iter().map(|id| r.town(id).unwrap().clone()).collect();
    let dealt = sp_core::region::deal_nicknames(&towns, &mut rng, &Default::default(), Some(&r.id));
    json!({ "region": r.id, "towns": dealt.iter().map(town_json).collect::<Vec<_>>() })
}

pub fn nicknames(region_id: &str) -> Value {
    json!(sp_core::region::nicknames_for(Some(region_id)).iter().map(|n| json!({ "name": n, "helmet": helmet_slug(n) })).collect::<Vec<_>>())
}

// -------------------------------------------------------------------- free agents, player detail, negotiation

const ATTR_NAMES: [&str; 8] = ["Speed", "Acceleration", "Agility", "Strength", "Hands", "Endurance", "Intelligence", "Discipline"];

/// Position groups where the club cannot field the minimum (injured players do not count).
fn needs(lg: &League) -> Vec<&'static str> {
    let me = lg.me();
    position_groups()
        .into_iter()
        .filter(|(_, ps)| {
            let have = me.roster.iter().filter(|p| ps.contains(&p.position) && !lg.injured_games.contains_key(&p.id)).count();
            let need: usize = ps.iter().map(|&p| min_active_coverage(p)).sum();
            have < need
        })
        .map(|(l, _)| l)
        .collect()
}

pub fn free_agents(lg: &mut League) -> Value {
    let win = lg.my_win_pct();
    let town = lg.me().town.clone();
    let ids: Vec<u32> = lg.free_agents.iter().map(|p| p.id).collect();
    for id in ids {
        lg.sync_awards_for(id);
    }
    let me = lg.me();
    let rows: Vec<Value> = lg
        .free_agents
        .iter()
        .map(|p| {
            let keys = sp_core::ovr::ovr_key_attrs(p.position);
            json!({
                "id": p.id, "name": p.name, "pos": p.position.label(), "ovr": p.ovr(), "age": p.age, "size": p.size_label(), "hometown": p.hometown,
                "local": town_miles(&p.hometown, &town.name) <= 1.0,
                "ask": sp_economy::Negotiation::target(p, &town, 3, win, None), "askText": money(sp_economy::Negotiation::target(p, &town, 3, win, None)),
                "keys": keys.iter().map(|&a| json!({ "code": a.code(), "v": p.ratings[a] })).collect::<Vec<_>>(),
            })
        })
        .collect();
    json!({
        "header": header(lg), "rows": rows, "needs": needs(lg), "groups": position_groups().iter().map(|(l, _)| *l).collect::<Vec<_>>(),
        "rosterCount": me.roster.len(), "active": me.active_ids.len(), "activeMax": sp_core::team::ACTIVE_ROSTER_SIZE,
        "cash": lg.my_finance().cash, "cashText": money(lg.my_finance().cash),
        "tryoutCost": League::TRYOUT_COST, "tryoutCostText": money(League::TRYOUT_COST), "tryoutHeld": lg.tryout_held_this_week,
        "canTryout": lg.my_finance().cash >= League::TRYOUT_COST && !lg.tryout_held_this_week && !lg.game_over(),
    })
}

/// A player on your roster or the free-agent street.
pub fn player_detail(lg: &mut League, id: u32) -> Value {
    lg.sync_awards_for(id);
    let me = lg.me();
    let mine = me.roster.iter().any(|p| p.id == id);
    // Anyone in the league: your roster, free agents, draft prospects, other clubs, or a player who just retired.
    let other_team = lg.teams.iter().enumerate().find_map(|(t, tm)| tm.roster.iter().find(|p| p.id == id).map(|p| (t, p)));
    let Some(p) = me
        .roster
        .iter()
        .chain(lg.free_agents.iter())
        .chain(lg.draft.iter().flat_map(|d| d.prospects.iter()))
        .chain(other_team.map(|(_, p)| p))
        .chain(lg.last_retired_entries.iter().map(|e| &e.player))
        .find(|p| p.id == id)
    else {
        return json!({});
    };
    let club = other_team.map(|(t, _)| &lg.teams[t]).unwrap_or(me);
    let club_name = other_team.map(|(t, _)| lg.teams[t].town.team_name());
    let fin = lg.my_finance();
    let c = fin.contracts.get(&id);
    let keys = sp_core::ovr::ovr_key_attrs(p.position);
    let expiring = lg.pending_expired.contains(&id);
    json!({
        "id": p.id, "name": p.name, "pos": p.position.label(), "ovr": p.ovr(), "age": p.age, "ht": format!("{}'{}\"", p.height_in / 12, p.height_in % 12), "wt": p.weight_lb,
        "hometown": p.hometown, "job": p.day_job, "number": club.numbers.get(&p.id).copied().or(p.jersey), "teamSlug": helmet_slug(&club.town.nickname), "lastName": p.name.rsplit(' ').next().unwrap_or(""), "acquired": p.acquired, "yearsLabel": p.years_label(), "awards": p.awards,
        "club": if mine { None } else { club_name }, "retired": lg.last_retired_entries.iter().any(|e| e.player.id == id) && other_team.is_none() && !mine,
        "alsoKicks": p.also_kicks_punts, "fan": lg.fan_favorite_ids.contains(&id),
        "local": town_miles(&p.hometown, &me.town.name) <= 1.0, "mine": mine, "freeAgent": lg.free_agents.iter().any(|f| f.id == id), "expiring": expiring, "out": lg.injured_games.get(&id), "outReason": lg.out_reason.get(&id),
        "active": me.active_ids.contains(&id),
        "attrs": Attr::ALL.iter().enumerate().map(|(i, &a)| json!({ "code": a.code(), "name": ATTR_NAMES[i], "v": p.ratings[a], "key": keys.contains(&a) })).collect::<Vec<_>>(),
        "contract": c.map(|c| json!({ "ppg": c.ppg, "ppgText": money(c.ppg), "years": c.seasons_left, "winBonus": money(c.win_bonus), "commute": money(c.commute), "releaseCost": money(c.release_cost()) })),
        "releaseCost": if expiring { 0 } else { c.map(|c| c.release_cost()).unwrap_or(0) },
        "header": header(lg),
        "titles": lg.titles_won(id).iter().rev().map(|(s, a)| json!({ "season": s, "abbr": a })).collect::<Vec<_>>(),
        "awardsWon": lg.awards_won(id).iter().rev().map(|w| json!({ "season": w.season, "title": w.title, "team": w.team })).collect::<Vec<_>>(),
        "absences": absence_log(lg, id),
        "lastChanges": lg.last_changes.get(&id).map(|(b, a)| Attr::ALL.iter().enumerate().map(|(i, at)| json!({ "code": at.code(), "before": b[i], "after": a[i] })).collect::<Vec<_>>()),
        "jobInfo": job_info(lg, &p.day_job),
        "stats": player_stats(lg, p),
        "moves": if mine { p.position.can_switch_to().iter().map(|&q| json!({ "pos": q.label(), "ovr": p.ovr_at(q) })).collect::<Vec<_>>() } else { vec![] },
    })
}

fn job_info(lg: &League, name: &str) -> Value {
    let job = lg.jobs.lookup(name);
    let pct = |risk: i32, per: f64| -> String {
        let v = risk as f64 * per * 100.0;
        if v < 1.0 { format!("{v:.2}% per game") } else { format!("{v:.1}% per game") }
    };
    json!({
        "name": job.name,
        "injury": job.injury_risk, "injuryBand": sp_core::jobs::risk_band(job.injury_risk), "injuryPct": pct(job.injury_risk, sp_core::jobs::tuning::INJURY_PER_POINT),
        "travel": job.travel_risk, "travelBand": sp_core::jobs::risk_band(job.travel_risk), "travelPct": pct(job.travel_risk, sp_core::jobs::tuning::TRAVEL_MISS_PER_POINT),
    })
}

/// Every injury and work absence, newest season first.
fn absence_log(lg: &League, id: u32) -> Value {
    let all = lg.absences_for(id);
    let total: i32 = all.iter().map(|a| a.missed).sum();
    let mut seasons: Vec<i32> = all.iter().map(|a| a.season).collect();
    seasons.sort_unstable();
    seasons.dedup();
    seasons.reverse();
    let groups: Vec<Value> = seasons
        .iter()
        .map(|&s| {
            let evs: Vec<&&sp_league::absence_log::AbsenceEvent> = all.iter().filter(|a| a.season == s).collect();
            json!({
                "season": s, "missed": evs.iter().map(|a| a.missed).sum::<i32>(),
                "rows": evs.iter().rev().map(|a| {
                    let conduct = !a.work && a.what.starts_with("Suspended");
                    json!({ "kind": if a.work { "WORK" } else if conduct { "CONDUCT" } else { "TEAM" }, "what": a.what,
                            "when": if a.week > 14 { "Playoffs".to_string() } else { format!("Week {}", a.week) },
                            "status": if a.open { format!("out {} more", a.games - a.missed) } else { format!("missed {} of {}", a.missed, a.games) } })
                }).collect::<Vec<_>>(),
            })
        })
        .collect();
    json!({ "total": total, "seasons": groups, "empty": all.is_empty() })
}

/// Season-by-season lines plus career totals, regular season and playoffs.
fn player_stats(lg: &League, p: &Player) -> Value {
    let offseason = lg.phase == LeaguePhase::Offseason;
    let now_team = lg.teams.iter().position(|t| t.roster.iter().any(|x| x.id == p.id));
    let table = |playoffs: bool| -> Value {
        let rows = if playoffs { lg.playoff_rows.get(&p.id) } else { lg.season_rows.get(&p.id) };
        let rows: Vec<&sp_league::league::SeasonRow> = rows.map(|r| r.iter().collect()).unwrap_or_default();
        let cur: stats::Stats = if offseason { Default::default() } else { (if playoffs { &lg.playoff_stats } else { &lg.season_stats }).get(&p.id).map(|s| s.to_map()).unwrap_or_default() };
        let mut career: stats::Stats = Default::default();
        for m in rows.iter().map(|r| &r.stats).chain(std::iter::once(&cur)) {
            for (k, v) in m {
                let e = career.entry(k.clone()).or_insert(0.0);
                if k.ends_with("_long") { *e = e.max(*v) } else { *e += v }
            }
        }
        let cols = stats::player_columns(p.position, &career);
        let mut lines: Vec<(String, String, stats::Stats)> = rows.iter().map(|r| (format!("S{}", r.season), r.abbr.clone().unwrap_or_else(|| if r.team >= 0 { lg.teams[r.team as usize].town.abbr() } else { "FA".into() }), r.stats.clone())).collect();
        // no current-season line for a player who has retired (or never signed): he has no season to show
        let playing = now_team.is_some() || lg.free_agents.iter().any(|f| f.id == p.id) || !cur.is_empty();
        if !offseason && playing && (!playoffs || !cur.is_empty()) {
            lines.push((format!("S{}", lg.season), now_team.map(|t| lg.teams[t].town.abbr()).unwrap_or_else(|| "FA".into()), cur.clone()));
        }
        let mut out: Vec<Value> = lines.iter().map(|(y, t, m)| json!({ "yr": y, "team": t, "cells": cols.iter().map(|c| (c.1)(m)).collect::<Vec<_>>(), "total": false })).collect();
        if !rows.is_empty() {
            out.push(json!({ "yr": if playoffs { "Total" } else { "Career" }, "team": "", "cells": cols.iter().map(|c| (c.1)(&career)).collect::<Vec<_>>(), "total": true }));
        }
        json!({ "heads": cols.iter().map(|c| c.0).collect::<Vec<_>>(), "rows": out })
    };
    json!({ "regular": table(false), "playoffs": table(true) })
}

pub fn negotiation(lg: &mut League, id: u32, seasons: i32) -> Value {
    lg.sync_awards_for(id);
    let win = lg.my_win_pct();
    let premium = lg.resign_premium(id);
    let town = lg.me().town.clone();
    let Some(p) = lg.me().roster.iter().chain(lg.free_agents.iter()).find(|p| p.id == id).cloned() else { return json!({}) };
    let want = sp_economy::Negotiation::target(&p, &town, seasons, win, premium.as_ref());
    json!({
        "id": id, "name": p.name, "pos": p.position.label(), "ovr": p.ovr(), "want": want, "wantText": money(want),
        "awardNote": sp_economy::PayScale::award_note(p.awards),
        "notes": premium.map(|r| r.notes()).unwrap_or_default(),
        "maxRounds": sp_economy::Negotiation::MAX_ROUNDS,
    })
}

// -------------------------------------------------------------------- finances

use sp_economy::{CashStatus, LedgerKind};
use sp_economy::business::{Facility, promo_templates};

fn kind_name(k: LedgerKind) -> &'static str {
    use LedgerKind::*;
    match k {
        Gate => "Gate", Concessions => "Concessions", Merch => "Merch", Sponsors => "Sponsors", LeagueShare => "League share", PlayoffBonus => "Playoff bonus",
        PlayerPay => "Player pay", Retainers => "Reserve pay", Rent => "Stadium rent", Travel => "Travel", Insurance => "Insurance", Medical => "Medical", Other => "Other",
        Promo => "Promo nights", Upkeep => "Stadium upkeep", Interest => "Loan interest", LoanPayment => "Loan payments", LoanProceeds => "Loan proceeds", Facilities => "Facilities", SponsorCost => "Sponsor costs",
    }
}

fn bars(map: &std::collections::BTreeMap<LedgerKind, i64>) -> Value {
    let mut v: Vec<(&str, i64)> = map.iter().filter(|(_, a)| **a != 0).map(|(k, a)| (kind_name(*k), *a)).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    Value::Array(v.into_iter().map(|(n, a)| json!({ "name": n, "amount": a, "text": money(a) })).collect())
}

fn sum_kinds<'a>(it: impl Iterator<Item = &'a std::collections::BTreeMap<LedgerKind, i64>>) -> std::collections::BTreeMap<LedgerKind, i64> {
    let mut out = std::collections::BTreeMap::new();
    for m in it {
        for (&k, &v) in m {
            *out.entry(k).or_insert(0) += v;
        }
    }
    out
}

fn effect_json(c: Option<&sp_economy::business::SponsorClause>) -> Value {
    match c {
        None => json!({ "text": "No downside", "tone": "good" }),
        Some(c) => {
            let good = (c.fan_per_game > 0.0 || c.att_mult > 1.0 || c.win_bonus > 0 || c.pay_mult >= 1.3) && c.fan_per_game >= 0.0 && c.att_mult >= 1.0 && c.cost_per_game == 0;
            let bad = c.fan_per_game < 0.0 || c.att_mult < 1.0 || c.cost_per_game > 0 || c.min_win_pct > 0.0 || c.pay_mult < 1.0;
            let text = c.effects();
            let tone = if bad { "warn" } else if good || text == "No downside" { "good" } else { "neutral" };
            json!({ "text": text, "tone": tone })
        }
    }
}

pub fn finances(lg: &League) -> Value {
    use sp_economy::{EconTuning, TownEconomy};
    let me = lg.me();
    let u = lg.user();
    let f = lg.my_finance();
    let town = &f.town;
    let net = f.season_net();
    let (status, tone) = match f.status(me) {
        CashStatus::Ok => ("Solvent", "good"),
        CashStatus::Low => ("Cash is low", "warn"),
        CashStatus::Negative => ("Overdrawn", "bad"),
        CashStatus::FinalNotice => ("Final notice from the bank", "bad"),
        CashStatus::Bankrupt => ("Bankrupt", "bad"),
    };
    let last = f.games.last();
    let home: Vec<&sp_economy::GameFinance> = f.games.iter().filter(|g| g.home).collect();
    let rev = sum_kinds(f.games.iter().map(|g| &g.revenue));
    let exp = sum_kinds(f.games.iter().map(|g| &g.expenses));
    let total_rev: i64 = rev.values().sum();
    let total_exp: i64 = exp.values().sum();

    // tickets
    let price = f.ticket_price;
    let att = |p: f64| lg.project_attendance(p, sp_core::Weather::Clear, 0.5);
    let mut home_weeks: Vec<i32> = lg.schedule.iter().filter(|g| g.home == u && g.week >= lg.week).map(|g| g.week).collect();
    home_weeks.sort();
    let in_season = lg.phase == LeaguePhase::RegularSeason;
    let next_promo = home_weeks.first().map(|&w| f.promo_for(w).is_some()).unwrap_or(false);
    let band = lg.project_attendance_band(price, sp_core::Weather::Clear, 0.5, next_promo);
    let conc = TownEconomy::concession_per_cap(town) * f.concession_mult();
    let cur = att(price);
    let proj: Vec<Value> = [price - 2.0, price, price + 2.0, price + 4.0].iter().filter(|&&p| p >= 1.0).map(|&p| json!({ "price": format!("{:.2}", p), "current": (p - price).abs() < 1e-9, "fans": att(p), "take": money((att(p) as f64 * (p + conc)).round() as i64) })).collect();
    let unavailable: std::collections::HashSet<u32> = lg.injured_games.keys().copied().collect();
    let favorites: Vec<Value> = lg.fan_favorites(me, &unavailable, false).iter().map(|fv| json!({ "name": fv.name, "pos": me.roster.iter().find(|p| p.id == fv.player_id).map(|p| p.position.label()).unwrap_or(""), "reason": fv.reason, "boost": format!("{:.1}", fv.boost * 100.0), "out": lg.out_reason.get(&fv.player_id) })).collect();
    let promos_set: Vec<Value> = f.promos.iter().map(|(&w, id)| json!({ "week": w, "name": sp_economy::business::promo_by_id(id).map(|p| p.name.clone()).unwrap_or_default(), "played": w < lg.week || !in_season })).collect();
    let promo_list: Vec<Value> = promo_templates()
        .iter()
        .map(|p| json!({
            "id": p.id, "name": p.name, "cost": p.cost(town), "costText": money(p.cost(town)), "crowd": format!("{:.2}", p.attendance_mult), "risk": (p.risk_chance * 100.0).round() as i32,
            "riskText": p.risk_text, "fine": if p.mishap_fine_mult > 0.0 { Some(money((p.cost(town) as f64 * p.mishap_fine_mult).round() as i64)) } else { None },
            "weeks": home_weeks.iter().take(8).map(|&w| json!({ "week": w, "on": f.promos.get(&w).map(|x| x == &p.id).unwrap_or(false), "locked": f.promos.get(&w).map(|x| x != &p.id).unwrap_or(false) })).collect::<Vec<_>>(),
        }))
        .collect();

    // sponsors
    let slots = f.sponsor_slots();
    let sponsors: Vec<Value> = f.sponsors.iter().map(|s| json!({ "name": s.name, "perGame": money(s.per_game), "seasons": s.seasons_left, "clause": s.clause, "effect": effect_json(s.effects()) })).collect();
    let offers: Vec<Value> = f.offers.iter().map(|o| json!({ "name": o.name, "perGame": money(o.per_game), "seasons": o.seasons, "clause": o.clause, "expires": o.expires_week, "effect": effect_json(o.effects()) })).collect();

    // facilities
    let facilities: Vec<Value> = Facility::ALL
        .iter()
        .map(|&fac| {
            let spec = fac.spec();
            let lvl = f.level(fac);
            let cost = f.upgrade_cost(fac);
            json!({
                "key": spec.key, "name": spec.name, "level": lvl, "max": spec.max_level, "effect": spec.effect, "upkeep": money(spec.upkeep_per_level),
                "cost": cost, "costText": cost.map(money), "canBuy": cost.map(|c| f.cash >= c && !f.under_debt_covenant()).unwrap_or(false),
                "rebuild": money(spec.cost_at((lvl - 1).max(0))),
            })
        })
        .collect();

    // payroll
    let mut rows: Vec<(&Player, &sp_economy::Contract, bool)> = me.roster.iter().filter_map(|p| f.contracts.get(&p.id).map(|c| (p, c, me.active_ids.contains(&p.id)))).collect();
    rows.sort_by(|a, b| b.1.ppg.cmp(&a.1.ppg));
    let active_pay: i64 = rows.iter().filter(|r| r.2).map(|r| r.1.active_pay()).sum();
    let retainer: i64 = rows.iter().filter(|r| !r.2).map(|r| sp_economy::PayScale::retainer(r.1.ppg)).sum();
    let payroll: Vec<Value> = rows.iter().map(|(p, c, a)| json!({ "id": p.id, "name": p.name, "pos": p.position.label(), "ovr": p.ovr(), "active": a, "years": c.seasons_left, "pay": money(if *a { c.active_pay() } else { sp_economy::PayScale::retainer(c.ppg) }) })).collect();

    // loan
    let limit = f.credit_limit();

    // council
    let o = lg.council_outcome();
    let goals: Vec<Value> = f.goals.iter().map(|g| json!({ "label": g.label(), "reward": money(g.reward), "met": g.is_met(&o), "progress": g.progress(&o) })).collect();
    let all_bonus = sp_economy::CouncilGoals::grant(&f.goals.iter().map(|g| sp_economy::council::GoalResult { goal: g.clone(), met: true }).collect::<Vec<_>>());
    let reports: Vec<Value> = f
        .reports
        .iter()
        .rev()
        .take(5)
        .map(|r| json!({
            "season": r.season, "record": r.record, "finish": r.finish, "avgCrowd": r.avg_crowd, "capacity": r.capacity, "fanInterest": r.fan_interest.round(), "cashEnd": money(r.cash_end),
            "revenue": money(r.revenue.values().sum()), "expenses": money(r.expenses.values().sum()), "net": r.revenue.values().sum::<i64>() - r.expenses.values().sum::<i64>(),
            "leagueAvgRevenue": money(r.league_avg_revenue), "leagueAvgNet": money(r.league_avg_net), "grant": money(r.grant),
            "goals": r.goals.iter().map(|g| json!({ "label": g.goal.label(), "met": g.met, "reward": money(g.goal.reward) })).collect::<Vec<_>>(),
        }))
        .collect();

    json!({
        "header": header(lg),
        "cash": f.cash, "cashText": money(f.cash), "seasonNet": net, "seasonNetText": money(net.abs()), "status": status, "statusTone": tone, "loan": f.loan_balance, "loanText": money(f.loan_balance),
        "lastGame": last.map(|g| money(g.net())), "lastGameNet": last.map(|g| g.net()),
        "avgCrowd": if home.is_empty() { None } else { Some((home.iter().map(|g| g.attendance).sum::<i64>() as f64 / home.len() as f64).round() as i64) }, "capacity": f.capacity(),
        "fanInterest": f.fan_interest.round(),
        "revenue": bars(&rev), "revenueTotal": money(total_rev), "expenses": bars(&exp), "expensesTotal": money(total_exp), "hasGames": !f.games.is_empty(),
        "tickets": {
            "price": price, "priceText": format!("{:.2}", price), "typical": cur, "full": ((cur as f64 / f.capacity().max(1) as f64) * 100.0).round() as i32,
            "lo": band.lo, "hi": band.hi, "nextPromo": next_promo, "gate": money((cur as f64 * price).round() as i64), "concessions": money((cur as f64 * conc).round() as i64), "proj": proj,
            "favorites": favorites, "promosSet": promos_set, "promos": promo_list, "homeWeeksLeft": home_weeks.len(),
        },
        "sponsors": { "list": sponsors, "offers": offers, "slots": slots, "fanInterest": f.fan_interest.round(), "nextSlot": f.next_slot_interest().map(|x| x.round()), "full": f.sponsors.len() >= slots },
        "facilities": { "list": facilities, "upkeep": money(f.upkeep_per_game()), "frozen": f.under_debt_covenant(), "frozenBelow": money((limit as f64 * EconTuning::UPGRADE_DEBT_COVENANT).round() as i64) },
        "payroll": { "active": money(active_pay), "reserve": money(retainer), "season": money((active_pay + retainer) * 14), "rows": payroll },
        "loanInfo": { "balance": f.loan_balance, "balanceText": money(f.loan_balance), "limit": money(limit), "room": limit - f.loan_balance, "roomText": money(limit - f.loan_balance), "rate": format!("{:.1}", f.loan_rate() * 100.0), "minPay": (EconTuning::LOAN_MIN_PAY_PCT * 100.0).round() as i32, "cash": f.cash },
        "council": { "goals": goals, "allBonus": money(all_bonus), "reports": reports, "season": lg.season },
    })
}

// -------------------------------------------------------------------- strategy

fn fit_json(r: &sp_league::insights::FitResult) -> Value {
    json!({ "fit": r.fit.label(), "reason": r.reason, "parts": r.parts.iter().map(|p| json!({ "text": p.text, "fit": p.fit.label() })).collect::<Vec<_>>() })
}

pub fn strategy(lg: &League) -> Value {
    use sp_core::strategy::*;
    let me = lg.me();
    let st = me.strategy;
    let o = st.offense;
    let schemes: Vec<Value> = [
        (OffenseScheme::ProStyle, "proStyle", "Pro-Style"), (OffenseScheme::Spread, "spread", "Spread"), (OffenseScheme::AirRaid, "airRaid", "Air Raid"),
        (OffenseScheme::GroundAndPound, "groundAndPound", "Ground & Pound"), (OffenseScheme::Option, "option", "Option"), (OffenseScheme::Balanced, "balanced", "Balanced"),
    ]
    .iter()
    .map(|&(s, id, name)| { let mut v = fit_json(&Insights::offense_scheme(me, s)); v["id"] = json!(id); v["name"] = json!(name); v["on"] = json!(o.scheme == s); v })
    .collect();
    let fronts: Vec<Value> = [(DefenseFront::FourThree, "fourThree", "4-3"), (DefenseFront::ThreeFour, "threeFour", "3-4"), (DefenseFront::Nickel, "nickel", "Nickel")]
        .iter()
        .map(|&(f, id, name)| { let mut v = fit_json(&Insights::defense_front(me, f)); v["id"] = json!(id); v["name"] = json!(name); v["on"] = json!(st.defense.front == f); v })
        .collect();
    let mut ks: Vec<&Player> = me.active().filter(|p| p.position == Position::K || p.also_kicks_punts).collect();
    ks.sort_by(|a, b| b.ovr_at(Position::K).cmp(&a.ovr_at(Position::K)));
    let kicker = ks.first().map(|k| format!("{} (ST {}, DI {})", k.name, k.ratings[Attr::Strength], k.ratings[Attr::Discipline])).unwrap_or_else(|| "none active".into());
    let opp = next_game(lg).map(|n| { let rep = Insights::opponent(&lg.teams[n.opponent]); json!({ "team": team_ref(lg, n.opponent), "summary": rep.summary, "tips": rep.tips }) });
    json!({
        "header": header(lg),
        "strategy": serde_json::to_value(st).unwrap_or(Value::Null),
        "schemes": schemes, "fronts": fronts,
        "pocket": fit_json(&Insights::offense_slider(me, "pocket", 0.0, Some(o.pocket))),
        "runPass": fit_json(&Insights::offense_slider(me, "runPass", o.run_pass_bias, None)),
        "depth": fit_json(&Insights::offense_slider(me, "depth", o.pass_depth, None)),
        "tempo": fit_json(&Insights::offense_slider(me, "tempo", o.tempo, None)),
        "blitz": fit_json(&Insights::blitz(me, st.defense.blitz_frequency)),
        "kicker": kicker, "opponent": opp,
    })
}

// -------------------------------------------------------------------- depth chart

use sp_core::returners::{RETURNER_ATTRS, ReturnRole, return_score};

fn offense_packages() -> Vec<(&'static str, Vec<(Position, usize)>)> {
    use Position::*;
    vec![
        ("Base (3-WR)", vec![(Qb, 1), (Rb, 1), (Wr, 3), (Te, 1), (Ot, 2), (Og, 2), (C, 1)]),
        ("Pro Set", vec![(Qb, 1), (Rb, 1), (Fb, 1), (Wr, 2), (Te, 1), (Ot, 2), (Og, 2), (C, 1)]),
        ("2-TE", vec![(Qb, 1), (Rb, 1), (Wr, 2), (Te, 2), (Ot, 2), (Og, 2), (C, 1)]),
        ("Heavy", vec![(Qb, 1), (Rb, 1), (Fb, 1), (Wr, 1), (Te, 2), (Ot, 2), (Og, 2), (C, 1)]),
        ("Spread (4-WR)", vec![(Qb, 1), (Rb, 1), (Wr, 4), (Ot, 2), (Og, 2), (C, 1)]),
    ]
}

fn defense_packages() -> Vec<(&'static str, Vec<(Position, usize)>)> {
    use Position::*;
    vec![
        ("4-3", vec![(De, 2), (Dt, 2), (Mlb, 1), (Olb, 2), (Cb, 2), (S, 2)]),
        ("Nickel", vec![(De, 2), (Dt, 2), (Mlb, 1), (Olb, 1), (Cb, 3), (S, 2)]),
        ("3-4", vec![(De, 2), (Dt, 1), (Mlb, 2), (Olb, 2), (Cb, 2), (S, 2)]),
        ("Goal line", vec![(De, 2), (Dt, 2), (Mlb, 2), (Olb, 2), (Cb, 2), (S, 1)]),
    ]
}

fn key_attrs(pos: Position) -> [Attr; 3] {
    use Attr::*;
    use Position::*;
    match pos {
        Qb => [Strength, Discipline, Intelligence], Rb => [Speed, Acceleration, Hands], Fb => [Strength, Hands, Endurance], Wr => [Speed, Hands, Agility],
        Te => [Hands, Strength, Agility], Ot | Og | C => [Strength, Intelligence, Discipline], De => [Acceleration, Strength, Agility], Dt => [Strength, Intelligence, Acceleration],
        Mlb => [Intelligence, Strength, Speed], Olb => [Speed, Intelligence, Agility], Cb => [Speed, Agility, Hands], S => [Intelligence, Speed, Hands], K | P => [Strength, Discipline, Strength],
    }
}

fn depth_row(lg: &League, p: &Player, rate: i32, attrs: &[Attr], off_pos: bool) -> Value {
    let me = lg.me();
    json!({
        "id": p.id, "name": p.name, "number": me.numbers.get(&p.id).copied().or(p.jersey), "pos": p.position.label(), "rate": rate, "ovr": p.ovr(), "out": lg.injured_games.get(&p.id), "offPos": off_pos,
        "attrs": attrs.iter().map(|&a| json!({ "code": a.code(), "v": p.ratings[a] })).collect::<Vec<_>>(),
    })
}

pub fn depth(lg: &League) -> Value {
    use sp_core::strategy::{DefenseFront, OffenseScheme};
    let me = lg.me();
    let units: [(&str, Vec<(Position, &str)>); 3] = [
        ("Offense", vec![(Position::Qb, "Quarterback"), (Position::Rb, "Running back"), (Position::Fb, "Fullback"), (Position::Wr, "Wide receiver"), (Position::Te, "Tight end"), (Position::Ot, "Offensive tackle"), (Position::Og, "Guard"), (Position::C, "Center")]),
        ("Defense", vec![(Position::De, "Defensive end"), (Position::Dt, "Defensive tackle"), (Position::Mlb, "Middle linebacker"), (Position::Olb, "Outside linebacker"), (Position::Cb, "Cornerback"), (Position::S, "Safety")]),
        ("Special", vec![(Position::K, "Kicker"), (Position::P, "Punter")]),
    ];
    let off_default = match me.strategy.offense.scheme {
        OffenseScheme::ProStyle | OffenseScheme::Option => "Pro Set",
        OffenseScheme::Spread | OffenseScheme::AirRaid => "Spread (4-WR)",
        OffenseScheme::GroundAndPound => "Heavy",
        OffenseScheme::Balanced => "Base (3-WR)",
    };
    let def_default = match me.strategy.defense.front { DefenseFront::FourThree => "4-3", DefenseFront::ThreeFour => "3-4", DefenseFront::Nickel => "Nickel" };
    let pkg_json = |list: Vec<(&'static str, Vec<(Position, usize)>)>| -> Value {
        Value::Array(list.into_iter().map(|(n, m)| json!({ "name": n, "slots": m.iter().map(|(p, c)| (p.label().to_string(), json!(c))).collect::<serde_json::Map<_, _>>() })).collect())
    };
    let unit_json: Vec<Value> = units
        .iter()
        .map(|(unit, groups)| {
            let gs: Vec<Value> = groups
                .iter()
                .map(|&(pos, title)| {
                    let attrs = key_attrs(pos);
                    let rows: Vec<Value> = me.depth_order(pos).iter().map(|p| depth_row(lg, p, p.ovr_at(pos), &attrs, p.position != pos)).collect();
                    json!({ "key": pos.label(), "title": title, "tag": pos.label(), "custom": me.depth.contains_key(&pos), "rows": rows })
                })
                .collect();
            json!({ "name": unit, "groups": gs })
        })
        .collect();
    let returns: Vec<Value> = ReturnRole::ALL
        .iter()
        .map(|&role| {
            let listed: std::collections::HashSet<u32> = me.return_depth.get(&role).into_iter().flatten().copied().collect();
            let full = me.return_order(role);
            let mut shown: Vec<&Player> = full.iter().copied().filter(|p| listed.contains(&p.id)).collect();
            shown.extend(full.iter().copied().filter(|p| !listed.contains(&p.id)));
            shown.truncate(listed.len().max(4));
            let rows: Vec<Value> = shown.iter().map(|p| depth_row(lg, p, return_score(p).round() as i32, &RETURNER_ATTRS, false)).collect();
            json!({ "key": role.tag(), "title": role.label(), "tag": role.tag(), "custom": me.return_depth.contains_key(&role), "rows": rows })
        })
        .collect();
    let custom_count = me.depth.len() + me.return_depth.len();
    json!({
        "header": header(lg), "units": unit_json, "returns": returns, "customCount": custom_count, "autoDepth": lg.auto_depth,
        "offensePackages": pkg_json(offense_packages()), "defensePackages": pkg_json(defense_packages()), "offDefault": off_default, "defDefault": def_default,
    })
}

/// Active players who could be listed at `key` (a position label, or KR / PR), best fit first.
pub fn depth_candidates(lg: &League, key: &str) -> Value {
    let me = lg.me();
    let (rows, attrs): (Vec<(&Player, i32)>, Vec<Attr>) = if let Some(pos) = Position::ALL.iter().copied().find(|p| p.label() == key) {
        let mut v: Vec<(&Player, i32)> = me.active().map(|p| (p, p.ovr_at(pos))).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        (v, key_attrs(pos).to_vec())
    } else {
        let mut v: Vec<(&Player, i32)> = me.active().map(|p| (p, return_score(p).round() as i32)).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        (v, RETURNER_ATTRS.to_vec())
    };
    Value::Array(rows.iter().take(40).map(|(p, r)| depth_row(lg, p, *r, &attrs, true)).collect())
}

// -------------------------------------------------------------------- trades

use sp_league::trade::{PickRef, TradeAdd, TradeMood, TradeOffer, TradeTuning};

fn pick_key(p: &PickRef) -> String {
    p.key()
}

fn parse_pick(s: &str) -> Option<PickRef> {
    let mut it = s.split(':');
    Some(PickRef { season: it.next()?.parse().ok()?, round: it.next()?.parse().ok()?, original_team: it.next()?.parse().ok()? })
}

pub fn parse_offer(v: &Value) -> TradeOffer {
    let ids = |k: &str| -> std::collections::BTreeSet<u32> { v[k].as_array().map(|a| a.iter().filter_map(|x| x.as_u64().map(|n| n as u32)).collect()).unwrap_or_default() };
    let picks = |k: &str| -> Vec<PickRef> { v[k].as_array().map(|a| a.iter().filter_map(|x| x.as_str().and_then(parse_pick)).collect()).unwrap_or_default() };
    TradeOffer {
        partner: v["partner"].as_u64().unwrap_or(0) as usize,
        give_players: ids("givePlayers"),
        get_players: ids("getPlayers"),
        give_picks: picks("givePicks"),
        get_picks: picks("getPicks"),
        give_cash: v["giveCash"].as_i64().unwrap_or(0),
        get_cash: v["getCash"].as_i64().unwrap_or(0),
    }
}

fn trade_player(lg: &League, p: &Player, from: usize, to: usize) -> Value {
    let line = lg.trade_line(p, from, to);
    let lock = lg.untradeable_reason(from, p);
    let out = lg.injured_games.get(&p.id).copied().unwrap_or(0);
    let deal = match &line.contract {
        None => "no contract".to_string(),
        Some(c) => format!("{}/g{} · {} yr", money(c.ppg), if line.new_commute > 0 { format!(" + {} commute", money(line.new_commute)) } else { " · local".into() }, c.seasons_left),
    };
    json!({
        "id": p.id, "name": p.name, "pos": p.position.label(), "ovr": p.ovr(), "age": p.age, "lock": lock, "out": out, "deal": deal,
        "line": trade_line_json(lg, &line),
    })
}

fn trade_line_json(lg: &League, l: &sp_league::trade::TradePlayerLine) -> Value {
    json!({
        "name": l.player.name, "pos": l.player.position.label(), "ovr": l.player.ovr(), "age": l.player.age, "hometown": l.player.hometown, "job": l.player.day_job,
        "ppg": l.contract.as_ref().map(|c| money(c.ppg)), "winBonus": l.contract.as_ref().map(|c| money(c.win_bonus)),
        "newCommute": if l.new_commute > 0 { money(l.new_commute) } else { "none".into() }, "oldCommute": if l.old_commute > 0 { money(l.old_commute) } else { "none".into() },
        "years": l.seasons_left, "commitment": money(l.commitment), "market": money(l.market), "surplus": l.surplus_per_game(), "surplusText": money(l.surplus_per_game().abs()),
        "release": money(l.release_cost), "toName": lg.teams[l.to_team].town.name, "toMe": l.to_team == lg.user(),
    })
}

fn trade_pick(lg: &League, p: &PickRef) -> Value {
    let w = lg.pick_worth(p, None);
    let tier = if w >= 330.0 { "PREMIUM" } else if w >= 150.0 { "HIGH" } else if w >= 50.0 { "SOLID" } else { "LATE" };
    let seasons = sp_economy::rookie_scale::RookieScale::seasons_for_round(p.round);
    let first = lg.tradable_pick_seasons().first().copied();
    json!({
        "key": pick_key(p), "label": lg.pick_label(p), "tier": tier,
        "note": if Some(p.season) == first { format!("Rookie contract: {} season{}", seasons, if seasons == 1 { "" } else { "s" }) } else { "Future pick, order not set yet".to_string() },
    })
}

pub fn trade_desk(lg: &League, partner: Option<usize>) -> Value {
    let u = lg.user();
    let others: Vec<usize> = (0..lg.teams.len()).filter(|&i| i != u && !lg.finances[i].bankrupt).collect();
    let t = partner.filter(|p| others.contains(p)).unwrap_or_else(|| others[0]);
    let closed = lg.trade_closed_reason();
    let mood = lg.trade_mood(t);
    let mut theirs: Vec<&Player> = lg.teams[t].roster.iter().collect();
    theirs.sort_by(|a, b| b.ovr().cmp(&a.ovr()));
    let mut mine: Vec<&Player> = lg.teams[u].roster.iter().collect();
    mine.sort_by(|a, b| b.ovr().cmp(&a.ovr()));
    json!({
        "header": header(lg),
        "closed": closed,
        "windowText": closed.clone().unwrap_or_else(|| if lg.phase == LeaguePhase::Offseason { "The window is open all offseason.".to_string() } else { format!("The window closes after week {}.", TradeTuning::DEADLINE_WEEK) }),
        "teams": others.iter().map(|&i| team_ref(lg, i)).collect::<Vec<_>>(),
        "partner": t,
        "partnerInfo": { "team": team_ref(lg, t), "mood": mood, "roster": lg.teams[t].roster.len(), "cash": money(lg.finances[t].cash),
            "blurb": match mood { "Rebuilding" => "Prizes picks and young players. Sells veterans cheaper.", "Contending" => "Prizes veterans. Holds on to its picks.", _ => "Takes value where it finds it." } },
        "getPlayers": theirs.iter().map(|p| trade_player(lg, p, t, u)).collect::<Vec<_>>(),
        "givePlayers": mine.iter().map(|p| trade_player(lg, p, u, t)).collect::<Vec<_>>(),
        "getPicks": lg.picks_of(t).iter().map(|p| trade_pick(lg, p)).collect::<Vec<_>>(),
        "givePicks": lg.picks_of(u).iter().map(|p| trade_pick(lg, p)).collect::<Vec<_>>(),
        "myCash": money(lg.my_finance().cash),
    })
}

pub fn trade_eval(lg: &League, o: &TradeOffer) -> Value {
    if o.is_empty() {
        return json!({ "empty": true });
    }
    let v = lg.evaluate_trade(o);
    let b = lg.trade_book(o);
    let signed = |x: i64| format!("{}{}", if x >= 0 { "+" } else { "-" }, money(x.abs()));
    let ask = v.ask.as_ref().map(|a| match a {
        TradeAdd::Player { label, player_id } => json!({ "label": label, "kind": "player", "playerId": player_id }),
        TradeAdd::Pick { label, pick } => json!({ "label": label, "kind": "pick", "pick": pick_key(pick) }),
        TradeAdd::Cash { label, cash } => json!({ "label": label, "kind": "cash", "cash": cash }),
    });
    let line_rows = |ls: &[sp_league::trade::TradePlayerLine], tag: &str| -> Vec<Value> {
        ls.iter().map(|l| json!({ "tag": tag, "name": l.player.name, "pos": l.player.position.label(), "ovr": l.player.ovr(),
            "text": format!("{} {}{} · {} yr · {} {} in all · market {}/g",
                l.contract.as_ref().map(|c| format!("{}/g", money(c.ppg))).unwrap_or_else(|| "no contract".into()),
                if l.new_commute > 0 { format!("+{} commute", money(l.new_commute)) } else { "no commute".into() },
                if l.new_commute != l.old_commute { format!(" (was {})", if l.old_commute > 0 { money(l.old_commute) } else { "none".into() }) } else { String::new() },
                l.seasons_left, if l.to_team == lg.user() { "you pay" } else { "they pay" }, money(l.commitment), money(l.market)) })).collect()
    };
    let mut lines = line_rows(&b.incoming, "IN");
    lines.extend(line_rows(&b.outgoing, "OUT"));
    json!({
        "empty": false,
        "accepted": v.accepted, "blocked": v.blocked, "mood": match v.mood { TradeMood::Yes => "yes", TradeMood::Close => "close", TradeMood::No => "no" },
        "line": v.line, "notes": v.notes, "ratio": v.ratio().clamp(0.0, 1.0), "ask": ask,
        "book": {
            "payrollDelta": b.payroll_delta, "payroll": signed(b.payroll_delta), "contracts": signed(b.commitment_in - b.commitment_out), "contractsSub": format!("in {} · out {}", money(b.commitment_in), money(b.commitment_out)),
            "cash": signed(b.cash_delta), "cashNeg": b.cash_delta < 0, "cashAfter": money(b.cash_after), "roster": b.roster_after, "rosterNow": lg.me().roster.len(), "warnings": b.warnings, "lines": lines,
        },
    })
}

// ------------------------------------------------------------------ league hub tabs

/// A helmet slug for a team known only by its full name (award finalists, past champions). Falls back to the last word.
fn helmet_for_name(lg: &League, name: &str) -> String {
    if let Some(t) = lg.teams.iter().find(|t| t.town.team_name() == name) {
        return helmet_slug(&t.town.nickname);
    }
    if let Some(t) = lg.region.all_towns().find(|t| t.team_name() == name) {
        return helmet_slug(&t.nickname);
    }
    helmet_slug(name.rsplit(' ').next().unwrap_or(name))
}

pub fn league_schedule(lg: &League) -> Value {
    let Some(u) = lg.user_team else { return json!({ "games": [] }) };
    let games: Vec<Value> = lg
        .team_schedule(u)
        .iter()
        .map(|g| {
            let home = g.home == u;
            let opp = if home { g.away } else { g.home };
            let rec = lg.results.iter().position(|r| !r.playoff && r.week == g.week && r.home == g.home && r.away == g.away);
            let result = rec.map(|i| {
                let r = &lg.results[i];
                let (my, their) = if home { (r.home_score, r.away_score) } else { (r.away_score, r.home_score) };
                format!("{} {}–{}", if my > their { "W" } else if my < their { "L" } else { "T" }, my, their)
            });
            let rival = is_rivalry(&lg.teams[u].town.id, &lg.teams[opp].town.id);
            json!({ "week": g.week, "home": home, "opp": team_ref(lg, opp), "result": result, "rival": rival,
                    "next": result.is_none() && g.week == lg.week && lg.phase == LeaguePhase::RegularSeason,
                    "box": rec.filter(|&i| lg.results[i].box_score.is_some()) })
        })
        .collect();
    json!({ "games": games })
}

pub fn league_playoffs(lg: &League) -> Value {
    let Some(po) = lg.playoffs.as_ref() else { return json!({ "set": false }) };
    let game = |g: &sp_league::playoffs::PlayoffGame, label: &str| {
        json!({ "label": label, "away": team_ref(lg, g.low_seed), "home": team_ref(lg, g.high_seed), "awaySeed": po.seed_of(g.low_seed), "homeSeed": po.seed_of(g.high_seed),
                "score": g.result.as_ref().map(|r| format!("{}–{}", r.away_score, r.home_score)) })
    };
    let mut games: Vec<Value> = po.semifinals.iter().map(|g| game(g, "Semifinal")).collect();
    if let Some(f) = &po.final_game {
        games.push(game(f, "Final"));
    }
    let seeds: Vec<Value> = po.seeds.iter().enumerate().map(|(i, &t)| json!({ "seed": i + 1, "team": team_ref(lg, t), "isMe": Some(t) == lg.user_team })).collect();
    json!({ "set": true, "seeds": seeds, "games": games, "champion": po.champion.map(|c| lg.teams[c].town.team_name()) })
}

pub fn league_awards(lg: &League) -> Value {
    let me = lg.user_team.map(|u| lg.teams[u].town.team_name()).unwrap_or_default();
    let locked = lg.awards_history.last().map(|a| a.season) == Some(lg.season);
    let awards = if locked { lg.awards_history.last().cloned().unwrap() } else { lg.award_race(false) };
    let status = if locked {
        format!("Season {} final", awards.season)
    } else if awards.games_played == 0 {
        format!("Season {}: no games yet. The race starts after week 1.", awards.season)
    } else {
        format!("Season {} race, after {} game{}", awards.season, awards.games_played, if awards.games_played == 1 { "" } else { "s" })
    };
    let cats: Vec<Value> = awards
        .categories
        .iter()
        .map(|c| {
            let f: Vec<Value> = c.finalists.iter().map(|f| json!({ "playerId": f.player_id, "name": f.name, "pos": f.pos, "team": f.team, "helmet": helmet_for_name(lg, &f.team), "line": f.line, "mine": f.team == me })).collect();
            json!({ "id": c.id, "title": c.title, "blurb": c.blurb, "finalists": f })
        })
        .collect();
    let past: Vec<Value> = lg
        .awards_history
        .iter()
        .rev()
        .skip(if locked { 1 } else { 0 })
        .map(|a| {
            let w: Vec<Value> = a.categories.iter().filter_map(|c| c.winner().map(|w| json!({ "id": c.id, "text": format!("{} · {}, {}", w.name, w.pos, w.team) }))).collect();
            json!({ "season": a.season, "winners": w })
        })
        .collect();
    json!({ "locked": locked, "status": status, "categories": cats, "past": past })
}

pub fn league_hall(lg: &League) -> Value {
    let nick_of = |abbr: &str, nick: &Option<String>| -> Option<String> {
        nick.clone()
            .or_else(|| lg.teams.iter().find(|t| t.town.abbr() == abbr).map(|t| t.town.nickname.clone()))
            .or_else(|| lg.region.all_towns().find(|t| t.abbr() == abbr).map(|t| t.nickname.clone()))
    };
    let row = |e: &sp_league::hall_of_fame::HallEntry| {
        let pos = sp_core::Position::ALL.iter().copied().find(|p| p.label() == e.pos).unwrap_or(Position::Qb);
        let number = e.number.unwrap_or_else(|| sp_core::jersey::pick_jersey_number(pos, e.player_id, &Default::default()));
        let last = e.name.split_once(' ').map(|(_, l)| l.to_string()).unwrap_or_else(|| e.name.clone());
        let team = nick_of(&e.abbr, &e.nickname).map(|n| helmet_slug(&n)).unwrap_or_default();
        json!({ "name": e.name, "pos": e.pos, "abbr": e.abbr, "firstSeason": e.first_season, "lastSeason": e.last_season, "seasons": e.seasons, "titles": e.titles,
                "awards": e.awards, "score": e.score.round() as i64, "line": e.line, "number": number, "lastName": last, "teamSlug": team })
    };
    let mut classes: Vec<Value> = Vec::new();
    for e in lg.inductees() {
        let r = row(e);
        match classes.last_mut() {
            Some(c) if c["season"] == e.last_season => c["rows"].as_array_mut().unwrap().push(r),
            _ => classes.push(json!({ "season": e.last_season, "rows": [r] })),
        }
    }
    let misses: Vec<Value> = lg.near_misses().into_iter().take(10).map(row).collect();
    json!({ "count": lg.inductees().len(), "bar": HofTuning::INDUCT.round() as i64, "classes": classes, "misses": misses })
}

pub fn league_history(lg: &League) -> Value {
    let mut seen: std::collections::BTreeMap<usize, usize> = Default::default();
    let mut rows: Vec<Value> = Vec::new();
    for h in &lg.history {
        let n = seen.entry(h.champion).or_default();
        *n += 1;
        let rec = h.records.get(h.champion).and_then(|r| r.rsplit(' ').next()).unwrap_or("").to_string();
        rows.push(json!({ "season": h.season, "name": h.champion_name, "helmet": helmet_for_name(lg, &h.champion_name), "record": rec, "title": *n, "isYou": Some(h.champion) == lg.user_team }));
    }
    rows.reverse();
    let top = lg.history.iter().map(|h| h.champion).fold(std::collections::BTreeMap::<usize, usize>::new(), |mut m, c| { *m.entry(c).or_default() += 1; m });
    let mut best: Option<(usize, usize)> = None;
    for h in &lg.history {
        let v = top[&h.champion];
        if best.is_none_or(|(_, bv)| v > bv) {
            best = Some((h.champion, v));
        }
    }
    let dynasty = best.map(|(k, v)| json!({ "name": lg.history.iter().rev().find(|h| h.champion == k).map(|h| h.champion_name.clone()), "titles": v }));
    json!({ "rows": rows, "dynasty": dynasty })
}

pub fn league_tab(lg: &League, tab: &str) -> Value {
    match tab {
        "schedule" => league_schedule(lg),
        "playoffs" => league_playoffs(lg),
        "awards" => league_awards(lg),
        "hall" => league_hall(lg),
        "history" => league_history(lg),
        _ => json!({}),
    }
}

// ------------------------------------------------------------------ news / gazette

fn story_json(lg: &League, issue: &sp_league::gazette::GazetteIssue, s: &sp_league::gazette::GazetteStory) -> Value {
    use sp_league::gazette::GazetteSection as S;
    let team = (s.team >= 0 && (s.team as usize) < lg.teams.len()).then_some(s.team as usize);
    // Player stories wear the shirt.
    let jersey = if matches!(s.section, S::Spotlight | S::Absences | S::Gossip) && s.player_id >= 0 {
        lg.teams.iter().find_map(|t| t.roster.iter().find(|p| p.id as i64 == s.player_id).map(|p| (t, p))).and_then(|(t, p)| {
            t.numbers.get(&p.id).copied().or(p.jersey).map(|n| json!({ "number": n, "teamSlug": helmet_slug(&t.town.nickname), "lastName": p.name.rsplit(' ').next().unwrap_or("") }))
        })
    } else {
        None
    };
    let game = s.game.as_ref().map(|g| json!({ "away": team_ref(lg, g.away), "home": team_ref(lg, g.home), "as": g.away_score, "hs": g.home_score, "tag": g.tag }));
    // The box score behind a story, while it is still in memory (this season's games; saves keep your own games only). Stories
    // without a recorded game use that club's game of the week.
    let box_idx = if issue.season != lg.season {
        None
    } else {
        let in_week = lg.results.iter().enumerate().filter(|(_, r)| r.week == issue.week && r.box_score.is_some());
        match &s.game {
            Some(g) => in_week.filter(|(_, r)| r.home == g.home && r.away == g.away).map(|(i, _)| i).next(),
            None => team.and_then(|t| in_week.filter(|(_, r)| r.home == t || r.away == t).map(|(i, _)| i).next()),
        }
    };
    json!({ "box": box_idx, "section": format!("{:?}", s.section), "headline": s.headline, "body": s.body, "helmet": team.map(|t| helmet_slug(&lg.teams[t].town.nickname)),
            "playerId": if s.player_id >= 0 { json!(s.player_id) } else { Value::Null }, "kind": s.kind, "game": game, "jersey": jersey })
}

pub fn news(lg: &League, picked: i64) -> Value {
    let region = lg.region.name.to_uppercase();
    let helmet = lg.user_team.map(|u| helmet_slug(&lg.teams[u].town.nickname));
    let n = lg.gazette.len();
    if n == 0 {
        return json!({ "empty": true, "region": region, "helmet": helmet });
    }
    let idx = if picked < 0 { n - 1 } else { (picked as usize).min(n - 1) };
    let issue = &lg.gazette[idx];
    let stories: Vec<Value> = issue.stories.iter().map(|s| story_json(lg, issue, s)).collect();
    json!({ "empty": false, "region": region, "helmet": helmet, "idx": idx, "count": n, "season": issue.season, "label": issue.label,
            "number": if issue.number > 0 { issue.number as usize } else { idx + 1 }, "stories": stories })
}

// ------------------------------------------------------------------ stats: leaders, team stats, box score

use crate::stats::{self, RowIn, Stats};

fn abbr_of(lg: &League, t: usize) -> String {
    lg.teams[t].town.abbr()
}

/// Rows for every rostered player that qualifies, with playoff or regular-season numbers.
fn stat_rows(lg: &League, c: &stats::Category, playoffs: bool, team: Option<usize>) -> Vec<RowIn> {
    let src = if playoffs { &lg.playoff_stats } else { &lg.season_stats };
    let mut out = Vec::new();
    for (t, tm) in lg.teams.iter().enumerate() {
        if team.is_some_and(|x| x != t) {
            continue;
        }
        for p in &tm.roster {
            let Some(s) = src.get(&p.id).map(|s| s.to_map()) else { continue };
            if !c.qualifies(&s, p.position) {
                continue;
            }
            out.push(RowIn { id: p.id, name: short_name(&p.name), team: abbr_of(lg, t), pos: p.position.label().to_string(), mine: Some(t) == lg.user_team, stats: s });
        }
    }
    out
}

/// Career totals across every finished season plus the one in progress, retired players included.
fn career_rows(lg: &League, c: &stats::Category, playoffs: bool, active_only: bool) -> Vec<RowIn> {
    let rows_by_id = if playoffs { &lg.playoff_rows } else { &lg.season_rows };
    let live = if playoffs { &lg.playoff_stats } else { &lg.season_stats };
    let offseason = lg.phase == LeaguePhase::Offseason; // the finished season is already in the rows
    let mut ids: std::collections::BTreeSet<u32> = rows_by_id.keys().copied().collect();
    if !offseason {
        ids.extend(live.keys().copied());
    }
    let mut who: std::collections::HashMap<u32, (&Player, i64)> = Default::default();
    for (t, tm) in lg.teams.iter().enumerate() {
        for p in &tm.roster {
            who.insert(p.id, (p, t as i64));
        }
    }
    for p in &lg.free_agents {
        who.entry(p.id).or_insert((p, -1));
    }
    let mut out = Vec::new();
    for id in ids {
        let active = who.get(&id);
        if active_only && active.is_none() {
            continue;
        }
        let mut total: Stats = Stats::new();
        let mut add = |m: &Stats| {
            for (k, v) in m {
                let e = total.entry(k.clone()).or_insert(0.0);
                if k.ends_with("_long") { *e = e.max(*v) } else { *e += v }
            }
        };
        for r in rows_by_id.get(&id).into_iter().flatten() {
            add(&r.stats);
        }
        if !offseason {
            if let Some(s) = live.get(&id) {
                add(&s.to_map());
            }
        }
        let retired = lg.retired_players.get(&id);
        let pos = active.map(|(p, _)| p.position).or_else(|| retired.and_then(|r| Position::ALL.iter().copied().find(|x| x.label() == r.pos)));
        if total.is_empty() || !c.qualifies(&total, pos.unwrap_or(Position::ALL[0])) {
            continue;
        }
        let name = match (active, retired) {
            (Some((p, _)), _) => short_name(&p.name),
            (None, Some(r)) => short_name(&r.name),
            _ => "Retired player".to_string(),
        };
        let team = match active {
            None => "Ret".to_string(),
            Some((_, t)) if *t < 0 => "FA".to_string(),
            Some((_, t)) => abbr_of(lg, *t as usize),
        };
        let pos_label = active.map(|(p, _)| p.position.label().to_string()).or_else(|| retired.map(|r| r.pos.clone())).unwrap_or_else(|| "–".into());
        out.push(RowIn { id, name, team, pos: pos_label, mine: active.is_some_and(|(_, t)| Some(*t as usize) == lg.user_team && *t >= 0), stats: total });
    }
    out
}

pub fn leaders(lg: &League, cat: &str, playoffs: bool, career: bool, active_only: bool) -> Value {
    let c = stats::by_name(cat);
    let rows = if career { career_rows(lg, &c, playoffs, active_only) } else { stat_rows(lg, &c, playoffs, None) };
    let mut v = stats::table(&c, &rows, |_| false);
    v["names"] = json!(stats::categories().iter().map(|c| c.name).collect::<Vec<_>>());
    v["season"] = json!(lg.season);
    v
}

pub fn team_stats(lg: &League, team: i64, playoffs: bool) -> Value {
    let n = lg.teams.len();
    let team = if team < 0 { lg.user_team.unwrap_or(0) } else { (team as usize).min(n - 1) };
    let src = if playoffs { &lg.playoff_stats } else { &lg.season_stats };
    let sum = |t: usize, k: &str| -> f64 { lg.teams[t].roster.iter().map(|p| src.get(&p.id).and_then(|s| s.to_map().get(k).copied()).unwrap_or(0.0)).sum() };
    let gms = |t: usize| -> f64 { if playoffs { 0.0 } else { lg.standings.records[t].games() as f64 } };
    type Item<'a> = (&'a str, Box<dyn Fn(usize) -> f64 + 'a>, bool, fn(f64) -> String, bool);
    fn int(v: f64) -> String { format!("{}", v.round() as i64) }
    fn one(v: f64) -> String { format!("{v:.1}") }
    fn half(v: f64) -> String { if v == v.round() { format!("{}", v as i64) } else { format!("{v:.1}") } }
    let ppg = |t: usize| if gms(t) == 0.0 { 0.0 } else { lg.standings.records[t].points_for as f64 / gms(t) };
    let papg = |t: usize| if gms(t) == 0.0 { 0.0 } else { lg.standings.records[t].points_against as f64 / gms(t) };
    let offense: Vec<Item> = vec![
        ("Points / game", Box::new(ppg), true, one, true),
        ("Total yards", Box::new(|t| sum(t, "pass_yds") + sum(t, "rush_yds")), true, int, false),
        ("Passing yards", Box::new(|t| sum(t, "pass_yds")), true, int, false),
        ("Rushing yards", Box::new(|t| sum(t, "rush_yds")), true, int, false),
        ("Passing TD", Box::new(|t| sum(t, "pass_td")), true, int, false),
        ("Rushing TD", Box::new(|t| sum(t, "rush_td")), true, int, false),
        ("Giveaways", Box::new(|t| sum(t, "pass_int") + sum(t, "fumbles_lost")), false, int, false),
        ("Sacks allowed", Box::new(|t| sum(t, "sacks_taken")), false, int, false),
    ];
    let defense: Vec<Item> = vec![
        ("Points allowed / game", Box::new(papg), false, one, true),
        ("Sacks", Box::new(|t| sum(t, "sacks")), true, half, false),
        ("Interceptions", Box::new(|t| sum(t, "int")), true, int, false),
        ("Forced fumbles", Box::new(|t| sum(t, "ff")), true, int, false),
        ("Fumble recoveries", Box::new(|t| sum(t, "fr")), true, int, false),
        ("Tackles for loss", Box::new(|t| sum(t, "tfl")), true, half, false),
        ("Passes defended", Box::new(|t| sum(t, "pd")), true, int, false),
    ];
    let special: Vec<Item> = vec![
        ("Field goals made", Box::new(|t| sum(t, "fgm")), true, int, false),
        ("Field goal %", Box::new(|t| if sum(t, "fga") == 0.0 { 0.0 } else { sum(t, "fgm") / sum(t, "fga") * 100.0 }), true, one, false),
        ("Punt average", Box::new(|t| if sum(t, "punts") == 0.0 { 0.0 } else { sum(t, "punt_yds") / sum(t, "punts") }), true, one, false),
        ("Kick return average", Box::new(|t| if sum(t, "kr") == 0.0 { 0.0 } else { sum(t, "kr_yds") / sum(t, "kr") }), true, one, false),
    ];
    let block = |title: &str, items: Vec<Item>| {
        let rows: Vec<Value> = items
            .iter()
            .filter(|i| !(playoffs && i.4))
            .map(|(label, f, high, fmt, _)| {
                let mine = f(team);
                let better = (0..n).filter(|&t| if *high { f(t) > mine } else { f(t) < mine }).count();
                json!({ "label": label, "text": fmt(mine), "rank": better + 1, "tone": if better < 3 { "good" } else if better + 3 >= n { "bad" } else { "" } })
            })
            .collect();
        json!({ "title": title, "rows": rows })
    };
    let rec = &lg.standings.records[team];
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| lg.standings.records[b].pct().total_cmp(&lg.standings.records[a].pct()));
    let rank = order.iter().position(|&t| t == team).unwrap_or(0) + 1;
    let tables: Vec<Value> = {
        let cats = stats::categories();
        let order = [&cats[0], &cats[1], &cats[2]];
        let ol = stats::ol_category();
        let mut v: Vec<Value> = order.iter().map(|c| stats::table(c, &stat_rows(lg, c, playoffs, Some(team)), |_| false)).collect();
        v.push(stats::table(&ol, &stat_rows(lg, &ol, playoffs, Some(team)), |_| false));
        v.extend(cats[3..].iter().map(|c| stats::table(c, &stat_rows(lg, c, playoffs, Some(team)), |_| false)));
        v
    };
    json!({
        "team": team_ref(lg, team), "isMe": Some(team) == lg.user_team, "count": n, "rank": rank, "pf": rec.points_for, "pa": rec.points_against,
        "totals": [block("Offense", offense), block("Defense", defense), block("Special teams", special)], "tables": tables,
        "teams": (0..n).map(|t| team_ref(lg, t)).collect::<Vec<_>>(),
    })
}

fn clock(s: i32) -> String {
    format!("{}:{:02}", s / 60, s % 60)
}

pub fn box_score(lg: &League, idx: usize) -> Value {
    use sp_sim::Side;
    let Some(b) = lg.results.get(idx).and_then(|r| r.box_score.as_ref()) else { return json!({}) };
    let r = &lg.results[idx];
    let u = lg.user_team;
    let mine = Some(r.home) == u || Some(r.away) == u;
    let my_side = if Some(r.away) == u { Side::Away } else { Side::Home };
    let sname = |s: Side| if s == Side::Home { &b.home_name } else { &b.away_name };
    let slug = |s: Side| helmet_for_name(lg, sname(s));
    let side_key = |s: Side| if s == Side::Home { "home" } else { "away" };
    let winner = b.winner();
    let named = !mine && !b.why_named.home.is_empty();
    let why = if named { if my_side == Side::Home { &b.why_named.home } else { &b.why_named.away } } else if my_side == Side::Home { &b.why.home } else { &b.why.away };
    let who = if !named { "we".to_string() } else if my_side == Side::Home { if b.home_nick.is_empty() { b.home_name.clone() } else { b.home_nick.clone() } } else if b.away_nick.is_empty() { b.away_name.clone() } else { b.away_nick.clone() };
    let why_title = match winner { None => "How it went".to_string(), Some(w) if w == my_side => format!("Why {who} won"), _ => format!("Why {who} lost") };
    let pog = b.player_of_game.as_ref().map(|p| {
        let jersey = lg.teams.iter().find_map(|t| t.roster.iter().find(|x| x.id == p.player.id).map(|x| (t, x))).and_then(|(t, x)| t.numbers.get(&x.id).copied().or(x.jersey).map(|n| json!({ "number": n, "teamSlug": helmet_slug(&t.town.nickname), "lastName": x.name.rsplit(' ').next().unwrap_or("") })));
        json!({ "id": p.player.id, "name": p.player.name, "pos": p.player.pos.label(), "summary": p.summary, "helmet": slug(p.side), "jersey": jersey })
    });
    let cats: [(&str, &str, Vec<&str>); 8] = [
        ("passing", "Passing", vec!["Passing"]), ("rushing", "Rushing", vec!["Rushing"]), ("receiving", "Receiving", vec!["Receiving"]),
        ("offensiveLine", "Offensive line", vec!["Offensive line"]), ("defense", "Defense", vec!["Defense"]), ("kicking", "Kicking", vec!["Kicking"]),
        ("punting", "Punting", vec!["Punting"]), ("returns", "Returns", vec!["Kick returns", "Punt returns"]),
    ];
    let tables_for = |s: Side| {
        let pt = if s == Side::Home { &b.player_tables.home } else { &b.player_tables.away };
        let mut m = serde_json::Map::new();
        for (key, title, names) in &cats {
            let lines = match *key {
                "passing" => &pt.passing, "rushing" => &pt.rushing, "receiving" => &pt.receiving, "offensiveLine" => &pt.offensive_line,
                "defense" => &pt.defense, "kicking" => &pt.kicking, "punting" => &pt.punting, _ => &pt.returns,
            };
            let tabs: Vec<Value> = names
                .iter()
                .map(|nm| {
                    let c = stats::by_name(nm);
                    let rows: Vec<RowIn> = lines.iter().filter(|l| c.qualifies(&l.raw, l.pos)).map(|l| RowIn { id: l.id, name: short_name(&l.name), team: String::new(), pos: l.pos.label().to_string(), mine: s == my_side, stats: l.raw.clone() }).collect();
                    // Season-only columns (games played, per-game rates) mean nothing for one game.
                    stats::table(&c, &rows, |h| h == "G" || h.ends_with("/G"))
                })
                .collect();
            m.insert(key.to_string(), json!({ "title": title, "tables": tabs }));
        }
        Value::Object(m)
    };
    let n = if b.overtime { 5 } else { 4 };
    json!({
        "idx": idx, "mine": mine, "final": if b.tie { "Final · tie".to_string() } else { format!("Final{}", if b.overtime { " · OT" } else { "" }) },
        "weather": format!("{:?}", b.weather).to_lowercase(), "quarters": n,
        "home": { "name": b.home_name, "record": b.home_record, "score": b.home_score, "q": b.quarters.home, "helmet": slug(Side::Home) },
        "away": { "name": b.away_name, "record": b.away_record, "score": b.away_score, "q": b.quarters.away, "helmet": slug(Side::Away) },
        "tie": b.tie, "won": winner == Some(my_side), "whyTitle": why_title, "why": why,
        "playerOfGame": pog,
        "scoring": b.scoring.iter().map(|p| json!({ "when": format!("Q{} {}", p.quarter, clock(p.clock)), "side": if p.side == Side::Home { "H" } else { "A" }, "text": p.text, "score": format!("{}-{}", p.home_score, p.away_score) })).collect::<Vec<_>>(),
        "keyPlays": b.key_plays.iter().take(8).map(|k| format!("Q{}: {}", k.quarter, k.text)).collect::<Vec<_>>(),
        "notes": b.injury_notes.iter().chain(b.fatigue_notes.iter()).collect::<Vec<_>>(),
        "teamStats": b.team_stats.iter().map(|t| json!({ "label": t.label, "home": t.home, "away": t.away, "share": t.home_share })).collect::<Vec<_>>(),
        "players": { side_key(Side::Home): tables_for(Side::Home), side_key(Side::Away): tables_for(Side::Away) },
        "mySide": side_key(my_side),
        "drives": b.drives.iter().map(|d| json!({ "side": if d.offense == Side::Home { "H" } else { "A" }, "when": format!("Q{} {}", d.quarter, clock(d.clock_start)), "text": format!("{} plays, {} yds", d.plays, d.yards), "result": d.result })).collect::<Vec<_>>(),
    })
}

// ------------------------------------------------------------------ season-end stories

pub fn story(lg: &League, kind: &str) -> Value {
    let u = lg.user_team;
    match kind {
        "champion" => {
            let Some(c) = lg.champion() else { return json!({}) };
            let town = &lg.teams[c].town;
            let season = lg.season - if lg.phase == LeaguePhase::Offseason { 1 } else { 0 };
            json!({ "name": town.team_name(), "helmet": helmet_slug(&town.nickname), "iWon": Some(c) == u, "season": season })
        }
        "retire" => {
            let mut seen = std::collections::HashSet::new();
            let all: Vec<&sp_league::league::RetiredEntry> = lg.last_retired_entries.iter().filter(|e| seen.insert(e.player.id)).collect();
            // A free agent who retires still wore a club's colors: he goes under that club (yours, if he was yours).
            let last_club = |e: &sp_league::league::RetiredEntry| -> Option<&sp_league::league::SeasonRow> {
                if e.team.is_some() { None } else { lg.season_rows.get(&e.player.id).and_then(|r| r.iter().rev().find(|r| r.team >= 0)) }
            };
            let is_mine = |e: &sp_league::league::RetiredEntry| e.team == u || last_club(e).is_some_and(|r| Some(r.team as usize) == u);
            let row = |e: &sp_league::league::RetiredEntry| {
                let p = &e.player;
                let club = last_club(e);
                let nick = e.team.map(|t| lg.teams[t].town.nickname.clone()).or_else(|| club.and_then(|c| c.nickname.clone()));
                let years = lg.season_rows.get(&p.id).map(|r| r.len()).unwrap_or(0);
                let where_ = match (e.team, club) {
                    (Some(t), _) => format!(" · {}", lg.teams[t].town.name),
                    (None, Some(c)) => format!(" · {}", c.nickname.clone().or_else(|| c.abbr.clone()).unwrap_or_else(|| "free agent".into())),
                    _ => " · free agent".to_string(),
                };
                json!({ "id": p.id, "name": p.name, "hof": e.hall_of_fame, "helmet": nick.map(|n| helmet_slug(&n)).unwrap_or_default(),
                        "line": format!("{} · age {} · OVR {} · {}{}", p.position.label(), p.age, p.ovr(), if years == 0 { "no seasons played".to_string() } else { format!("{} season{}", years, if years == 1 { "" } else { "s" }) }, where_) })
            };
            let mut mine: Vec<&_> = all.iter().copied().filter(|e| is_mine(e)).collect();
            let mut others: Vec<&_> = all.iter().copied().filter(|e| !is_mine(e)).collect();
            mine.sort_by(|a, b| b.player.ovr().cmp(&a.player.ovr()));
            others.sort_by(|a, b| b.player.ovr().cmp(&a.player.ovr()));
            json!({ "count": all.len(), "season": lg.season, "mine": mine.iter().map(|e| row(e)).collect::<Vec<_>>(), "others": others.iter().map(|e| row(e)).collect::<Vec<_>>() })
        }
        "gameover" => {
            let Some(uu) = u else { return json!({}) };
            let titles = lg.history.iter().filter(|h| h.champion == uu).count();
            let r = lg.user_career_record();
            let career = format!("{}-{}{}", r.wins, r.losses, if r.ties > 0 { format!("-{}", r.ties) } else { String::new() });
            json!({ "name": lg.teams[uu].town.team_name(), "town": lg.teams[uu].town.name, "seasons": lg.season - if lg.phase == LeaguePhase::Offseason { 0 } else { 1 },
                    "titles": titles, "career": career, "finalSeason": record_of(lg, uu) })
        }
        _ => json!({}),
    }
}

// ------------------------------------------------------------------ settings

pub fn settings(lg: &League) -> Value {
    let Some(u) = lg.user_team else { return Value::Null };
    json!({ "team": lg.teams[u].town.team_name(), "phase": phase_label(lg), "season": lg.season, "record": record_of(lg, u), "seed": lg.seed.to_string(), "town": town_json(&lg.teams[u].town) })
}

// ------------------------------------------------------------------ legacy

pub fn legacy(lg: &League) -> Value {
    let c = lg.owner_career();
    let Some(u) = lg.user_team else { return json!({}) };
    let best = c.best_season().map(|s| json!({ "record": s.record(), "season": s.season }));
    let money = |v: i64| sp_league::text::money(v);
    json!({
        "team": c.team_name, "helmet": helmet_slug(&lg.teams[u].town.nickname), "count": c.seasons.len(),
        "record": c.record(), "pct": format!("{:.1}", c.pct() * 100.0), "titles": c.titles(), "runnerUps": c.runner_ups(), "divisionTitles": c.division_titles(),
        "playoffTrips": c.playoff_appearances(), "playoffRecord": if c.playoff_appearances() == 0 { "-".to_string() } else { format!("{}-{}", c.playoff_wins(), c.playoff_losses()) },
        "playoffExact": c.playoff_record_exact(), "best": best,
        "seasons": c.seasons.iter().rev().map(|s| json!({ "season": s.season, "record": s.record(), "finish": if s.finish == "Playoffs" { "Made the playoffs".to_string() } else { s.finish.clone() },
            "divisionTitle": s.division_title, "champion": s.champion(), "playoffs": if s.exact && s.made_playoffs() { Some(format!("{}-{}", s.playoff_wins, s.playoff_losses)) } else { None }, "madePlayoffs": s.made_playoffs(), "runnerUp": s.finish == "Runner-up" })).collect::<Vec<_>>(),
        "awards": c.awards.iter().rev().map(|a| json!({ "text": format!("{}: {} {}", a.award, a.pos, a.player), "season": a.season })).collect::<Vec<_>>(),
        "hof": c.hall_of_famers.iter().map(|e| json!({ "title": format!("{} {}", e.pos, e.name), "line": format!("{} seasons ({}-{}){}", e.seasons, e.first_season, e.last_season, if e.titles > 0 { format!(" · {} title{}", e.titles, if e.titles == 1 { "" } else { "s" }) } else { String::new() }) })).collect::<Vec<_>>(),
        "help": c.help.iter().rev().map(|h| json!({ "team": h.team, "season": h.season, "amount": money(h.amount) })).collect::<Vec<_>>(),
        "clubsHelped": c.clubs_helped(), "totalGiven": money(c.total_given()), "gifts": c.times_helped(),
    })
}

// ------------------------------------------------------------------ finances: the green-bar sheet

pub fn finance_sheet(lg: &League) -> Value {
    use LedgerKind::*;
    let f = lg.my_finance();
    let plain = |v: i64| -> String {
        let s = v.abs().to_string();
        let mut out = String::new();
        for (i, c) in s.chars().enumerate() {
            if i > 0 && (s.len() - i) % 3 == 0 {
                out.push(',');
            }
            out.push(c);
        }
        if v < 0 { format!("({out})") } else { out }
    };
    let n = |v: i64| if v == 0 { "-".to_string() } else { plain(v) };
    let rev_cols: [(&str, &[LedgerKind]); 6] = [("Gate", &[Gate]), ("Conc", &[Concessions]), ("Merch", &[Merch]), ("Spons", &[Sponsors]), ("LgShr", &[LeagueShare]), ("PlyOff", &[PlayoffBonus])];
    let exp_cols: [(&str, &[LedgerKind]); 11] = [
        ("Pay", &[PlayerPay]), ("Resv", &[Retainers]), ("Rent", &[Rent]), ("Travl", &[Travel]), ("Insur", &[Insurance]), ("Medic", &[Medical]),
        ("Promo", &[Promo]), ("SpCst", &[SponsorCost]), ("Upkp", &[Upkeep]), ("Intr", &[Interest, LoanPayment]), ("Other", &[Other]),
    ];
    let sum = |m: &std::collections::BTreeMap<LedgerKind, i64>, ks: &[LedgerKind]| -> i64 { ks.iter().map(|k| m.get(k).copied().unwrap_or(0)).sum() };
    let games = &f.games;
    let mut tot_r = std::collections::BTreeMap::new();
    let mut tot_e = std::collections::BTreeMap::new();
    let (mut crowd, mut home_games, mut cap_sum, mut running) = (0i64, 0i64, 0i64, 0i64);
    let mut rows = Vec::new();
    for g in games {
        running += g.net();
        for (k, v) in &g.revenue { *tot_r.entry(*k).or_insert(0) += v; }
        for (k, v) in &g.expenses { *tot_e.entry(*k).or_insert(0) += v; }
        if g.home {
            crowd += g.attendance;
            cap_sum += g.capacity;
            home_games += 1;
        }
        let row = |week: String, ha: String, cr: String, full: String, r: &std::collections::BTreeMap<LedgerKind, i64>, e: &std::collections::BTreeMap<LedgerKind, i64>, rt: i64, et: i64, ytd: i64, cash: i64| {
            json!({ "week": week, "ha": ha, "crowd": cr, "full": full, "rev": rev_cols.iter().map(|c| n(sum(r, c.1))).collect::<Vec<_>>(), "revT": n(rt),
                    "exp": exp_cols.iter().map(|c| n(sum(e, c.1))).collect::<Vec<_>>(), "expT": n(et), "net": if rt - et == 0 { "-".to_string() } else { plain(rt - et) }, "netNeg": rt - et < 0,
                    "ytd": if ytd == 0 { "-".to_string() } else { plain(ytd) }, "ytdNeg": ytd < 0, "cash": plain(cash), "cashNeg": cash < 0 })
        };
        rows.push(row(g.week.to_string(), if g.home { "H" } else { "A" }.to_string(), if g.home { plain(g.attendance) } else { "-".into() },
            if g.home && g.capacity > 0 { format!("{}%", (g.attendance as f64 / g.capacity as f64 * 100.0).round() as i64) } else { "-".into() },
            &g.revenue, &g.expenses, g.total_revenue(), g.total_expenses(), running, g.cash_after));
    }
    let total_rev: i64 = tot_r.values().sum();
    let total_exp: i64 = tot_e.values().sum();
    let profit = total_rev - total_exp;
    let totals = json!({ "week": "TOT", "ha": format!("{home_games}H"), "crowd": if home_games == 0 { "-".to_string() } else { plain((crowd as f64 / home_games as f64).round() as i64) },
        "full": if home_games == 0 || cap_sum == 0 { "-".to_string() } else { format!("{}%", (crowd as f64 / cap_sum as f64 * 100.0).round() as i64) },
        "rev": rev_cols.iter().map(|c| n(sum(&tot_r, c.1))).collect::<Vec<_>>(), "revT": n(total_rev), "exp": exp_cols.iter().map(|c| n(sum(&tot_e, c.1))).collect::<Vec<_>>(), "expT": n(total_exp),
        "net": plain(profit), "netNeg": profit < 0, "ytd": plain(profit), "ytdNeg": profit < 0, "cash": plain(f.cash), "cashNeg": f.cash < 0 });
    let per_game = if games.is_empty() { 0 } else { (profit as f64 / games.len() as f64).round() as i64 };
    let margin = if total_rev == 0 { 0 } else { (profit as f64 / total_rev as f64 * 100.0).round() as i64 };
    let line = |label: &str, value: String, tone: &str, bold: bool, rule: bool| json!({ "label": label, "value": value, "tone": tone, "bold": bold, "rule": rule });
    let paren = |v: i64| format!("({})", plain(v));
    let pl = vec![
        line("Gate + concessions + merch", plain(sum(&tot_r, &[Gate, Concessions, Merch])), "good", false, false),
        line("Sponsor income", plain(sum(&tot_r, &[Sponsors])), "good", false, false),
        line("League share + playoffs", plain(sum(&tot_r, &[LeagueShare, PlayoffBonus])), "good", false, false),
        line("TOTAL REVENUE", plain(total_rev), "good", true, false),
        line("Player pay + reserve pay", paren(sum(&tot_e, &[PlayerPay, Retainers])), "bad", false, false),
        line("Promo nights", paren(sum(&tot_e, &[Promo])), "bad", false, false),
        line("Sponsor obligations", paren(sum(&tot_e, &[SponsorCost])), "bad", false, false),
        line("Rent, travel, insurance, medical", paren(sum(&tot_e, &[Rent, Travel, Insurance, Medical])), "bad", false, false),
        line("Upkeep, loans, other", paren(sum(&tot_e, &[Upkeep, Interest, LoanPayment, Other])), "bad", false, false),
        line("TOTAL EXPENSES", paren(total_exp), "bad", true, false),
        line(&format!("NET {}  ({margin}% margin)", if profit >= 0 { "PROFIT" } else { "LOSS" }), plain(profit), if profit >= 0 { "good" } else { "bad" }, true, true),
        line("Average per game", plain(per_game), if per_game >= 0 { "good" } else { "bad" }, false, false),
        line("Pace for a 14-game season", plain(per_game * 14), if per_game >= 0 { "good" } else { "bad" }, false, false),
    ];
    let kind_name = |k: LedgerKind| match k {
        Gate => "Gate", Concessions => "Concessions", Merch => "Merch", Sponsors => "Sponsors", LeagueShare => "League share", PlayoffBonus => "Playoff bonus", PlayerPay => "Player pay",
        Retainers => "Reserve pay", Rent => "Stadium rent", Travel => "Travel", Insurance => "Insurance", Medical => "Medical", Other => "Other", Promo => "Promos", Upkeep => "Upkeep",
        Interest => "Loan interest", LoanPayment => "Loan payments", LoanProceeds => "Loan", Facilities => "Facilities", SponsorCost => "Sponsor costs",
    };
    let other: Vec<Value> = f.ledger.iter().filter(|e| e.season == lg.season && (!e.memo.is_empty() || e.kind == Facilities || e.kind == LoanProceeds))
        .map(|e| json!({ "week": e.week, "text": if e.memo.is_empty() { kind_name(e.kind).to_string() } else { format!("{}: {}", kind_name(e.kind), e.memo) }, "amount": plain(e.amount), "neg": e.amount < 0 })).collect();
    json!({
        "season": lg.season, "games": games.len(), "profit": profit >= 0, "pl": pl, "revHeads": rev_cols.iter().map(|c| c.0).collect::<Vec<_>>(), "expHeads": exp_cols.iter().map(|c| c.0).collect::<Vec<_>>(),
        "rows": rows, "totals": totals, "other": other,
    })
}
