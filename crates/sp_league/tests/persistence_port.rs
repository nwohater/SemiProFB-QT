//! Ported from the Dart `persistence_test.dart`, `region_test.dart`, `seed_test.dart`, `jersey_league_test.dart`,
//! `returner_stats_test.dart` and `absence_log_test.dart`.
use sp_core::jobs::JobTable;
use sp_core::names::NameBank;
use sp_core::returners::ReturnRole;
use sp_core::sizes::SizeTable;
use sp_core::strategy::*;
use sp_core::team::ACTIVE_ROSTER_SIZE;
use sp_core::{Position, Region, Rng};
use sp_economy::Facility;
use sp_league::persistence::LeagueSave;
use sp_league::{League, LeaguePhase, NewLeague};
use std::collections::BTreeSet;

fn ca(seed: u64, user: Option<usize>) -> League {
    League::new(seed, NewLeague { user_team: user, ..Default::default() })
}

fn sig(lg: &League) -> String {
    format!(
        "{}|{:?}|{}|{:?}|{}|{}|{}",
        lg.season,
        lg.phase,
        lg.week,
        lg.champion(),
        lg.results.iter().map(|r| format!("{}:{}-{}:{}-{}", r.week, r.home, r.away, r.home_score, r.away_score)).collect::<Vec<_>>().join(","),
        lg.finances.iter().map(|f| format!("{}/{:.2}/{}", f.cash, f.fan_interest, f.loan_balance)).collect::<Vec<_>>().join(","),
        lg.standings.records.iter().map(|r| r.to_string()).collect::<Vec<_>>().join(",")
    )
}

fn round_trip(lg: &League) -> League {
    LeagueSave::decode(&LeagueSave::encode(lg)).expect("loads")
}

// ---------------------------------------------------------------- persistence

#[test]
fn round_trip_save_mid_season_load_and_both_continue_identically() {
    let mut a = ca(41, Some(0));
    for _ in 0..6 {
        a.advance();
    }
    let mut b = round_trip(&a);
    assert_eq!(sig(&b), sig(&a));
    a.sim_season();
    b.sim_season();
    assert_eq!(sig(&b), sig(&a));
    assert_eq!(b.champion(), a.champion());
}

#[test]
fn saved_team_player_strategy_and_roster_details_survive() {
    let mut a = ca(42, Some(3));
    a.teams[3].strategy = Strategy {
        offense: OffenseStrategy { scheme: OffenseScheme::AirRaid, run_pass_bias: 0.77, pass_depth: 0.8, pocket: PocketStyle::Rolling, fourth_down_aggression: 0.6, ..Default::default() },
        defense: DefenseStrategy { front: DefenseFront::ThreeFour, coverage: Coverage::Man, blitz_frequency: 0.33, ..Default::default() },
        ..Default::default()
    };
    let b = round_trip(&a);
    assert_eq!(b.user_team, Some(3));
    assert_eq!(b.used_nicknames, a.used_nicknames);
    assert_eq!(b.teams.len(), 12);
    for i in 0..12 {
        assert_eq!(b.teams[i].town.team_name(), a.teams[i].town.team_name());
        assert_eq!(b.teams[i].roster, a.teams[i].roster);
        assert_eq!(b.teams[i].active_ids, a.teams[i].active_ids);
    }
    let s = b.teams[3].strategy;
    assert_eq!(s.offense.scheme, OffenseScheme::AirRaid);
    assert_eq!(s.offense.run_pass_bias, 0.77);
    assert_eq!(s.offense.pocket, PocketStyle::Rolling);
    assert_eq!((s.defense.front, s.defense.coverage, s.defense.blitz_frequency), (DefenseFront::ThreeFour, Coverage::Man, 0.33));
}

