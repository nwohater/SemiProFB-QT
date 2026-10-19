//! Ported from the Dart `awards_test.dart`, `surprise_bills_test.dart`, `gazette_test.dart`, `owner_career_test.dart`, `ai_business_test.dart`.
use sp_core::Position;
use sp_league::ai_business::{ai_in_austerity, ai_payroll_budget, ai_reserve};
use sp_league::awards::{AwardTuning, defense_score};
use sp_league::gazette::{GAZETTE_ART_VARIANTS, GAZETTE_STORY_KINDS, GazetteSection, GazetteTuning, gazette_kind_family, gazette_variant_for};
use sp_league::hall_of_fame::HofTuning;
use sp_league::inbox::InboxKind;
use sp_league::persistence::LeagueSave;
use sp_league::surprise_bills::SurpriseBillTuning;
use sp_league::text::surprise_bills;
use sp_league::{League, LeaguePhase, NewLeague};
use std::collections::{BTreeMap, BTreeSet};

fn ca(seed: u64, user: Option<usize>) -> League {
    League::new(seed, NewLeague { user_team: user, ..Default::default() })
}

fn pos_of(lg: &League, id: u32) -> Position {
    lg.teams.iter().flat_map(|t| t.roster.iter()).find(|p| p.id == id).unwrap().position
}

// ---------------------------------------------------------------------- awards

#[test]
fn the_race_has_three_ordered_finalists_per_category_with_the_right_kinds_of_player() {
    let mut lg = ca(7, Some(0));
    assert!(lg.award_race(false).categories.iter().all(|c| c.finalists.is_empty()), "nothing before a game is played");
    lg.sim_season();
    let a = lg.award_race(false);
    assert!(!a.finalized);
    assert_eq!(a.categories.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(), ["mvp", "opoy", "dpoy", "ol", "roy"]);
    for c in a.categories.iter().filter(|c| c.id != "roy") {
        assert_eq!(c.finalists.len(), 3, "{}", c.id);
        for i in 1..3 {
            assert!(c.finalists[i - 1].score >= c.finalists[i].score);
        }
        assert_eq!(c.finalists.iter().map(|f| f.player_id).collect::<BTreeSet<_>>().len(), 3);
    }
    let off = [Position::Qb, Position::Rb, Position::Fb, Position::Wr, Position::Te];
    let def = [Position::De, Position::Dt, Position::Mlb, Position::Olb, Position::Cb, Position::S];
    assert!(a.category("opoy").unwrap().finalists.iter().all(|f| off.contains(&pos_of(&lg, f.player_id))));
    assert!(a.category("dpoy").unwrap().finalists.iter().all(|f| def.contains(&pos_of(&lg, f.player_id))));
    let ol = a.category("ol").unwrap();
    assert_eq!(ol.finalists.len(), 3);
    assert!(ol.finalists.iter().all(|f| [Position::Ot, Position::Og, Position::C].contains(&pos_of(&lg, f.player_id))));
    assert!(ol.winner().unwrap().line.contains("pancakes"));
    assert!(!a.category("mvp").unwrap().winner().unwrap().line.is_empty());
    // Rookies have no earlier season on record and are first-year players.
    for f in &a.category("roy").unwrap().finalists {
        let p = lg.teams.iter().flat_map(|t| t.roster.iter()).find(|p| p.id == f.player_id).unwrap();
        assert!(!lg.season_rows.contains_key(&p.id));
        assert!(p.is_rookie());
    }
}

#[test]
fn awards_are_final_after_week_14_before_the_playoffs_and_show_in_the_winners_bio() {
    let mut lg = ca(11, Some(2));
    while lg.phase == LeaguePhase::RegularSeason {
        assert!(lg.awards_history.is_empty());
        lg.advance();
    }
    assert_eq!(lg.awards_history.len(), 1);
    let locked = lg.awards_history[0].clone();
    assert!(locked.finalized);
    let mvp = locked.category("mvp").unwrap().winner().unwrap().clone();
    assert!(lg.awards_won(mvp.player_id).iter().any(|w| w.id == "mvp"));
    lg.sim_season();
    lg.begin_offseason();
    assert_eq!(lg.awards_history.len(), 1, "the offseason reuses the locked awards");
    assert_eq!(lg.awards_history[0].category("mvp").unwrap().winner().unwrap().player_id, mvp.player_id);
    assert_eq!(lg.inbox.iter().filter(|i| i.kind == InboxKind::Award && i.title.contains("awards")).count(), 1);
}

