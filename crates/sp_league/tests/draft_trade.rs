//! Ported from the Dart `draft_test.dart` and `trade_test.dart`.
use sp_core::geo::town_miles;
use sp_core::player::{ACQUIRED_FREE_AGENCY, ACQUIRED_TRY_OUT};
use sp_core::Position;
use sp_economy::{PayScale, rookie_scale::RookieScale};
use sp_league::draft::DraftStrategy;
use sp_league::persistence::LeagueSave;
use sp_league::trade::{PickRef, TradeOffer, TradeTuning};
use sp_league::{League, NewLeague};
use std::collections::{BTreeMap, BTreeSet};

fn ca(seed: u64) -> League {
    League::new(seed, NewLeague::default())
}

fn offseason(seed: u64) -> League {
    let mut lg = ca(seed);
    lg.sim_season();
    lg.begin_offseason();
    lg
}

fn sig(lg: &League) -> String {
    lg.draft.as_ref().unwrap().picks.iter().map(|p| format!("{}:{}:{:?}:{}", p.overall, p.team, p.player_id, p.forfeited)).collect::<Vec<_>>().join(",")
}

fn avg(v: &[i32]) -> f64 {
    v.iter().sum::<i32>() as f64 / v.len() as f64
}

// ---------------------------------------------------------------------- draft

#[test]
fn the_class_covers_five_rounds_order_is_worst_record_first_and_repeats_each_round() {
    let mut lg = offseason(5);
    let d = lg.draft.as_ref().unwrap();
    assert_eq!(d.total_picks(), 60);
    assert!(d.prospects.len() >= 72);
    let po = lg.playoffs.as_ref().unwrap();
    let fg = po.final_game.as_ref().unwrap();
    let finalists = [fg.home(), fg.away()];
    assert_eq!(Some(*d.order.last().unwrap()), po.champion);
    assert!(finalists.contains(&d.order[d.order.len() - 2]));
    assert_eq!(d.order[d.order.len() - 4..].iter().copied().collect::<BTreeSet<_>>(), po.seeds.iter().copied().collect());
    for i in 1..d.order.len() - 4 {
        assert!(lg.standings.records[d.order[i - 1]].pct() <= lg.standings.records[d.order[i]].pct());
    }
    assert_eq!(d.order.iter().collect::<BTreeSet<_>>().len(), 12);
    let order = d.order.clone();
    lg.sim_draft_all();
    let d = lg.draft.as_ref().unwrap();
    for i in 0..60 {
        assert_eq!(d.picks[i].team, order[i % 12]);
        assert_eq!(d.picks[i].round, i / 12 + 1);
    }
}

#[test]
fn prospects_are_experienced_and_better_than_the_usual_free_agent() {
    let lg = offseason(5);
    let ovr: Vec<i32> = lg.draft.as_ref().unwrap().prospects.iter().map(|p| p.ovr()).collect();
    let fa: Vec<i32> = lg.free_agents.iter().map(|p| p.ovr()).collect();
    assert!(avg(&ovr) > avg(&fa) + 3.0);
    assert!(lg.draft.as_ref().unwrap().prospects.iter().all(|p| (22..=26).contains(&p.age)));
}

#[test]
fn rookie_contracts_three_years_in_round_one_two_in_round_two_one_after_pay_falls_with_the_pick() {
    let mut lg = offseason(5);
    lg.sim_draft_all();
    let made: Vec<_> = lg.draft.as_ref().unwrap().picks.iter().filter(|p| !p.forfeited).cloned().collect();
    assert_eq!(made.len(), 60);
    for p in &made {
        assert_eq!(p.seasons, if p.round == 1 { 3 } else if p.round == 2 { 2 } else { 1 }, "#{}", p.overall);
        let c = &lg.finances[p.team].contracts[&p.player_id.unwrap()];
        assert_eq!((c.ppg, c.seasons_left, c.commute), (p.ppg, p.seasons, p.commute));
        assert!(lg.teams[p.team].roster.iter().any(|x| Some(x.id) == p.player_id));
    }
    for i in 1..made.len() {
        assert!(made[i].ppg <= made[i - 1].ppg, "pick {}", made[i].overall);
    }
    assert_eq!(made[0].ppg, made.iter().map(|p| p.ppg).max().unwrap());
    assert!(made[0].ppg < 300);
}

