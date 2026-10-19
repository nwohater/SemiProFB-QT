//! Ported from the Dart `inbox_test.dart`, `conduct_test.dart`, `council_test.dart`, `inbox_content_test.dart`.
use sp_core::{Attr, Ratings};
use sp_economy::business::sponsor_clauses;
use sp_economy::{CouncilGoal, CouncilGoals, GoalKind, GoalResult};
use sp_league::conduct::weekly_risk;
use sp_league::inbox::{ConductState, InboxFeed, InboxKind, PleaState, PostOpts, MAX_INBOX_ITEMS};
use sp_league::persistence::LeagueSave;
use sp_league::text::{conduct_asks, conduct_incidents, fill, gazette_text, inbox_text, is_local_hero, local_hero_name};
use sp_league::{League, LeaguePhase, NewLeague};
use std::collections::BTreeSet;

fn ca(seed: u64, user: usize) -> League {
    League::new(seed, NewLeague { user_team: Some(user), ..Default::default() })
}

fn placeholders(t: &str) -> Vec<String> {
    regex::Regex::new(r"\{([a-z]+)\}").unwrap().captures_iter(t).map(|c| c[1].to_string()).collect()
}

// ----------------------------------------------------------------- inbox

#[test]
fn a_broke_club_pleads_once_giving_moves_cash_clears_the_plea_and_survives_a_save() {
    let mut lg = ca(5, 2);
    lg.finances[7].cash = -30_000; // deep enough that one game cannot lift the club above the plea line
    lg.advance();
    let pleas: Vec<_> = lg.inbox.iter().filter(|i| i.kind == InboxKind::Plea && i.plea_team == Some(7)).cloned().collect();
    assert_eq!(pleas.len(), 1);
    let plea = pleas[0].clone();
    assert!(plea.plea_ask >= 2500);
    assert_eq!(plea.plea_state, PleaState::Open);
    lg.advance();
    assert_eq!(lg.inbox.iter().filter(|i| i.kind == InboxKind::Plea && i.plea_team == Some(7) && i.plea_state == PleaState::Open).count(), 1);
    let (mine, theirs) = (lg.my_finance().cash, lg.finances[7].cash);
    assert!(lg.help_club(plea.id, plea.plea_ask).is_some());
    assert_eq!(lg.my_finance().cash, mine - plea.plea_ask);
    assert_eq!(lg.finances[7].cash, theirs + plea.plea_ask);
    assert_eq!(lg.inbox.iter().find(|i| i.id == plea.id).unwrap().plea_state, PleaState::Helped);
    assert!(lg.help_club(plea.id, 100).is_none());
    // The gift is enough to see the club through the season and into the next.
    lg.sim_season();
    assert!(!lg.finances[7].bankrupt);
    let back = LeagueSave::decode(&LeagueSave::encode(&lg)).unwrap();
    assert_eq!(back.inbox.iter().find(|i| i.id == plea.id).unwrap().plea_state, PleaState::Helped);
}

#[test]
fn the_accountant_recap_notes_a_quiet_home_night_and_a_botched_promo() {
    let mut lg = ca(8, 0);
    let home_week = lg.schedule.iter().find(|g| g.home == 0).unwrap().week;
    while lg.week <= home_week {
        lg.advance();
    }
    let quiet = lg.inbox.iter().rev().find(|i| i.kind == InboxKind::Report).unwrap();
    assert!(inbox_text().list("reportNoPromo").iter().any(|l| quiet.text.contains(l.as_str())), "{}", quiet.text);

    let mut mishap = false;
    for s in 1..=80 {
        let mut a = ca(s + 40, 0);
        let w = a.schedule.iter().find(|g| g.home == 0).unwrap().week;
        assert!(a.schedule_promo(w, "beer"));
        while a.week <= w {
            a.advance();
        }
        let gf = a.my_finance().games.iter().find(|g| g.home && g.week == w).unwrap();
        if !gf.promo_mishap {
            continue;
        }
        mishap = true;
        let recap = a.inbox.iter().rev().find(|i| i.kind == InboxKind::Report).unwrap();
        assert!(recap.text.contains("$2 Beer Night"), "{}", recap.text);
        assert!(recap.text.contains("goalpost"));
        assert!(a.inbox.iter().rev().find(|i| i.kind == InboxKind::Promo).unwrap().title.contains("trouble"));
        break;
    }
    assert!(mishap);
}

