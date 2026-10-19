use serde::{Deserialize, Serialize};
use sp_core::Weather;
use sp_economy::GameFinance;
use sp_sim::BoxScore;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameRecord {
    /// 1-14 regular season; 15+ playoffs.
    pub week: i32,
    pub home: usize,
    pub away: usize,
    pub home_score: i32,
    pub away_score: i32,
    pub overtime: bool,
    pub playoff: bool,
    pub weather: Weather,
    /// Kept optional to save memory.
    pub box_score: Option<BoxScore>,
    pub home_finance: Option<GameFinance>,
    pub away_finance: Option<GameFinance>,
}

impl GameRecord {
    pub fn new(week: i32, home: usize, away: usize, home_score: i32, away_score: i32, overtime: bool, playoff: bool, weather: Weather) -> Self {
        Self { week, home, away, home_score, away_score, overtime, playoff, weather, box_score: None, home_finance: None, away_finance: None }
    }

    pub fn tie(&self) -> bool {
        self.home_score == self.away_score
    }

    pub fn winner(&self) -> Option<usize> {
        if self.tie() {
            None
        } else if self.home_score > self.away_score {
            Some(self.home)
        } else {
            Some(self.away)
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamRecord {
    pub wins: i32,
    pub losses: i32,
    pub ties: i32,
    pub points_for: i32,
    pub points_against: i32,
    pub div_wins: i32,
    pub div_losses: i32,
    pub div_ties: i32,
    /// +n win streak, -n loss streak.
    pub streak: i32,
}

impl TeamRecord {
    pub fn games(&self) -> i32 {
        self.wins + self.losses + self.ties
    }

    pub fn pct(&self) -> f64 {
        if self.games() == 0 { 0.0 } else { (self.wins as f64 + self.ties as f64 * 0.5) / self.games() as f64 }
    }

    pub fn div_pct(&self) -> f64 {
        let g = self.div_wins + self.div_losses + self.div_ties;
        if g == 0 { 0.0 } else { (self.div_wins as f64 + self.div_ties as f64 * 0.5) / g as f64 }
    }

    pub fn diff(&self) -> i32 {
        self.points_for - self.points_against
    }
}

impl std::fmt::Display for TeamRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}-{}", self.wins, self.losses, self.ties)
    }
}

/// Regular-season standings with tiebreakers: win pct, head-to-head (among tied group), division record,
/// point differential, points for, then a seeded coin flip.
#[derive(Clone, Debug)]
pub struct Standings {
    pub team_count: usize,
    pub records: Vec<TeamRecord>,
    pub games: Vec<GameRecord>,
    pub tiebreak_seed: u64,
    division_of: fn(usize) -> usize,
}

impl Standings {
    pub fn new(team_count: usize, division_of: fn(usize) -> usize, tiebreak_seed: u64) -> Self {
        Self { team_count, records: vec![TeamRecord::default(); team_count], games: Vec::new(), tiebreak_seed, division_of }
    }

    pub fn division_of(&self, team: usize) -> usize {
        (self.division_of)(team)
    }

    pub fn add(&mut self, g: &GameRecord) {
        if g.playoff {
            return;
        }
        let g = g.clone_light();
        let same_div = (self.division_of)(g.home) == (self.division_of)(g.away);
        {
            let h = &mut self.records[g.home];
            h.points_for += g.home_score;
            h.points_against += g.away_score;
        }
        {
            let a = &mut self.records[g.away];
            a.points_for += g.away_score;
            a.points_against += g.home_score;
        }
        if g.tie() {
            for t in [g.home, g.away] {
                let r = &mut self.records[t];
                r.ties += 1;
                if same_div {
                    r.div_ties += 1;
                }
                r.streak = 0;
            }
        } else {
            let (w, l) = if g.winner() == Some(g.home) { (g.home, g.away) } else { (g.away, g.home) };
            {
                let r = &mut self.records[w];
                r.wins += 1;
                r.streak = if r.streak > 0 { r.streak + 1 } else { 1 };
                if same_div {
                    r.div_wins += 1;
                }
            }
            {
                let r = &mut self.records[l];
                r.losses += 1;
                r.streak = if r.streak < 0 { r.streak - 1 } else { -1 };
                if same_div {
                    r.div_losses += 1;
                }
            }
        }
        self.games.push(g);
    }

