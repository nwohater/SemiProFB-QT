use crate::enums::{Attr, Position};
use crate::jobs::JobTable;
use crate::names::NameBank;
use crate::player::{ACQUIRED_WALK_ON, Player};
use crate::position_templates::position_template;
use crate::ratings::Ratings;
use crate::rng::Rng;
use crate::sizes::SizeTable;
use crate::tiers::Tier;

/// Options for [`PlayerGenerator::generate`].
#[derive(Clone, Debug)]
pub struct GenOptions {
    pub tier: Tier,
    pub hometown: String,
    pub age: Option<i32>,
    pub years_pro: Option<i32>,
    pub acquired: String,
}

impl Default for GenOptions {
    fn default() -> Self {
        Self { tier: Tier::Regular, hometown: "Unknown".into(), age: None, years_pro: None, acquired: ACQUIRED_WALK_ON.into() }
    }
}

impl GenOptions {
    pub fn tier(mut self, t: Tier) -> Self {
        self.tier = t;
        self
    }
    pub fn age(mut self, a: i32) -> Self {
        self.age = Some(a);
        self
    }
    pub fn years_pro(mut self, y: i32) -> Self {
        self.years_pro = Some(y);
        self
    }
    pub fn hometown(mut self, h: &str) -> Self {
        self.hometown = h.into();
        self
    }
    pub fn acquired(mut self, a: &str) -> Self {
        self.acquired = a.into();
        self
    }
}

/// Generates players from position templates (features/player-generation.md).
/// Names come from `names` (two draws), sizes from `sizes` (three draws), jobs from `jobs` (one draw), so saves replay deterministically.
pub struct PlayerGenerator {
    pub rng: Rng,
    pub next_id: u32,
    pub names: NameBank,
    pub sizes: SizeTable,
    pub jobs: JobTable,
}

impl PlayerGenerator {
    /// A generator over the full bundled name, size and job tables.
    pub fn new(rng: Rng) -> Self {
        Self::with_tables(rng, NameBank::bundled(), SizeTable::bundled(), JobTable::bundled())
    }

    /// A generator over the small built-in tables (fast, for tests).
    pub fn built_in(rng: Rng) -> Self {
        Self::with_tables(rng, NameBank::built_in(), SizeTable::built_in(), JobTable::built_in())
    }

    pub fn with_tables(rng: Rng, names: NameBank, sizes: SizeTable, jobs: JobTable) -> Self {
        Self { rng, next_id: 1, names, sizes, jobs }
    }

    fn new_name(&mut self) -> String {
        let f = self.rng.pick(&self.names.first).clone();
        let l = self.rng.pick(&self.names.last).clone();
        format!("{f} {l}")
    }

