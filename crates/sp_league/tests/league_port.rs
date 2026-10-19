//! Ported from the Dart `league_test.dart` and `offseason_test.dart`.
use sp_core::generator::GenOptions;
use sp_core::player::ACQUIRED_FREE_AGENCY;
use sp_core::team::{ACTIVE_ROSTER_SIZE, ROSTER_MIX};
use sp_core::region::nickname_table;
use sp_core::{Attr, Position, Rng, Tier, Weather};
use sp_economy::{Contract, Facility, LedgerKind, Negotiation, OfferOutcome, ResignPremium};
use sp_league::inbox::InboxKind;
use sp_league::persistence::LeagueSave;
use sp_league::standings::{GameRecord, Standings};
use sp_league::text::{is_local_hero, local_hero_name};
use sp_league::{League, LeaguePhase, NewLeague};

fn ca(seed: u64, user: Option<usize>) -> League {
    League::new(seed, NewLeague { user_team: user, ..Default::default() })
}

fn season(seed: u64, user: Option<usize>) -> League {
    let mut lg = ca(seed, user);
    lg.sim_season();
    lg
}

// ------------------------------------------------------------------ league_test

#[test]
fn standings_records_ties_count_half_and_division_record() {
    let g = |h, a, hs, as_| GameRecord::new(1, h, a, hs, as_, false, false, Weather::Clear);
    let mut st = Standings::new(12, sp_league::schedule::ScheduleGenerator::division_of, 0);
    st.add(&g(0, 1, 20, 10));
    st.add(&g(2, 5, 14, 14));
    assert_eq!(st.records[0].to_string(), "1-0-0");
    assert_eq!(st.records[1].to_string(), "0-1-0");
    assert_eq!(st.records[2].ties, 1);
    assert_eq!(st.records[2].pct(), 0.5);
    assert_eq!(st.records[0].div_wins, 1);
    assert_eq!(st.records[2].div_ties, 0); // cross-division tie
}

#[test]
fn a_three_team_cycle_falls_through_to_point_differential() {
    let g = |h, a, hs, as_| GameRecord::new(1, h, a, hs, as_, false, false, Weather::Clear);
    let mut st = Standings::new(12, sp_league::schedule::ScheduleGenerator::division_of, 0);
    st.add(&g(0, 1, 10, 7));
    st.add(&g(1, 2, 21, 0));
    st.add(&g(2, 0, 17, 14));
    let order = st.rank(&[0, 1, 2]);
    assert_eq!(order.len(), 3);
    assert_eq!(order[0], 1);
}

#[test]
fn full_season_84_games_playoffs_and_a_champion() {
    let lg = season(42, Some(0));
    assert_eq!(lg.results.iter().filter(|r| !r.playoff).count(), 84);
    assert_eq!(lg.results.iter().filter(|r| r.playoff).count(), 3);
    assert!(lg.champion().is_some());
    let po = lg.playoffs.as_ref().unwrap();
    assert_eq!(po.seeds.iter().collect::<std::collections::HashSet<_>>().len(), 4);
    assert!(lg.results.iter().filter(|r| r.playoff).all(|r| !r.tie()));
    assert!(lg.standings.records.iter().all(|r| r.games() == 14));
    assert_eq!(lg.standings.records.iter().map(|r| r.wins).sum::<i32>(), lg.standings.records.iter().map(|r| r.losses).sum::<i32>());
}

#[test]
fn playoff_seeds_are_three_division_winners_plus_one_wild_card() {
    let lg = season(7, Some(0));
    let winners: Vec<usize> = (0..3).map(|d| lg.standings.division_order(d)[0]).collect();
    let po = lg.playoffs.as_ref().unwrap();
    assert_eq!(po.seeds[..3].iter().copied().collect::<std::collections::HashSet<_>>(), winners.iter().copied().collect());
    assert!(!winners.contains(&po.seeds[3]));
}

#[test]
fn deterministic_by_seed() {
    let sig = |seed| {
        let lg = season(seed, Some(0));
        format!("{:?}|{}", lg.champion(), lg.standings.records.iter().map(|r| r.to_string()).collect::<Vec<_>>().join(","))
    };
    assert_eq!(sig(5), sig(5));
}