#[test]
fn day_one_letters_weekly_reports_deterministic() {
    let (mut a, mut b) = (ca(5, 2), ca(5, 2));
    assert_eq!(a.inbox[0].kind, InboxKind::Council);
    assert_eq!(a.unread_inbox(Some(InboxFeed::Money)), a.inbox.len());
    a.advance();
    b.advance();
    assert_eq!(a.inbox.iter().map(|i| i.text.clone()).collect::<Vec<_>>(), b.inbox.iter().map(|i| i.text.clone()).collect::<Vec<_>>());
    let report: Vec<_> = a.inbox.iter().filter(|i| i.kind == InboxKind::Report).collect();
    assert_eq!(report.len(), 1);
    assert!(report[0].text.contains("Revenue"));
}

#[test]
fn a_season_produces_both_feeds_player_voiced_messages_read_state_and_a_save_round_trip() {
    let mut lg = ca(11, 0);
    for _ in 0..3 {
        lg.sim_season();
        lg.begin_offseason();
        lg.finish_offseason();
    }
    let travel = lg.inbox.iter().filter(|i| i.feed == InboxFeed::Travel).count();
    let from = lg.inbox.iter().filter(|i| i.from.is_some()).count();
    assert!(lg.inbox.len() <= MAX_INBOX_ITEMS);
    assert!(travel > 0 && from > 0);
    assert!(lg.inbox.iter().any(|i| i.kind == InboxKind::Council && i.title.contains("Council")));
    lg.mark_inbox_read(Some(InboxFeed::Travel));
    assert_eq!(lg.unread_inbox(Some(InboxFeed::Travel)), 0);
    let back = LeagueSave::decode(&LeagueSave::encode(&lg)).unwrap();
    let key = |lg: &League| lg.inbox.iter().map(|i| format!("{:?}|{:?}|{}|{:?}|{}", i.feed, i.kind, i.text, i.from, i.unread)).collect::<Vec<_>>();
    assert_eq!(key(&back), key(&lg));
}

#[test]
fn sponsor_events_flavor_bonuses_expiry_leaving_lapsed_offers() {
    let mut lg = ca(21, 0);
    let mut kinds: BTreeSet<&str> = BTreeSet::new();
    for _ in 0..4 {
        // Take every offer on the table (up to the slot limit) so deals exist to talk about.
        while !lg.my_finance().offers.is_empty() && lg.my_finance().sponsors.len() < 4 {
            if !lg.my_finance_mut().accept_offer(0) {
                break;
            }
        }
        lg.sim_season();
        lg.begin_offseason();
        lg.finish_offseason();
    }
    for i in lg.inbox.iter().filter(|i| i.kind == InboxKind::Sponsor) {
        kinds.insert(if i.title.contains("win bonus") {
            "bonus"
        } else if i.title.contains("ended") {
            "ended"
        } else if i.title.contains("has left") {
            "left"
        } else if i.title.contains("expired") {
            "lapsed"
        } else if i.title.contains("watching") {
            "watch"
        } else {
            "other"
        });
    }
    assert!(kinds.contains("other"));
    assert!(kinds.len() >= 3, "saw {kinds:?}");
    assert!(lg.inbox.iter().all(|i| !i.text.contains('{') && !i.title.contains('{')), "unfilled placeholder");
}

#[test]
fn accepting_and_cancelling_a_sponsor_writes_a_letter_and_injury_messages_carry_the_player() {
    let mut lg = ca(21, 0);
    let offer = lg.my_finance().offers[0].clone();
    assert!(lg.accept_sponsor(0));
    assert_eq!(lg.inbox.last().unwrap().title, format!("{}: deal signed", offer.name));
    assert!(lg.inbox.last().unwrap().text.contains(&offer.clause));
    assert!(!lg.inbox.last().unwrap().text.contains('{'));
    let cash = lg.my_finance().cash;
    let last = lg.my_finance().sponsors.len() - 1;
    assert!(lg.cancel_sponsor_deal(last));
    assert_eq!(lg.inbox.last().unwrap().title, format!("{}: deal cancelled", offer.name));
    assert!(lg.my_finance().cash < cash);
    assert!(!lg.accept_sponsor(99));
    for _ in 0..3 {
        lg.sim_season();
        lg.begin_offseason();
        lg.finish_offseason();
    }
    let hurt: Vec<_> = lg.inbox.iter().filter(|i| matches!(i.kind, InboxKind::Injury | InboxKind::WorkInjury | InboxKind::Trip | InboxKind::Returned)).collect();
    assert!(!hurt.is_empty());
    assert!(hurt.iter().all(|i| i.player_id.is_some()));
    assert!(lg.inbox.iter().any(|i| i.kind == InboxKind::Returned));
    let back = LeagueSave::decode(&LeagueSave::encode(&lg)).unwrap();
    assert_eq!(back.inbox.iter().map(|i| i.player_id).collect::<Vec<_>>(), lg.inbox.iter().map(|i| i.player_id).collect::<Vec<_>>());
}