#[test]
fn owner_levers_persist_price_promos_facilities_loan_sponsors_offers() {
    let mut a = ca(43, Some(0));
    a.set_ticket_price(7.5);
    a.my_finance_mut().cash = 200_000;
    let home_week = a.schedule.iter().find(|g| g.home == 0).unwrap().week;
    assert!(a.schedule_promo(home_week, "bobblehead"));
    assert!(a.upgrade_facility(Facility::Lights));
    assert!(a.take_loan(12_000));
    assert!(a.my_finance_mut().accept_offer(0));
    let b = round_trip(&a);
    let (f, g) = (b.my_finance(), a.my_finance());
    assert_eq!(f.ticket_price, 7.5);
    assert_eq!(f.promos, g.promos);
    assert_eq!(f.level(Facility::Lights), 1);
    assert_eq!(f.loan_balance, 12_000);
    assert_eq!(f.sponsors.iter().map(|s| &s.name).collect::<Vec<_>>(), g.sponsors.iter().map(|s| &s.name).collect::<Vec<_>>());
    assert_eq!((f.offers.len(), f.cash, f.contracts.len()), (g.offers.len(), g.cash, g.contracts.len()));
}

#[test]
fn save_in_the_offseason_load_and_finish_deterministic_with_the_original() {
    let mut a = ca(44, Some(0));
    a.sim_season();
    a.begin_offseason();
    let mut b = round_trip(&a);
    assert_eq!(b.phase, LeaguePhase::Offseason);
    assert_eq!(b.free_agents.len(), a.free_agents.len());
    assert_eq!(b.pending_expired, a.pending_expired);
    a.finish_offseason();
    b.finish_offseason();
    assert_eq!(sig(&b), sig(&a));
    assert_eq!(b.teams.iter().map(|t| t.roster.len()).collect::<Vec<_>>(), a.teams.iter().map(|t| t.roster.len()).collect::<Vec<_>>());
    a.sim_season();
    b.sim_season();
    assert_eq!(sig(&b), sig(&a));
}

#[test]
fn history_career_rows_injuries_and_playoff_bracket_round_trip() {
    let mut a = ca(45, Some(0));
    a.sim_season();
    a.begin_offseason();
    a.finish_offseason();
    for _ in 0..16 {
        if !matches!(a.phase, LeaguePhase::RegularSeason | LeaguePhase::Semifinals | LeaguePhase::Championship) {
            break;
        }
        a.advance();
    }
    let b = round_trip(&a);
    assert_eq!(b.history.len(), 1);
    assert_eq!(b.history[0].champion_name, a.history[0].champion_name);
    assert_eq!(b.season_rows.len(), a.season_rows.len());
    assert_eq!(b.injured_games, a.injured_games);
    assert_eq!(b.playoffs.as_ref().unwrap().seeds, a.playoffs.as_ref().unwrap().seeds);
    assert_eq!(b.playoffs.as_ref().unwrap().champion, a.playoffs.as_ref().unwrap().champion);
    assert_eq!(b.playoffs.as_ref().unwrap().final_game, a.playoffs.as_ref().unwrap().final_game);
    assert_eq!(b.town_last_used, a.town_last_used);
}

#[test]
fn save_size_stays_reasonable() {
    let mut a = ca(46, Some(0));
    a.sim_season();
    assert!(LeagueSave::encode(&a).len() < 6 * 1024 * 1024);
}

// ---------------------------------------------------------- sizes in a league

#[test]
fn every_player_has_a_plausible_height_and_weight_and_linemen_outweigh_receivers() {
    let lg = ca(51, Some(0));
    let avg = |pos: Position| {
        let ps: Vec<f64> = lg.teams.iter().flat_map(|t| t.roster.iter().filter(|p| p.position == pos).map(|p| p.weight_lb as f64)).collect();
        ps.iter().sum::<f64>() / ps.len() as f64
    };
    for t in &lg.teams {
        for p in &t.roster {
            assert!((60..=84).contains(&p.height_in) && (150..=400).contains(&p.weight_lb));
        }
    }
    assert!(avg(Position::Ot) > avg(Position::Wr) + 80.0);
}