#[test]
fn injured_players_miss_games_and_then_return() {
    let mut lg = ca(11, Some(0));
    for _ in 0..4 {
        lg.advance();
    }
    assert!(!lg.injured_games.is_empty() || lg.season_stats.values().any(|s| s.get("injuries") > 0.0));
    while lg.phase == LeaguePhase::RegularSeason {
        lg.advance();
    }
    assert!(lg.injured_games.values().all(|n| *n > 0));
}

#[test]
fn season_stats_accumulate() {
    let lg = season(21, Some(0));
    let pass: f64 = lg.season_stats.values().map(|p| p.get("pass_yds")).sum();
    assert!(pass > 84.0 * 2.0 * 150.0, "{pass}");
    assert!(lg.season_stats.values().filter(|p| p.get("gp") >= 14.0).count() > 40);
}

#[test]
fn team_parity_points_per_game_in_a_plausible_range() {
    let lg = season(31, Some(0));
    for rec in &lg.standings.records {
        let ppg = rec.points_for as f64 / rec.games() as f64;
        assert!((5.0..=42.0).contains(&ppg), "{ppg}");
    }
}

#[test]
fn every_game_books_money_and_cash_matches_the_ledger() {
    let lg = season(5, None);
    for r in &lg.results {
        assert!(r.home_finance.is_some() && r.away_finance.is_some());
    }
    for i in 0..12 {
        let f = &lg.finances[i];
        let t = &lg.teams[i].town;
        let start = 40_000 + (t.wealth + t.biz) as i64 * 4000;
        assert_eq!(f.cash, start + f.ledger.iter().map(|e| e.amount).sum::<i64>(), "{}", t.name);
        assert_eq!(f.games.len(), 14 + lg.results.iter().filter(|r| r.playoff && (r.home == i || r.away == i)).count());
    }
}

#[test]
fn home_games_earn_gate_and_away_games_do_not_pay_rent() {
    let lg = season(6, None);
    let f = &lg.finances[0];
    assert!(f.games.iter().filter(|g| g.home).all(|g| g.attendance > 0));
    assert!(f.games.iter().filter(|g| !g.home).all(|g| !g.expenses.contains_key(&LedgerKind::Rent)));
}

#[test]
#[should_panic(expected = "Game over")]
fn user_bankruptcy_ends_the_game_and_advance_throws() {
    let mut lg = ca(7, Some(0));
    lg.finances[0].cash = -5_000_000;
    let mut guard = 0;
    while !lg.game_over() && guard < 20 {
        lg.advance();
        guard += 1;
    }
    assert!(lg.game_over());
    assert_ne!(lg.phase, LeaguePhase::Complete);
    lg.advance();
}

#[test]
fn ai_bankruptcy_is_flagged_but_the_league_keeps_playing() {
    let mut lg = ca(8, Some(0));
    lg.finances[5].cash = -5_000_000;
    lg.sim_season();
    assert!(lg.bankrupt_ai_teams().contains(&5));
    assert!(lg.complete());
}

#[test]
fn median_franchise_survives_the_season_on_autopilot() {
    let (mut bankrupt, mut total) = (0, 0);
    for seed in 1..=6 {
        let lg = season(seed * 13, None);
        for f in &lg.finances {
            total += 1;
            if f.bankrupt {
                bankrupt += 1;
            }
        }
    }
    assert!((bankrupt as f64 / total as f64) < 0.1);
}

#[test]
fn ticket_price_clamps_and_projection_falls_as_price_rises() {
    let mut lg = ca(3, Some(0));
    lg.set_ticket_price(0.1);
    assert_eq!(lg.my_finance().ticket_price, 1.0);
    lg.set_ticket_price(99.0);
    assert_eq!(lg.my_finance().ticket_price, 25.0);
    assert!(lg.project_attendance(4.0, Weather::Clear, 0.5) > lg.project_attendance(12.0, Weather::Clear, 0.5));
}

