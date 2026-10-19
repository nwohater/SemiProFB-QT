mod common;
use common::*;
use sp_core::generator::GenOptions;
use sp_core::strategy::{OffenseScheme, OffenseStrategy, Strategy};
use sp_core::team::ROSTER_MIX;
use sp_core::team_generator::TeamGenerator;
use sp_core::{PlayerGenerator, Rng, Team, Tier, Town, Weather};
use sp_sim::{GameConfig, GameSim, Side};

#[test]
fn same_seed_gives_identical_game() {
    let (a, b) = (play(5), play(5));
    assert_eq!((a.home_score, a.away_score, a.team_stats.home.total_yards()), (b.home_score, b.away_score, b.team_stats.home.total_yards()));
    assert_eq!(a.scoring_plays, b.scoring_plays);
    assert_eq!(a.player_stats.home, b.player_stats.home);
}

#[test]
fn different_seeds_usually_differ() {
    let scores: std::collections::HashSet<String> = (0..10).map(|i| format!("{}-{}", play(i).home_score, play(i).away_score)).collect();
    assert!(scores.len() > 3, "{scores:?}");
}

#[test]
fn scores_are_nonnegative_and_cover_kicks() {
    for seed in 1..=40 {
        let r = play(seed);
        assert!(r.home_score >= 0 && r.away_score >= 0);
        for side in Side::ALL {
            let ts = &r.team_stats[side];
            assert!(r.score(side) >= 3 * ts.fg_made + ts.xp_made);
        }
    }
}

#[test]
fn team_stats_equal_sum_of_player_stats() {
    for seed in 1..=25 {
        let r = play(seed);
        for side in Side::ALL {
            let ts = &r.team_stats[side];
            let ps = &r.player_stats[side];
            assert_eq!(sum_stat(ps, "pass_att"), ts.pass_att as f64, "seed {seed} pass_att");
            assert_eq!(sum_stat(ps, "pass_cmp"), ts.pass_cmp as f64);
            assert_eq!(sum_stat(ps, "pass_yds"), ts.pass_yards as f64);
            assert_eq!(sum_stat(ps, "rec_yds"), ts.pass_yards as f64);
            assert_eq!(sum_stat(ps, "rush_yds"), ts.rush_yards as f64);
            assert_eq!(sum_stat(ps, "sacks_taken"), ts.sacks_taken as f64);
            assert_eq!(sum_stat(ps, "fgm"), ts.fg_made as f64);
        }
    }
}

#[test]
fn sacks_made_equal_sacks_taken() {
    for seed in 1..=25 {
        let r = play(seed);
        for side in Side::ALL {
            assert_eq!(sum_stat(&r.player_stats[side.other()], "sacks"), r.team_stats[side].sacks_taken as f64, "seed {seed}");
            assert_eq!(sum_stat(&r.player_stats[side], "sacks_allowed"), r.team_stats[side].sacks_taken as f64);
        }
    }
}

#[test]
fn playoff_games_never_tie() {
    for seed in 1..=150 {
        let r = play_towns(seed, 0, 1, GameConfig { playoff: true, ..Default::default() });
        assert!(!r.tie, "seed {seed}");
    }
}

#[test]
fn events_are_kept_when_asked_and_injuries_are_bounded() {
    let r = play_towns(3, 0, 1, GameConfig { keep_events: true, ..Default::default() });
    assert!(r.injuries.len() < 15);
    assert!(!r.events.is_empty());
    assert!(play(3).events.is_empty());
}

fn build_team(rng: &mut Rng, tier: Tier, town: &Town) -> Team {
    let mut pg = PlayerGenerator::new(rng.fork(9));
    pg.next_id = 1000;
    let mut roster = Vec::new();
    for &(pos, n) in &ROSTER_MIX {
        for _ in 0..n {
            roster.push(pg.generate(pos, GenOptions::default().tier(tier).hometown(&town.name).age(26)));
        }
    }
    let mut t = Team::new(town.clone(), roster);
    t.active_ids = t.roster.iter().map(|p| p.id).take(46).collect();
    t
}

#[test]
fn stronger_team_wins_more_often() {
    let mut rng = Rng::new(77);
    let strong = build_team(&mut rng, Tier::ExCollege, &towns()[0]);
    let weak = build_team(&mut rng, Tier::WalkOn, &towns()[1]);
    let n = 80;
    let wins = (0..n).filter(|&i| {
        let r = GameSim::new(Rng::new(1000 + i), &strong, &weak, GameConfig::default()).simulate();
        r.home_score > r.away_score
    });
    let wins = wins.count();
    assert!(wins as f64 / n as f64 > 0.7, "{wins}/{n}");
}

#[test]
fn rain_increases_drops_on_average() {
    let drops = |w: Weather| -> f64 {
        (1..=60)
            .map(|seed| {
                let r = play_towns(seed, 0, 1, GameConfig { weather: w, ..Default::default() });
                Side::ALL.iter().map(|&s| sum_stat(&r.player_stats[s], "drops")).sum::<f64>()
            })
            .sum()
    };
    assert!(drops(Weather::Rain) > drops(Weather::Clear));
}

#[test]
fn run_heavy_plan_runs_more_than_it_passes() {
    let mut rng = Rng::new(5);
    let mut tg = TeamGenerator::new(PlayerGenerator::new(rng.fork(1)));
    let mut a = tg.generate(&towns()[2], &[]);
    let b = tg.generate(&towns()[3], &[]);
    a.strategy = Strategy { offense: OffenseStrategy { run_pass_bias: 0.15, scheme: OffenseScheme::GroundAndPound, ..Default::default() }, ..Default::default() };
    let (mut runs, mut passes) = (0, 0);
    for i in 0..40 {
        let r = GameSim::new(Rng::new(200 + i), &a, &b, GameConfig::default()).simulate();
        runs += r.team_stats.home.rush_plays;
        passes += r.team_stats.home.pass_att;
    }
    assert!(runs > passes, "{runs} runs vs {passes} passes");
}

#[test]
fn punt_return_touchdowns_are_believable_credited_and_add_up() {
    let (mut returns, mut tds, mut td_plays) = (0, 0, 0);
    for seed in 1..=400 {
        let r = play_towns(seed, 2, 5, GameConfig::default());
        let (mut hp, mut ap) = (0, 0);
        for p in &r.scoring_plays {
            match p.side {
                Side::Home => hp += p.points,
                Side::Away => ap += p.points,
            }
            if p.text.contains("punt return") {
                td_plays += 1;
            }
        }
        assert_eq!((hp, ap), (r.home_score, r.away_score), "seed {seed}");
        for side in Side::ALL {
            for st in r.player_stats[side].values() {
                returns += st.get("pr").round() as i32;
                tds += st.get("pr_td").round() as i32;
                if st.get("pr_td") > 0.0 {
                    assert!(st.get("pr_long") > 0.0);
                }
            }
        }
    }
    assert_eq!(tds, td_plays, "every credited TD is a scoring play");
    assert!(tds > 0);
    let rate = tds as f64 / returns as f64;
    assert!((0.001..=0.02).contains(&rate), "{tds} of {returns} returns");
}