    pub fn generate(&mut self, position: Position, opts: GenOptions) -> Player {
        let t = position_template(position);
        let tier = opts.tier;
        let age = opts.age.unwrap_or_else(|| self.age(position));
        // Shared latent factors give the correlations from sec 3.
        let athletic = self.rng.next_gaussian();
        let mind = self.rng.next_gaussian();
        let strong_over_speed = self.rng.next_gaussian();

        let mut values = [0i32; 8];
        let mut pots = [0i32; 8];
        let upside = self.rng.gaussian(0.0, 0.45).exp(); // lognormal per player
        for attr in Attr::ALL {
            let latent = match attr {
                Attr::Speed => 0.55 * athletic - 0.30 * strong_over_speed,
                Attr::Acceleration => 0.55 * athletic - 0.25 * strong_over_speed,
                Attr::Agility => 0.50 * athletic - 0.20 * strong_over_speed,
                Attr::Strength => 0.40 * strong_over_speed,
                Attr::Endurance => 0.25 * athletic,
                Attr::Intelligence => 0.30 * mind,
                Attr::Discipline => 0.30 * mind,
                Attr::Hands => 0.0,
            };
            let noise = self.rng.next_gaussian() * (1.0 - latent * latent / 4.0).sqrt().clamp(0.6, 1.0);
            let mut v = t.mean_of(attr) + tier.q_shift() + (latent + noise) * t.sd * tier.sd_mult() * 0.9;
            v += age_penalty(attr, age);
            let peak = (v.round() as i32).clamp(10, 99);
            let gap = self.gap(attr, age, upside);
            if age <= 25 {
                // Template means describe the player's peak: young players are generated below it, ceiling = peak.
                values[attr.index()] = (peak - gap).clamp(10, peak);
                pots[attr.index()] = peak;
            } else {
                values[attr.index()] = peak;
                pots[attr.index()] = (peak + gap).clamp(peak, 99);
            }
        }

        // Big bodies are strong and slow: weight follows the same latent factor as strength over speed.
        let wz = 0.6 * strong_over_speed + 0.8 * self.rng.next_gaussian();
        let hz = 0.5 * wz + 0.87 * self.rng.next_gaussian();
        let nose = self.rng.chance(0.3);
        let (height, weight) = self.sizes.draw(position, hz, wz, nose);

        let dual = (position == Position::K || position == Position::P) && self.rng.chance(0.2);
        let id = self.next_id;
        self.next_id += 1;
        let name = self.new_name();
        let day_job = self.jobs.pick(&mut self.rng).name.clone();
        let mut p = Player::new(id, name, age, position, Ratings::new(values), Ratings::new(pots), opts.hometown, day_job);
        p.also_kicks_punts = dual;
        p.height_in = height;
        p.weight_lb = weight;
        // Experience is a guess from age unless the caller knows better (a draftee is a rookie).
        p.years_pro = opts.years_pro.unwrap_or((age - 22).clamp(0, 30));
        p.acquired = opts.acquired;
        p
    }

    fn age(&mut self, p: Position) -> i32 {
        let base = self.rng.weighted(&[(21, 15.0), (24, 35.0), (28, 30.0), (32, 15.0), (36, 5.0)]);
        let mut age = base + self.rng.range(-1, 2);
        if matches!(p, Position::K | Position::P | Position::Qb) {
            age += self.rng.range(0, 3);
        }
        age.clamp(20, 38)
    }

    fn gap(&mut self, a: Attr, age: i32, upside: f64) -> i32 {
        let mut base = if age <= 22 {
            12.0
        } else if age <= 25 {
            7.0
        } else if age <= 29 {
            3.0
        } else {
            0.5
        };
        let mental = matches!(a, Attr::Intelligence | Attr::Discipline);
        if mental && age > 25 && age <= 31 {
            base += 2.0;
        }
        if !mental && age > 29 {
            base = 0.0;
        }
        let g = base * upside + self.rng.gaussian(0.0, base * 0.25);
        (g.round() as i32).clamp(0, 40)
    }
}