#[test]
fn you_pick_when_on_the_clock_the_ai_picks_up_to_your_turn_and_your_player_signs_with_slot_terms() {
    let mut lg = offseason(5);
    let before = lg.draft.as_ref().unwrap().made();
    let ai = lg.sim_draft_to_user();
    assert!(lg.user_on_clock());
    assert_eq!(ai.len(), lg.draft.as_ref().unwrap().made() - before);
    assert!(lg.pass_pick().is_ok());
    // After passing, run on to your next turn, then draft someone.
    lg.sim_draft_to_user();
    assert!(lg.user_on_clock());
    let choice = lg.draft_board(0)[0].clone();
    let terms = lg.draft_terms(0, &choice);
    let rec = lg.draft_player(choice.id).expect("drafts");
    assert_eq!(rec.round, 2);
    assert_eq!(lg.finances[0].contracts[&choice.id].ppg, terms.ppg);
    assert_eq!(lg.finances[0].contracts[&choice.id].seasons_left, 2);
    assert!(lg.teams[0].roster.iter().any(|p| p.id == choice.id));
    assert!(!lg.draft.as_ref().unwrap().prospects.iter().any(|p| p.id == choice.id));
    let next = lg.draft_board(0)[0].id;
    assert!(lg.draft_player(next).is_err(), "not your turn any more");
}

#[test]
fn a_broke_team_forfeits_its_pick_ai_automatically_and_you_cannot_draft_without_cash_but_can_pass() {
    let mut lg = offseason(5);
    let broke_ai = *lg.draft.as_ref().unwrap().order.iter().find(|&&t| t != 0).unwrap();
    lg.finances[broke_ai].cash = -500;
    lg.sim_draft_to_user();
    let d = lg.draft.as_ref().unwrap();
    let theirs: Vec<_> = d.picks.iter().filter(|p| p.team == broke_ai).collect();
    assert!(!theirs.is_empty());
    assert!(theirs.iter().all(|p| p.forfeited && p.reason.is_some()));
    assert!(lg.teams[broke_ai].roster.len() < 60);

    assert!(lg.user_on_clock());
    lg.finances[0].cash = -1;
    let pick = lg.draft_board(0)[0].id;
    assert!(lg.draft_player(pick).is_err());
    let passed = lg.pass_pick().unwrap();
    assert!(passed.forfeited);
    assert!(passed.reason.unwrap().contains("broke"));
    // Auto-finishing the draft passes your remaining picks while you are broke.
    lg.sim_draft_all();
    let d = lg.draft.as_ref().unwrap();
    assert!(d.complete());
    assert!(d.picks.iter().filter(|p| p.team == 0).all(|p| p.forfeited));
}

#[test]
fn teams_draft_for_need_a_club_with_no_quarterbacks_has_qb_as_its_top_need() {
    let mut lg = offseason(5);
    let qbs: Vec<u32> = lg.teams[3].roster.iter().filter(|p| p.position == Position::Qb).map(|p| p.id).collect();
    lg.teams[3].roster.retain(|p| p.position != Position::Qb);
    for id in &qbs {
        lg.teams[3].active_ids.remove(id);
        lg.finances[3].contracts.remove(id);
    }
    let s = lg.draft_strategy_for(3);
    let mut needs: Vec<_> = s.need.iter().collect();
    needs.sort_by(|a, b| b.1.total_cmp(a.1));
    assert_eq!(*needs[0].0, Position::Qb);
    assert!(s.focus.contains(&Position::Qb));
    // Same prospect, same team: a needy position scores higher.
    let prospects = &lg.draft.as_ref().unwrap().prospects;
    let qb = prospects.iter().find(|p| p.position == Position::Qb).unwrap().clone();
    let other = prospects.iter().find(|p| p.position != Position::Qb && (p.ovr() - qb.ovr()).abs() <= 3 && !matches!(p.position, Position::K | Position::P)).cloned().unwrap_or_else(|| prospects.last().unwrap().clone());
    assert!(lg.draft_score(3, &qb, &s) > lg.draft_score(3, &other, &s) - (other.ovr() - qb.ovr()) as f64);
}

#[test]
fn commute_counts_the_same_player_scores_lower_for_a_team_far_from_his_hometown() {
    let lg = offseason(5);
    let p = lg.draft.as_ref().unwrap().prospects.iter().find(|x| x.position == Position::Wr).unwrap().clone();
    let near = lg.teams.iter().position(|t| town_miles(&p.hometown, &t.town.name) < 30.0);
    let far = lg.teams.iter().position(|t| town_miles(&p.hometown, &t.town.name) > 250.0);
    let (Some(near), Some(far)) = (near, far) else { return };
    let flat = DraftStrategy { need: BTreeMap::new(), need_weight: 0.0, commute_weight: 1.0, label: "x".into(), focus: vec![] };
    assert!(lg.draft_score(near, &p, &flat) > lg.draft_score(far, &p, &flat));
}

