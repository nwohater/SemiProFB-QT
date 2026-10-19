use crate::enums::Position;
use crate::rng::Rng;
use std::collections::HashMap;

/// Height (inches) and weight (pounds) range for a position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SizeRange {
    pub min_height: i32,
    pub max_height: i32,
    pub min_weight: i32,
    pub max_weight: i32,
}

const fn r(a: i32, b: i32, c: i32, d: i32) -> SizeRange {
    SizeRange { min_height: a, max_height: b, min_weight: c, max_weight: d }
}

const BUILT_IN: [(&str, SizeRange); 15] = [
    ("QB", r(69, 78, 180, 250)),
    ("RB", r(65, 74, 175, 235)),
    ("FB", r(69, 76, 220, 270)),
    ("WR", r(68, 79, 165, 235)),
    ("TE", r(74, 80, 225, 275)),
    ("T", r(75, 81, 285, 365)),
    ("G", r(73, 79, 285, 355)),
    ("C", r(73, 79, 280, 340)),
    ("DT", r(72, 79, 270, 345)),
    ("NT", r(72, 78, 300, 380)),
    ("DE", r(73, 80, 245, 310)),
    ("LB", r(70, 78, 215, 270)),
    ("DB", r(68, 77, 170, 225)),
    ("K", r(67, 76, 165, 215)),
    ("P", r(69, 77, 175, 225)),
];

const SIZES_CSV: &str = include_str!("../../../data/position_sizes.csv");

/// Per-position size ranges from a CSV with a header row: `position,min_height,max_height,min_weight,max_weight`.
/// Codes are QB RB FB WR TE T G C DT NT DE LB DB. Codes missing from the file keep the built-in ranges.
#[derive(Clone, Debug)]
pub struct SizeTable {
    by_code: HashMap<String, SizeRange>,
}

impl SizeTable {
    pub fn new(overrides: HashMap<String, SizeRange>) -> Self {
        let mut by_code: HashMap<String, SizeRange> = BUILT_IN.iter().map(|&(k, v)| (k.to_string(), v)).collect();
        for (k, v) in overrides {
            by_code.insert(k.to_uppercase(), v);
        }
        Self { by_code }
    }

    pub fn built_in() -> Self {
        Self::new(HashMap::new())
    }

    pub fn bundled() -> Self {
        Self::from_csv(SIZES_CSV)
    }

    /// The header row is skipped; blank or malformed lines are ignored.
    pub fn from_csv(csv: &str) -> Self {
        let mut rows = HashMap::new();
        for raw in csv.replace('\r', "").split('\n').skip(1) {
            let parts: Vec<String> = raw.split(',').map(|s| s.replace('"', "").trim().to_string()).collect();
            if parts.len() < 5 || parts[0].is_empty() {
                continue;
            }
            let n: Vec<Option<i32>> = parts[1..5].iter().map(|p| p.parse().ok()).collect();
            if n.iter().any(|v| v.is_none()) {
                continue;
            }
            let n: Vec<i32> = n.into_iter().flatten().collect();
            if n[1] < n[0] || n[3] < n[2] {
                continue;
            }
            rows.insert(parts[0].to_uppercase(), SizeRange { min_height: n[0], max_height: n[1], min_weight: n[2], max_weight: n[3] });
        }
        Self::new(rows)
    }

    pub fn get(&self, code: &str) -> Option<SizeRange> {
        self.by_code.get(&code.to_uppercase()).copied()
    }

