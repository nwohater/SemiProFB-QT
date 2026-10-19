use sp_core::team_generator::TeamGenerator;
use sp_core::{PlayerGenerator, Region, Rng};
use sp_sim::{GameConfig, GameSim, Side};

fn main() {
    let seed: u64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(5);
    let mut rng = Rng::new(seed);
    let mut tg = TeamGenerator::new(PlayerGenerator::new(rng.fork(1)));
    let ca = Region::california();
    let h = tg.generate(&ca.starting[0], &[]);
    let a = tg.generate(&ca.starting[1], &[]);
    let r = GameSim::new(rng.fork(2), &h, &a, GameConfig::default()).simulate();
    println!("{} {} - {} {}{}", h, r.home_score, r.away_score, a, if r.overtime { " (OT)" } else { "" });
    for s in Side::ALL {
        let t = &r.team_stats[s];
        println!(
            "{:?}: {} plays, {} pass yds, {} rush yds, {} TO, {} sacks taken, {}/{} 3rd, {} pen/{} yds, poss {}:{:02}",
            s, t.plays, t.pass_yards, t.rush_yards, t.turnovers, t.sacks_taken, t.third_conv, t.third_att, t.penalties, t.penalty_yards,
            t.seconds_of_possession / 60, t.seconds_of_possession % 60
        );
    }
    for p in r.scoring_plays.iter().take(8) {
        println!("  Q{} {:?} +{} {}", p.quarter, p.side, p.points, p.text);
    }
}