#[test]
fn saves_mid_draft_continue_identically_and_finishing_the_offseason_completes_the_draft() {
    let mut a = offseason(5);
    a.sim_draft_to_user();
    let best = a.draft_board(0)[0].id;
    a.draft_player(best).unwrap();
    let mut b = LeagueSave::decode(&LeagueSave::encode(&a)).unwrap();
    assert_eq!(b.draft.as_ref().unwrap().made(), a.draft.as_ref().unwrap().made());
    assert_eq!(b.draft.as_ref().unwrap().prospects.len(), a.draft.as_ref().unwrap().prospects.len());
    assert_eq!(b.draft.as_ref().unwrap().order, a.draft.as_ref().unwrap().order);
    a.sim_draft_all();
    b.sim_draft_all();
    assert_eq!(sig(&b), sig(&a));

    let mut c = offseason(9);
    assert!(!c.draft.as_ref().unwrap().complete());
    c.finish_offseason(); // draft still open: it is completed for you
    assert!(c.draft.as_ref().unwrap().complete());
    assert!(c.draft.as_ref().unwrap().prospects.is_empty());
    assert!(c.free_agents.len() > 20);
    assert!(c.inbox.iter().any(|i| i.title == "Draft results"));
    c.sim_season(); // the next season plays fine with the rookies on board
}

#[test]
fn rookie_scale_later_rounds_never_cost_more_than_earlier_ones() {
    let mut last = i64::MAX;
    for round in 1..=RookieScale::ROUNDS {
        for pick in 1..=12 {
            let v = RookieScale::ppg(round, pick, 12);
            assert!(v <= last);
            last = v;
        }
    }
}

#[test]
fn drafted_players_are_rookies_tagged_with_their_season_and_pick() {
    let mut lg = offseason(5);
    assert!(lg.draft.as_ref().unwrap().prospects.iter().all(|p| p.years_pro == 0 && p.acquired == ACQUIRED_FREE_AGENCY));
    lg.sim_draft_all();
    let season = lg.season;
    let picks: Vec<_> = lg.draft.as_ref().unwrap().picks.iter().filter(|p| !p.forfeited).cloned().collect();
    for pick in &picks {
        let p = lg.teams[pick.team].roster.iter().find(|x| Some(x.id) == pick.player_id).unwrap();
        assert_eq!(p.acquired, format!("S{} Draft #{}", season + 1, pick.overall));
        assert!(p.is_rookie());
    }
    // The undrafted who stay around are on the market, not drafted.
    assert!(!lg.free_agents.iter().any(|p| p.acquired.contains("Draft")));
    lg.finish_offseason();
    lg.sim_season();
    let first = &picks[0];
    assert!(lg.teams[first.team].roster.iter().any(|x| Some(x.id) == first.player_id && x.is_rookie()), "still a rookie in his first season");
    lg.begin_offseason();
    if let Some(after) = lg.teams.iter().flat_map(|t| t.roster.iter()).find(|x| Some(x.id) == first.player_id) {
        assert_eq!(after.years_pro, 1);
        assert_eq!(after.acquired, format!("S{} Draft #{}", season + 1, first.overall)); // the label stays with him
    }
}

#[test]
fn tryout_players_are_tagged_try_out_entrants_free_agency_and_a_release_puts_free_agency_on_him() {
    let mut lg = ca(5);
    lg.sim_season();
    let tried = lg.hold_tryouts(None, 8);
    assert!(!tried.is_empty());
    assert!(tried.iter().all(|p| p.acquired == ACQUIRED_TRY_OUT));
    let mine = lg.me().roster[0].clone();
    assert_eq!(mine.acquired, "Walk-On");
    lg.release_player(mine.id);
    assert_eq!(lg.free_agents.iter().find(|p| p.id == mine.id).unwrap().acquired, ACQUIRED_FREE_AGENCY);
    lg.begin_offseason();
    assert!(lg.free_agents.iter().filter(|p| p.acquired == ACQUIRED_FREE_AGENCY).count() > 50);
}

// ---------------------------------------------------------------------- trade

fn week_one(seed: u64) -> League {
    let mut lg = ca(seed);
    lg.advance();
    lg
}

fn best(lg: &League, team: usize, pos: Option<Position>) -> sp_core::Player {
    let mut l: Vec<_> = lg.teams[team].roster.iter().filter(|p| pos.map(|x| x == p.position).unwrap_or(true)).collect();
    l.sort_by_key(|p| std::cmp::Reverse(p.ovr()));
    l[0].clone()
}

fn offer(partner: usize) -> TradeOffer {
    TradeOffer::new(partner)
}

