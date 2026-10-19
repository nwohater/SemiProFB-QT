use sp_core::generator::GenOptions;
use sp_core::team_generator::TeamGenerator;
use sp_core::{PlayerGenerator, Position, Region, Rng, Team, Town, Weather};
use sp_economy::business::{promo_by_id, sponsor_clauses};
use sp_economy::finance::AttOpts;
use sp_economy::*;

fn towns() -> &'static [Town] {
    &Region::california().starting
}

fn team(seed: u64, idx: usize) -> Team {
    TeamGenerator::new(PlayerGenerator::new(Rng::new(seed))).generate(&towns()[idx], &[])
}

fn gi(week: i32, home_won: bool, margin: i32) -> GameInput {
    GameInput::new(week, false, home_won, false, margin, Weather::Clear, false, 0.5)
}

fn settle(seed: u64, home: &Team, away: &Team, hf: &mut TeamFinance, af: &mut TeamFinance, week: i32, season: i32) -> (GameFinance, GameFinance) {
    EconomyEngine::settle(&mut Rng::new(seed), home, away, hf, af, &gi(week, true, 7), &gi(week, false, 7), season)
}

struct Setup {
    hf: TeamFinance,
    af: TeamFinance,
    home: Team,
    away: Team,
}

fn setup() -> Setup {
    let (home, away) = (team(21, 3), team(22, 5));
    Setup { hf: EconomyEngine::create(&home, &mut Rng::new(1)), af: EconomyEngine::create(&away, &mut Rng::new(2)), home, away }
}

#[test]
fn ticket_price_goodwill() {
    let t = &towns()[0];
    let r = TownEconomy::ref_price(t);
    assert!(EconomyEngine::price_fan_effect(t, r * 0.5, 900, 1000, 0.0) > 0.5);
    assert_eq!(EconomyEngine::price_fan_effect(t, r, 900, 1000, 0.0), 0.0);
    assert_eq!(EconomyEngine::price_fan_effect(t, r * 0.5, 500, 1000, 0.0), 0.0); // park not full
    assert_eq!(EconomyEngine::price_fan_effect(t, r * 0.5, 1000, 1000, 6.0), 0.0); // capped per season
    assert!(EconomyEngine::price_fan_effect(t, r * 1.6, 1000, 1000, 0.0) < 0.0);
    assert_eq!(EconomyEngine::price_fan_effect(t, r * 1.6, 1000, 1000, -4.0), 0.0);
}

#[test]
fn pay_scale_is_nonlinear_and_shorter_deals_cost_more() {
    let (low, mid, high) = (PayScale::ovr_curve(45), PayScale::ovr_curve(65), PayScale::ovr_curve(85));
    assert!(mid > low && high - mid > mid - low);
    let p = PlayerGenerator::built_in(Rng::new(2)).generate(Position::Qb, GenOptions::default().age(27).hometown("Fresno"));
    let t = &towns()[0];
    let (one, two, three) = (PayScale::ask(&p, t, 1), PayScale::ask(&p, t, 2), PayScale::ask(&p, t, 3));
    assert!(one > two && two > three && three >= 20);
}

#[test]
fn locals_are_cheaper_and_far_players_commute() {
    let mut g = PlayerGenerator::built_in(Rng::new(3));
    let t = &towns()[0]; // Fresno
    let local = g.generate(Position::Wr, GenOptions::default().age(26).hometown("Fresno"));
    let mut far = local.clone();
    far.hometown = "Pasadena".into();
    assert!(PayScale::ask(&far, t, 3) > PayScale::ask(&local, t, 3));
    assert!(PayScale::commute(sp_core::geo::town_miles("Pasadena", "Fresno")) > 0);
    assert_eq!(PayScale::commute(0.0), 0);
}

#[test]
fn retainer_and_release_cost() {
    assert!((20..=50).contains(&PayScale::retainer(200)));
    let c = Contract::new(1, 340, 2);
    assert_eq!(c.release_cost(), (14.0 * 2.0 * 340.0 * 0.125_f64).round() as i64);
}

