use crate::enums::{Attr, Position};
use crate::player::Player;
use crate::ratings::Ratings;
use crate::rng::Rng;

/// One offseason's rating change for a player (D12: before -> after only).
#[derive(Clone, Debug)]
pub struct PlayerChange {
    pub before: Player,
    pub after: Player,
}

impl PlayerChange {
    pub fn delta(&self, a: Attr) -> i32 {
        self.after.ratings[a] - self.before.ratings[a]
    }

    pub fn ovr_delta(&self) -> i32 {
        self.after.ovr() - self.before.ovr()
    }
}

/// (start age, base loss per year, extra loss per year after start).
fn arc(a: Attr) -> (i32, f64, f64) {
    match a {
        Attr::Speed => (28, 1.2, 0.9),
        Attr::Acceleration => (28, 1.1, 0.8),
        Attr::Agility => (29, 1.0, 0.7),
        Attr::Endurance => (30, 0.8, 0.5),
        Attr::Strength => (31, 0.6, 0.5),
        Attr::Hands => (33, 0.5, 0.3),
        Attr::Intelligence => (36, 0.4, 0.3),
        Attr::Discipline => (36, 0.3, 0.3),
    }
}

/// Stable -2..+2 year shift from the player id: some players age early, some late.
pub fn decline_shift(player_id: u32) -> i32 {
    let h = (player_id as u64).wrapping_mul(2_654_435_761) & 0xFFFF;
    (h % 5) as i32 - 2
}

/// Advance one year. `snaps_last_season` boosts growth slightly for players who actually played.
///
/// Growth: each attribute moves toward its own hidden ceiling. Decline: authored arcs per attribute
/// (speed/accel/agility first, intelligence/discipline last) with a per-player early/late-decliner shift.
/// Current ratings never exceed the ceiling.
pub fn advance(p: &Player, rng: &mut Rng, snaps_last_season: i32, growth_mult: f64) -> PlayerChange {
    let age = p.age;
    let shift = decline_shift(p.id);
    let played = if snaps_last_season > 300 { 1.12 } else if snaps_last_season > 100 { 1.0 } else { 0.88 };
    let mut new_vals = [0i32; 8];
    for a in Attr::ALL {
        let cur = p.ratings[a];
        let cap = p.potentials[a];
        let mut v = cur as f64;
        // Growth toward the ceiling (strongest when young).
        let gap = cap - cur;
        if gap > 0 {
            let rate = if age <= 22 { 0.42 } else if age <= 25 { 0.30 } else if age <= 28 { 0.17 } else { 0.06 };
            v += gap as f64 * rate * played * growth_mult * (0.55 + rng.next_f64() * 0.9);
        }
        // Decline arc.
        let (start_age, base, extra) = arc(a);
        let start = start_age + shift;
        if age >= start {
            let yrs = age - start;
            v -= (base + extra * yrs.min(8) as f64) * (0.6 + rng.next_f64() * 0.8);
        }
        v += rng.gaussian(0.0, 0.6);
        new_vals[a.index()] = (v.round() as i32).clamp(1, cap);
    }
    let mut after = p.clone();
    after.age = age + 1;
    after.years_pro = p.years_pro + 1;
    after.ratings = Ratings::new(new_vals);
    PlayerChange { before: p.clone(), after }
}