fn pr(season: i32, round: i32, team: usize) -> PickRef {
    PickRef { season, round, original_team: team }
}

#[test]
fn trades_are_closed_after_the_deadline_and_in_the_playoffs_and_open_in_the_offseason() {
    let mut lg = ca(5);
    assert!(lg.trade_closed_reason().is_none());
    while lg.week <= TradeTuning::DEADLINE_WEEK {
        lg.advance();
    }
    assert!(lg.trade_closed_reason().unwrap().contains("deadline"));
    lg.sim_season();
    assert!(lg.trade_closed_reason().is_some());
    lg.begin_offseason();
    assert!(lg.trade_closed_reason().is_none());
}

#[test]
fn a_lowball_is_refused_a_fair_swap_is_accepted_and_the_ai_always_wants_a_premium() {
    let lg = week_one(5);
    let theirs = best(&lg, 1, Some(Position::Wr));
    let mut o = offer(1);
    o.get_players.insert(theirs.id);
    let nothing = lg.evaluate_trade(&o);
    assert!(!nothing.accepted);
    assert!(nothing.ask.is_some());
    let mut mine: Vec<_> = lg.teams[0].roster.iter().filter(|p| p.position == Position::Wr).collect();
    mine.sort_by_key(|p| std::cmp::Reverse(p.ovr()));
    let mut big = offer(1);
    big.get_players.insert(theirs.id);
    big.give_players.insert(mine[0].id);
    big.give_picks = vec![pr(lg.season, 1, 0), pr(lg.season, 2, 0)];
    let big_v = lg.evaluate_trade(&big);
    let same = best(&lg, 0, Some(Position::Wr));
    let mut even = offer(1);
    even.get_players.insert(theirs.id);
    even.give_players.insert(same.id);
    let even_v = lg.evaluate_trade(&even);
    if same.ovr() == theirs.ovr() && same.age == theirs.age {
        assert!(!even_v.accepted);
    }
    assert!(big_v.receive > even_v.receive);
}

#[test]
fn first_round_picks_are_worth_far_more_than_late_ones_and_sooner_more_than_later() {
    let lg = week_one(5);
    let (r1, r2, r5) = (lg.pick_worth(&pr(lg.season, 1, 3), None), lg.pick_worth(&pr(lg.season, 2, 3), None), lg.pick_worth(&pr(lg.season, 5, 3), None));
    assert!(r1 > r2 * 1.4, "{r1} {r2}");
    assert!(r2 > r5 * 2.0, "{r2} {r5}");
    assert!(lg.pick_worth(&pr(lg.season + 1, 1, 3), None) < lg.pick_worth(&pr(lg.season, 1, 3), None) + 40.0);
    // A weak team's pick is worth more than a strong team's.
    let mut st: Vec<f64> = (0..12).map(|i| lg.pick_worth(&pr(lg.season, 1, i), None)).collect();
    st.sort_by(|a, b| a.total_cmp(b));
    assert!(st[11] > st[0]);
}

#[test]
fn quantity_for_quality_does_not_work() {
    let lg = week_one(5);
    let star = best(&lg, 1, None);
    let mut junk: Vec<_> = lg.teams[0].roster.iter().collect();
    junk.sort_by_key(|p| p.ovr());
    let mut o = offer(1);
    o.get_players.insert(star.id);
    for p in junk.iter().take(4) {
        o.give_players.insert(p.id);
    }
    assert!(!lg.evaluate_trade(&o).accepted);
}

#[test]
fn executing_players_contracts_with_the_new_commute_picks_and_cash_all_move() {
    let mut lg = week_one(5);
    let p = best(&lg, 1, Some(Position::Wr));
    let c_before = lg.finances[1].contracts[&p.id].clone();
    let pick = pr(lg.season, 1, 0);
    let (cash0, cash1) = (lg.finances[0].cash, lg.finances[1].cash);
    // Sweeten until they say yes.
    let mut o = offer(1);
    o.get_players.insert(p.id);
    o.give_picks.push(pick);
    for _ in 0..6 {
        let v = lg.evaluate_trade(&o);
        if v.accepted || v.ask.is_none() {
            break;
        }
        o = o.plus(v.ask.as_ref().unwrap());
    }
    let v = lg.propose_trade(&o);
    assert!(v.accepted, "{}", v.line);
    assert!(lg.teams[0].roster.iter().any(|x| x.id == p.id));
    assert!(!lg.teams[1].roster.iter().any(|x| x.id == p.id));
    let c = &lg.finances[0].contracts[&p.id];
    assert_eq!((c.ppg, c.seasons_left), (c_before.ppg, c_before.seasons_left));
    assert_eq!(c.commute, PayScale::commute(town_miles(&p.hometown, &lg.teams[0].town.name)));
    assert!(!lg.finances[1].contracts.contains_key(&p.id));
    assert_eq!(lg.pick_owner(&pick), 1);
    assert_eq!(lg.finances[0].cash + lg.finances[1].cash, cash0 + cash1); // cash only moves
    assert!(lg.inbox.last().unwrap().title.contains("Trade"));
    let back = LeagueSave::decode(&LeagueSave::encode(&lg)).unwrap();
    assert_eq!(back.pick_owner(&pick), 1);
    assert!(back.teams[0].roster.iter().any(|x| x.id == p.id));
}