#[test]
fn award_premium_is_capped() {
    assert_eq!(PayScale::award_mod(0), 1.0);
    assert!((PayScale::award_mod(3) - 1.6).abs() < 1e-9);
    assert!((PayScale::award_mod(50) - 2.0).abs() < 1e-9);
    assert_eq!(PayScale::award_note(3).as_deref(), Some("3 awards: +60%"));
    assert_eq!(PayScale::award_note(0), None);
}

#[test]
fn rookie_scale_slides_down_each_round() {
    assert_eq!(sp_economy::rookie_scale::RookieScale::ppg(1, 1, 12), 230);
    assert_eq!(sp_economy::rookie_scale::RookieScale::ppg(1, 12, 12), 150);
    assert_eq!(sp_economy::rookie_scale::RookieScale::seasons_for_round(1), 3);
    assert_eq!(sp_economy::rookie_scale::RookieScale::seasons_for_round(4), 1);
}

#[test]
fn gate_model() {
    let t = &towns()[3]; // Chico
    let r = TownEconomy::ref_price(t);
    let d = AttOpts::default();
    assert!(TownEconomy::attendance(t, 90.0, r, &d) > TownEconomy::attendance(t, 10.0, r, &d));
    // loyal towns still show up when the team stinks
    let ca = towns();
    let loyal = ca.iter().find(|x| x.id == "bakersfield").unwrap();
    let fickle = ca.iter().find(|x| x.id == "monterey").unwrap();
    let drop = |x: &Town| TownEconomy::hype(x, 0.0) / TownEconomy::hype(x, 50.0);
    assert!(drop(loyal) > drop(fickle));
    assert!(TownEconomy::attendance(t, 50.0, r * 2.0, &d) < TownEconomy::attendance(t, 50.0, r, &d));
    assert_eq!(TownEconomy::attendance(t, 100.0, 0.5, &d), t.capacity as i64);
    let base = TownEconomy::attendance(t, 50.0, r, &d);
    assert!(TownEconomy::attendance(t, 50.0, r, &AttOpts { weather: Weather::Rain, ..d }) < base);
    assert!(TownEconomy::attendance(t, 50.0, r, &AttOpts { rivalry: true, ..d }) > base);
}

#[test]
fn quiet_nights_have_a_wider_crowd_band_than_promo_nights() {
    let t = &towns()[3];
    let pricey = TownEconomy::ref_price(t) * 1.8; // stay off the sellout cap so the band is visible
    let quiet = TownEconomy::attendance_band(t, 50.0, pricey, &AttOpts::default());
    let promo = TownEconomy::attendance_band(t, 50.0, pricey, &AttOpts { promo: true, ..Default::default() });
    assert!(quiet.lo < quiet.mid && quiet.hi > quiet.mid);
    assert!(quiet.hi - quiet.lo > promo.hi - promo.lo);
}

#[test]
fn every_rostered_player_gets_a_contract_and_sponsors_exist() {
    let t = team(5, 3);
    let fin = EconomyEngine::create(&t, &mut Rng::new(1));
    assert_eq!(fin.contracts.len(), t.roster.len());
    assert!(!fin.sponsors.is_empty() && fin.cash > 0);
    for c in fin.contracts.values() {
        assert!((1..=3).contains(&c.seasons_left) && c.ppg >= 20);
    }
}

#[test]
fn cash_change_equals_ledger_sum_and_only_active_players_are_paid() {
    let (home, away) = (team(6, 3), team(7, 5));
    let mut hf = EconomyEngine::create(&home, &mut Rng::new(2));
    let mut af = EconomyEngine::create(&away, &mut Rng::new(3));
    let before = hf.cash;
    let (h, a) = EconomyEngine::settle(&mut Rng::new(4), &home, &away, &mut hf, &mut af, &gi(1, true, 7), &gi(1, false, 7), 1);
    assert_eq!(hf.cash - before, h.net());
    assert_eq!(hf.ledger.iter().map(|e| e.amount).sum::<i64>(), h.net());
    assert!(h.revenue.contains_key(&LedgerKind::Gate) && h.expenses.contains_key(&LedgerKind::Rent));
    assert!(a.expenses.contains_key(&LedgerKind::Travel) && !a.expenses.contains_key(&LedgerKind::Rent));
    let expected: i64 = home.roster.iter().filter(|p| home.active_ids.contains(&p.id)).map(|p| {
        let c = &hf.contracts[&p.id];
        c.active_pay() + c.win_bonus
    }).sum();
    assert_eq!(h.expenses[&LedgerKind::PlayerPay], expected);
    assert!(h.expenses[&LedgerKind::Retainers] > 0);
    assert!(a.revenue[&LedgerKind::Gate] > 0);
}

