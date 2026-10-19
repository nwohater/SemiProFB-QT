use crate::enums::Position;
use std::collections::HashSet;

/// Jersey number bands (inclusive). Tuning lives here, not in the assignment code.
pub const JERSEY_BANDS: [(i32, i32); 5] = [(1, 19), (20, 49), (50, 79), (80, 89), (90, 99)];

/// Which bands each position may wear, as indexes into [`JERSEY_BANDS`].
pub fn jersey_bands_for(pos: Position) -> &'static [usize] {
    use Position::*;
    match pos {
        Qb | K | P => &[0],
        Wr => &[0, 3],
        Cb | S => &[0, 1],
        Olb | Mlb => &[0, 1, 2, 4],
        Rb | Fb => &[1],
        Te => &[1, 3],
        Ot | Og | C => &[2],
        De | Dt => &[2, 4],
    }
}

/// The number a player should take: his id picks the band and the starting spot, then it walks forward to the
/// first free number, trying his other bands, and finally any free number. Deterministic, so no RNG is needed.
pub fn pick_jersey_number(pos: Position, player_id: u32, taken: &HashSet<i32>) -> i32 {
    let bands = jersey_bands_for(pos);
    let h = (player_id as u64).wrapping_mul(2_654_435_761) & 0x7fff_ffff;
    let first = (h % bands.len() as u64) as usize;
    for b in 0..bands.len() {
        let (lo, hi) = JERSEY_BANDS[bands[(first + b) % bands.len()]];
        let size = (hi - lo + 1) as u64;
        let start = (h / 7) % size;
        for i in 0..size {
            let n = lo + ((start + i) % size) as i32;
            if !taken.contains(&n) {
                return n;
            }
        }
    }
    (1..=99).find(|n| !taken.contains(n)).unwrap_or(0) // more than 99 players: not reachable with real roster sizes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_respect_position_bands() {
        for p in Position::ALL {
            let n = pick_jersey_number(p, 12345, &HashSet::new());
            let ok = jersey_bands_for(p).iter().any(|&b| (JERSEY_BANDS[b].0..=JERSEY_BANDS[b].1).contains(&n));
            assert!(ok, "{} got {n}", p.label());
        }
    }

    #[test]
    fn deterministic_and_skips_taken() {
        let a = pick_jersey_number(Position::Qb, 7, &HashSet::new());
        assert_eq!(a, pick_jersey_number(Position::Qb, 7, &HashSet::new()));
        let taken: HashSet<i32> = [a].into();
        assert_ne!(a, pick_jersey_number(Position::Qb, 7, &taken));
    }

    #[test]
    fn falls_back_to_any_free_number_when_bands_are_full() {
        let taken: HashSet<i32> = (1..=19).collect(); // all QB numbers gone
        let n = pick_jersey_number(Position::Qb, 3, &taken);
        assert!((20..=99).contains(&n));
    }
}
