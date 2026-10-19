use crate::Money;
use crate::finance::EconTuning;
use sp_core::geo::town_miles;
use sp_core::{Player, Position, Town};

/// Pay-per-game asks (features/economy.md sec 4). All values are tuning seeds.
/// Asks use current ratings + age only (D11): never hidden potential.
pub struct PayScale;

impl PayScale {
    pub fn ovr_curve(ovr: i32) -> f64 {
        100.0 * (0.034 * (ovr as f64 - 30.0)).exp()
    }

    pub fn position_mod(p: Position) -> f64 {
        use Position::*;
        match p {
            Qb => 1.22,
            K | P => 0.55,
            Fb => 0.8,
            Ot | Og | C => 0.95,
            Rb | Wr | Cb => 1.05,
            _ => 1.0,
        }
    }

    pub fn age_mod(age: i32) -> f64 {
        if age <= 22 {
            0.97
        } else if age >= 33 {
            0.82
        } else if age >= 30 {
            0.92
        } else {
            1.0
        }
    }

    /// Smaller towns pay less (day-job players, smaller market): 0.42 (tiny) .. 1.0 (big), tracking what their gate can afford.
    pub fn market_mod(t: &Town) -> f64 {
        (0.42 + 0.58 * (((t.population as f64).log10() - 4.4) / 1.35)).clamp(0.42, 1.0)
    }

    /// D23: shorter deals cost more per game.
    pub fn length_mod(seasons: i32) -> f64 {
        match seasons {
            1 => 1.25,
            2 => 1.10,
            _ => 1.0,
        }
    }

    /// D28: local players are cheaper, out of town cost more.
    pub fn hometown_mod(miles: f64) -> f64 {
        if miles <= 1.0 {
            0.80
        } else if miles <= 40.0 {
            0.92
        } else if miles <= 120.0 {
            1.0
        } else if miles <= 250.0 {
            1.07
        } else {
            1.15
        }
    }

    /// Per-game commute add-on for out-of-town players (car/bus), capped.
    pub fn commute(miles: f64) -> Money {
        if miles <= 40.0 { 0 } else { 60.min((miles * 0.12).round() as Money) }
    }

    /// Award winners know their worth: +20% per award.
    pub fn award_mod(awards: i32) -> f64 {
        1.0 + EconTuning::AWARD_PREMIUM * awards.min(EconTuning::AWARD_PREMIUM_MAX) as f64
    }

    /// Shown next to the ask, e.g. "3 awards: +60%". `None` when he has won none.
    pub fn award_note(awards: i32) -> Option<String> {
        if awards <= 0 {
            return None;
        }
        let pct = ((Self::award_mod(awards) - 1.0) * 100.0).round() as i32;
        Some(format!("{awards} award{}: +{pct}%", if awards == 1 { "" } else { "s" }))
    }

    pub fn ask(p: &Player, team: &Town, seasons: i32) -> Money {
        let miles = town_miles(&p.hometown, &team.name);
        let v = Self::ovr_curve(p.ovr()) * Self::position_mod(p.position) * Self::age_mod(p.age) * Self::market_mod(team) * Self::length_mod(seasons) * Self::hometown_mod(miles) * Self::award_mod(p.awards);
        20.max(v.round() as Money)
    }

    /// D22: retainer for rostered but inactive players = share of their pay per game.
    pub fn retainer(ppg: Money) -> Money {
        4.max((ppg as f64 * 0.15).round() as Money)
    }
}
