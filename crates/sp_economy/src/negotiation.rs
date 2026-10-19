use crate::Money;
use crate::contract::Contract;
use crate::finance::EconTuning;
use crate::pay_scale::PayScale;
use sp_core::geo::town_miles;
use sp_core::{Player, Town};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OfferOutcome {
    Accept,
    Counter,
    Reject,
}

#[derive(Clone, Debug)]
pub struct OfferResponse {
    pub outcome: OfferOutcome,
    pub counter_ppg: Option<Money>,
    /// Flavor text for the player's reply.
    pub line: String,
}

/// Extra a player of yours asks for when his contract is up: he knows what is in the bank, and a veteran
/// who rode the bench wants to play. Only applies to re-signing your own players.
#[derive(Clone, Copy, Debug, Default)]
pub struct ResignPremium {
    pub cash: Money,
    pub wants_to_play: bool,
}

impl ResignPremium {
    pub fn flush(&self) -> f64 {
        let mut v = 0.0;
        for (threshold, premium) in EconTuning::FLUSH_PREMIUM {
            if self.cash >= threshold && premium > v {
                v = premium;
            }
        }
        v
    }

    pub fn benched(&self) -> f64 {
        if self.wants_to_play { EconTuning::BENCHED_PREMIUM } else { 0.0 }
    }

    pub fn mult(&self) -> f64 {
        (1.0 + self.flush()) * (1.0 + self.benched())
    }

    /// Plain-language reasons shown next to the ask.
    pub fn notes(&self) -> Vec<String> {
        let mut v = Vec::new();
        if self.wants_to_play {
            v.push(format!("He sat on the bench and wants to play: +{}%", (self.benched() * 100.0).round() as i32));
        }
        if self.flush() > 0.0 {
            v.push(format!("He knows the club is flush: +{}%", (self.flush() * 100.0).round() as i32));
        }
        v
    }
}

/// Accept / counter / reject (max 2 rounds). Asks use current ratings and age only (D11).
pub struct Negotiation;

impl Negotiation {
    pub const MAX_ROUNDS: i32 = 2;

    /// Winning teams get a small discount; losing teams pay a premium.
    pub fn mood(team_win_pct: f64) -> f64 {
        1.0 - 0.10 * (team_win_pct - 0.5)
    }

    pub fn target(p: &Player, team: &Town, seasons: i32, team_win_pct: f64, resign: Option<&ResignPremium>) -> Money {
        (PayScale::ask(p, team, seasons) as f64 * Self::mood(team_win_pct) * resign.map(|r| r.mult()).unwrap_or(1.0)).round() as Money
    }

    #[allow(clippy::too_many_arguments)]
    pub fn respond(p: &Player, team: &Town, ppg: Money, seasons: i32, signing_bonus: Money, round: i32, team_win_pct: f64, resign: Option<&ResignPremium>) -> OfferResponse {
        let want = Self::target(p, team, seasons, team_win_pct, resign);
        let value = ppg as f64 + signing_bonus as f64 / (seasons * 14) as f64;
        let accept_at = if round >= Self::MAX_ROUNDS { want as f64 * 0.95 } else { want as f64 };
        let wants_to_play = resign.map(|r| r.wants_to_play).unwrap_or(false);
        if value >= accept_at {
            return OfferResponse { outcome: OfferOutcome::Accept, counter_ppg: None, line: yes(p) };
        }
        if value >= want as f64 * 0.85 {
            let counter = (want as f64 * if round >= Self::MAX_ROUNDS { 0.97 } else { 1.02 }).round() as Money;
            if wants_to_play {
                return OfferResponse { outcome: OfferOutcome::Counter, counter_ppg: Some(counter), line: format!("I want to play, coach. I sat too much last year. ${counter} a game and I'm in.") };
            }
            return OfferResponse { outcome: OfferOutcome::Counter, counter_ppg: Some(counter), line: counter_line(p, counter) };
        }
        if wants_to_play {
            return OfferResponse { outcome: OfferOutcome::Reject, counter_ppg: None, line: "I didn't take a day off work to hold a clipboard. I want to play, and that's not the number.".into() };
        }
        OfferResponse { outcome: OfferOutcome::Reject, counter_ppg: None, line: no(p) }
    }

    /// Builds the contract for an accepted deal.
    pub fn contract_for(p: &Player, team: &Town, ppg: Money, seasons: i32) -> Contract {
        Contract { player_id: p.id, ppg, seasons_left: seasons, win_bonus: (ppg as f64 * 0.1).round() as Money, commute: PayScale::commute(town_miles(&p.hometown, &team.name)) }
    }
}

// Player replies. Each pool has four variants, picked by the player's id so the same player always sounds the same.
fn yes(p: &Player) -> String {
    [
        "Deal. I'll tell the boss I need Saturdays off.",
        "You've got yourself a ball player.",
        "Sign me up. My truck is already in the lot.",
        "Done. I'm going to tell everyone at the shop I'm a professional now.",
    ][(p.id % 4) as usize]
        .to_string()
}

fn no(p: &Player) -> String {
    [
        "Come on. I make more fixing air conditioners.",
        "That wouldn't cover my gas.",
        "My cousin offered me double to play beer-league softball.",
        "I'd have to take a pay cut to play football. Even my wife laughed.",
    ][(p.id % 4) as usize]
        .to_string()
}

fn counter_line(p: &Player, ppg: Money) -> String {
    let job = p.day_job.to_lowercase();
    [
        format!("I'd do it for ${ppg} a game. My {job} schedule is flexible."),
        format!("Make it ${ppg} a game and we've got a deal. The {job} job can live without me on Saturdays."),
        format!("${ppg} a game and I'll bring my own tape. Yes, I know it's a lot of tape."),
        format!("Meet me at ${ppg} a game. My {job} boss already said he'd cover for me, kind of."),
    ][(p.id % 4) as usize]
        .clone()
}