/// Retirement chance for the coming year (kickers and punters last longer; weak players quit sooner).
pub fn retire_chance(p: &Player, season_ending_injury: bool) -> f64 {
    let kp = matches!(p.position, Position::K | Position::P);
    let age = p.age - if kp { 4 } else { 0 };
    let mut c: f64 = if age < 30 {
        0.0
    } else {
        match age {
            30 => 0.03,
            31 => 0.06,
            32 => 0.11,
            33 => 0.19,
            34 => 0.30,
            35 => 0.42,
            36 => 0.55,
            _ => 0.75,
        }
    };
    if p.ovr() < 45 && age >= 30 {
        c += 0.1;
    }
    if season_ending_injury {
        c += 0.15;
    }
    c.clamp(0.0, 0.95)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::{GenOptions, PlayerGenerator};
    use crate::tiers::Tier;

    fn mean(xs: impl Iterator<Item = f64>) -> f64 {
        let v: Vec<f64> = xs.collect();
        v.iter().sum::<f64>() / v.len() as f64
    }

    fn make_gen(seed: u64) -> PlayerGenerator {
        PlayerGenerator::built_in(Rng::new(seed))
    }

    #[test]
    fn invariants_hold_over_six_seasons() {
        let mut g = make_gen(1);
        let mut rng = Rng::new(2);
        for i in 0..3000 {
            let mut p = g.generate(Position::ALL[i % 16], GenOptions::default().tier(Tier::ALL[i % 5]));
            for _ in 0..6 {
                let c = advance(&p, &mut rng, 0, 1.0);
                assert_eq!(c.after.age, p.age + 1);
                for a in Attr::ALL {
                    assert!((1..=c.after.potentials[a]).contains(&c.after.ratings[a]));
                }
                p = c.after;
            }
        }
    }

    #[test]
    fn young_improve_veterans_lose_speed_but_keep_mind() {
        let mut g = make_gen(3);
        let mut rng = Rng::new(4);
        let young: Vec<_> = (0..1500).map(|_| g.generate(Position::Wr, GenOptions::default().age(21))).collect();
        let vets: Vec<_> = (0..1500).map(|_| g.generate(Position::Wr, GenOptions::default().age(33))).collect();
        let yc: Vec<_> = young.iter().map(|p| advance(p, &mut rng, 0, 1.0)).collect();
        let vc: Vec<_> = vets.iter().map(|p| advance(p, &mut rng, 0, 1.0)).collect();
        assert!(mean(yc.iter().map(|c| c.ovr_delta() as f64)) > 1.5);
        assert!(mean(vc.iter().map(|c| c.delta(Attr::Speed) as f64)) < -1.5);
        assert!(mean(vc.iter().map(|c| c.delta(Attr::Intelligence) as f64)) > -1.5);
        assert!(mean(vc.iter().map(|c| c.ovr_delta() as f64)) < 0.0);
    }

    #[test]
    fn hidden_potential_drives_growth() {
        let mut rng = Rng::new(5);
        let base = make_gen(6).generate(Position::Rb, GenOptions::default().age(22));
        let mut run = |gap: i32| {
            let r = base.ratings.to_array();
            let pots = r.map(|v| (v + gap).clamp(1, 99));
            let p = Player::new(7, "T".into(), 22, Position::Rb, Ratings::new(r), Ratings::new(pots), "X".into(), "Y".into());
            advance(&p, &mut rng, 0, 1.0).ovr_delta() as f64
        };
        let big = mean((0..400).map(|_| run(20)));
        let none = mean((0..400).map(|_| run(0)));
        assert!(big > none + 2.0, "{big} vs {none}");
    }

    #[test]
    fn decline_shift_spans_five_values() {
        let shifts: std::collections::HashSet<i32> = (1..200).map(decline_shift).collect();
        assert_eq!(shifts.len(), 5);
        assert!(shifts.iter().all(|s| (-2..=2).contains(s)));
    }

    #[test]
    fn deterministic_by_seed() {
        let p = make_gen(8).generate(Position::Cb, GenOptions::default().age(25));
        let a = advance(&p, &mut Rng::new(9), 0, 1.0).after.ratings.to_string();
        let b = advance(&p, &mut Rng::new(9), 0, 1.0).after.ratings.to_string();
        assert_eq!(a, b);
    }

    #[test]
    fn retirement_odds_rise_with_age_and_kickers_last_longer() {
        let mut g = make_gen(10);
        let qb30 = g.generate(Position::Qb, GenOptions::default().age(30));
        let qb36 = g.generate(Position::Qb, GenOptions::default().age(36));
        let k36 = g.generate(Position::K, GenOptions::default().age(36));
        assert!(retire_chance(&qb36, false) > retire_chance(&qb30, false));
        assert!(retire_chance(&k36, false) < retire_chance(&qb36, false));
        assert!(retire_chance(&qb30, false) < 0.05);
        assert_eq!(retire_chance(&g.generate(Position::Qb, GenOptions::default().age(27)), false), 0.0);
    }

    #[test]
    fn progression_adds_a_year_and_keeps_acquisition() {
        let mut g = make_gen(8);
        let young = g.generate(Position::Wr, GenOptions::default().age(21));
        let next = advance(&young, &mut Rng::new(1), 0, 1.0).after;
        assert_eq!(next.years_pro, 1);
        assert!(!next.is_rookie());
        assert_eq!(next.acquired, young.acquired);
    }
}