#[test]
fn winning_lifts_fan_interest_losing_lowers_it() {
    let (home, away) = (team(8, 3), team(9, 5));
    let mut hf = EconomyEngine::create(&home, &mut Rng::new(2));
    let mut af = EconomyEngine::create(&away, &mut Rng::new(3));
    let (h0, a0) = (hf.fan_interest, af.fan_interest);
    EconomyEngine::settle(&mut Rng::new(1), &home, &away, &mut hf, &mut af, &gi(1, true, 14), &gi(1, false, 14), 1);
    assert!(hf.fan_interest > h0 && af.fan_interest < a0);
}

#[test]
fn injuries_cost_medical_money() {
    let (home, away) = (team(10, 3), team(11, 5));
    let mut hf = EconomyEngine::create(&home, &mut Rng::new(2));
    let mut af = EconomyEngine::create(&away, &mut Rng::new(3));
    let mut hi = gi(1, true, 7);
    hi.injury_severities = vec!["seasonEnding".into(), "minor".into()];
    let (h, _) = EconomyEngine::settle(&mut Rng::new(1), &home, &away, &mut hf, &mut af, &hi, &gi(1, false, 7), 1);
    assert_eq!(h.expenses[&LedgerKind::Medical], 1400 + 120);
}

#[test]
fn bankruptcy_ladder_three_straight_negative_game_days() {
    let (home, away) = (team(12, 3), team(13, 5));
    let mut hf = EconomyEngine::create(&home, &mut Rng::new(2));
    let mut af = EconomyEngine::create(&away, &mut Rng::new(3));
    hf.cash = -1_000_000;
    assert_eq!(hf.status(&home), CashStatus::Negative);
    for w in 1..=3 {
        assert!(!hf.bankrupt);
        settle(w as u64, &home, &away, &mut hf, &mut af, w, 1);
        if w == 2 {
            assert_eq!(hf.status(&home), CashStatus::FinalNotice);
        }
    }
    assert!(hf.bankrupt);
    assert_eq!(hf.status(&home), CashStatus::Bankrupt);
}

#[test]
fn recovering_above_zero_resets_the_overdue_counter() {
    let (home, away) = (team(14, 3), team(15, 5));
    let mut hf = EconomyEngine::create(&home, &mut Rng::new(2));
    let mut af = EconomyEngine::create(&away, &mut Rng::new(3));
    hf.cash = -10;
    hf.overdue_games = 1;
    hf.ticket_price = 1.0;
    settle(1, &home, &away, &mut hf, &mut af, 1, 1);
    if hf.cash >= 0 {
        assert_eq!(hf.overdue_games, 0);
    }
}

#[test]
fn promo_night_lifts_the_crowd_costs_money_and_is_logged() {
    let (mut with, mut without) = (0, 0);
    for k in 0..20 {
        let mut s = setup();
        let (h0, _) = settle(k, &s.home, &s.away, &mut s.hf, &mut s.af, 1, 1);
        without += h0.attendance;
        let mut s2 = setup();
        s2.hf.promos.insert(1, "fireworks".into());
        let (h1, _) = settle(k, &s2.home, &s2.away, &mut s2.hf, &mut s2.af, 1, 1);
        with += h1.attendance;
        assert!(h1.expenses[&LedgerKind::Promo] > 0);
        assert_ne!(s2.hf.cash - s.hf.cash, 0);
    }
    assert!(with > without);
}

