use crate::enums::{Attr, Attr::*, Position, Position::*};
use crate::ratings::Ratings;

/// The four attributes that define each position. OVR is their plain average.
pub fn ovr_key_attrs(pos: Position) -> [Attr; 4] {
    match pos {
        Qb => [Agility, Strength, Intelligence, Discipline],
        Rb => [Speed, Acceleration, Agility, Hands],
        Fb => [Strength, Hands, Endurance, Intelligence],
        Wr => [Speed, Acceleration, Agility, Hands],
        Te => [Agility, Strength, Hands, Intelligence],
        Ot | Og | C => [Agility, Strength, Intelligence, Discipline],
        De | Dt => [Acceleration, Agility, Strength, Intelligence],
        Olb => [Speed, Agility, Strength, Intelligence],
        Mlb => [Speed, Strength, Endurance, Intelligence],
        Cb => [Speed, Acceleration, Agility, Hands],
        S => [Speed, Agility, Hands, Intelligence],
        K | P => [Strength, Endurance, Intelligence, Discipline],
    }
}

/// Per-attribute weights derived from [`ovr_key_attrs`] (0.25 each). Order: SP AC AG ST HA EN IN DI. Each sums to 1.0.
pub fn ovr_weights(pos: Position) -> [f64; 8] {
    let keys = ovr_key_attrs(pos);
    let mut w = [0.0; 8];
    for a in Attr::ALL {
        if keys.contains(&a) {
            w[a.index()] = 0.25;
        }
    }
    w
}

/// Overall (1-99): the average of the position's four key attributes.
pub fn overall(r: &Ratings, pos: Position) -> i32 {
    let sum: f64 = ovr_key_attrs(pos).iter().map(|&a| 0.25 * r[a] as f64).sum();
    (sum.round() as i32).clamp(1, 99)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weights_sum_to_one() {
        for p in Position::ALL {
            let w = ovr_weights(p);
            assert!((w.iter().sum::<f64>() - 1.0).abs() < 1e-9, "{}", p.label());
        }
    }
}