// ------------------------------------------------------- inbox copy tables

fn pool(name: &str, keys: &[&str], min: usize) {
    let list = inbox_text().list(name);
    assert!(list.len() >= min, "{name} has {}", list.len());
    assert_eq!(list.iter().collect::<BTreeSet<_>>().len(), list.len(), "{name} has duplicate lines");
    for t in list {
        for k in placeholders(t) {
            assert!(keys.contains(&k.as_str()), "{name} unknown {{{k}}} in: {t}");
        }
    }
}

#[test]
fn inbox_copy_tables_have_enough_unique_lines_and_known_placeholders() {
    let money: &[&str] = &["t", "o"];
    for n in ["reportWinBig", "reportWin", "reportLoss", "reportLossBig", "reportTie", "reportLosing", "reportAway"] {
        pool(n, money, 8);
    }
    pool("reportNoPromo", &[], 8);
    pool("reportPromoMishap", &["promo", "why"], 8);
    pool("promoGood", &["promo"], 8);
    pool("promoBad", &["promo", "why"], 8);
    for n in ["bankLow", "bankNegative", "bankFinal"] {
        pool(n, &[], 8);
    }
    pool("sponsorsWaiting", &["n"], 8);
    pool("sponsorOfferNews", &["s", "w"], 8);
    pool("councilWelcome", &["town"], 8);
    pool("councilTitle", &["town"], 8);
    pool("councilWarn", &["town", "rec"], 8);
    pool("councilOk", &["town", "rec"], 8);
    pool("councilSeason", &["s"], 8);
    pool("councilGoals", &["list"], 8);
    for n in ["councilReviewAll", "councilReviewPartial", "councilReviewNone"] {
        pool(n, &["verdict"], 4);
    }
    pool("councilReviewEmpty", &["verdict"], 2);
    pool("gameInjuryPlayer", &["p", "g", "body", "pos"], 8);
    pool("gameInjuryMinor", &["p", "g", "body", "pos"], 8);
    pool("bodyParts", &[], 8);
    pool("returned", &["p", "pos"], 8);
    pool("returnedTrip", &["p", "pos", "job"], 8);
    pool("returnedWorkInjury", &["p", "pos", "job"], 8);
    pool("genericInjury", &["p", "job", "g"], 8);
    pool("genericTrip", &["p", "job", "g"], 8);
    pool("sponsorClauseGeneric", &["s"], 8);
    pool("sponsorBonus", &["s", "amt"], 8);
    pool("sponsorWatch", &["s", "need", "pct"], 8);
    pool("sponsorLeft", &["s", "need"], 8);
    pool("sponsorExpired", &["s"], 8);
    pool("sponsorOfferLapsed", &["s"], 8);
    pool("sponsorSigned", &["s", "amt", "n", "seasons", "clause"], 8);
    pool("sponsorCancelled", &["s", "amt"], 8);
    pool("awardMine", &["p", "a"], 8);
    pool("fanFavoriteBreakout", &["p", "pos", "town"], 8);
    pool("fanFavoriteLocal", &["p", "pos", "age", "town"], 8);
    for n in ["rumorLowCash", "rumorVeryLow", "rumorDoomed"] {
        pool(n, &["t", "town"], 8);
    }
    pool("pleas", &["t", "town", "amt"], 8);
    pool("pleaThanks", &["t", "town", "amt"], 8);
}

#[test]
fn every_local_hero_line_gives_the_player_name_back() {
    let name = "D'Angelo O'Brien";
    for t in inbox_text().list("fanFavoriteLocal") {
        let text = fill(t, &[("p", name.into()), ("pos", "QB".into()), ("age", "22".into()), ("town", "San Luis Obispo".into())]);
        assert!(is_local_hero(&text), "{t}");
        assert_eq!(local_hero_name(&text).as_deref(), Some(name), "{t}");
    }
    assert!(!is_local_hero("{p} ({pos}, {town}) has become a fan favorite."));
    assert!(local_hero_name("The crowd in Chico started a Bob Lee chant. It caught on. He is a fan favorite now.").is_none());
}