#[test]
fn promo_mishap_is_stored_even_without_an_extra_fine() {
    let mut seen = false;
    for k in 0..80 {
        let mut s = setup();
        s.hf.promos.insert(1, "fireworks".into());
        let (h, _) = settle(k + 90, &s.home, &s.away, &mut s.hf, &mut s.af, 1, 1);
        assert_eq!(h.promo_id.as_deref(), Some("fireworks"));
        if !h.promo_mishap {
            continue;
        }
        seen = true;
        assert_eq!(h.expenses[&LedgerKind::Promo], promo_by_id("fireworks").unwrap().cost(&s.home.town));
        break;
    }
    assert!(seen);
}

#[test]
fn the_same_ticket_price_does_not_always_draw_the_same_crowd() {
    let crowds: std::collections::HashSet<i64> = (0..40)
        .map(|k| {
            let mut s = setup();
            settle(k + 40, &s.home, &s.away, &mut s.hf, &mut s.af, 1, 1).0.attendance
        })
        .collect();
    assert!(crowds.len() > 8, "{}", crowds.len());
}

#[test]
fn promo_less_nights_swing_harder_than_promo_nights() {
    let spread = |promo: bool| {
        let samples: Vec<f64> = (0..80)
            .map(|k| {
                let mut s = setup();
                if promo {
                    s.hf.promos.insert(1, "chili".into());
                }
                settle(200 + k, &s.home, &s.away, &mut s.hf, &mut s.af, 1, 1).0.attendance as f64
            })
            .collect();
        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        let var = samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / samples.len() as f64;
        var.sqrt() / mean
    };
    assert!(spread(false) > spread(true));
}

#[test]
fn stadium_upgrades_add_seats_cost_cash_add_upkeep_and_max_out() {
    let mut s = setup();
    s.hf.cash = 500_000;
    let cap0 = s.hf.capacity();
    let c0 = s.hf.upgrade_cost(Facility::Seating).unwrap();
    let before = s.hf.cash;
    assert!(s.hf.upgrade(Facility::Seating, 0, 0));
    assert_eq!(s.hf.capacity(), cap0 + 400);
    assert_eq!(s.hf.cash, before - c0);
    assert!(s.hf.upgrade_cost(Facility::Seating).unwrap() > c0);
    assert_eq!(s.hf.upkeep_per_game(), 30);
    for _ in 0..6 {
        s.hf.upgrade(Facility::Seating, 0, 0);
    }
    assert_eq!(s.hf.level(Facility::Seating), 4);
    assert_eq!(s.hf.upgrade_cost(Facility::Seating), None);
    s.hf.cash = 10;
    assert!(!s.hf.upgrade(Facility::Lights, 0, 0));
}

#[test]
fn tearing_down_a_level_cuts_upkeep_and_seats_without_a_refund() {
    let mut s = setup();
    s.hf.cash = 500_000;
    let cap0 = s.hf.capacity();
    s.hf.upgrade(Facility::Seating, 0, 0);
    s.hf.upgrade(Facility::Seating, 0, 0);
    let cash = s.hf.cash;
    assert_eq!(s.hf.upkeep_per_game(), 60);
    assert!(s.hf.downgrade(Facility::Seating));
    assert_eq!((s.hf.level(Facility::Seating), s.hf.upkeep_per_game(), s.hf.capacity(), s.hf.cash), (1, 30, cap0 + 400, cash));
    assert!(s.hf.downgrade(Facility::Seating));
    assert!(!s.hf.downgrade(Facility::Seating));
    assert_eq!(s.hf.upkeep_per_game(), 0);
}

#[test]
fn upkeep_is_an_expense_and_concession_upgrades_raise_concession_revenue() {
    let (mut base, mut up) = (0, 0);
    for k in 0..20 {
        let mut s = setup();
        base += settle(k, &s.home, &s.away, &mut s.hf, &mut s.af, 1, 1).0.revenue[&LedgerKind::Concessions];
        let mut s2 = setup();
        s2.hf.facilities.insert(Facility::Concessions, 3);
        let (h1, _) = settle(k, &s2.home, &s2.away, &mut s2.hf, &mut s2.af, 1, 1);
        up += h1.revenue[&LedgerKind::Concessions];
        assert_eq!(h1.expenses[&LedgerKind::Upkeep], 45);
    }
    assert!(up > base);
}