#[test]
fn promos_only_for_future_home_weeks_they_play_out_and_cost_money() {
    let mut lg = ca(4, Some(0));
    let home_week = lg.schedule.iter().find(|g| g.home == 0).unwrap().week;
    let away_week = lg.schedule.iter().find(|g| g.away == 0).unwrap().week;
    lg.finances[0].cash = 100_000;
    assert!(!lg.schedule_promo(away_week, "fireworks"));
    assert!(!lg.schedule_promo(home_week, "nope"));
    assert!(lg.schedule_promo(home_week, "fireworks"));
    lg.cancel_promo(home_week);
    assert!(lg.schedule_promo(home_week, "bobblehead"));
    for _ in 1..=home_week {
        lg.advance();
    }
    let g = lg.my_finance().games.iter().find(|x| x.week == home_week).unwrap();
    assert!(g.expenses[&LedgerKind::Promo] > 0);
    assert!(!lg.schedule_promo(home_week, "chili")); // already played
}

#[test]
fn facility_upgrade_and_loan_flow_through_a_season_and_the_books_balance() {
    let mut lg = ca(5, Some(0));
    let start = lg.my_finance().cash;
    assert!(lg.upgrade_facility(Facility::Seating));
    assert!(lg.take_loan(15_000));
    lg.sim_season();
    let f = lg.my_finance();
    assert_eq!(f.cash, start + f.ledger.iter().map(|e| e.amount).sum::<i64>());
    assert!(f.loan_balance < 15_000);
    assert_eq!(f.capacity(), lg.teams[0].town.capacity as i64 + 400);
    assert!(f.games.iter().any(|g| g.capacity == f.capacity()));
}

#[test]
fn owner_gets_sponsor_offers_on_day_one_and_after_each_season() {
    let mut lg = ca(6, Some(0));
    assert!(!lg.my_finance().offers.is_empty());
    let first = lg.my_finance().offers[0].name.clone();
    assert!(lg.my_finance_mut().accept_offer(0));
    assert!(lg.my_finance().sponsors.iter().any(|s| s.name == first));
    lg.sim_season();
    lg.begin_offseason();
    lg.finish_offseason();
    assert!(!lg.my_finance().offers.is_empty());
}

// ---------------------------------------------------------------- offseason_test

#[test]
fn begin_offseason_ratings_change_ages_plus_one_invariants_hold() {
    let mut lg = season(3, Some(0));
    let age_before: std::collections::HashMap<u32, i32> = lg.teams[0].roster.iter().map(|p| (p.id, p.age)).collect();
    let sum = lg.begin_offseason();
    assert_eq!(lg.phase, LeaguePhase::Offseason);
    assert!(!sum.user_changes.is_empty());
    for c in &sum.user_changes {
        assert_eq!(c.after.age, age_before[&c.before.id] + 1);
        for a in Attr::ALL {
            assert!((1..=c.after.potentials[a]).contains(&c.after.ratings[a]));
        }
    }
    assert_eq!(sum.user_changes.len() + sum.user_retired.len(), age_before.len());
    assert_eq!(lg.history.len(), 1);
    assert!(lg.free_agents.len() > 50);
}

#[test]
fn contracts_roll_down_the_user_decides_on_expiring_deals_ai_re_signs_or_releases() {
    let mut lg = season(4, Some(0));
    let before: std::collections::HashMap<u32, i32> = lg.finances[0].contracts.iter().map(|(k, c)| (*k, c.seasons_left)).collect();
    let sum = lg.begin_offseason();
    assert_eq!(sum.expiring.iter().copied().collect::<std::collections::BTreeSet<_>>(), lg.pending_expired);
    for id in &sum.expiring {
        assert_eq!(before[id], 1, "only one-year-left contracts expire");
    }
    for i in 1..12 {
        assert!(lg.finances[i].contracts.values().all(|c| c.seasons_left >= 1), "team {i}");
        let a: std::collections::BTreeSet<u32> = lg.finances[i].contracts.keys().copied().collect();
        let b: std::collections::BTreeSet<u32> = lg.teams[i].roster.iter().map(|p| p.id).collect();
        assert_eq!(a, b, "contracts and roster stay in sync (team {i})");
    }
}