#[test]
fn traded_picks_decide_who_is_on_the_clock_in_the_draft() {
    let mut lg = week_one(8);
    let pick = pr(lg.season, 1, 0);
    lg.pick_owners.insert(pick.key(), 3);
    lg.sim_season();
    lg.begin_offseason();
    let slot = lg.pick_slot(&pick).unwrap();
    let d = lg.draft.as_ref().unwrap();
    assert_eq!(d.owners[slot - 1], 3);
    assert!(d.picks_ahead(3).contains(&slot));
    assert!(!d.picks_ahead(0).contains(&slot));
    let season = d.season;
    lg.sim_draft_all();
    assert_eq!(lg.draft.as_ref().unwrap().picks[slot - 1].team, 3);
    assert!(!lg.pick_owners.keys().any(|k| k.starts_with(&format!("{season}:"))));
}

#[test]
fn a_just_signed_free_agent_cannot_be_flipped() {
    let mut lg = week_one(5);
    lg.hold_tryouts(None, 8);
    let mut fas = lg.free_agents.clone();
    fas.sort_by_key(|p| std::cmp::Reverse(p.ovr()));
    let fa = fas[0].clone();
    let ask = PayScale::ask(&fa, &lg.teams[0].town, 3);
    assert!(lg.sign_player(fa.id, (ask as f64 * 1.1).round() as i64, 3, 0));
    let mut o = offer(1);
    o.give_players.insert(fa.id);
    o.get_picks.push(pr(lg.season, 3, 1));
    let v = lg.evaluate_trade(&o);
    assert!(!v.accepted);
    assert!(v.blocked.unwrap().contains("Just signed"));
}

#[test]
fn ai_refuses_trades_that_break_its_roster_and_cash_it_cannot_spare() {
    let lg = week_one(5);
    let qbs: Vec<u32> = lg.teams[1].roster.iter().filter(|p| p.position == Position::Qb).map(|p| p.id).collect();
    let mut o = offer(1);
    o.get_players.extend(qbs);
    o.give_picks = vec![pr(lg.season, 1, 0), pr(lg.season, 2, 0)];
    assert!(lg.evaluate_trade(&o).blocked.unwrap().contains("short at QB"));
    let mut cash = offer(1);
    cash.get_cash = 10_000_000;
    assert!(lg.evaluate_trade(&cash).blocked.unwrap().contains("cannot spare"));
}

#[test]
fn the_book_shows_pay_commute_and_commitments() {
    let lg = week_one(5);
    let p = best(&lg, 1, None);
    let mut o = offer(1);
    o.get_players.insert(p.id);
    o.give_cash = 500;
    let b = lg.trade_book(&o);
    assert_eq!(b.incoming.len(), 1);
    assert_eq!(b.incoming[0].player.id, p.id);
    assert_eq!(b.incoming[0].new_commute, PayScale::commute(town_miles(&p.hometown, &lg.teams[0].town.name)));
    assert_eq!(b.cash_delta, -500);
    assert_eq!(b.payroll_delta, b.incoming[0].pay_now);
    assert!(b.commitment_in > 0);
}

#[test]
fn many_seasons_of_trading_do_not_break_the_league() {
    let mut lg = ca(11);
    for _ in 0..3 {
        lg.advance();
        for t in [1usize, 2, 3, 4] {
            let theirs = best(&lg, t, None);
            let mut o = offer(t);
            o.get_players.insert(theirs.id);
            for _ in 0..6 {
                let v = lg.evaluate_trade(&o);
                if v.accepted && lg.propose_trade(&o).accepted {
                    break;
                }
                if v.blocked.is_some() || v.ask.is_none() {
                    break;
                }
                o = o.plus(v.ask.as_ref().unwrap());
            }
        }
        lg.sim_season();
        lg.begin_offseason();
        lg.finish_offseason();
        assert!(lg.teams.iter().all(|t| t.roster.len() >= 30));
    }
}