#[test]
fn loans_add_cash_charge_interest_and_take_a_minimum_payment() {
    let mut s = setup();
    assert!(!s.hf.take_loan(1_000_000, 0, 0)); // above the credit limit
    let limit = s.hf.credit_limit();
    assert!(s.hf.take_loan(20_000, 0, 0));
    assert_eq!(s.hf.loan_balance, 20_000);
    assert!(limit > 20_000);
    let rate = s.hf.loan_rate();
    let (h, _) = settle(1, &s.home, &s.away, &mut s.hf, &mut s.af, 1, 1);
    assert_eq!(h.expenses[&LedgerKind::Interest], (20_000.0 * rate).round() as i64);
    assert_eq!(h.expenses[&LedgerKind::LoanPayment], 600);
    assert_eq!(s.hf.loan_balance, 19_400);
    assert!(s.hf.repay_loan(400, 0, 0));
    assert_eq!(s.hf.loan_balance, 19_000);
    s.hf.cash = 100;
    assert!(!s.hf.repay_loan(5000, 0, 0)); // can't repay more than the cash on hand
}

#[test]
fn debt_bites_rate_climbs_the_line_shrinks_and_the_bank_freezes_spending() {
    let mut s = setup();
    s.hf.cash = 1_000_000;
    let base = s.hf.loan_rate();
    assert!((base - EconTuning::LOAN_BASE_RATE).abs() < 1e-9);
    let limit = s.hf.credit_limit();
    assert!(s.hf.take_loan((limit as f64 * 0.8).round() as i64, 0, 0));
    assert!(s.hf.loan_rate() > base + 0.01);
    assert!(s.hf.under_debt_covenant());
    assert!(!s.hf.upgrade(Facility::Seating, 0, 0));
    let bal = s.hf.loan_balance;
    s.hf.repay_loan(bal, 0, 0);
    assert!(s.hf.upgrade(Facility::Seating, 0, 0));
    s.hf.last_win_pct = 0.0;
    assert!(s.hf.credit_limit() < limit);
    s.hf.last_win_pct = 1.0;
    assert!(s.hf.credit_limit() > limit);
}

#[test]
fn non_payroll_costs_creep_up_every_season_to_a_cap() {
    assert_eq!(EconTuning::cost_index(1), 1.0);
    assert!(EconTuning::cost_index(5) > EconTuning::cost_index(2));
    assert_eq!(EconTuning::cost_index(500), 1.0 + EconTuning::INFLATION_CAP);
    let mut s = setup();
    let (h1, _) = settle(1, &s.home, &s.away, &mut s.hf, &mut s.af, 1, 1);
    let mut s2 = setup();
    let (h9, _) = settle(1, &s2.home, &s2.away, &mut s2.hf, &mut s2.af, 1, 9);
    assert!(h9.expenses[&LedgerKind::Rent] > h1.expenses[&LedgerKind::Rent]);
    assert_eq!(h9.expenses[&LedgerKind::PlayerPay], h1.expenses[&LedgerKind::PlayerPay]);
}

#[test]
fn a_loan_keeps_a_cash_strapped_team_out_of_bankruptcy() {
    let mut s = setup();
    s.hf.cash = -500;
    assert!(s.hf.take_loan(30_000, 0, 0));
    settle(1, &s.home, &s.away, &mut s.hf, &mut s.af, 1, 1);
    assert!(s.hf.cash > 0 && s.hf.overdue_games == 0);
}