    fn h2h(&self, team: usize, group: &std::collections::HashSet<usize>) -> f64 {
        let (mut w, mut g) = (0.0, 0);
        for x in &self.games {
            if !((x.home == team && group.contains(&x.away)) || (x.away == team && group.contains(&x.home))) {
                continue;
            }
            g += 1;
            if x.tie() {
                w += 0.5;
            } else if x.winner() == Some(team) {
                w += 1.0;
            }
        }
        if g == 0 { 0.5 } else { w / g as f64 }
    }

    fn flip(&self, team: usize) -> u64 {
        (((team as u64 + 1).wrapping_mul(2_654_435_761)) ^ self.tiebreak_seed.wrapping_mul(40503)) & 0xFFFF
    }

    /// Teams in `pool` ordered best -> worst with tiebreakers.
    pub fn rank(&self, pool: &[usize]) -> Vec<usize> {
        let mut teams: Vec<usize> = pool.to_vec();
        teams.sort_by(|&a, &b| self.records[b].pct().total_cmp(&self.records[a].pct()));
        let mut out = Vec::new();
        let mut i = 0;
        while i < teams.len() {
            let mut j = i;
            while j + 1 < teams.len() && self.records[teams[j + 1]].pct() == self.records[teams[i]].pct() {
                j += 1;
            }
            let mut group: Vec<usize> = teams[i..=j].to_vec();
            if group.len() > 1 {
                let set: std::collections::HashSet<usize> = group.iter().copied().collect();
                group.sort_by(|&a, &b| {
                    self.h2h(b, &set)
                        .total_cmp(&self.h2h(a, &set))
                        .then_with(|| self.records[b].div_pct().total_cmp(&self.records[a].div_pct()))
                        .then_with(|| self.records[b].diff().cmp(&self.records[a].diff()))
                        .then_with(|| self.records[b].points_for.cmp(&self.records[a].points_for))
                        .then_with(|| self.flip(b).cmp(&self.flip(a)))
                });
            }
            out.extend(group);
            i = j + 1;
        }
        out
    }

    pub fn division_order(&self, division: usize) -> Vec<usize> {
        let pool: Vec<usize> = (0..self.team_count).filter(|&t| (self.division_of)(t) == division).collect();
        self.rank(&pool)
    }
}

impl GameRecord {
    /// A copy without the heavy parts (box score, per-game money), for the standings' own game list.
    pub fn clone_light(&self) -> GameRecord {
        GameRecord { box_score: None, home_finance: None, away_finance: None, ..self.clone() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn div(t: usize) -> usize {
        t / 4
    }

    fn g(home: usize, away: usize, hs: i32, as_: i32) -> GameRecord {
        GameRecord::new(1, home, away, hs, as_, false, false, Weather::Clear)
    }

    #[test]
    fn records_streaks_and_division_marks() {
        let mut s = Standings::new(12, div, 0);
        s.add(&g(0, 1, 21, 14)); // same division
        s.add(&g(0, 5, 10, 17)); // cross-division
        s.add(&g(2, 0, 7, 7)); // tie
        let r = &s.records[0];
        assert_eq!((r.wins, r.losses, r.ties, r.div_wins, r.div_ties, r.streak), (1, 1, 1, 1, 1, 0));
        assert_eq!((s.records[1].losses, s.records[1].streak), (1, -1));
        assert_eq!((s.records[0].points_for, s.records[0].points_against), (38, 38));
    }

    #[test]
    fn playoff_games_do_not_count() {
        let mut s = Standings::new(12, div, 0);
        let mut p = g(0, 1, 30, 3);
        p.playoff = true;
        s.add(&p);
        assert_eq!(s.records[0].games(), 0);
    }

    #[test]
    fn ranking_uses_head_to_head_then_division_then_differential() {
        let mut s = Standings::new(12, div, 0);
        // 0 and 1 both 1-1, but 1 beat 0 head to head.
        s.add(&g(0, 1, 10, 20));
        s.add(&g(0, 2, 30, 0));
        s.add(&g(1, 3, 0, 3));
        let order = s.rank(&[0, 1]);
        assert_eq!(order, vec![1, 0]);
    }

    #[test]
    fn unbeaten_team_ranks_first_in_its_division() {
        let mut s = Standings::new(12, div, 7);
        for o in 1..4 {
            s.add(&g(0, o, 20, 10));
        }
        assert_eq!(s.division_order(0)[0], 0);
        assert_eq!(s.division_order(0).len(), 4);
    }

    #[test]
    fn ties_break_deterministically_by_seed() {
        let s = Standings::new(12, div, 3);
        assert_eq!(s.rank(&[0, 1, 2, 3]), Standings::new(12, div, 3).rank(&[0, 1, 2, 3]));
    }
}