#[test]
fn re_sign_via_negotiation_fair_accepted_lowball_rejected_unsigned_expiring_leave() {
    let mut lg = season(5, Some(0));
    let sum = lg.begin_offseason();
    assert!(!sum.expiring.is_empty());
    let id = sum.expiring[0];
    let p = lg.teams[0].roster.iter().find(|x| x.id == id).unwrap().clone();
    let resign = lg.resign_premium(id);
    let want = Negotiation::target(&p, &lg.teams[0].town, 3, lg.standings.records[0].pct(), resign.as_ref());
    assert_eq!(lg.negotiate(id, (want as f64 * 0.5).round() as i64, 3, 0, 1).outcome, OfferOutcome::Reject);
    let mid = lg.negotiate(id, (want as f64 * 0.9).round() as i64, 3, 0, 1);
    assert_eq!(mid.outcome, OfferOutcome::Counter);
    assert!(mid.counter_ppg.is_some());
    assert!(!lg.sign_player(id, (want as f64 * 0.5).round() as i64, 3, 0));
    assert!(lg.sign_player(id, want, 3, 0));
    assert!(!lg.pending_expired.contains(&id));
    assert_eq!(lg.finances[0].contracts[&id].seasons_left, 3);
    let others: Vec<u32> = lg.pending_expired.iter().copied().collect();
    let res = lg.finish_offseason();
    let re_signed: std::collections::HashSet<u32> = res.auto_signed.iter().map(|x| x.id).collect();
    for o in others.iter().filter(|o| !re_signed.contains(o)) {
        assert!(!lg.teams[0].roster.iter().any(|x| x.id == *o));
    }
    assert!(lg.teams[0].roster.iter().any(|x| x.id == id));
}

#[test]
fn re_sign_asks_rise_when_flush_and_when_a_50_plus_player_rode_the_bench() {
    let mut lg = season(5, Some(0));
    let sum = lg.begin_offseason();
    let id = sum.expiring[0];
    let p = lg.teams[0].roster.iter().find(|x| x.id == id).unwrap().clone();
    let town = lg.teams[0].town.clone();
    let win = lg.standings.records[0].pct();
    let plain = Negotiation::target(&p, &town, 3, win, None);
    assert_eq!(ResignPremium { cash: 50_000, wants_to_play: false }.mult(), 1.0);
    let flush = Negotiation::target(&p, &town, 3, win, Some(&ResignPremium { cash: 150_000, wants_to_play: false }));
    assert!(flush > plain);
    assert!(Negotiation::target(&p, &town, 3, win, Some(&ResignPremium { cash: 0, wants_to_play: true })) > plain);
    assert!(Negotiation::target(&p, &town, 3, win, Some(&ResignPremium { cash: 500_000, wants_to_play: true })) > flush);
    assert!(lg.resign_premium(u32::MAX).is_none());
    let r = Negotiation::respond(&p, &town, (plain as f64 * 0.5).round() as i64, 3, 0, 1, 0.5, Some(&ResignPremium { cash: 0, wants_to_play: true }));
    assert!(r.line.contains("play"), "{}", r.line);
}

#[test]
fn signing_a_free_agent_adds_him_with_a_contract_bonus_costs_cash_release_costs_a_share() {
    let mut lg = season(6, Some(0));
    lg.begin_offseason();
    let fa = lg.free_agents.iter().find(|p| p.position == Position::Wr).unwrap().clone();
    let want = Negotiation::target(&fa, &lg.teams[0].town, 2, lg.standings.records[0].pct(), None);
    let cash_before = lg.finances[0].cash;
    assert!(lg.sign_player(fa.id, want, 2, 500));
    assert!(lg.teams[0].roster.iter().any(|p| p.id == fa.id));
    assert!(!lg.free_agents.iter().any(|p| p.id == fa.id));
    assert_eq!(lg.finances[0].cash, cash_before - 500);
    let cost = lg.release_player(fa.id);
    assert_eq!(cost, (2.0 * 14.0 * want as f64 * Contract::RELEASE_PCT).round() as i64);
    assert!(lg.free_agents.iter().any(|p| p.id == fa.id));
}