#[test]
fn sizes_survive_save_load_and_progression() {
    let mut a = ca(52, Some(0));
    a.sim_season();
    let before: std::collections::HashMap<u32, (i32, i32)> = a.teams.iter().flat_map(|t| t.roster.iter().map(|p| (p.id, (p.height_in, p.weight_lb)))).collect();
    let b = round_trip(&a);
    for p in b.teams.iter().flat_map(|t| t.roster.iter()) {
        assert_eq!((p.height_in, p.weight_lb), before[&p.id]);
    }
    a.begin_offseason();
    for p in a.teams.iter().flat_map(|t| t.roster.iter()) {
        if let Some(sz) = before.get(&p.id) {
            assert_eq!((p.height_in, p.weight_lb), *sz);
        }
    }
}

#[test]
fn a_custom_size_table_is_honored_for_new_entrants_and_free_agents() {
    let table = SizeTable::from_csv("position,min_height,max_height,min_weight,max_weight\nQB,70,70,200,200\n");
    let mut lg = League::new(54, NewLeague { sizes: Some(table), ..Default::default() });
    assert!(lg.teams[3].roster.iter().filter(|p| p.position == Position::Qb).all(|p| p.height_in == 70 && p.weight_lb == 200));
    lg.sim_season();
    lg.begin_offseason();
    assert!(lg.free_agents.iter().any(|p| p.position == Position::Qb));
    assert!(lg.free_agents.iter().filter(|p| p.position == Position::Qb && p.age < 28).all(|p| p.height_in == 70 && p.weight_lb == 200));
}

// ------------------------------------------------------- box scores in saves

#[test]
fn your_games_keep_their_box_scores_through_save_and_load_other_games_do_not() {
    let mut a = League::new(71, NewLeague { keep_box_scores: true, ..Default::default() });
    for _ in 0..5 {
        a.advance();
    }
    let b = round_trip(&a);
    assert!(b.keep_box_scores);
    for (i, (ra, rb)) in a.results.iter().zip(&b.results).enumerate() {
        let mine = ra.home == 0 || ra.away == 0;
        assert_eq!(rb.box_score.is_some(), mine, "game {i}");
        if mine {
            let (x, y) = (ra.box_score.as_ref().unwrap(), rb.box_score.as_ref().unwrap());
            assert_eq!(y.home_score, ra.home_score);
            assert_eq!(y.why.home, x.why.home);
            assert_eq!(y.scoring.len(), x.scoring.len());
        }
    }
}

#[test]
fn a_loaded_league_keeps_recording_box_scores_for_new_games_and_saves_stay_a_sane_size() {
    let mut a = League::new(72, NewLeague { keep_box_scores: true, ..Default::default() });
    a.advance();
    let mut b = round_trip(&a);
    b.advance();
    assert!(b.results.last().unwrap().box_score.is_some());
    let mut c = League::new(73, NewLeague { keep_box_scores: true, ..Default::default() });
    c.sim_season();
    assert!(LeagueSave::encode(&c).len() < 8 * 1024 * 1024);
}

#[test]
fn your_depth_chart_is_saved_and_used_after_loading() {
    let mut a = ca(81, Some(0));
    let mut rbs: Vec<_> = a.me().active().filter(|p| p.position == Position::Rb).map(|p| (p.id, p.ovr())).collect();
    rbs.sort_by_key(|x| std::cmp::Reverse(x.1));
    let (best, worst) = (rbs[0].0, rbs.last().unwrap().0);
    a.teams[0].set_depth(Position::Rb, vec![worst, best]);
    let mut b = round_trip(&a);
    assert_eq!(b.me().depth[&Position::Rb], vec![worst, best]);
    assert!(b.teams[1].depth.is_empty());
    a.sim_season();
    b.sim_season();
    assert_eq!(sig(&b), sig(&a));
}

// ------------------------------------------------------------------ job risks

