mod common;
use common::*;
use sp_core::returners::ReturnRole;
use sp_core::team_generator::TeamGenerator;
use sp_core::{PlayerGenerator, Position, Rng};
use sp_sim::team_ctx::{DefPackage, OffPackage};
use sp_sim::{GameConfig, GameSim, Side, TeamCtx};
use std::collections::HashSet;

#[test]
fn depth_order_lists_your_players_first_then_the_rest_by_rating() {
    let mut t = team(1, 3);
    let mut rbs: Vec<(u32, i32)> = t.active().filter(|p| p.position == Position::Rb).map(|p| (p.id, p.ovr())).collect();
    rbs.sort_by_key(|&(_, o)| std::cmp::Reverse(o));
    assert!(rbs.len() >= 2);
    assert_eq!(t.depth_order(Position::Rb)[0].id, rbs[0].0); // automatic
    let worst = rbs.last().unwrap().0;
    t.set_depth(Position::Rb, vec![worst]);
    let order = t.depth_order(Position::Rb);
    assert_eq!(order[0].id, worst);
    assert_eq!(order.len(), rbs.len());
    t.auto_sort(Position::Rb);
    assert_eq!(t.depth_order(Position::Rb)[0].id, rbs[0].0);
}

#[test]
fn the_depth_chart_decides_who_plays() {
    let mut rng = Rng::new(5);
    let mut tg = TeamGenerator::new(PlayerGenerator::new(rng.fork(1)));
    let mut a = tg.generate(&towns()[0], &[]);
    let b = tg.generate(&towns()[1], &[]);
    let mut rbs: Vec<(u32, i32)> = a.active().filter(|p| p.position == Position::Rb).map(|p| (p.id, p.ovr())).collect();
    rbs.sort_by_key(|&(_, o)| std::cmp::Reverse(o));
    let (best, worst) = (rbs[0].0, rbs.last().unwrap().0);
    let carries = |team: &sp_core::Team, id: u32| -> f64 { (0..30).map(|i| stat(&GameSim::new(Rng::new(100 + i), team, &b, GameConfig::default()).simulate(), Side::Home, id, "rush_att")).sum() };
    let auto = carries(&a, best);
    a.set_depth(Position::Rb, vec![worst, best]);
    let worst_first = carries(&a, worst);
    let best_now = carries(&a, best);
    assert!(worst_first > best_now * 1.5, "{worst_first} vs {best_now}");
    assert!(auto > worst_first * 0.5, "{auto} vs {worst_first}");
}

#[test]
fn injured_players_in_the_chart_are_skipped_and_out_of_position_placement_works() {
    let mut rng = Rng::new(7);
    let mut tg = TeamGenerator::new(PlayerGenerator::new(rng.fork(1)));
    let mut a = tg.generate(&towns()[2], &[]);
    let b = tg.generate(&towns()[3], &[]);
    let mut qbs: Vec<(u32, i32)> = a.active().filter(|p| p.position == Position::Qb).map(|p| (p.id, p.ovr())).collect();
    qbs.sort_by_key(|&(_, o)| std::cmp::Reverse(o));
    let (q1, q2) = (qbs[0].0, qbs.last().unwrap().0);
    a.set_depth(Position::Qb, vec![q1, q2]);
    // QB1 is hurt: QB2 plays.
    let r = GameSim::with_unavailable(Rng::new(3), &a, &b, GameConfig::default(), &HashSet::from([q1]), &HashSet::new()).simulate();
    assert_eq!(stat(&r, Side::Home, q1, "pass_att"), 0.0);
    assert!(stat(&r, Side::Home, q2, "pass_att") > 10.0);
    // A safety listed at CB is used at CB.
    let s = a.active().find(|p| p.position == Position::S).unwrap().id;
    let mut ids = vec![s];
    ids.extend(a.active().filter(|p| p.position == Position::Cb).map(|p| p.id));
    a.set_depth(Position::Cb, ids);
    let ctx = TeamCtx::new(&a, Side::Home, &HashSet::new());
    let unit = ctx.defense(DefPackage::FourThree);
    assert!(unit.db.iter().any(|p| p.id == s));
}

