//! Multi-season balance harness (all teams on autopilot): cash, roster age, folds, facility spending.
//! Usage: balance [seeds=4] [seasons=8]
use sp_economy::Facility;
use sp_league::{League, NewLeague};

fn med(v: &mut Vec<i64>) -> i64 {
    v.sort();
    v[v.len() / 2]
}

fn main() {
    let seeds: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(4);
    let seasons: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(8);
    let mut cash: Vec<Vec<i64>> = vec![vec![]; seasons];
    let mut age: Vec<Vec<f64>> = vec![vec![]; seasons];
    let mut folds = vec![0usize; seasons];
    let mut fac: Vec<Vec<i64>> = vec![vec![]; seasons];
    let mut loans: Vec<Vec<i64>> = vec![vec![]; seasons];
    let mut net: Vec<Vec<i64>> = vec![vec![]; seasons];
    for s in 1..=seeds {
        let mut lg = League::new(s as u64 * 17, NewLeague { user_team: None, ..Default::default() });
        for y in 0..seasons {
            let before: Vec<i64> = lg.finances.iter().map(|f| f.cash).collect();
            lg.sim_season();
            folds[y] += lg.bankrupt_ai_teams().len();
            for i in 0..12 {
                let f = &lg.finances[i];
                cash[y].push(f.cash);
                net[y].push(f.cash - before[i]);
                fac[y].push(Facility::ALL.iter().map(|&k| f.level(k) as i64).sum());
                loans[y].push(f.loan_balance);
            }
            let ages: Vec<f64> = lg.teams.iter().flat_map(|t| t.roster.iter().map(|p| p.age as f64)).collect();
            age[y].push(ages.iter().sum::<f64>() / ages.len() as f64);
            lg.begin_offseason();
            lg.finish_offseason();
        }
    }
    println!("season  medCash   maxCash  medNet  avgAge  folds  facLvls  loanTeams");
    for y in 0..seasons {
        let mut c = cash[y].clone();
        let mx = *c.iter().max().unwrap();
        let mc = med(&mut c);
        let mut n = net[y].clone();
        let mn = med(&mut n);
        println!(
            "{:>6} {:>8} {:>9} {:>7} {:>7.1} {:>6} {:>8.2} {:>9}",
            y + 1,
            mc,
            mx,
            mn,
            age[y].iter().sum::<f64>() / seeds as f64,
            folds[y],
            fac[y].iter().sum::<i64>() as f64 / fac[y].len() as f64,
            loans[y].iter().filter(|l| **l > 0).count()
        );
    }
}