#[test]
fn day_job_risks_players_get_hurt_or_stuck_on_work_trips_and_the_reason_survives_a_save() {
    let table = JobTable::from_csv("Job,InjuryRisk,TravelRisk\nRoofer,99,99\n");
    let mut lg = League::new(12, NewLeague { jobs: Some(table), ..Default::default() });
    assert!(lg.me().roster.iter().all(|p| p.day_job == "Roofer"));
    let res = lg.advance();
    assert!(!res.work_news.is_empty());
    assert!(!lg.out_reason.is_empty());
    let id = *lg.out_reason.keys().next().unwrap();
    assert!(lg.injured_games[&id] > 0);
    let mut b = round_trip(&lg);
    assert_eq!(b.out_reason, lg.out_reason);
    assert_eq!(b.jobs.lookup("Roofer").injury_risk, 99);
    lg.advance();
    b.advance();
    assert_eq!(sig(&b), sig(&lg));
}

#[test]
fn a_desk_job_almost_never_costs_games_a_risky_one_often_does() {
    let outs = |csv: &str| {
        let mut lg = League::new(5, NewLeague { jobs: Some(JobTable::from_csv(csv)), ..Default::default() });
        (0..10).map(|_| lg.advance().work_news.len()).sum::<usize>()
    };
    assert!(outs("Job,InjuryRisk,TravelRisk\nMattress Tester,2,4\n") < outs("Job,InjuryRisk,TravelRisk\nLogger,94,56\n"));
}

#[test]
fn auto_set_roster_injured_players_are_left_off_and_the_setting_survives_a_save() {
    let mut lg = ca(9, Some(0));
    let star = lg.me().roster.iter().max_by_key(|p| p.ovr()).unwrap().id;
    assert!(lg.me().active_ids.contains(&star));
    lg.injured_games.insert(star, 2);
    lg.auto_set_roster = true;
    lg.auto_set_active();
    assert!(!lg.me().active_ids.contains(&star));
    assert_eq!(lg.me().active_ids.len(), ACTIVE_ROSTER_SIZE);
    let mut b = round_trip(&lg);
    assert!(b.auto_set_roster);
    lg.advance();
    b.advance();
    assert_eq!(sig(&b), sig(&lg));
    assert_eq!(lg.me().active_ids.len(), ACTIVE_ROSTER_SIZE);
}

// --------------------------------------------------------------------- regions

fn region_league(id: &str, seed: u64, user: usize) -> League {
    let r = Region::by_id(Some(id));
    League::new(seed, NewLeague { user_team: Some(user), town_ids: Some(r.default_league_towns()), ..Default::default() })
}

#[test]
fn texas_league_twelve_teams_three_divisions_plays_a_season_and_an_offseason_and_survives_save_load() {
    let tx = Region::by_id(Some("tx"));
    let mut lg = region_league("tx", 5, 0);
    assert_eq!(lg.region.id, "tx");
    assert_eq!(lg.teams.len(), 12);
    for i in 0..12 {
        assert_eq!(lg.teams[i].town.division, tx.divisions[i / 4]);
    }
    lg.sim_season();
    assert!(lg.complete());
    assert_eq!(round_trip(&lg).region.id, "tx");
    lg.begin_offseason();
    lg.finish_offseason();
    assert!(lg.teams.iter().all(|t| tx.all_towns().any(|x| x.id == t.town.id)));
}

#[test]
fn florida_and_pennsylvania_leagues_play_seasons_and_offseasons_and_survive_save_load() {
    for (id, seed, user) in [("fl", 6u64, 2usize), ("pa", 8, 1)] {
        let region = Region::by_id(Some(id));
        let mut lg = region_league(id, seed, user);
        assert_eq!(lg.region.id, id);
        lg.sim_season();
        assert_eq!(round_trip(&lg).region.id, id);
        lg.begin_offseason();
        lg.finish_offseason();
        assert!(lg.teams.iter().all(|t| region.all_towns().any(|x| x.id == t.town.id)), "{id}");
    }
    assert!(!Region::by_id(Some("pa")).all_towns().any(|t| t.name == "Pittsburgh" || t.name == "Philadelphia"));
}