#[test]
fn finish_offseason_starts_season_two_with_a_fresh_schedule_standings_and_full_ai_rosters() {
    let mut lg = season(8, Some(0));
    lg.begin_offseason();
    let res = lg.finish_offseason();
    assert_eq!(res.new_season, 2);
    assert_eq!(lg.phase, LeaguePhase::RegularSeason);
    assert_eq!(lg.week, 1);
    assert!(lg.results.is_empty());
    assert_eq!(lg.schedule.len(), 84);
    for i in 1..12 {
        for &(pos, n) in &ROSTER_MIX {
            assert!(lg.teams[i].roster.iter().filter(|p| p.position == pos).count() + 1 >= n, "{} {}", lg.teams[i].town.name, pos.label());
        }
        assert!(lg.teams[i].active().count() >= 40);
    }
    lg.sim_season();
    assert!(lg.champion().is_some());
}

#[test]
fn a_bankrupt_ai_team_folds_and_is_replaced_by_a_town_from_the_same_division_with_players_freed() {
    let mut lg = ca(8, Some(0));
    lg.finances[5].cash = -9_000_000; // Coastal division, slot 5
    lg.sim_season();
    assert!(lg.bankrupt_ai_teams().contains(&5));
    let old_name = lg.teams[5].town.team_name();
    let old_ids: std::collections::HashSet<u32> = lg.teams[5].roster.iter().map(|p| p.id).collect();
    lg.begin_offseason();
    let res = lg.finish_offseason();
    assert!(res.replaced.iter().any(|e| e.0 == old_name));
    assert_ne!(lg.teams[5].town.team_name(), old_name);
    assert_eq!(lg.teams[5].town.division, "Coastal");
    assert!(!lg.finances[5].bankrupt);
    assert_eq!(lg.teams.iter().map(|t| t.town.id.clone()).collect::<std::collections::HashSet<_>>().len(), 12);
    let everyone: std::collections::HashSet<u32> = lg.teams.iter().flat_map(|t| t.roster.iter().map(|p| p.id)).chain(lg.free_agents.iter().map(|p| p.id)).collect();
    assert!(old_ids.iter().filter(|id| everyone.contains(id)).count() > old_ids.len() / 3);
}

#[test]
fn a_new_league_deals_unique_nicknames_from_the_managed_table() {
    let (a, b) = (ca(9, Some(0)), ca(9, Some(0)));
    let names: Vec<String> = a.teams.iter().map(|t| t.town.nickname.clone()).collect();
    assert_eq!(names.iter().collect::<std::collections::HashSet<_>>().len(), 12);
    assert!(names.iter().all(|n| nickname_table().contains(n)));
    assert_eq!(a.used_nicknames, names.iter().cloned().collect());
    assert_eq!(b.teams.iter().map(|t| t.town.nickname.clone()).collect::<Vec<_>>(), names);
    let kept = League::new(10, NewLeague { nicknames: [("chico".to_string(), "Heatwave".to_string())].into(), ..Default::default() });
    assert_eq!(kept.teams.iter().find(|t| t.town.id == "chico").unwrap().town.nickname, "Heatwave");
    assert_eq!(kept.teams.iter().map(|t| t.town.nickname.clone()).collect::<std::collections::HashSet<_>>().len(), 12);
}

#[test]
fn a_reused_town_gets_a_new_nickname_and_mascot() {
    let mut lg = season(9, Some(0));
    let mut seen = std::collections::HashSet::new();
    let mut reused = false;
    for _ in 0..8 {
        lg.finances[4].bankrupt = true;
        if lg.phase == LeaguePhase::RegularSeason {
            lg.sim_season();
        }
        lg.begin_offseason();
        lg.finish_offseason();
        let id = lg.teams[4].town.id.clone();
        if !seen.insert(id) {
            reused = true;
        }
        assert_eq!(lg.teams.iter().map(|t| t.town.id.clone()).collect::<std::collections::HashSet<_>>().len(), 12);
    }
    assert!(reused);
    let names: Vec<String> = lg.teams.iter().map(|t| t.town.nickname.clone()).collect();
    assert_eq!(names.iter().collect::<std::collections::HashSet<_>>().len(), 12);
    assert!(names.iter().all(|n| lg.used_nicknames.contains(n)));
}