#[test]
fn sponsor_market_batches_never_repeat_and_avoid_lists_are_honored() {
    let s = setup();
    for seed in 0..200 {
        let batch = SponsorMarket::generate(&s.home.town, &mut Rng::new(seed), 6, 0, &[], &[]);
        let names: std::collections::HashSet<&String> = batch.iter().map(|o| &o.name).collect();
        let ids: std::collections::HashSet<&String> = batch.iter().map(|o| &o.clause_id).collect();
        assert_eq!((names.len(), ids.len()), (6, 6));
        let an: Vec<String> = batch.iter().map(|o| o.name.clone()).collect();
        let ac: Vec<String> = batch.iter().map(|o| o.clause_id.clone()).collect();
        let more = SponsorMarket::generate(&s.home.town, &mut Rng::new(seed + 1000), 1, 0, &an, &ac).remove(0);
        assert!(!an.contains(&more.name) && !ac.contains(&more.clause_id));
    }
}

#[test]
fn sponsor_slots_follow_fan_interest_and_held_deals_stay() {
    let mut s = setup();
    let slots: Vec<usize> = [0.0, 29.0, 30.0, 44.0, 45.0, 64.0, 65.0, 100.0].iter().map(|&i| SponsorMarket::slots_for(i)).collect();
    assert_eq!(slots, [1, 1, 2, 2, 3, 3, 4, 4]);
    s.hf.offers.extend(SponsorMarket::generate(&s.home.town, &mut Rng::new(3), 6, 0, &[], &[]));
    s.hf.fan_interest = 40.0; // 2 slots, already holding 2
    assert_eq!(s.hf.sponsor_slots(), 2);
    assert!(!s.hf.accept_offer(0));
    assert_eq!(s.hf.next_slot_interest(), Some(45.0));
    s.hf.fan_interest = 20.0; // fewer slots than deals held: nothing is cancelled
    assert_eq!(s.hf.sponsors.len(), 2);
    s.hf.fan_interest = 45.0;
    assert!(s.hf.accept_offer(0));
}

#[test]
fn sponsor_accept_four_slot_cap_and_cancel_cost() {
    let mut s = setup();
    s.hf.fan_interest = 70.0;
    s.hf.offers.extend(SponsorMarket::generate(&s.home.town, &mut Rng::new(3), 6, 0, &[], &[]));
    assert_eq!(s.hf.sponsors.len(), 2);
    assert!(s.hf.accept_offer(0) && s.hf.accept_offer(0));
    assert_eq!(s.hf.sponsors.len(), 4);
    assert!(!s.hf.accept_offer(0)); // full
    let (cash, pay) = (s.hf.cash, s.hf.sponsors[0].per_game);
    assert!(s.hf.cancel_sponsor(0, 0, 0));
    assert_eq!((s.hf.cash, s.hf.sponsors.len()), (cash - pay, 3));
}

#[test]
fn performance_clause_ends_a_deal_ai_auto_renews_owner_gets_offers() {
    let mut s = setup();
    let mut fussy = SponsorDeal::new("Fussy Motors", 500, 3);
    fussy.clause = "win 55%".into();
    fussy.min_win_pct = 0.55;
    s.hf.sponsors = vec![fussy, SponsorDeal::new("Chill Co", 400, 3)];
    s.hf.roll_season(&mut Rng::new(1), 0.3, false);
    assert_eq!(s.hf.sponsors.iter().map(|x| x.name.as_str()).collect::<Vec<_>>(), ["Chill Co"]);
    assert!(!s.hf.offers.is_empty());
    let mut s2 = setup();
    s2.af.sponsors = vec![SponsorDeal::new("Old Deal", 300, 1)];
    s2.af.roll_season(&mut Rng::new(2), 0.5, true);
    assert_eq!(s2.af.sponsors.len(), 1); // renewed with a new one
    assert_ne!(s2.af.sponsors[0].name, "Old Deal");
}

#[test]
fn sponsor_clauses_actually_change_the_books() {
    let mut s = setup();
    let perwin = sponsor_clauses().iter().find(|c| c.id == "perwin").unwrap();
    let mut deal = SponsorDeal::new("Winners Inc", 400, 2);
    deal.clause_id = perwin.id.clone();
    s.hf.sponsors = vec![deal];
    assert_eq!(s.hf.sponsor_income(true), 400 + 160);
    assert_eq!(s.hf.sponsor_income(false), 400);
}