#[test]
fn pennsylvania_plays_snow_games() {
    let mut lg = region_league("pa", 8, 1);
    lg.sim_season();
    assert!(lg.results.iter().any(|r| r.weather == sp_core::Weather::Snow));
}

#[test]
fn texas_random_deal_four_per_division_in_order() {
    let tx = Region::by_id(Some("tx"));
    let ids = tx.deal_league_towns(&mut Rng::new(3));
    assert_eq!(ids.len(), 12);
    for i in 0..12 {
        assert_eq!(tx.town(&ids[i]).unwrap().division, tx.divisions[i / 4]);
    }
    assert_eq!(Region::of_towns(&ids).id, "tx");
}

#[test]
fn every_region_town_has_coordinates_and_an_abbreviation_and_ids_are_unique_across_regions() {
    let all: Vec<_> = Region::all().iter().flat_map(|r| r.all_towns()).collect();
    for t in &all {
        assert!(sp_core::geo::known_town(&t.name), "{}", t.name);
        assert!(sp_core::region::town_abbreviation(&t.id).is_some(), "{}", t.id);
    }
    let ids: Vec<&String> = all.iter().map(|t| &t.id).collect();
    assert_eq!(ids.iter().collect::<BTreeSet<_>>().len(), ids.len());
    for r in Region::all() {
        let abbrs: Vec<String> = r.all_towns().map(|t| t.abbr()).collect();
        assert_eq!(abbrs.iter().collect::<BTreeSet<_>>().len(), abbrs.len(), "{}", r.name);
        for d in &r.divisions {
            assert_eq!(r.starting.iter().filter(|t| &t.division == d).count(), 4, "{} {d}", r.name);
            assert_eq!(r.replacement.iter().filter(|t| &t.division == d).count(), 5, "{} {d}", r.name);
        }
    }
}

#[test]
fn no_region_uses_a_town_that_hosts_a_major_pro_team() {
    let pro = [
        "Miami", "Tampa", "Orlando", "Jacksonville", "St. Petersburg", "Philadelphia", "Pittsburgh", "Los Angeles", "San Francisco", "San Diego", "Oakland", "Sacramento", "San Jose", "Anaheim", "Houston", "Dallas",
        "Arlington", "San Antonio", "Austin", "Fort Worth",
    ];
    for t in Region::all().iter().flat_map(|r| r.all_towns()) {
        assert!(!pro.contains(&t.name.as_str()), "{}", t.name);
    }
}

// ------------------------------------------------------------------------ seed

#[test]
fn the_league_seed_is_kept_and_survives_save_and_load() {
    let mut lg = ca(1234, Some(3));
    assert_eq!(lg.seed, 1234);
    lg.advance();
    assert_eq!(round_trip(&lg).seed, 1234);
}

#[test]
fn a_full_set_of_chosen_nicknames_is_used_as_given() {
    let ca_region = Region::california();
    let ids = ca_region.default_league_towns();
    let nicks = sp_core::region::nickname_table();
    let pool: Vec<String> = nicks.iter().rev().take(12).cloned().collect();
    let chosen = ids.iter().cloned().zip(pool.iter().cloned()).collect();
    let lg = League::new(7, NewLeague { nicknames: chosen, ..Default::default() });
    for i in 0..12 {
        assert_eq!(lg.teams[i].town.nickname, pool[i]);
    }
    assert_eq!(lg.teams.iter().map(|t| t.town.nickname.clone()).collect::<BTreeSet<_>>().len(), 12);
}