#[test]
fn replacement_nicknames_are_not_in_use_and_not_previously_used() {
    let mut lg = season(11, Some(0));
    let mut retired: std::collections::HashSet<String> = std::collections::HashSet::new();
    for _ in 0..12 {
        retired.extend(lg.teams.iter().map(|t| t.town.nickname.clone()));
        if lg.phase == LeaguePhase::RegularSeason {
            lg.sim_season();
        }
        for i in 0..lg.finances.len() {
            lg.finances[i].bankrupt = i == 4;
        }
        lg.begin_offseason();
        let before = lg.teams[4].town.nickname.clone();
        lg.finish_offseason();
        let after = lg.teams[4].town.nickname.clone();
        assert_ne!(after, before);
        assert!(!retired.contains(&after), "{after} was used before");
        assert_eq!(lg.teams.iter().filter(|t| t.town.nickname == after).count(), 1);
        assert!(lg.used_nicknames.contains(&after));
    }
}

#[test]
fn fan_favorites_stay_between_12_and_20_never_ol_k_p_and_regenerate() {
    let mut lg = ca(12, None);
    let eligible = |p: &sp_core::Player| !matches!(p.position, Position::Ot | Position::Og | Position::C | Position::K | Position::P);
    let favs = |lg: &League| -> Vec<sp_core::Player> { lg.teams.iter().flat_map(|t| t.roster.iter().filter(|p| lg.fan_favorite_ids.contains(&p.id)).cloned()).collect() };
    assert!((12..=20).contains(&lg.fan_favorite_ids.len()));
    for t in &lg.teams {
        assert!(t.roster.iter().any(|p| lg.fan_favorite_ids.contains(&p.id)), "{}", t.town.name);
    }
    assert!(favs(&lg).iter().all(eligible));
    for s in 1..=8 {
        lg.sim_season();
        lg.begin_offseason();
        lg.finish_offseason();
        assert!((12..=20).contains(&lg.fan_favorite_ids.len()), "season {s}");
        assert!(favs(&lg).iter().all(eligible));
    }
    let copy = LeagueSave::decode(&LeagueSave::encode(&lg)).unwrap();
    assert_eq!(copy.fan_favorite_ids, lg.fan_favorite_ids);
}

#[test]
fn a_new_local_fan_favorite_is_named_in_the_inbox_and_survives_the_free_agent_cap() {
    let mut lg = season(4, Some(0));
    lg.fan_favorite_ids.clear();
    lg.roll_fan_favorites(&mut Rng::new(3));
    lg.write_inbox_offseason(false);
    let mail: Vec<_> = lg.inbox.iter().filter(|i| i.kind == InboxKind::Fan && i.player_id.is_some()).cloned().collect();
    assert!(!mail.is_empty());
    for m in &mail {
        let p = lg.free_agents.iter().find(|x| Some(x.id) == m.player_id).unwrap();
        assert!(lg.fan_favorite_ids.contains(&p.id));
        assert!(is_local_hero(&m.text));
        assert_eq!(local_hero_name(&m.text).as_deref(), Some(p.name.as_str()));
    }
    for i in lg.inbox.iter().filter(|i| i.kind == InboxKind::Fan && !is_local_hero(&i.text)) {
        assert!(i.player_id.is_none());
    }
    let heroes: Vec<u32> = mail.iter().map(|m| m.player_id.unwrap()).collect();
    for _ in 0..400 {
        let mut g = lg.tg.players.generate(Position::Wr, GenOptions::default().tier(Tier::Regular).age(27).years_pro(4).acquired(ACQUIRED_FREE_AGENCY));
        g.ratings = sp_core::Ratings::new([90; 8]);
        g.potentials = sp_core::Ratings::new([99; 8]);
        lg.free_agents.push(g);
    }
    lg.begin_offseason();
    for id in heroes {
        assert!(lg.free_agents.iter().any(|p| p.id == id), "fan favorite {id} was cut from the pool");
    }
}

