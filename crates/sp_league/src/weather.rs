use sp_core::{Region, Rng, Town, Weather};

#[derive(Clone, Copy, Debug)]
pub struct GameWeather {
    pub weather: Weather,
    pub wind_arrow: f64,
}

/// Simple weather by home town (D19): clear most of the time, rain/wind sometimes, snow ~never.
pub struct WeatherModel;

impl WeatherModel {
    pub fn pick(rng: &mut Rng, home: &Town, week: i32, region: &Region) -> GameWeather {
        let coastal = home.division == region.coastal;
        let late = week >= 9; // later weeks are wetter
        let snow = if region.cold {
            if late { 10.0 } else { 3.0 }
        } else if (home.id == "chico" || home.id == "amarillo") && late {
            1.0
        } else {
            0.0
        };
        let w = rng.weighted(&[
            (Weather::Clear, (if coastal { 62.0 } else { 70.0 }) - if region.cold { 10.0 } else { 0.0 }),
            (Weather::Wind, if coastal { 14.0 } else { 10.0 }),
            (Weather::Rain, (if coastal { 18.0 } else { 15.0 }) + if late { 6.0 } else { 0.0 }),
            (Weather::Snow, snow),
        ]);
        GameWeather { weather: w, wind_arrow: rng.gaussian(0.0, 0.5).clamp(-1.0, 1.0) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snow_only_where_it_snows() {
        let ca = Region::california();
        let pa = Region::by_id(Some("pa"));
        let mut rng = Rng::new(1);
        let (mut ca_snow, mut pa_snow) = (0, 0);
        for i in 0..4000 {
            if WeatherModel::pick(&mut rng, &ca.starting[i % 12], 12, ca).weather == Weather::Snow && ca.starting[i % 12].id != "chico" {
                ca_snow += 1;
            }
            if WeatherModel::pick(&mut rng, &pa.starting[i % 12], 12, pa).weather == Weather::Snow {
                pa_snow += 1;
            }
        }
        assert_eq!(ca_snow, 0);
        assert!(pa_snow > 100, "{pa_snow}");
    }

    #[test]
    fn wind_arrow_stays_in_range() {
        let ca = Region::california();
        let mut rng = Rng::new(2);
        for _ in 0..1000 {
            let w = WeatherModel::pick(&mut rng, &ca.starting[0], 3, ca);
            assert!((-1.0..=1.0).contains(&w.wind_arrow));
        }
    }
}
