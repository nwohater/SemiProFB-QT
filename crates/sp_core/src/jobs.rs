use crate::rng::Rng;
use std::collections::HashMap;

/// A player's day job. Risks are 1-99 (99 = nearly always).
/// `injury_risk`: how likely he is to get hurt on the job. `travel_risk`: how likely work travel keeps him out of a game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Job {
    pub name: String,
    pub injury_risk: i32,
    pub travel_risk: i32,
}

impl Job {
    fn new(name: &str, injury_risk: i32, travel_risk: i32) -> Self {
        Self { name: name.to_string(), injury_risk, travel_risk }
    }
}

/// Tuning for the day-job risk rolls: per-game chance = risk x these factors (so risk 50 -> 2.0% / 0.75%).
pub mod tuning {
    /// Chance per risk point, per player per game, that his work trip makes him miss the next game.
    pub const TRAVEL_MISS_PER_POINT: f64 = 0.0004;
    /// Chance per risk point, per player per game, that he gets hurt on the job.
    pub const INJURY_PER_POINT: f64 = 0.00015;

    pub struct Severity {
        pub name: &'static str,
        pub weight: f64,
        pub min_games: i32,
        pub max_games: i32,
    }

    /// Severity of a job injury: games missed, weighted. (min, max) inclusive.
    pub const SEVERITY: [Severity; 3] = [
        Severity { name: "minor", weight: 60.0, min_games: 1, max_games: 1 },
        Severity { name: "multiWeek", weight: 34.0, min_games: 2, max_games: 4 },
        Severity { name: "serious", weight: 6.0, min_games: 5, max_games: 9 },
    ];
}

const JOBS_CSV: &str = include_str!("../../../data/jobs.csv");

/// Jobs loaded from a CSV with a header row: `Job,InjuryRisk,TravelRisk`.
#[derive(Clone, Debug)]
pub struct JobTable {
    pub jobs: Vec<Job>,
    by_name: HashMap<String, Job>,
}

impl JobTable {
    pub fn new(jobs: Vec<Job>) -> Self {
        let by_name = jobs.iter().map(|j| (j.name.to_lowercase(), j.clone())).collect();
        Self { jobs, by_name }
    }

    /// Used for a job that is not in the table (e.g. an old save).
    pub fn unknown() -> Job {
        Job::new("Unknown", 15, 15)
    }

    pub fn built_in() -> Self {
        Self::new(vec![
            Job::new("Plumber", 53, 45),
            Job::new("Dentist", 9, 8),
            Job::new("Electrician", 53, 32),
            Job::new("Insurance Agent", 5, 25),
            Job::new("Janitor", 25, 15),
            Job::new("Farmer", 64, 30),
            Job::new("Welder", 69, 37),
            Job::new("Truck Driver", 37, 97),
            Job::new("Landscaper", 48, 29),
            Job::new("Tow Truck Driver", 40, 60),
        ])
    }

    /// Falls back to [`Self::built_in`] when the CSV has no usable rows.
    pub fn from_csv(csv: &str) -> Self {
        let mut out = Vec::new();
        for raw in csv.replace('\r', "").split('\n').skip(1) {
            let parts: Vec<String> = raw.split(',').map(|s| s.replace('"', "").trim().to_string()).collect();
            if parts.len() < 3 || parts[0].is_empty() {
                continue;
            }
            let (Ok(inj), Ok(trv)) = (parts[1].parse::<i32>(), parts[2].parse::<i32>()) else { continue };
            out.push(Job::new(&parts[0], inj.clamp(0, 99), trv.clamp(0, 99)));
        }
        if out.is_empty() { Self::built_in() } else { Self::new(out) }
    }

    pub fn bundled() -> Self {
        Self::from_csv(JOBS_CSV)
    }

    /// (name, injury risk, travel risk) rows, for saving.
    pub fn rows(&self) -> Vec<(String, i32, i32)> {
        self.jobs.iter().map(|j| (j.name.clone(), j.injury_risk, j.travel_risk)).collect()
    }

    pub fn from_rows(rows: &[(String, i32, i32)]) -> Self {
        if rows.is_empty() {
            return Self::built_in();
        }
        Self::new(rows.iter().map(|(n, i, t)| Job { name: n.clone(), injury_risk: *i, travel_risk: *t }).collect())
    }

    /// The job by name (case-insensitive), or [`Self::unknown`]'s risks under that name.
    pub fn lookup(&self, name: &str) -> Job {
        self.by_name.get(&name.to_lowercase()).cloned().unwrap_or_else(|| {
            let u = Self::unknown();
            Job::new(name, u.injury_risk, u.travel_risk)
        })
    }

    /// One draw from `rng`.
    pub fn pick(&self, rng: &mut Rng) -> &Job {
        rng.pick(&self.jobs)
    }
}

/// Plain-words band for a 1-99 risk.
pub fn risk_band(risk: i32) -> &'static str {
    if risk >= 75 {
        "Very high"
    } else if risk >= 50 {
        "High"
    } else if risk >= 25 {
        "Moderate"
    } else {
        "Low"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_parses_clamps_and_skips_bad_rows() {
        let t = JobTable::from_csv("Job,InjuryRisk,TravelRisk\nPilot,120,40\nbad,x,1\n,1,1\nChef,10,20\r\n");
        assert_eq!(t.jobs.len(), 2);
        assert_eq!(t.lookup("pilot").injury_risk, 99);
    }

    #[test]
    fn empty_csv_falls_back() {
        assert_eq!(JobTable::from_csv("Job,InjuryRisk,TravelRisk\n").jobs.len(), JobTable::built_in().jobs.len());
    }

    #[test]
    fn unknown_job_gets_default_risks_under_its_name() {
        let j = JobTable::built_in().lookup("Astronaut");
        assert_eq!((j.name.as_str(), j.injury_risk, j.travel_risk), ("Astronaut", 15, 15));
    }

    #[test]
    fn bundled_table_loads() {
        assert!(JobTable::bundled().jobs.len() > 10);
    }

    #[test]
    fn bands() {
        assert_eq!([risk_band(10), risk_band(30), risk_band(60), risk_band(80)], ["Low", "Moderate", "High", "Very high"]);
    }
}
