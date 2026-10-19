use crate::enums::Position;
use crate::jersey::{jersey_bands_for, pick_jersey_number};
use crate::player::Player;
use crate::returners::{RETURNER_POSITIONS, ReturnRole, return_score};
use crate::strategy::Strategy;
use crate::town::Town;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Default rostered position mix (D32): 50 players, trimmed to 46 active.
pub const ROSTER_MIX: [(Position, usize); 16] = {
    use Position::*;
    [
        (Qb, 3), (Rb, 4), (Fb, 1), (Wr, 6), (Te, 3),
        (Ot, 4), (Og, 4), (C, 1), // OL 9
        (De, 4), (Dt, 4), // DL 8
        (Olb, 3), (Mlb, 3), // LB 6
        (Cb, 4), (S, 4), // DB 8
        (K, 1), (P, 1),
    ]
};

pub const ACTIVE_ROSTER_SIZE: usize = 46;

/// Minimum players to dress per position (D14 min coverage rule, tunable).
pub fn min_active_coverage(p: Position) -> usize {
    use Position::*;
    match p {
        Qb => 2, Rb => 2, Fb => 1, Wr => 4, Te => 2,
        Ot => 3, Og => 3, C => 1, De => 3, Dt => 3,
        Olb => 2, Mlb => 2, Cb => 3, S => 3, K => 1, P => 1,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Team {
    pub town: Town,
    pub roster: Vec<Player>,
    pub active_ids: HashSet<u32>,
    pub strategy: Strategy,
    /// Your depth chart: for a position, player ids in the order you want them used (first = starter).
    /// A position with no entry is fully automatic (best rating first, rotating tired players out).
    pub depth: HashMap<Position, Vec<u32>>,
    /// Jersey numbers for the players on this roster (player id -> number). `ensure_numbers` fills in anyone missing,
    /// so signings, trades and draft picks need no special handling.
    pub numbers: HashMap<u32, i32>,
    /// Your return men, first = used first. A role with no entry is automatic.
    pub return_depth: HashMap<ReturnRole, Vec<u32>>,
}

impl Team {
    pub fn new(town: Town, roster: Vec<Player>) -> Self {
        Self { town, roster, active_ids: HashSet::new(), strategy: Strategy::default(), depth: HashMap::new(), numbers: HashMap::new(), return_depth: HashMap::new() }
    }

    /// Gives every player a unique number and drops numbers of players who left. Players with fewer options pick first.
    pub fn ensure_numbers(&mut self) {
        let ids: HashSet<u32> = self.roster.iter().map(|p| p.id).collect();
        self.numbers.retain(|id, _| ids.contains(id));
        let mut taken = HashSet::new();
        let mut dup = Vec::new();
        let mut keep: Vec<(u32, i32)> = self.numbers.iter().map(|(&k, &v)| (k, v)).collect();
        keep.sort(); // so which duplicate loses is deterministic
        for (id, n) in keep {
            if !taken.insert(n) {
                dup.push(id);
            }
        }
        for id in dup {
            self.numbers.remove(&id);
        }
        let mut todo: Vec<usize> = (0..self.roster.len()).filter(|&i| !self.numbers.contains_key(&self.roster[i].id)).collect();
        todo.sort_by(|&a, &b| {
            let (pa, pb) = (&self.roster[a], &self.roster[b]);
            jersey_bands_for(pa.position).len().cmp(&jersey_bands_for(pb.position).len()).then(pa.id.cmp(&pb.id))
        });
        // Arrivals ask for the number they last wore first, then everyone else picks.
        let mut rest = Vec::new();
        for i in todo {
            let (id, want) = (self.roster[i].id, self.roster[i].jersey);
            match want {
                Some(n) if !taken.contains(&n) => {
                    self.numbers.insert(id, n);
                    taken.insert(n);
                }
                _ => rest.push(i),
            }
        }
        for i in rest {
            let (id, pos) = (self.roster[i].id, self.roster[i].position);
            let n = pick_jersey_number(pos, id, &taken);
            self.numbers.insert(id, n);
            taken.insert(n);
        }
        for p in &mut self.roster {
            p.jersey = self.numbers.get(&p.id).copied();
        }
    }

    /// A player's number on this team, or `None` if he is not on it.
    pub fn number_of(&mut self, player_id: u32) -> Option<i32> {
        if !self.roster.iter().any(|p| p.id == player_id) {
            return None;
        }
        if !self.numbers.contains_key(&player_id) || self.numbers.len() != self.roster.len() {
            self.ensure_numbers(); // someone arrived or left since the last pass
        }
        self.numbers.get(&player_id).copied()
    }

    /// The order this position is actually used in: your listed players (still active), then the rest of that
    /// position's active players by rating.
    pub fn depth_order(&self, pos: Position) -> Vec<&Player> {
        let by_id: HashMap<u32, &Player> = self.roster.iter().map(|p| (p.id, p)).collect();
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        for id in self.depth.get(&pos).into_iter().flatten() {
            if let Some(&p) = by_id.get(id) {
                if self.active_ids.contains(id) && seen.insert(*id) {
                    out.push(p);
                }
            }
        }
        let mut rest: Vec<&Player> = self.active().filter(|p| p.position == pos && !seen.contains(&p.id)).collect();
        rest.sort_by_key(|p| std::cmp::Reverse(p.ovr_at(pos)));
        out.extend(rest);
        out
    }

    pub fn set_depth(&mut self, pos: Position, ids: Vec<u32>) {
        self.depth.insert(pos, ids);
    }

    /// Back to automatic for one position.
    pub fn auto_sort(&mut self, pos: Position) {
        self.depth.remove(&pos);
    }

    pub fn auto_sort_all(&mut self) {
        self.depth.clear();
        self.return_depth.clear();
    }

    /// The order return men are used in: your listed players (still active), then the best of the rest by return score.
    pub fn return_order(&self, role: ReturnRole) -> Vec<&Player> {
        let by_id: HashMap<u32, &Player> = self.roster.iter().map(|p| (p.id, p)).collect();
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        for id in self.return_depth.get(&role).into_iter().flatten() {
            if let Some(&p) = by_id.get(id) {
                if self.active_ids.contains(id) && seen.insert(*id) {
                    out.push(p);
                }
            }
        }
        let mut rest: Vec<&Player> = self.active().filter(|p| RETURNER_POSITIONS.contains(&p.position) && !seen.contains(&p.id)).collect();
        rest.sort_by(|a, b| return_score(b).total_cmp(&return_score(a)));
        out.extend(rest);
        out
    }

    pub fn set_return_depth(&mut self, role: ReturnRole, ids: Vec<u32>) {
        self.return_depth.insert(role, ids);
    }

    pub fn auto_sort_return(&mut self, role: ReturnRole) {
        self.return_depth.remove(&role);
    }

    pub fn active(&self) -> impl Iterator<Item = &Player> {
        self.roster.iter().filter(|p| self.active_ids.contains(&p.id))
    }

    pub fn at_position(&self, pos: Position, active_only: bool) -> Vec<&Player> {
        let mut list: Vec<&Player> = if active_only {
            self.active().filter(|p| p.position == pos).collect()
        } else {
            self.roster.iter().filter(|p| p.position == pos).collect()
        };
        list.sort_by_key(|p| std::cmp::Reverse(p.ovr()));
        list
    }
}

impl std::fmt::Display for Team {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.town.team_name())
    }
}

/// Best `ACTIVE_ROSTER_SIZE` available players, keeping positional minimums first.
pub fn pick_active_list(team: &Team, unavailable: &HashSet<u32>) -> HashSet<u32> {
    let mut sorted: Vec<&Player> = team.roster.iter().filter(|p| !unavailable.contains(&p.id)).collect();
    sorted.sort_by_key(|p| std::cmp::Reverse(p.ovr()));
    let mut chosen = HashSet::new();
    let mut counts: HashMap<Position, usize> = HashMap::new();
    for p in &sorted {
        let c = counts.entry(p.position).or_insert(0);
        if *c < min_active_coverage(p.position) {
            chosen.insert(p.id);
            *c += 1;
        }
    }
    for p in &sorted {
        if chosen.len() >= ACTIVE_ROSTER_SIZE {
            break;
        }
        chosen.insert(p.id);
    }
    chosen
}