#[test]
fn awards_are_announced_in_the_inbox_saved_and_a_first_season_rookie_class_is_not_eligible_later() {
    let mut lg = ca(11, Some(2));
    lg.sim_season();
    let before = lg.award_race(false);
    lg.begin_offseason();
    assert_eq!(lg.awards_history.len(), 1);
    let saved = lg.awards_history[0].clone();
    assert!(saved.finalized && saved.season == 1);
    assert_eq!(saved.category("mvp").unwrap().winner().unwrap().player_id, before.category("mvp").unwrap().winner().unwrap().player_id);
    let msg: Vec<_> = lg.inbox.iter().filter(|i| i.kind == InboxKind::Award && i.title.contains("awards")).collect();
    assert_eq!(msg.len(), 1);
    assert!(msg[0].text.contains("Most Valuable Player"));
    let back = LeagueSave::decode(&LeagueSave::encode(&lg)).unwrap();
    assert_eq!(back.awards_history.len(), 1);
    assert_eq!(back.awards_history[0], saved);
    lg.finish_offseason();
    lg.sim_season();
    let two = lg.award_race(false);
    for f in &two.category("roy").unwrap().finalists {
        assert!(!lg.season_rows.contains_key(&f.player_id));
    }
}

#[test]
fn sacks_count_for_the_defensive_award() {
    let m = |v: &[(&str, f64)]| -> BTreeMap<String, f64> { v.iter().map(|(k, x)| (k.to_string(), *x)).collect() };
    let rusher = m(&[("tackles_solo", 45.0), ("tackles_ast", 20.0), ("sacks", 14.0), ("tfl", 9.0), ("qb_hits", 18.0), ("ff", 2.0)]);
    let backer = m(&[("tackles_solo", 70.0), ("tackles_ast", 35.0), ("sacks", 1.0), ("tfl", 6.0), ("int", 1.0), ("pd", 4.0)]);
    assert!(defense_score(&rusher) * AwardTuning::position_mult(Position::De) > defense_score(&backer) * AwardTuning::position_mult(Position::Mlb));
    // And over real seasons the league's sack leader is among the top few in the defensive race.
    let mut top_five = 0;
    for s in 1..=6u64 {
        let mut lg = ca(s * 13, None);
        lg.sim_season();
        let mut rows: Vec<(f64, f64)> = Vec::new();
        for t in &lg.teams {
            for p in &t.roster {
                let Some(st) = lg.season_stats.get(&p.id) else { continue };
                let st = st.to_map();
                if st.get("gp").copied().unwrap_or(0.0) < 7.0 {
                    continue;
                }
                rows.push((defense_score(&st) * AwardTuning::position_mult(p.position), st.get("sacks").copied().unwrap_or(0.0)));
            }
        }
        let leader = rows.iter().copied().reduce(|a, b| if a.1 >= b.1 { a } else { b }).unwrap();
        if rows.iter().filter(|r| r.0 > leader.0).count() < 5 {
            top_five += 1;
        }
    }
    assert!(top_five >= 4, "{top_five}");
}

