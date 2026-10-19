use sp_core::Rng;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ScheduledGame {
    /// 1-based.
    pub week: i32,
    pub home: usize,
    pub away: usize,
}

type Pair = (usize, usize);

/// 12 teams, 3 divisions of 4 (indices 0-3, 4-7, 8-11): 14 games, one per team per week.
/// Division rivals twice (home + away), everyone else once; 7 home / 7 away per team.
pub struct ScheduleGenerator;

impl ScheduleGenerator {
    pub const TEAMS: usize = 12;
    pub const WEEKS: usize = 14;

    pub fn division_of(team: usize) -> usize {
        team / 4
    }

    pub fn generate(rng: &mut Rng) -> Vec<ScheduledGame> {
        for attempt in 0..50 {
            let mut r = rng.fork(attempt);
            let rounds = Self::rounds();
            let games = Self::orient(&rounds, &mut r);
            let mut order: Vec<usize> = (0..Self::WEEKS).collect();
            for i in (1..order.len()).rev() {
                let j = r.next_int(i + 1);
                order.swap(i, j);
            }
            // Repair by swapping rounds until no team has 3+ consecutive road games.
            let mut bad = Self::violations(&order.iter().map(|&i| games[i].clone()).collect::<Vec<_>>());
            let mut step = 0;
            while step < 3000 && bad > 0 {
                step += 1;
                let x = r.next_int(Self::WEEKS);
                let y = r.next_int(Self::WEEKS);
                if x == y {
                    continue;
                }
                let mut cand = order.clone();
                cand.swap(x, y);
                let v = Self::violations(&cand.iter().map(|&i| games[i].clone()).collect::<Vec<_>>());
                if v <= bad {
                    order = cand;
                    bad = v;
                }
            }
            if bad == 0 {
                let mut out = Vec::new();
                for (w, &i) in order.iter().enumerate() {
                    for g in &games[i] {
                        out.push(ScheduledGame { week: w as i32 + 1, home: g.0, away: g.1 });
                    }
                }
                return out;
            }
        }
        panic!("could not build a valid schedule");
    }

    /// 14 perfect matchings: 11 from the circle-method round robin + 3 repeat-division rounds.
    fn rounds() -> Vec<Vec<Pair>> {
        let n = Self::TEAMS;
        let mut rounds = Vec::new();
        let mut ring: Vec<usize> = (0..n).collect();
        for _ in 0..n - 1 {
            rounds.push((0..n / 2).map(|i| (ring[i], ring[n - 1 - i])).collect());
            let last = ring.pop().unwrap();
            ring.insert(1, last);
        }
        // Each division K4 splits into 3 perfect matchings; combine across divisions.
        for m in 0..3 {
            let mut round = Vec::new();
            for d in 0..3 {
                let b = d * 4;
                match m {
                    0 => round.extend([(b, b + 1), (b + 2, b + 3)]),
                    1 => round.extend([(b, b + 2), (b + 1, b + 3)]),
                    _ => round.extend([(b, b + 3), (b + 1, b + 2)]),
                }
            }
            rounds.push(round);
        }
        rounds
    }