#[test]
fn every_sponsor_clause_has_several_event_lines() {
    for c in sponsor_clauses() {
        let lines = inbox_text().sponsor_clause_events.get(&c.id).unwrap_or_else(|| panic!("missing events for {}", c.id));
        assert!(lines.len() >= 5, "{}", c.id);
        assert_eq!(lines.iter().collect::<BTreeSet<_>>().len(), lines.len(), "{}", c.id);
        for t in lines {
            for k in placeholders(t) {
                assert_eq!(k, "s", "{t}");
            }
        }
    }
}

#[test]
fn every_job_category_has_several_injury_and_trip_lines() {
    for (name, c) in &inbox_text().job_categories {
        assert!(c.injury.len() >= 6 && c.trip.len() >= 6, "{name}");
        assert_eq!(c.injury.iter().collect::<BTreeSet<_>>().len(), c.injury.len(), "{name} injury dupes");
        assert_eq!(c.trip.iter().collect::<BTreeSet<_>>().len(), c.trip.len(), "{name} trip dupes");
        for t in c.injury.iter().chain(&c.trip) {
            for k in placeholders(t) {
                assert!(["p", "job", "g"].contains(&k.as_str()), "{name}: {t}");
            }
        }
    }
}

#[test]
fn gazette_tables_are_unique_and_use_known_placeholders() {
    let known = ["w", "l", "ws", "ls", "p", "pos", "job", "ajob", "ajob_cap", "t", "line", "g", "town", "n", "pteam", "home", "amt", "gm", "title", "note", "text"];
    let t = gazette_text();
    for name in ["rivalryHead", "upsetHead", "shutoutHead", "blowoutHead", "hotHead", "coldHead", "brokeHead", "champHead", "semiHead", "quietHead", "spotHead", "starHead"] {
        let list = t.list(name);
        assert!(list.len() >= 2, "{name}");
        assert_eq!(list.iter().collect::<BTreeSet<_>>().len(), list.len(), "{name} dupes");
    }
    for name in ["scoreBody", "gossipPaidHead", "workInjury", "moneyNotes"] {
        for line in t.list(name) {
            for k in regex::Regex::new(r"\{(\w+)\}").unwrap().captures_iter(line).map(|c| c[1].to_string()) {
                assert!(known.contains(&k.as_str()), "{name}: unknown {{{k}}} in {line}");
            }
        }
    }
}

// --------------------------------------------------------------- conduct

fn with_sense(p: &sp_core::Player, disc: i32, intel: i32) -> sp_core::Player {
    let mut q = p.clone();
    let mut v = q.ratings.to_array();
    v[Attr::Discipline.index()] = disc;
    v[Attr::Intelligence.index()] = intel;
    q.ratings = Ratings::new(v);
    q.potentials = Ratings::new([99; 8]);
    q
}

#[test]
fn conduct_risk_rises_as_discipline_and_intelligence_fall_and_is_zero_for_sensible_players() {
    let lg = ca(5, 0);
    let base = lg.me().roster[0].clone();
    assert_eq!(weekly_risk(&with_sense(&base, 80, 80)), 0.0);
    assert_eq!(weekly_risk(&with_sense(&base, 45, 45)), 0.0);
    let mid = weekly_risk(&with_sense(&base, 32, 32));
    let low = weekly_risk(&with_sense(&base, 15, 15));
    assert!(low > mid && mid > 0.0);
    assert!(weekly_risk(&with_sense(&base, 15, 60)) > weekly_risk(&with_sense(&base, 60, 15)), "discipline counts more");
}

#[test]
fn a_season_produces_letters_paying_costs_cash_and_ignoring_him_sits_him() {
    let mut lg = ca(5, 0);
    lg.auto_set_roster = true;
    let mut found = false;
    for _ in 0..40 {
        if found {
            break;
        }
        lg.advance();
        found = lg.inbox.iter().any(|m| m.kind == InboxKind::Conduct);
        if lg.phase != LeaguePhase::RegularSeason {
            break;
        }
    }
    assert!(found, "no incident in a season");
    let item = lg.inbox.iter().find(|m| m.kind == InboxKind::Conduct).unwrap().clone();
    assert!(item.conduct_cost > 0 && item.conduct_games >= 1);
    assert!(item.text.contains('$'));
    let cash = lg.my_finance().cash;
    if cash >= item.conduct_cost {
        assert!(lg.pay_conduct(item.id));
        assert_eq!(lg.my_finance().cash, cash - item.conduct_cost);
        assert_eq!(lg.inbox.iter().find(|m| m.id == item.id).unwrap().conduct_state, ConductState::Paid);
        assert!(!lg.pay_conduct(item.id), "cannot pay twice");
        assert!(!lg.let_him_sit(item.id));
    }
}