#[test]
fn hall_of_fame_retirees_are_scored_the_best_are_inducted_ranking_survives_a_save() {
    let mut lg = ca(5, Some(0));
    for _ in 0..12 {
        lg.sim_season();
        lg.begin_offseason();
        lg.finish_offseason();
    }
    assert!(!lg.hall_of_fame.is_empty());
    let ins = lg.inductees();
    assert!(!ins.is_empty(), "12 seasons should produce at least one great career");
    assert!(ins.iter().all(|e| e.score >= HofTuning::INDUCT && e.seasons >= HofTuning::MIN_SEASONS));
    assert!(lg.near_misses().iter().all(|e| !e.inducted && e.score >= HofTuning::KEEP));
    let misses: Vec<f64> = lg.near_misses().iter().map(|e| e.score).collect();
    for i in 1..misses.len() {
        assert!(misses[i - 1] >= misses[i]);
    }
    let ids: Vec<u32> = lg.hall_of_fame.iter().map(|e| e.player_id).collect();
    assert_eq!(ids.iter().collect::<BTreeSet<_>>().len(), ids.len(), "nobody enters twice");
    let back = LeagueSave::decode(&LeagueSave::encode(&lg)).unwrap();
    assert_eq!(back.hall_of_fame, lg.hall_of_fame);
    assert!(lg.inbox.iter().any(|i| i.title.starts_with("Hall of Fame")));
}

// ----------------------------------------------------------------- surprise bills

#[test]
fn a_poor_club_never_gets_a_bill() {
    let mut lg = ca(3, Some(0));
    lg.my_finance_mut().cash = 50_000;
    assert_eq!(lg.roll_surprise_bill(None), 0);
    assert!(!lg.surprise_bill_attempted);
    assert!(!lg.inbox.iter().any(|i| i.kind == InboxKind::Bill));
}

#[test]
fn bills_land_sometimes_cost_20_to_50_percent_of_cash_are_logged_and_mailed() {
    let (mut hits, trials) = (0, 400);
    for s in 1..=trials {
        let mut lg = ca(s, Some(0));
        lg.my_finance_mut().cash = 200_000;
        let before = lg.my_finance().cash;
        let amt = lg.roll_surprise_bill(None);
        assert!(lg.surprise_bill_attempted);
        assert_eq!(lg.roll_surprise_bill(None), 0, "one attempt per season");
        if amt == 0 {
            assert_eq!(lg.my_finance().cash, before);
            continue;
        }
        hits += 1;
        assert!(amt as f64 >= before as f64 * 0.2 - 100.0 && amt as f64 <= before as f64 * 0.5 + 100.0);
        assert_eq!(lg.my_finance().cash, before - amt);
        let mail: Vec<_> = lg.inbox.iter().filter(|i| i.kind == InboxKind::Bill).collect();
        assert_eq!(mail.len(), 1);
        assert!(mail[0].text.contains('$') && !mail[0].text.contains('{'));
    }
    let rate = hits as f64 / trials as f64;
    assert!((rate - 0.25).abs() < 0.07, "{rate}");
}

#[test]
fn some_bills_are_a_league_relief_payment_the_money_goes_to_the_poorest_club_named_in_the_letter() {
    let mut relief = 0;
    for s in 1..=300 {
        let mut lg = ca(s, Some(0));
        lg.my_finance_mut().cash = 300_000;
        let mut poorest = 1;
        for i in 1..lg.teams.len() {
            if lg.finances[i].cash < lg.finances[poorest].cash {
                poorest = i;
            }
        }
        let cash_before: Vec<i64> = lg.finances.iter().map(|f| f.cash).collect();
        let amt = lg.roll_surprise_bill(None);
        if amt == 0 {
            continue;
        }
        let mail = lg.inbox.iter().find(|i| i.kind == InboxKind::Bill).unwrap().clone();
        let name = lg.teams[poorest].town.team_name();
        if !mail.text.contains(&name) {
            assert_eq!(lg.finances[poorest].cash, cash_before[poorest], "an ordinary bill pays nobody");
            continue;
        }
        relief += 1;
        assert_eq!(lg.finances[poorest].cash, cash_before[poorest] + amt);
        assert_eq!(lg.my_finance().cash, cash_before[0] - amt);
        assert!(!mail.title.contains('{') && !mail.text.contains('{'));
    }
    assert!(relief > 0);
}

