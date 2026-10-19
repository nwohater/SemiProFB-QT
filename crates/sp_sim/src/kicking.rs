use sp_core::{Attr, Player, Rng, Weather};

/// Wind arrow: positive = tailwind (yards of effect scale), sign relative to the kick direction.
#[derive(Clone, Copy, Debug)]
pub struct KickConditions {
    pub weather: Weather,
    /// -1 (headwind) .. +1 (tailwind).
    pub wind_arrow: f64,
}

impl Default for KickConditions {
    fn default() -> Self {
        Self { weather: Weather::Clear, wind_arrow: 0.0 }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct KickResult {
    pub good: bool,
    pub distance: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct KickoffResult {
    pub spot: i32,
    pub return_yards: i32,
    pub touchback: bool,
    pub td: bool,
}

/// Kicker ST = power (range), DI = accuracy (D-ratings spec).
/// Field goal distance = yards to end zone + 17 (snap + end zone).
pub struct Kicking;

impl Kicking {
    pub fn fg_success_probability(kicker: &Player, distance: i32, c: &KickConditions) -> f64 {
        let power = kicker.ratings[Attr::Strength] as f64;
        let acc = kicker.ratings[Attr::Discipline] as f64;
        let distance = distance as f64;
        // Max comfortable range scales with ST: ST 58 -> ~50 yds, ST 90 -> ~63 yds.
        let range = 20.0 + power * 0.52 + c.wind_arrow * 4.0;
        let range_penalty = if distance > range { (distance - range) * 0.06 } else { 0.0 };
        // Accuracy base: DI 62 -> ~0.92 on a chip shot, falling with distance.
        let base = 0.70 + acc * 0.0036 - (distance - 20.0) * 0.007;
        let mut p = base - range_penalty;
        match c.weather {
            Weather::Rain => p -= 0.04,
            Weather::Snow => p -= 0.08,
            Weather::Wind => p -= 0.05 * c.wind_arrow.abs(),
            Weather::Clear => {}
        }
        p.clamp(0.02, 0.99)
    }

    pub fn field_goal(rng: &mut Rng, kicker: &Player, yards_to_end_zone: i32, c: &KickConditions) -> KickResult {
        let distance = yards_to_end_zone + 17;
        KickResult { good: rng.chance(Self::fg_success_probability(kicker, distance, c)), distance }
    }

    pub fn xp_probability(kicker: &Player, c: &KickConditions) -> f64 {
        Self::fg_success_probability(kicker, 33, c).clamp(0.5, 0.995)
    }

    /// Punt distance from ST (power) with DI (placement) trimming variance.
    pub fn punt_distance(rng: &mut Rng, punter: &Player, c: &KickConditions) -> i32 {
        let power = punter.ratings[Attr::Strength] as f64;
        let acc = punter.ratings[Attr::Discipline] as f64;
        let mut mean = 28.0 + power * 0.28 + c.wind_arrow * 3.0;
        if c.weather == Weather::Snow {
            mean -= 3.0;
        }
        let sd = (9.0 - acc * 0.06).max(3.0);
        (rng.gaussian(mean, sd).round() as i32).clamp(15, 75)
    }

    /// Kickoff: `spot` is the receiving team's own yard line after the return.
    pub fn kickoff(rng: &mut Rng, kicker: &Player, returner: Option<&Player>, c: &KickConditions) -> KickoffResult {
        let power = kicker.ratings[Attr::Strength] as f64;
        let kick_dist = rng.gaussian(50.0 + power * 0.18 + c.wind_arrow * 2.0, 6.0);
        if kick_dist >= 65.0 {
            return KickoffResult { spot: 25, return_yards: 0, touchback: true, td: false };
        }
        let landing = ((65.0 - kick_dist).round() as i32).clamp(1, 40);
        let speed = returner.map(|r| r.ratings[Attr::Speed] as f64).unwrap_or(50.0);
        let ret_yards = match returner {
            None => 15,
            Some(r) => rng.gaussian(18.0 + (speed - 50.0) * 0.18 + (r.ratings[Attr::Agility] as f64 - 50.0) * 0.08, 8.0).round() as i32,
        };
        let td = rng.chance(0.006 + 0.0002 * (speed - 55.0).clamp(-20.0, 40.0));
        let spot = (landing + ret_yards).clamp(5, 60);
        KickoffResult { spot, return_yards: if td { 100 - landing } else { spot - landing }, touchback: false, td }
    }

    pub fn kickoff_return_spot(rng: &mut Rng, kicker: &Player, returner: Option<&Player>, c: &KickConditions) -> i32 {
        let r = Self::kickoff(rng, kicker, returner, c);
        if r.td { 60 } else { r.spot }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sp_core::generator::GenOptions;
    use sp_core::{Position, PlayerGenerator, Ratings};

    fn kicker(g: &mut PlayerGenerator, st: i32, di: i32) -> Player {
        let mut p = g.generate(Position::K, GenOptions::default().age(27));
        let mut v = p.ratings.to_array();
        let mut pots = p.potentials.to_array();
        v[Attr::Strength.index()] = st;
        v[Attr::Discipline.index()] = di;
        pots[Attr::Strength.index()] = 99;
        pots[Attr::Discipline.index()] = 99;
        p.ratings = Ratings::new(v);
        p.potentials = Ratings::new(pots);
        p
    }

    fn setup() -> (PlayerGenerator, KickConditions) {
        (PlayerGenerator::built_in(Rng::new(1)), KickConditions::default())
    }

    #[test]
    fn fg_probability_falls_with_distance_and_rises_with_di() {
        let (mut g, clear) = setup();
        let k = kicker(&mut g, 60, 60);
        assert!(Kicking::fg_success_probability(&k, 25, &clear) > Kicking::fg_success_probability(&k, 50, &clear));
        let better = kicker(&mut g, 60, 85);
        assert!(Kicking::fg_success_probability(&better, 40, &clear) > Kicking::fg_success_probability(&k, 40, &clear));
    }

    #[test]
    fn long_range_needs_strength() {
        let (mut g, clear) = setup();
        let weak = kicker(&mut g, 35, 70);
        let strong = kicker(&mut g, 85, 70);
        assert!(Kicking::fg_success_probability(&strong, 55, &clear) > Kicking::fg_success_probability(&weak, 55, &clear) + 0.2);
    }

    #[test]
    fn rain_and_snow_lower_accuracy() {
        let (mut g, clear) = setup();
        let k = kicker(&mut g, 60, 60);
        let at = |w| Kicking::fg_success_probability(&k, 35, &KickConditions { weather: w, wind_arrow: 0.0 });
        assert!(Kicking::fg_success_probability(&k, 35, &clear) > at(Weather::Rain));
        assert!(at(Weather::Rain) > at(Weather::Snow));
    }

    #[test]
    fn average_kicker_matches_targets() {
        let (mut g, clear) = setup();
        let k = kicker(&mut g, 58, 62);
        let short = Kicking::fg_success_probability(&k, 35, &clear);
        let far = Kicking::fg_success_probability(&k, 52, &clear);
        assert!((0.78..=0.95).contains(&short), "{short}");
        assert!((0.30..=0.65).contains(&far), "{far}");
    }

    #[test]
    fn punt_distance_reflects_strength_and_is_deterministic() {
        let (mut g, clear) = setup();
        let weak = kicker(&mut g, 40, 60);
        let strong = kicker(&mut g, 90, 60);
        let avg = |p: &Player| {
            let mut r = Rng::new(5);
            (0..500).map(|_| Kicking::punt_distance(&mut r, p, &clear) as f64).sum::<f64>() / 500.0
        };
        assert!(avg(&strong) > avg(&weak) + 8.0);
        assert_eq!(Kicking::punt_distance(&mut Rng::new(3), &strong, &clear), Kicking::punt_distance(&mut Rng::new(3), &strong, &clear));
    }

    #[test]
    fn kickoff_return_spot_stays_on_the_field() {
        let (mut g, clear) = setup();
        let k = kicker(&mut g, 60, 60);
        let mut r = Rng::new(21);
        for _ in 0..500 {
            assert!((5..=60).contains(&Kicking::kickoff_return_spot(&mut r, &k, None, &clear)));
        }
    }
}