#[test]
fn letting_him_sit_puts_him_out_at_least_one_game_and_logs_it() {
    let mut lg = ca(5, 0);
    let id = lg.me().roster[0].id;
    lg.post_inbox(InboxFeed::Travel, InboxKind::Conduct, "x: test", "text", PostOpts { player_id: Some(id), conduct_cost: 500, conduct_games: 2, ..Default::default() });
    let item_id = lg.inbox.last().unwrap().id;
    let fans = lg.my_finance().fan_interest;
    lg.resolve_open_conduct(); // ignored: counts as sitting
    assert!((lg.my_finance().fan_interest - (fans - 1.5)).abs() < 1e-9, "the town hears about it: 0.75 a game");
    lg.advance();
    assert_eq!(lg.inbox.iter().find(|i| i.id == item_id).unwrap().conduct_state, ConductState::Sits);
    assert!(lg.absences_for(id).iter().any(|a| a.what.starts_with("Suspended") && !a.work));
}

#[test]
fn every_incident_fills_cleanly_and_every_tier_has_plenty_to_draw_from() {
    for tier in 0..=2 {
        assert!(conduct_incidents().iter().filter(|i| i.tier == tier).count() >= 15);
    }
    let vars = [("p", "Bo Kowalski".to_string()), ("pos", "LB".into()), ("town", "Visalia".into()), ("job", "roofer".into()), ("amt", "$900".into()), ("g", "2".into()), ("gm", "2 games".into())];
    for i in conduct_incidents() {
        assert!(!fill(&i.body, &vars).contains('{'), "{}", i.title);
    }
    for a in conduct_asks() {
        assert!(!fill(a, &vars).contains('{'), "{a}");
    }
}

// --------------------------------------------------------------- council

#[test]
fn a_new_game_starts_with_three_council_goals_one_per_lane_and_a_letter_in_the_inbox() {
    use sp_economy::council::goal_spec;
    let lg = ca(5, 0);
    let goals = &lg.my_finance().goals;
    assert_eq!(goals.len(), 3);
    assert_eq!(goals.iter().map(|g| goal_spec(g.kind).lane).collect::<BTreeSet<_>>(), BTreeSet::from(["field", "books", "crowd"]));
    assert!(goals.iter().all(|g| g.reward > 0));
    assert!(lg.inbox.iter().any(|i| i.kind == InboxKind::Council && i.title.contains("goals")));
}

#[test]
fn season_end_files_a_report_pays_the_grant_and_sets_new_goals() {
    let mut lg = ca(6, 0);
    lg.sim_season();
    let goals = lg.my_finance().goals.clone();
    lg.begin_offseason();
    let f = lg.my_finance();
    assert_eq!(f.reports.len(), 1);
    let rep = &f.reports[0];
    assert_eq!(rep.season, 1);
    assert_eq!(rep.goals.iter().map(|g| g.goal.kind).collect::<Vec<_>>(), goals.iter().map(|g| g.kind).collect::<Vec<_>>());
    assert!(rep.total_revenue() > 0 && rep.league_avg_revenue > 0);
    assert_eq!(rep.grant, CouncilGoals::grant(&rep.goals));
    assert_eq!(f.ledger.iter().any(|e| e.memo == "Town Council grant"), rep.grant > 0);
    lg.finish_offseason();
    assert_eq!(lg.my_finance().goals.len(), 3);
    assert_eq!(lg.my_finance().reports.len(), 1);
}

#[test]
fn grant_sweep_earns_a_bonus_partial_pays_only_met_goals() {
    let a = CouncilGoal { kind: GoalKind::Wins, target: 6, reward: 1500 };
    let b = CouncilGoal { kind: GoalKind::BreakEven, target: 0, reward: 1500 };
    assert_eq!(CouncilGoals::grant(&[GoalResult { goal: a, met: true }, GoalResult { goal: b, met: false }]), 1500);
    assert_eq!(CouncilGoals::grant(&[GoalResult { goal: a, met: true }, GoalResult { goal: b, met: true }]), 3800);
    assert_eq!(CouncilGoals::grant(&[GoalResult { goal: a, met: false }]), 0);
}

#[test]
fn goals_and_reports_survive_a_save_and_load() {
    let mut lg = ca(7, 0);
    lg.sim_season();
    lg.begin_offseason();
    let back = LeagueSave::decode(&LeagueSave::encode(&lg)).unwrap();
    assert_eq!(back.my_finance().goals, lg.my_finance().goals);
    assert_eq!(back.my_finance().reports[0], lg.my_finance().reports[0]);
}