/// Older players generated post-peak: physical attrs reduced, mental maintained.
fn age_penalty(a: Attr, age: i32) -> f64 {
    if age <= 29 {
        return 0.0;
    }
    let years = (age - 29) as f64;
    match a {
        Attr::Speed | Attr::Acceleration | Attr::Agility => -1.6 * years,
        Attr::Endurance => -0.8 * years,
        Attr::Strength => -0.5 * years,
        Attr::Intelligence | Attr::Discipline => 0.3 * years,
        Attr::Hands => -0.2 * years,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tiers::Tier;

    fn mean(xs: impl Iterator<Item = f64>) -> f64 {
        let v: Vec<f64> = xs.collect();
        v.iter().sum::<f64>() / v.len() as f64
    }

    fn many(g: &mut PlayerGenerator, p: Position, n: usize, age: i32) -> Vec<Player> {
        (0..n).map(|_| g.generate(p, GenOptions::default().age(age))).collect()
    }

    #[test]
    fn deterministic_by_seed() {
        let a = PlayerGenerator::built_in(Rng::new(5)).generate(Position::Qb, GenOptions::default()).to_string();
        let b = PlayerGenerator::built_in(Rng::new(5)).generate(Position::Qb, GenOptions::default()).to_string();
        assert_eq!(a, b);
    }

    #[test]
    fn invariant_attr_within_potential_for_5k_players() {
        let mut g = PlayerGenerator::built_in(Rng::new(11));
        for i in 0..5000 {
            let p = g.generate(Position::ALL[i % 16], GenOptions::default().tier(Tier::ALL[i % 5]));
            for a in Attr::ALL {
                assert!((1..=99).contains(&p.ratings[a]));
                assert!(p.potentials[a] >= p.ratings[a] && p.potentials[a] <= 99);
            }
        }
    }

    #[test]
    fn lineman_strong_slow_receiver_fast_light() {
        let mut g = PlayerGenerator::built_in(Rng::new(3));
        let og = many(&mut g, Position::Og, 1500, 26);
        let wr = many(&mut g, Position::Wr, 1500, 26);
        let m = |ps: &[Player], a: Attr| mean(ps.iter().map(|p| p.ratings[a] as f64));
        assert!(m(&og, Attr::Strength) > m(&wr, Attr::Strength) + 20.0);
        assert!(m(&og, Attr::Speed) < m(&wr, Attr::Speed) - 25.0);
    }

    #[test]
    fn means_within_3_5_of_template_for_prime_age() {
        let mut g = PlayerGenerator::built_in(Rng::new(9));
        for pos in Position::ALL {
            let ps = many(&mut g, pos, 2000, 26);
            for a in Attr::ALL {
                let m = mean(ps.iter().map(|p| p.ratings[a] as f64));
                assert!((m - position_template(pos).mean_of(a)).abs() < 3.5, "{} {} mean {m}", pos.label(), a.code());
            }
        }
    }

    #[test]
    fn older_players_slower_but_smarter() {
        let mut g = PlayerGenerator::built_in(Rng::new(13));
        let young = many(&mut g, Position::Cb, 1000, 27);
        let old = many(&mut g, Position::Cb, 1000, 34);
        let m = |ps: &[Player], a: Attr| mean(ps.iter().map(|p| p.ratings[a] as f64));
        assert!(m(&old, Attr::Speed) < m(&young, Attr::Speed) - 5.0);
        assert!(m(&old, Attr::Intelligence) > m(&young, Attr::Intelligence));
    }

    #[test]
    fn young_players_have_bigger_potential_gaps() {
        let mut g = PlayerGenerator::built_in(Rng::new(17));
        let mut gap = |age| {
            let ps = many(&mut g, Position::Wr, 1000, age);
            mean(ps.iter().map(|p| mean(Attr::ALL.iter().map(|&a| (p.potentials[a] - p.ratings[a]) as f64))))
        };
        assert!(gap(21) > gap(32) + 4.0);
    }

    #[test]
    fn walk_ons_are_worse_than_ex_college() {
        let mut g = PlayerGenerator::built_in(Rng::new(19));
        let mut avg = |t| mean((0..1500).map(|_| g.generate(Position::Wr, GenOptions::default().tier(t).age(25)).ovr() as f64));
        assert!(avg(Tier::ExCollege) > avg(Tier::WalkOn) + 10.0);
    }

    #[test]
    fn experience_and_acquisition_defaults() {
        let mut g = PlayerGenerator::built_in(Rng::new(8));
        let old = g.generate(Position::Qb, GenOptions::default().age(30));
        let young = g.generate(Position::Wr, GenOptions::default().age(21));
        assert_eq!((old.acquired.as_str(), old.years_pro), ("Walk-On", 8));
        assert!(young.is_rookie() && young.years_label() == "R" && old.years_label() == "8");
        let tagged = g.generate(Position::Rb, GenOptions::default().age(23).years_pro(0).acquired(&crate::player::acquired_draft(2, 11)));
        assert_eq!(tagged.acquired, "S2 Draft #11");
    }

    #[test]
    fn bundled_tables_generate_players_with_job_size_and_name() {
        let mut g = PlayerGenerator::new(Rng::new(1));
        let p = g.generate(Position::Ot, GenOptions::default());
        assert!(p.name.contains(' ') && !p.day_job.is_empty() && p.weight_lb >= 280);
    }
}