#[test]
fn the_weekly_hook_waits_until_the_first_week_then_tries_once() {
    let mut lg = ca(9, Some(0));
    lg.my_finance_mut().cash = 300_000;
    assert_eq!(lg.maybe_roll_surprise_bill(SurpriseBillTuning::FIRST_WEEK - 1), 0);
    assert!(!lg.surprise_bill_attempted);
    lg.maybe_roll_surprise_bill(SurpriseBillTuning::FIRST_WEEK);
    assert!(lg.surprise_bill_attempted);
    let after = lg.my_finance().cash;
    assert_eq!(lg.maybe_roll_surprise_bill(SurpriseBillTuning::FIRST_WEEK + 1), 0);
    assert_eq!(lg.my_finance().cash, after);
}

#[test]
fn advance_rolls_midseason_not_at_start_season_and_the_flag_survives_a_save() {
    let mut lg = ca(11, Some(0));
    lg.my_finance_mut().cash = 300_000;
    for w in 1..SurpriseBillTuning::FIRST_WEEK {
        lg.advance();
        assert!(!lg.surprise_bill_attempted, "week {w}");
    }
    lg.my_finance_mut().cash = 300_000;
    lg.advance(); // week 5
    assert!(lg.surprise_bill_attempted);
    assert!(lg.inbox.iter().filter(|i| i.kind == InboxKind::Bill).count() <= 1);
    let copy = LeagueSave::decode(&LeagueSave::encode(&lg)).unwrap();
    assert!(copy.surprise_bill_attempted);
    lg.sim_season();
    lg.begin_offseason();
    lg.my_finance_mut().cash = 400_000;
    let bills_before = lg.inbox.iter().filter(|i| i.kind == InboxKind::Bill).count();
    lg.finish_offseason();
    assert!(!lg.surprise_bill_attempted);
    assert_eq!(lg.inbox.iter().filter(|i| i.kind == InboxKind::Bill).count(), bills_before);
}

#[test]
fn every_bill_has_a_title_a_body_with_an_amount_and_only_known_placeholders() {
    let titles: Vec<&String> = surprise_bills().iter().map(|b| &b.title).collect();
    assert!(titles.len() >= 35);
    assert_eq!(titles.iter().collect::<BTreeSet<_>>().len(), titles.len());
    let re = regex::Regex::new(r"\{(\w+)\}").unwrap();
    for b in surprise_bills() {
        assert!(b.body.contains("{amt}"), "{}", b.title);
        for m in re.captures_iter(&b.body) {
            assert!(["amt", "team", "town"].contains(&&m[1]), "{}", b.title);
        }
    }
}

// ----------------------------------------------------------------------- gazette

#[test]
fn every_week_gets_an_issue_with_a_front_page_and_a_score_line_per_game() {
    let mut lg = ca(21, Some(0));
    lg.sim_season();
    assert_eq!(lg.gazette.len(), 16);
    for i in &lg.gazette {
        assert_eq!(i.in_section(GazetteSection::Front).len(), 1, "{}", i.label);
        let games = if i.week <= 14 { 6 } else if i.week == 15 { 2 } else { 1 };
        assert_eq!(i.in_section(GazetteSection::Scores).len(), games, "{}", i.label);
    }
    assert_eq!(lg.gazette.last().unwrap().label, "Championship");
    assert!(lg.gazette.iter().all(|i| i.in_section(GazetteSection::Scores).iter().all(|s| s.game.is_some())));
    assert!(lg.gazette.last().unwrap().in_section(GazetteSection::Scores)[0].game.as_ref().unwrap().tag.contains("Championship"));
    assert!(lg.gazette.iter().any(|i| !i.in_section(GazetteSection::Spotlight).is_empty()));
    assert!(lg.gazette.iter().any(|i| !i.in_section(GazetteSection::Absences).is_empty()));
}

