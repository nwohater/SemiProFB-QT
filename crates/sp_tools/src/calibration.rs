use sp_core::team_generator::TeamGenerator;
use sp_core::{PlayerGenerator, Region, Rng, Team};
use sp_sim::{GameConfig, GameSim, Side};
use std::collections::BTreeMap;

pub struct CalibrationReport {
    pub games: usize,
    /// Per-team-per-game averages.
    pub avg: BTreeMap<&'static str, f64>,
}

impl std::fmt::Display for CalibrationReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Games: {}", self.games)?;
        for (k, v) in &self.avg {
            writeln!(f, "  {k:<22} {v:.2}")?;
        }
        Ok(())
    }
}

/// Targets (15-min quarters, semi-pro skew) from features/game-sim-engine.md sec 8 / D13. Same bands as the Dart harness.
pub const CALIBRATION_TARGETS: [(&str, f64, f64); 16] = [
    ("points", 18.0, 26.0),
    ("total_yards", 300.0, 400.0),
    ("pass_att", 26.0, 36.0),
    ("comp_pct", 54.0, 64.0),
    ("yds_per_att", 5.8, 7.4),
    ("rush_att", 24.0, 34.0),
    ("yds_per_carry", 3.6, 4.8),
    ("sacks_taken", 1.8, 3.6),
    ("ints_thrown", 0.7, 1.5),
    ("fumbles_lost", 0.4, 1.0),
    ("penalties", 5.0, 9.0),
    ("punts", 3.5, 6.0),
    ("plays_per_team", 60.0, 80.0),
    ("fg_pct", 65.0, 85.0),
    ("third_down_pct", 32.0, 44.0),
    ("injuries_per_game", 0.5, 2.5),
];

pub fn run_calibration(games: usize, seed: u64) -> CalibrationReport {
    let mut rng = Rng::new(seed);
    let mut tg = TeamGenerator::new(PlayerGenerator::new(rng.fork(1)));
    let teams: Vec<Team> = Region::california().starting.iter().map(|t| tg.generate(t, &[])).collect();
    let mut sums: BTreeMap<&'static str, f64> = BTreeMap::new();
    let mut add = |k: &'static str, v: f64| *sums.entry(k).or_insert(0.0) += v;
    let mut team_games = 0usize;
    let (mut ties, mut ots) = (0, 0);

    for g in 0..games {
        let a = rng.next_int(teams.len());
        let mut b;
        loop {
            b = rng.next_int(teams.len());
            if b != a {
                break;
            }
        }
        let r = GameSim::new(rng.fork(g as u64 + 100), &teams[a], &teams[b], GameConfig::default()).simulate();
        if r.tie {
            ties += 1;
        }
        if r.overtime {
            ots += 1;
        }
        add("injuries_per_game", r.injuries.len() as f64);
        for side in Side::ALL {
            let ts = &r.team_stats[side];
            team_games += 1;
            add("points", r.score(side) as f64);
            add("total_yards", ts.total_yards() as f64);
            add("pass_att", ts.pass_att as f64);
            add("pass_cmp", ts.pass_cmp as f64);
            add("pass_yds", ts.pass_yards as f64);
            add("rush_att", ts.rush_plays as f64);
            add("rush_yds", ts.rush_yards as f64);
            add("sacks_taken", ts.sacks_taken as f64);
            add("ints_thrown", ts.interceptions_thrown as f64);
            add("fumbles_lost", ts.fumbles_lost as f64);
            add("penalties", ts.penalties as f64);
            add("punts", ts.punts as f64);
            add("plays_per_team", (ts.plays + ts.sacks_taken) as f64);
            add("fg_att", ts.fg_att as f64);
            add("fg_made", ts.fg_made as f64);
            add("third_att", ts.third_att as f64);
            add("third_conv", ts.third_conv as f64);
            add("drives", ts.drives as f64);
        }
    }
    let tgs = team_games as f64;
    let s = |k: &str| sums.get(k).copied().unwrap_or(0.0);
    let per = |k: &str| s(k) / tgs;
    let mut avg = BTreeMap::new();
    avg.insert("points", per("points"));
    avg.insert("total_yards", per("total_yards"));
    avg.insert("pass_att", per("pass_att"));
    avg.insert("comp_pct", 100.0 * s("pass_cmp") / s("pass_att"));
    avg.insert("yds_per_att", s("pass_yds") / s("pass_att"));
    avg.insert("rush_att", per("rush_att"));
    avg.insert("yds_per_carry", s("rush_yds") / s("rush_att"));
    avg.insert("sacks_taken", per("sacks_taken"));
    avg.insert("ints_thrown", per("ints_thrown"));
    avg.insert("fumbles_lost", per("fumbles_lost"));
    avg.insert("penalties", per("penalties"));
    avg.insert("punts", per("punts"));
    avg.insert("plays_per_team", per("plays_per_team"));
    avg.insert("fg_pct", if s("fg_att") == 0.0 { 0.0 } else { 100.0 * s("fg_made") / s("fg_att") });
    avg.insert("third_down_pct", if s("third_att") == 0.0 { 0.0 } else { 100.0 * s("third_conv") / s("third_att") });
    avg.insert("drives_per_team", per("drives"));
    avg.insert("injuries_per_game", s("injuries_per_game") / games as f64);
    avg.insert("ties_pct", 100.0 * ties as f64 / games as f64);
    avg.insert("overtime_pct", 100.0 * ots as f64 / games as f64);
    CalibrationReport { games, avg }
}