#[test]
fn tired_starters_rotate_out_but_the_qb_never_does() {
    let a_team = {
        let mut t = team(9, 4);
        let dls: Vec<u32> = t.active().filter(|p| p.position == Position::Dt).map(|p| p.id).collect();
        assert!(dls.len() >= 3);
        t.set_depth(Position::Dt, dls);
        t
    };
    let dls: Vec<u32> = a_team.depth[&Position::Dt].clone();
    let mut ctx = TeamCtx::new(&a_team, Side::Home, &HashSet::new());
    let on_field = |ctx: &TeamCtx| -> HashSet<u32> { ctx.defense(DefPackage::FourThree).dl.iter().filter(|p| p.position == Position::Dt).map(|p| p.id).collect() };
    assert_eq!(on_field(&ctx), HashSet::from([dls[0], dls[1]]));
    ctx.fatigue.insert(dls[0], 0.9);
    let rotated = on_field(&ctx);
    assert!(!rotated.contains(&dls[0]));
    assert!(rotated.contains(&dls[2]));

    // QB1 stays in even when exhausted.
    let mut t2 = team(9, 4);
    let qbs: Vec<u32> = t2.active().filter(|p| p.position == Position::Qb).map(|p| p.id).collect();
    t2.set_depth(Position::Qb, qbs.clone());
    let mut ctx2 = TeamCtx::new(&t2, Side::Home, &HashSet::new());
    ctx2.fatigue.insert(qbs[0], 1.0);
    assert_eq!(ctx2.offense(OffPackage::Base).qb.id, qbs[0]);
}

#[test]
fn an_empty_or_fully_automatic_chart_plays_exactly_like_before() {
    let mut rng = Rng::new(11);
    let mut tg = TeamGenerator::new(PlayerGenerator::new(rng.fork(1)));
    let mut a = tg.generate(&towns()[5], &[]);
    let b = tg.generate(&towns()[6], &[]);
    let base = GameSim::new(Rng::new(8), &a, &b, GameConfig::default()).simulate();
    a.set_depth(Position::Wr, vec![]);
    a.auto_sort_all();
    let again = GameSim::new(Rng::new(8), &a, &b, GameConfig::default()).simulate();
    assert_eq!((again.home_score, again.away_score, again.team_stats.home.total_yards()), (base.home_score, base.away_score, base.team_stats.home.total_yards()));
}

#[test]
fn the_sim_uses_the_first_healthy_listed_returner_and_roles_are_separate() {
    let mut t = team(4, 0);
    let auto = TeamCtx::new(&t, Side::Home, &HashSet::new()).returner_for(ReturnRole::Kick).id;
    assert_eq!(auto, t.return_order(ReturnRole::Kick)[0].id);

    let qb = t.active().find(|p| p.position == Position::Qb).unwrap().id;
    let wr = t.active().find(|p| p.position == Position::Wr && p.id != auto).unwrap().id;
    t.set_return_depth(ReturnRole::Punt, vec![qb, wr]);
    let pick = |t: &sp_core::Team, out: &[u32], role| TeamCtx::new(t, Side::Home, &out.iter().copied().collect()).returner_for(role).id;
    assert_eq!(pick(&t, &[], ReturnRole::Punt), qb);
    assert_eq!(pick(&t, &[], ReturnRole::Kick), auto);
    // The QB is hurt: the next man listed takes it.
    assert_eq!(pick(&t, &[qb], ReturnRole::Punt), wr);
    // Everyone listed is hurt: back to automatic.
    let fallback = pick(&t, &[qb, wr], ReturnRole::Punt);
    assert!(fallback != qb && fallback != wr);
}