#[test]
fn writing_an_issue_does_not_touch_the_league_rng() {
    let mut lg = ca(22, Some(0));
    lg.advance();
    let before = lg.rng.state();
    let n = lg.gazette.len();
    lg.write_gazette(1, &BTreeMap::new());
    assert_eq!(lg.rng.state(), before);
    assert_eq!(lg.gazette.len(), n); // digests were cleared, so nothing new is written
}

#[test]
fn issues_are_trimmed_and_survive_a_save_and_load() {
    let mut lg = ca(23, Some(0));
    lg.sim_season();
    for _ in 0..2 {
        lg.begin_offseason();
        lg.finish_offseason();
        lg.sim_season();
    }
    assert_eq!(lg.gazette.len(), GazetteTuning::MAX_ISSUES);
    let back = LeagueSave::decode(&LeagueSave::encode(&lg)).unwrap();
    assert_eq!(back.gazette, lg.gazette);
}

#[test]
fn rotation_lead_kinds_vary_rare_stories_get_a_turn_and_the_same_kind_rarely_leads_twice() {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let (mut issues, mut repeats) = (0, 0);
    for seed in 1..=12 {
        let mut lg = ca(seed, Some(0));
        lg.sim_season();
        let mut prev: Option<String> = None;
        for i in &lg.gazette {
            let k = gazette_kind_family(i.lead().map(|l| l.kind.as_str()).unwrap_or(""));
            issues += 1;
            if prev.as_deref() == Some(k.as_str()) {
                repeats += 1;
            }
            prev = Some(k.clone());
            seen.insert(k);
        }
    }
    for k in ["upset", "star", "rivalry", "champion", "semifinal"] {
        assert!(seen.contains(k), "{seen:?}");
    }
    assert!(["shutout", "overtime", "blowout"].iter().filter(|k| seen.contains(**k)).count() >= 2, "{seen:?}");
    assert!((repeats as f64 / issues as f64) < 0.15, "{repeats}/{issues}");
}

#[test]
fn pictures_every_story_with_one_names_a_real_variant_and_variants_vary() {
    let mut lg = ca(25, Some(0));
    lg.sim_season();
    let re = regex::Regex::new(r"_\d+$").unwrap();
    let mut files: BTreeSet<String> = BTreeSet::new();
    for i in &lg.gazette {
        for st in i.stories.iter().filter(|s| !s.kind.is_empty()) {
            let base = re.replace(&st.kind, "").to_string();
            assert!(GAZETTE_STORY_KINDS.contains(&base.as_str()), "{base}");
            files.insert(st.kind.clone());
            assert!(matches!(st.section, GazetteSection::Front | GazetteSection::Briefs), "{}: only the lead and the briefs have a picture", i.label);
        }
        assert!(i.in_section(GazetteSection::Front).len() <= 1);
    }
    let seen: BTreeSet<usize> = (1..=6).flat_map(|season| (1..=16).map(move |w| gazette_variant_for(season, w))).collect();
    assert_eq!(seen, (1..=GAZETTE_ART_VARIANTS).collect());
    let numbered = regex::Regex::new(r"_\d$").unwrap();
    assert!(files.iter().any(|f| numbered.is_match(f)));
    assert!(files.iter().any(|f| !numbered.is_match(&f.replacen("rivalry_ot", "rivalry", 1))));
}

// ----------------------------------------------------------------- owner career

#[test]
fn every_finished_season_is_logged_with_a_consistent_finish_and_playoff_record() {
    let mut lg = ca(21, Some(2));
    for _ in 1..=6 {
        lg.sim_season();
        let champ = lg.champion() == lg.user_team;
        lg.begin_offseason();
        let o = lg.owner_seasons.last().unwrap().clone();
        assert_eq!(o.season, lg.season);
        assert_eq!(o.wins + o.losses + o.ties, 14);
        assert_eq!(o.champion(), champ);
        match o.finish.as_str() {
            "Champions" => assert_eq!((o.playoff_wins, o.playoff_losses), (2, 0)),
            "Runner-up" => assert_eq!((o.playoff_wins, o.playoff_losses), (1, 1)),
            "Semifinal exit" => assert_eq!((o.playoff_wins, o.playoff_losses), (0, 1)),
            other => {
                assert_eq!(other, "Missed the playoffs");
                assert_eq!(o.seed, 0);
                assert_eq!((o.playoff_wins, o.playoff_losses), (0, 0));
            }
        }
        lg.finish_offseason();
    }
    let c = lg.owner_career();
    assert_eq!(c.seasons.len(), 6);
    assert_eq!(c.wins() + c.losses() + c.ties(), 6 * 14);
    assert_eq!(c.playoff_appearances(), c.seasons.iter().filter(|s| s.seed > 0).count());
    assert!(c.titles() + c.runner_ups() <= c.playoff_appearances());
}