#[test]
fn ledger_still_equals_cash_with_every_lever_in_play() {
    let mut s = setup();
    let start = s.hf.cash;
    s.hf.upgrade(Facility::Concessions, 0, 0);
    s.hf.take_loan(10_000, 0, 0);
    s.hf.promos.insert(1, "chili".into());
    settle(1, &s.home, &s.away, &mut s.hf, &mut s.af, 1, 1);
    s.hf.repay_loan(500, 0, 0);
    assert_eq!(s.hf.cash, start + s.hf.ledger.iter().map(|e| e.amount).sum::<i64>());
}

#[test]
fn finance_survives_json() {
    let mut s = setup();
    s.hf.upgrade(Facility::Lights, 0, 0);
    s.hf.promos.insert(2, "beer".into());
    settle(1, &s.home, &s.away, &mut s.hf, &mut s.af, 1, 1);
    let json = serde_json::to_string(&s.hf.for_save(400)).unwrap();
    let back: TeamFinance = serde_json::from_str(&json).unwrap();
    assert_eq!(serde_json::to_string(&back).unwrap(), json);
    assert_eq!(back.cash, s.hf.cash);
    assert_eq!(back.contracts.len(), s.hf.contracts.len());
    assert_eq!(back.level(Facility::Lights), 1);
}

#[test]
fn player_replies_come_in_four_flavors_stay_consistent_and_fill_in_the_counter_price() {
    let (mut yes, mut no, mut counter) = (std::collections::HashSet::new(), std::collections::HashSet::new(), std::collections::HashSet::new());
    let mut pg = PlayerGenerator::new(Rng::new(3));
    let town = &towns()[0];
    let re = |s: &str, job: &str| {
        let digits: String = s.chars().map(|c| if c.is_ascii_digit() { '#' } else { c }).collect();
        let collapsed = digits.replace("##", "#").replace("##", "#");
        collapsed.replace(&job.to_lowercase(), "JOB")
    };
    for _ in 0..24 {
        let p = pg.generate(Position::Wr, GenOptions::default());
        let want = Negotiation::target(&p, town, 3, 0.5, None);
        let a = Negotiation::respond(&p, town, want * 2, 3, 0, 1, 0.5, None);
        let b = Negotiation::respond(&p, town, (want as f64 * 0.2).round() as i64, 3, 0, 1, 0.5, None);
        let c = Negotiation::respond(&p, town, (want as f64 * 0.9).round() as i64, 3, 0, 1, 0.5, None);
        assert_eq!((a.outcome, b.outcome, c.outcome), (OfferOutcome::Accept, OfferOutcome::Reject, OfferOutcome::Counter));
        assert!(c.line.contains(&format!("${}", c.counter_ppg.unwrap())));
        assert_eq!(Negotiation::respond(&p, town, want * 2, 3, 0, 1, 0.5, None).line, a.line); // same player, same voice
        yes.insert(a.line);
        no.insert(b.line);
        counter.insert(re(&c.line, &p.day_job));
    }
    assert_eq!((yes.len(), no.len()), (4, 4));
    assert!(counter.len() >= 4);
}

#[test]
fn resign_premium_stacks_flush_and_benched() {
    let calm = ResignPremium::default();
    assert_eq!(calm.mult(), 1.0);
    let rich = ResignPremium { cash: 250_000, wants_to_play: false };
    assert!((rich.mult() - 1.15).abs() < 1e-9);
    let both = ResignPremium { cash: 500_000, wants_to_play: true };
    assert!((both.mult() - 1.25 * 1.12).abs() < 1e-9);
    assert_eq!(both.notes().len(), 2);
}

#[test]
fn contract_for_an_accepted_deal_carries_commute_and_bonus() {
    let mut g = PlayerGenerator::built_in(Rng::new(5));
    let mut p = g.generate(Position::Wr, GenOptions::default().hometown("Pasadena"));
    p.awards = 0;
    let c = Negotiation::contract_for(&p, &towns()[0], 200, 2);
    assert_eq!((c.ppg, c.seasons_left, c.win_bonus), (200, 2, 20));
    assert!(c.commute > 0);
}