#[test]
fn promos_cannot_be_cancelled_once_played_and_the_offseason_clears_them() {
    let mut lg = ca(14, Some(0));
    let mut weeks: Vec<i32> = lg.schedule.iter().filter(|g| g.home == 0).map(|g| g.week).collect();
    weeks.sort();
    lg.finances[0].cash = 100_000;
    assert!(lg.schedule_promo(weeks[0], "beer"));
    assert!(lg.schedule_promo(weeks[3], "beer"));
    while lg.week <= weeks[0] {
        lg.advance();
    }
    assert!(!lg.cancel_promo(weeks[0]));
    assert!(lg.finances[0].promos.contains_key(&weeks[0]));
    assert!(lg.cancel_promo(weeks[3]));
    lg.schedule_promo(weeks[5], "beer");
    lg.sim_season();
    lg.begin_offseason();
    assert!(lg.finances[0].promos.is_empty());
}

fn avg_top_ovr(lg: &League) -> f64 {
    let (mut sum, mut n) = (0.0, 0.0);
    for t in &lg.teams {
        let mut top: Vec<i32> = t.roster.iter().map(|p| p.ovr()).collect();
        top.sort_by(|a, b| b.cmp(a));
        for o in top.iter().take(46) {
            sum += *o as f64;
            n += 1.0;
        }
    }
    sum / n
}

#[test]
fn multi_season_autopilot_league_stays_healthy_over_eight_seasons() {
    let mut lg = ca(11, None);
    let start_avg = avg_top_ovr(&lg);
    let mut folded = 0;
    let mut mid_avg = 0.0;
    for s in 1..=8 {
        lg.sim_season();
        assert!(lg.champion().is_some(), "season {s}");
        folded += lg.bankrupt_ai_teams().len();
        lg.begin_offseason();
        lg.finish_offseason();
        if s == 4 {
            mid_avg = avg_top_ovr(&lg);
        }
        assert_eq!(lg.teams.len(), 12);
        assert_eq!(lg.teams.iter().map(|t| t.town.id.clone()).collect::<std::collections::HashSet<_>>().len(), 12);
        let mut ids = std::collections::HashSet::new();
        for t in &lg.teams {
            for p in &t.roster {
                assert!(ids.insert(p.id), "duplicate player id {}", p.id);
                assert!((18..=45).contains(&p.age));
            }
        }
    }
    let end_avg = avg_top_ovr(&lg);
    // The yearly draft raises the league a few points, then it levels off: no runaway talent inflation.
    assert!((-6.0..=9.0).contains(&(end_avg - start_avg)), "talent drifted from {start_avg} to {end_avg}");
    assert!((end_avg - mid_avg).abs() < 3.5, "talent still moving: {mid_avg} after 4 seasons, {end_avg} after 8");
    assert!(folded < 10);
    assert_eq!(lg.season, 9);
    assert_eq!(lg.history.len(), 8);
}

#[test]
fn career_stat_rows_accumulate_per_season() {
    let mut lg = season(12, Some(0));
    lg.begin_offseason();
    lg.finish_offseason();
    lg.sim_season();
    lg.begin_offseason();
    let qb = lg.season_rows.values().find(|rows| rows.len() == 2 && rows.iter().any(|r| r.stats.get("pass_att").copied().unwrap_or(0.0) > 50.0)).expect("a quarterback with two seasons");
    assert_eq!(qb.iter().map(|r| r.season).collect::<Vec<_>>(), vec![1, 2]);
}

// ---------------------------------------------------------- signing off the street

#[test]
fn free_agents_can_be_signed_mid_season_and_dress_right_away() {
    let mut lg = ca(61, Some(0));
    lg.advance();
    lg.advance();
    assert_eq!(lg.phase, LeaguePhase::RegularSeason);
    let first = *lg.teams[0].active_ids.iter().next().unwrap();
    lg.teams[0].active_ids.remove(&first);
    let home = lg.me().town.name.clone();
    let fa = lg.tg.players.generate(Position::Wr, GenOptions::default().hometown(&home));
    lg.free_agents.push(fa.clone());
    let want = Negotiation::target(&fa, &lg.me().town, 1, lg.my_win_pct(), None);
    let cash = lg.finances[0].cash;
    assert!(lg.sign_player(fa.id, want, 1, 0));
    assert!(lg.me().roster.iter().any(|p| p.id == fa.id));
    assert!(!lg.free_agents.iter().any(|p| p.id == fa.id));
    assert!(lg.me().active_ids.contains(&fa.id));
    assert_eq!(lg.finances[0].contracts[&fa.id].seasons_left, 1);
    assert_eq!(lg.finances[0].cash, cash);
    lg.advance();
    assert!(lg.finances[0].games.last().unwrap().expenses[&LedgerKind::PlayerPay] > 0);
}