#[test]
fn the_career_survives_a_save_and_load_and_old_saves_are_rebuilt_from_history() {
    let mut lg = ca(22, Some(0));
    for _ in 1..=3 {
        lg.sim_season();
        lg.begin_offseason();
        lg.finish_offseason();
    }
    lg.log_club_help("Test Team", 4000);
    let back = LeagueSave::decode(&LeagueSave::encode(&lg)).unwrap();
    assert_eq!(back.owner_seasons, lg.owner_seasons);
    assert_eq!(back.owner_career().total_given(), 4000);
    assert_eq!(back.owner_career().clubs_helped(), 1);
    // An older save has no log: the seasons come back from history, with the playoff round unknown.
    lg.owner_seasons.clear();
    let rebuilt = lg.owner_career();
    assert_eq!(rebuilt.seasons.len(), 3);
    assert!(rebuilt.seasons.iter().all(|s| s.wins + s.losses + s.ties == 14));
}

#[test]
fn gifts_to_struggling_clubs_are_counted() {
    let mut lg = ca(23, Some(0));
    lg.log_club_help("A", 1000);
    lg.log_club_help("B", 2500);
    lg.log_club_help("A", 500);
    let c = lg.owner_career();
    assert_eq!((c.clubs_helped(), c.times_helped(), c.total_given()), (2, 3, 4000));
}

// ------------------------------------------------------------------ ai business

#[test]
fn payroll_budget_grows_with_spare_cash_and_shrinks_in_austerity() {
    let rev = 10_000.0;
    let reserve = ai_reserve(rev);
    let base = ai_payroll_budget(rev, reserve.round() as i64);
    assert!(ai_payroll_budget(rev, (reserve * 3.0).round() as i64) > base);
    let broke = ai_payroll_budget(rev, (reserve * 0.1).round() as i64);
    assert!(ai_in_austerity(rev, (reserve * 0.1).round() as i64));
    assert!(broke < base);
}

#[test]
fn rich_ai_owners_upgrade_facilities_book_promos_and_cash_does_not_pile_up() {
    let mut lg = ca(17, None);
    for _ in 0..5 {
        lg.sim_season();
        lg.begin_offseason();
        lg.finish_offseason();
    }
    let levels: Vec<i32> = lg.finances.iter().map(|f| f.facilities.values().sum()).collect();
    assert!(levels.iter().any(|l| *l > 0), "someone should have fixed up the stadium");
    assert!(lg.finances.iter().filter(|f| !f.promos.is_empty()).count() > 0);
    assert!(lg.finances.iter().map(|f| f.cash).max().unwrap() < 1_500_000);
    assert!(lg.teams.iter().all(|t| t.roster.len() >= 46));
}

#[test]
fn league_age_does_not_creep_up_much_over_eight_seasons() {
    let avg_age = |l: &League| {
        let ages: Vec<f64> = l.teams.iter().flat_map(|t| t.roster.iter().map(|p| p.age as f64)).collect();
        ages.iter().sum::<f64>() / ages.len() as f64
    };
    let mut lg = ca(5, None);
    let start = avg_age(&lg);
    for _ in 0..8 {
        lg.sim_season();
        lg.begin_offseason();
        lg.finish_offseason();
    }
    assert!(avg_age(&lg) < start + 1.2, "{} -> {}", start, avg_age(&lg));
}
