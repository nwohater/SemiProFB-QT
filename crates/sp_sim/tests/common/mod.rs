#![allow(dead_code)]
use sp_core::team_generator::TeamGenerator;
use sp_core::{PlayerGenerator, Region, Rng, Team};
use sp_sim::{GameConfig, GameResult, GameSim, PlayerStats, Side};
use std::collections::BTreeMap;

pub fn towns() -> &'static [sp_core::Town] {
    &Region::california().starting
}

pub fn team(seed: u64, town: usize) -> Team {
    TeamGenerator::new(PlayerGenerator::new(Rng::new(seed))).generate(&towns()[town], &[])
}

/// Two fresh teams from one seed, the way the Dart tests do it.
pub fn play_towns(seed: u64, home: usize, away: usize, config: GameConfig) -> GameResult {
    let mut rng = Rng::new(seed);
    let mut tg = TeamGenerator::new(PlayerGenerator::new(rng.fork(1)));
    let h = tg.generate(&towns()[home], &[]);
    let a = tg.generate(&towns()[away], &[]);
    GameSim::new(rng.fork(2), &h, &a, config).simulate()
}

pub fn play(seed: u64) -> GameResult {
    play_towns(seed, 0, 1, GameConfig::default())
}

pub fn sum_stat(m: &BTreeMap<u32, PlayerStats>, k: &str) -> f64 {
    m.values().map(|p| p.get(k)).sum()
}

pub fn stat(r: &GameResult, side: Side, id: u32, key: &str) -> f64 {
    r.player_stats[side].get(&id).map(|s| s.get(key)).unwrap_or(0.0)
}