#[test]
fn a_lowball_is_refused_in_season_and_a_full_active_list_does_not_auto_dress_him() {
    let mut lg = ca(62, Some(0));
    let home = lg.me().town.name.clone();
    let fa = lg.tg.players.generate(Position::Rb, GenOptions::default().hometown(&home));
    lg.free_agents.push(fa.clone());
    let want = Negotiation::target(&fa, &lg.me().town, 3, lg.my_win_pct(), None);
    assert!(!lg.sign_player(fa.id, (want as f64 * 0.4).round() as i64, 3, 0));
    assert_eq!(lg.me().active_ids.len(), ACTIVE_ROSTER_SIZE);
    assert!(lg.sign_player(fa.id, want, 3, 0));
    assert_eq!(lg.me().active_ids.len(), ACTIVE_ROSTER_SIZE);
    assert!(!lg.me().active_ids.contains(&fa.id));
}

#[test]
fn open_tryouts_cost_money_bring_local_candidates_and_are_deterministic() {
    let make = || ca(63, Some(0));
    let mut a = make();
    let cash = a.finances[0].cash;
    let before = a.free_agents.len();
    let made = a.hold_tryouts(Some(Position::P), 8);
    assert_eq!(made.len(), 8);
    assert!(made.iter().all(|p| p.position == Position::P));
    assert!(made.iter().filter(|p| p.hometown == a.me().town.name).count() > 2);
    assert_eq!(a.free_agents.len(), before + 8);
    assert_eq!(a.finances[0].cash, cash - League::TRYOUT_COST);
    assert_eq!(a.finances[0].ledger.last().unwrap().memo, "open tryouts");
    let mut b = make();
    assert_eq!(b.hold_tryouts(Some(Position::P), 8).iter().map(|p| p.name.clone()).collect::<Vec<_>>(), made.iter().map(|p| p.name.clone()).collect::<Vec<_>>());
    // only one tryout per week
    assert!(a.hold_tryouts(None, 30).is_empty());
    a.tryout_held_this_week = false;
    let mix = a.hold_tryouts(None, 30);
    assert_eq!(mix.iter().map(|p| p.position).collect::<std::collections::HashSet<_>>().len(), Position::ALL.len());
    a.tryout_held_this_week = false;
    assert_eq!(a.hold_tryouts(None, 8).iter().map(|p| p.position).collect::<std::collections::HashSet<_>>().len(), Position::ALL.len());
}

#[test]
fn tryouts_need_cash_ids_stay_unique_and_saves_keep_the_new_candidates() {
    let mut lg = ca(64, Some(0));
    lg.finances[0].cash = 100;
    assert!(lg.hold_tryouts(Some(Position::K), 8).is_empty());
    lg.finances[0].cash = 50_000;
    lg.hold_tryouts(Some(Position::K), 8);
    let mut ids = std::collections::HashSet::new();
    for t in &lg.teams {
        for p in &t.roster {
            assert!(ids.insert(p.id));
        }
    }
    for p in &lg.free_agents {
        assert!(ids.insert(p.id));
    }
    let mut copy = LeagueSave::decode(&LeagueSave::encode(&lg)).unwrap();
    assert_eq!(copy.free_agents.len(), lg.free_agents.len());
    assert!(copy.tryout_held_this_week);
    copy.tryout_held_this_week = false;
    lg.tryout_held_this_week = false;
    assert_eq!(copy.hold_tryouts(Some(Position::K), 8).iter().map(|p| p.name.clone()).collect::<Vec<_>>(), lg.hold_tryouts(Some(Position::K), 8).iter().map(|p| p.name.clone()).collect::<Vec<_>>());
}
