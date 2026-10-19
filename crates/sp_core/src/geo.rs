use crate::region::town_coords;

/// True when we have coordinates for the town (case-insensitive).
pub fn known_town(name: &str) -> bool {
    town_coords(&name.to_lowercase()).is_some()
}

/// Approximate road miles between two towns (great-circle x 1.25). Unknown towns count as far out of region.
pub fn town_miles(a: &str, b: &str) -> f64 {
    if a.eq_ignore_ascii_case(b) {
        return 0.0;
    }
    let (Some(pa), Some(pb)) = (town_coords(&a.to_lowercase()), town_coords(&b.to_lowercase())) else {
        return 400.0;
    };
    const R: f64 = 3958.8;
    let rad = |d: f64| d.to_radians();
    let d_lat = rad(pb.0 - pa.0);
    let d_lon = rad(pb.1 - pa.1);
    let h = (d_lat / 2.0).sin().powi(2) + rad(pa.0).cos() * rad(pb.0).cos() * (d_lon / 2.0).sin().powi(2);
    2.0 * R * h.sqrt().asin() * 1.25
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distances_are_sane() {
        assert_eq!(town_miles("Fresno", "fresno"), 0.0);
        let d = town_miles("Fresno", "Bakersfield");
        assert!((90.0..160.0).contains(&d), "{d}");
        assert_eq!(town_miles("Fresno", "Atlantis"), 400.0);
        assert_eq!(town_miles("Bakersfield", "Fresno"), d);
    }
}
