use sp_core::team::{ACTIVE_ROSTER_SIZE, min_active_coverage};
use sp_core::Position;
use sp_league::{League, LeaguePhase, NewLeague};

fn league(seed: u64) -> League {
    League::new(seed, NewLeague { small_tables: true, ..Default::default() })
}

fn check_rosters(lg: &League) {
    for (i, t) in lg.teams.iter().enumerate() {
        // (A user who never re-signs anyone can run the free-agent pool dry at a position; AI teams must always be covered.)
        for &pos in &Position::ALL {
            let n = t.roster.iter().filter(|p| p.position == pos).count();
            if Some(i) != lg.user_team {
                assert!(n >= min_active_coverage(pos), "season {} team {i} short at {}: {n}", lg.season, pos.label());
            }
        }
        if Some(i) != lg.user_team {
            assert!(t.roster.len() >= ACTIVE_ROSTER_SIZE.min(40), "team {i} roster {}", t.roster.len());
        }
        for id in &t.active_ids {
            assert!(t.roster.iter().any(|p| p.id == *id), "active id not on roster");
        }
        // every rostered player has exactly one contract
        for p in &t.roster {
            assert!(lg.finances[i].contracts.contains_key(&p.id), "season {} team {i}: {} has no contract", lg.season, p.name);
        }
        for cid in lg.finances[i].contracts.keys() {
            assert!(t.roster.iter().any(|p| p.id == *cid), "contract for a player not on the roster");
        }
    }
}

#[test]
fn a_franchise_runs_eight_seasons() {
    let mut lg = league(17);
    for y in 1..=8 {
        assert_eq!(lg.season, y);
        lg.sim_season();
        if lg.game_over() {
            return; // the owner went broke: also a legal ending
        }
        assert!(lg.complete());
        let summary = lg.begin_offseason();
        assert_eq!(summary.season, y);
        assert_eq!(lg.phase, LeaguePhase::Offseason);
        assert!(lg.draft.is_some());
        assert!(lg.free_agents.len() < 400, "{}", lg.free_agents.len());
        // The user re-signs nobody in this test, so every expiring player walks.
        lg.sim_draft_all();
        let res = lg.finish_offseason();
        assert_eq!(res.new_season, y + 1);
        assert_eq!(lg.phase, LeaguePhase::RegularSeason);
        assert_eq!(lg.week, 1);
        check_rosters(&lg);
    }
    assert_eq!(lg.history.len(), 8);
    assert!(lg.hall_of_fame.len() > 0 || lg.retired_players.len() > 10);
}

#[test]
fn the_league_stays_healthy_over_time() {
    let mut lg = League::new(5, NewLeague { user_team: None, small_tables: true, ..Default::default() });
    let mut ages = Vec::new();
    for _ in 0..8 {
        lg.sim_season();
        let all: Vec<f64> = lg.teams.iter().flat_map(|t| t.roster.iter().map(|p| p.age as f64)).collect();
        ages.push(all.iter().sum::<f64>() / all.len() as f64);
        lg.begin_offseason();
        lg.finish_offseason();
    }
    for a in &ages {
        assert!((22.0..34.0).contains(a), "{ages:?}");
    }
    // no one is hoarding or drowning: cash stays in a sane band
    for f in &lg.finances {
        assert!(f.cash > -5_000_000 && f.cash < 5_000_000, "{}", f.cash);
    }
}

#[test]
fn the_draft_fills_rosters_with_rookies_and_assigns_contracts() {
    let mut lg = league(3);
    lg.sim_season();
    lg.begin_offseason();
    let d = lg.draft.as_ref().unwrap();
    assert_eq!(d.total_picks(), 60);
    assert_eq!(d.order.len(), 12);
    let champ = lg.champion().unwrap();
    assert_eq!(*d.order.last().unwrap(), champ, "the champion picks last");
    let before = lg.teams[0].roster.len();
    let recs = lg.sim_draft_all();
    assert_eq!(recs.len(), 60);
    assert!(lg.draft.as_ref().unwrap().complete());
    let took = recs.iter().filter(|r| !r.forfeited).count();
    assert!(took >= 50, "{took}");
    assert!(lg.teams[0].roster.len() >= before);
    for r in recs.iter().filter(|r| !r.forfeited) {
        let id = r.player_id.unwrap();
        assert!(lg.finances[r.team].contracts.contains_key(&id));
        assert!(lg.teams[r.team].roster.iter().any(|p| p.id == id));
    }
}

#[test]
fn the_user_can_draft_re_sign_and_release() {
    let mut lg = league(8);
    lg.sim_season();
    let summary = lg.begin_offseason();
    // contracts: re-sign one expiring player if any
    if let Some(&id) = summary.expiring.first() {
        let resp = lg.negotiate(id, 10_000, 2, 0, 1);
        assert_eq!(resp.outcome, sp_economy::OfferOutcome::Accept);
        assert!(lg.sign_player(id, 10_000, 2, 0));
        assert!(!lg.pending_expired.contains(&id));
    }
    // draft: sim to the user and take the best on the board
    lg.sim_draft_to_user();
    if lg.user_on_clock() {
        let board = lg.draft_board(lg.user());
        let best = board[0].id;
        let rec = lg.draft_player(best).expect("user can draft");
        assert!(!rec.forfeited && rec.team == lg.user());
    }
    // release someone
    let victim = lg.me().roster.iter().find(|p| !lg.pending_expired.contains(&p.id)).unwrap().id;
    let cost = lg.release_player(victim);
    assert!(cost >= 0);
    assert!(lg.free_agents.iter().any(|p| p.id == victim));
    lg.finish_offseason();
    assert_eq!(lg.season, 2);
}

#[test]
fn trading_moves_players_picks_and_cash() {
    use sp_league::trade::TradeOffer;
    let mut lg = league(12);
    lg.sim_season();
    lg.begin_offseason();
    assert!(lg.trade_closed_reason().is_none());
    let me = lg.user();
    let partner = (0..12).find(|&t| t != me).unwrap();
    // A pure gift of a late pick should always be accepted.
    let pick = lg.picks_of(me).into_iter().last().unwrap();
    let mut offer = TradeOffer::new(partner);
    offer.give_picks.push(pick);
    let v = lg.evaluate_trade(&offer);
    assert!(v.accepted, "{} / {:?}", v.line, v.blocked);
    let cash_before = lg.finances[partner].cash;
    let accepted = lg.propose_trade(&offer).accepted;
    assert!(accepted);
    assert_eq!(lg.pick_owner(&pick), partner);
    assert_eq!(lg.finances[partner].cash, cash_before);
    // Asking for their best player for nothing is refused with a suggestion.
    let star = lg.teams[partner].roster.iter().max_by_key(|p| p.ovr()).unwrap().id;
    let mut greedy = TradeOffer::new(partner);
    greedy.get_players.insert(star);
    let v = lg.evaluate_trade(&greedy);
    assert!(!v.accepted);
    let book = lg.trade_book(&greedy);
    assert_eq!(book.incoming.len(), 1);
}

#[test]
fn trades_close_at_the_deadline() {
    let mut lg = league(6);
    for _ in 0..9 {
        lg.advance();
    }
    assert!(lg.trade_closed_reason().is_none()); // week 10 is still open
    lg.advance();
    assert!(lg.trade_closed_reason().is_some()); // the deadline passed after week 10
}