    /// Decide home/away: second division meeting reverses the first; cross-division games are oriented
    /// along an Euler circuit so every team gets exactly 4 home and 4 away.
    fn orient(rounds: &[Vec<Pair>], rng: &mut Rng) -> Vec<Vec<Pair>> {
        let n = Self::TEAMS;
        let mut out: Vec<Vec<Pair>> = rounds.iter().map(|_| Vec::new()).collect();
        let mut first_home: HashMap<(usize, usize), usize> = HashMap::new();
        let key = |a: usize, b: usize| if a < b { (a, b) } else { (b, a) };
        // Cross-division edges -> Euler orientation.
        let mut adj: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n]; // node -> (neighbor, edgeId)
        let mut edges: Vec<Pair> = Vec::new();
        let mut edge_round: Vec<usize> = Vec::new();
        for (ri, round) in rounds.iter().enumerate().take(11) {
            for &g in round {
                if Self::division_of(g.0) != Self::division_of(g.1) {
                    let id = edges.len();
                    edges.push(g);
                    edge_round.push(ri);
                    adj[g.0].push((g.1, id));
                    adj[g.1].push((g.0, id));
                }
            }
        }
        let mut used = vec![false; edges.len()];
        let mut oriented: Vec<Option<Pair>> = vec![None; edges.len()];
        let mut ptr = vec![0usize; n];
        // Hierholzer-style walk: follow unused edges until stuck, orienting along the walk.
        for start in 0..n {
            let mut cur = start;
            loop {
                let mut moved = false;
                while ptr[cur] < adj[cur].len() {
                    let (nb, id) = adj[cur][ptr[cur]];
                    ptr[cur] += 1;
                    if used[id] {
                        continue;
                    }
                    used[id] = true;
                    oriented[id] = Some((cur, nb)); // cur hosts nb
                    cur = nb;
                    moved = true;
                    break;
                }
                if !moved {
                    break;
                }
            }
        }
        for id in 0..edges.len() {
            out[edge_round[id]].push(oriented[id].expect("every cross edge oriented"));
        }
        // Division games: round-robin rounds (first meeting) then repeat rounds (reverse).
        for (ri, round) in rounds.iter().enumerate().take(11) {
            for &g in round {
                if Self::division_of(g.0) == Self::division_of(g.1) {
                    let host_first = rng.chance(0.5);
                    let (h, a) = if host_first { (g.0, g.1) } else { (g.1, g.0) };
                    first_home.insert(key(g.0, g.1), h);
                    out[ri].push((h, a));
                }
            }
        }
        for ri in 11..14 {
            for &g in &rounds[ri] {
                let h = first_home[&key(g.0, g.1)];
                let a = if h == g.0 { g.1 } else { g.0 };
                out[ri].push((a, h)); // reversed host
            }
        }
        out
    }

    /// Number of (team, week) spots where a team is on its third-or-later straight road game.
    fn violations(by_round: &[Vec<Pair>]) -> usize {
        let mut streak = vec![0; Self::TEAMS];
        let mut bad = 0;
        for round in by_round {
            for g in round {
                streak[g.0] = 0;
                streak[g.1] += 1;
                if streak[g.1] > 2 {
                    bad += 1;
                }
            }
        }
        bad
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sched(seed: u64) -> Vec<ScheduledGame> {
        ScheduleGenerator::generate(&mut Rng::new(seed))
    }

    #[test]
    fn fourteen_weeks_six_games_each() {
        for seed in 0..40 {
            let s = sched(seed);
            assert_eq!(s.len(), 14 * 6);
            for w in 1..=14 {
                let mut seen = std::collections::HashSet::new();
                for g in s.iter().filter(|g| g.week == w) {
                    assert!(seen.insert(g.home) && seen.insert(g.away), "seed {seed} week {w}");
                }
                assert_eq!(seen.len(), 12);
            }
        }
    }

    #[test]
    fn division_rivals_twice_everyone_else_once_seven_and_seven() {
        for seed in 0..40 {
            let s = sched(seed);
            for t in 0..12 {
                let home = s.iter().filter(|g| g.home == t).count();
                let away = s.iter().filter(|g| g.away == t).count();
                assert_eq!((home, away), (7, 7), "seed {seed} team {t}");
                for o in 0..12 {
                    if o == t {
                        continue;
                    }
                    let meets = s.iter().filter(|g| (g.home == t && g.away == o) || (g.home == o && g.away == t)).count();
                    let want = if ScheduleGenerator::division_of(t) == ScheduleGenerator::division_of(o) { 2 } else { 1 };
                    assert_eq!(meets, want, "seed {seed} {t} v {o}");
                }
            }
        }
    }

    #[test]
    fn division_rivals_swap_hosts() {
        for seed in 0..20 {
            let s = sched(seed);
            for a in 0..12 {
                for b in a + 1..12 {
                    if ScheduleGenerator::division_of(a) != ScheduleGenerator::division_of(b) {
                        continue;
                    }
                    let hosts: Vec<usize> = s.iter().filter(|g| (g.home == a && g.away == b) || (g.home == b && g.away == a)).map(|g| g.home).collect();
                    assert_ne!(hosts[0], hosts[1]);
                }
            }
        }
    }

    #[test]
    fn no_team_has_three_straight_road_games() {
        for seed in 0..40 {
            let s = sched(seed);
            for t in 0..12 {
                let mut streak = 0;
                for w in 1..=14 {
                    let g = s.iter().find(|g| g.week == w && (g.home == t || g.away == t)).unwrap();
                    streak = if g.away == t { streak + 1 } else { 0 };
                    assert!(streak <= 2, "seed {seed} team {t} week {w}");
                }
            }
        }
    }

    #[test]
    fn deterministic_by_seed() {
        assert_eq!(sched(9), sched(9));
        assert_ne!(sched(9), sched(10));
    }
}