#[test]
fn a_dealt_league_uses_exactly_the_towns_it_was_given_and_bankrupt_clubs_are_replaced_from_the_other_five() {
    let ca_region = Region::california();
    let ids = ca_region.deal_league_towns(&mut Rng::new(21));
    let mut lg = League::new(3, NewLeague { user_team: Some(2), town_ids: Some(ids.clone()), ..Default::default() });
    assert_eq!(lg.teams.iter().map(|t| t.town.id.clone()).collect::<Vec<_>>(), ids);
    assert_eq!(lg.me().town.id, ids[2]);
    let reserves: BTreeSet<String> = ca_region.all_towns().filter(|t| !ids.contains(&t.id)).map(|t| t.id.clone()).collect();
    assert_eq!(reserves.len(), 15);
    lg.sim_season();
    lg.finances[5].bankrupt = true;
    lg.begin_offseason();
    lg.finish_offseason();
    assert!(reserves.contains(&lg.teams[5].town.id), "replacement drawn from the unused towns of its division");
}

// ---------------------------------------------------------------------- jerseys

#[test]
fn no_team_ever_has_two_players_with_the_same_number_across_seasons_of_drafts_signings_and_trades() {
    fn no_dups(lg: &mut League, when: &str) {
        for t in &mut lg.teams {
            t.ensure_numbers();
            let ids: Vec<u32> = t.roster.iter().map(|p| p.id).collect();
            let nums: Vec<i32> = ids.iter().map(|id| t.number_of(*id).unwrap()).collect();
            assert_eq!(nums.iter().collect::<BTreeSet<_>>().len(), nums.len(), "{} {when}", t.town.name);
            assert!(nums.iter().all(|n| (1..=99).contains(n)), "{} {when}", t.town.name);
        }
    }
    let mut lg = ca(11, None);
    no_dups(&mut lg, "start");
    for s in 0..4 {
        lg.sim_season();
        no_dups(&mut lg, &format!("after season {}", s + 1));
        lg.begin_offseason();
        no_dups(&mut lg, &format!("offseason {}", s + 1));
        lg.finish_offseason();
        no_dups(&mut lg, &format!("rolled over {}", s + 1));
    }
    let mut back = round_trip(&lg);
    no_dups(&mut back, "after save");
    for i in 0..lg.teams.len() {
        assert_eq!(back.teams[i].numbers, lg.teams[i].numbers);
    }
}

// -------------------------------------------------------------------- returners

#[test]
fn a_quarterback_listed_as_punt_returner_returns_punts_and_the_stats_are_saved_on_him() {
    let mut lg = ca(7, Some(0));
    let qb = lg.me().active().find(|p| p.position == Position::Qb).unwrap().id;
    lg.teams[0].set_return_depth(ReturnRole::Punt, vec![qb]);
    lg.sim_season();
    let prs = lg.season_stats.get(&qb).map(|s| s.get("pr")).unwrap_or(0.0);
    assert!(prs > 0.0);
    let back = round_trip(&lg);
    assert_eq!(back.me().return_depth[&ReturnRole::Punt], vec![qb]);
    assert_eq!(back.season_stats.get(&qb).map(|s| s.get("pr")), Some(prs));
}

// --------------------------------------------------------------------- absences

#[test]
fn absence_log_both_kinds_get_logged_games_missed_never_exceed_games_out_and_it_survives_a_save() {
    let mut lg = ca(5, Some(0));
    lg.sim_season();
    assert!(!lg.absences.is_empty());
    assert!(lg.absences.iter().any(|a| a.work));
    assert!(lg.absences.iter().any(|a| !a.work));
    for a in &lg.absences {
        assert_eq!(a.season, 1);
        assert!(a.missed <= a.games, "{}", a.what);
        if !a.open {
            assert!(a.missed > 0, "{}", a.what);
        }
    }
    let id = lg.absences[0].player_id;
    assert!(lg.absences_for(id).iter().all(|a| a.player_id == id));
    let back = round_trip(&lg);
    assert_eq!(back.absences, lg.absences);
    lg.begin_offseason();
    assert!(lg.absences.iter().filter(|a| a.open).count() <= lg.injured_games.len());
}

#[allow(dead_code)]
fn _unused(_: NameBank) {}
