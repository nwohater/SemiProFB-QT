use crate::standings::Standings;
use serde::{Deserialize, Serialize};

/// The score of a finished playoff game (the full record lives in the league's results).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayoffResult {
    pub home_score: i32,
    pub away_score: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayoffGame {
    /// semifinal | final
    pub round: String,
    pub high_seed: usize,
    pub low_seed: usize,
    pub result: Option<PlayoffResult>,
}

impl PlayoffGame {
    pub fn new(round: &str, high_seed: usize, low_seed: usize) -> Self {
        Self { round: round.into(), high_seed, low_seed, result: None }
    }

    pub fn home(&self) -> usize {
        self.high_seed
    }

    pub fn away(&self) -> usize {
        self.low_seed
    }

    pub fn winner(&self) -> Option<usize> {
        let r = self.result?;
        if r.home_score == r.away_score {
            None
        } else if r.home_score > r.away_score {
            Some(self.home())
        } else {
            Some(self.away())
        }
    }
}

/// 4 teams: 3 division winners (seeds 1-3 by record) + best other team (seed 4). #1 v #4, #2 v #3, then the final.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Playoffs {
    /// Index 0 = seed 1.
    pub seeds: Vec<usize>,
    pub semifinals: Vec<PlayoffGame>,
    pub final_game: Option<PlayoffGame>,
    pub champion: Option<usize>,
}

impl Playoffs {
    pub fn from_standings(st: &Standings, divisions: usize) -> Self {
        let winners: Vec<usize> = (0..divisions).map(|d| st.division_order(d)[0]).collect();
        let seed_winners = st.rank(&winners);
        let rest: Vec<usize> = (0..st.team_count).filter(|t| !winners.contains(t)).collect();
        let wildcard = st.rank(&rest)[0];
        let mut seeds = seed_winners;
        seeds.push(wildcard);
        let semifinals = vec![PlayoffGame::new("semifinal", seeds[0], seeds[3]), PlayoffGame::new("semifinal", seeds[1], seeds[2])];
        Self { seeds, semifinals, final_game: None, champion: None }
    }

    pub fn semis_done(&self) -> bool {
        self.semifinals.iter().all(|g| g.result.is_some())
    }

    pub fn create_final(&mut self) {
        let mut winners: Vec<usize> = self.semifinals.iter().map(|g| g.winner().expect("playoff games never tie")).collect();
        // Higher seed (lower index) hosts.
        winners.sort_by_key(|w| self.seeds.iter().position(|s| s == w).unwrap());
        self.final_game = Some(PlayoffGame::new("final", winners[0], winners[1]));
    }

    /// 1-4, or 0 when the team is not in the field.
    pub fn seed_of(&self, team: usize) -> usize {
        self.seeds.iter().position(|&s| s == team).map(|i| i + 1).unwrap_or(0)
    }
}