    /// The CSV code for a position. DT sometimes draws from the heavier NT row (see [`Self::range_for`]).
    pub fn code_for(p: Position) -> &'static str {
        use Position::*;
        match p {
            Qb => "QB",
            Rb => "RB",
            Fb => "FB",
            Wr => "WR",
            Te => "TE",
            Ot => "T",
            Og => "G",
            C => "C",
            Dt => "DT",
            De => "DE",
            Olb | Mlb => "LB",
            Cb | S => "DB",
            K => "K",
            P => "P",
        }
    }

    /// About a third of defensive tackles are nose tackles, so `as_nose_tackle` picks that row.
    pub fn range_for(&self, p: Position, as_nose_tackle: bool) -> SizeRange {
        let code = if as_nose_tackle && p == Position::Dt { "NT" } else { Self::code_for(p) };
        self.by_code[code]
    }

    /// Draws a size. `weight_z` and `height_z` are standard-normal scores, mapped onto the range so most players
    /// land mid-range and few reach the extremes.
    pub fn draw(&self, p: Position, height_z: f64, weight_z: f64, as_nose_tackle: bool) -> (i32, i32) {
        let rg = self.range_for(p, as_nose_tackle);
        let at = |lo: i32, hi: i32, z: f64| lo as f64 + (hi - lo) as f64 * (0.5 + 0.2 * z).clamp(0.0, 1.0);
        (at(rg.min_height, rg.max_height, height_z).round() as i32, at(rg.min_weight, rg.max_weight, weight_z).round() as i32)
    }

    /// Convenience for tools: draws with its own randomness.
    pub fn random(&self, p: Position, rng: &mut Rng) -> (i32, i32) {
        let w = rng.next_gaussian();
        let hz = 0.5 * w + 0.87 * rng.next_gaussian();
        let nose = p == Position::Dt && rng.chance(0.3);
        self.draw(p, hz, w, nose)
    }
}

/// 6'2" style label.
pub fn height_label(inches: i32) -> String {
    format!("{}'{}\"", inches / 12, inches % 12)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_position_draws_inside_its_range_or_the_nose_tackle_range() {
        let t = SizeTable::bundled();
        let mut rng = Rng::new(1);
        for p in Position::ALL {
            let rg = t.range_for(p, false);
            let nt = if p == Position::Dt { t.range_for(p, true) } else { rg };
            for _ in 0..400 {
                let (h, w) = t.random(p, &mut rng);
                let ok_h = (rg.min_height..=rg.max_height).contains(&h) || (nt.min_height..=nt.max_height).contains(&h);
                let ok_w = (rg.min_weight..=rg.max_weight).contains(&w) || (nt.min_weight..=nt.max_weight).contains(&w);
                assert!(ok_h && ok_w, "{} {h} in {w} lb", p.label());
            }
        }
    }

    #[test]
    fn most_players_sit_mid_range_and_extremes_are_rare() {
        let t = SizeTable::bundled();
        let mut rng = Rng::new(4);
        let r = t.range_for(Position::Qb, false);
        let ws: Vec<i32> = (0..3000).map(|_| t.random(Position::Qb, &mut rng).1).collect();
        let mid = ws.iter().filter(|&&w| w > r.min_weight + 15 && w < r.max_weight - 15).count() as f64 / 3000.0;
        let top = ws.iter().filter(|&&w| w >= r.max_weight - 2).count() as f64 / 3000.0;
        assert!(mid > 0.7, "{mid}");
        assert!(top < 0.08, "{top}");
    }

    #[test]
    fn about_a_third_of_defensive_tackles_are_nose_tackles() {
        let t = SizeTable::bundled();
        let mut rng = Rng::new(5);
        let ws: Vec<i32> = (0..3000).map(|_| t.random(Position::Dt, &mut rng).1).collect();
        let heavy = ws.iter().filter(|&&w| w > t.range_for(Position::Dt, false).max_weight).count();
        assert!(heavy > 100, "{heavy}"); // beyond the DT max: only nose tackles reach it
    }

    #[test]
    fn malformed_rows_are_ignored_and_missing_codes_keep_defaults() {
        let t = SizeTable::from_csv("position,a,b,c,d\nqb,70,76,190,240\nWR,80,70,100,200\nzz,x,1,2,3\n");
        assert_eq!(t.get("QB").unwrap().max_weight, 240);
        assert_eq!(t.get("WR"), SizeTable::built_in().get("WR"));
    }

    #[test]
    fn nose_tackles_are_heavier() {
        let t = SizeTable::built_in();
        assert!(t.range_for(Position::Dt, true).min_weight > t.range_for(Position::Dt, false).min_weight);
    }

    #[test]
    fn height_labels() {
        assert_eq!(height_label(74), "6'2\"");
    }
}
