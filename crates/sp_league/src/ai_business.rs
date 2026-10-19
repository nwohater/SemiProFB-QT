use crate::league::League;
use sp_core::Rng;
use sp_economy::business::{PromoTemplate, promo_templates};
use sp_economy::{Facility, LedgerEntry, LedgerKind, TeamFinance, TownEconomy};

/// How AI owners treat their money. Tuning values live here, not in the offseason code.
pub struct AiBusinessTuning;

impl AiBusinessTuning {
    /// Cash an AI owner keeps untouched, as a share of one season of revenue.
    pub const RESERVE_SEASONS: f64 = 1.0;
    /// Spare cash above the reserve is spread over this many games of extra payroll budget (about three seasons).
    pub const SURPLUS_SPEND_OVER: f64 = 42.0;
    /// Cash below this share of the reserve puts the team in austerity: smaller budget, thin bench, no spending.
    pub const AUSTERITY_BELOW: f64 = 0.3;
    pub const AUSTERITY_BUDGET_MULT: f64 = 0.8;
    /// Ticket price: an AI owner reprices each offseason from how full the stands were. Sellouts raise it, empty seats cut it.
    pub const PRICE_STEP: f64 = 0.25;
    /// Chance an owner actually acts on it in a given offseason (some are slow to notice).
    pub const REPRICE_CHANCE: f64 = 0.35;
    pub const SELL_OUT_FILL: f64 = 0.97;
    pub const THIN_FILL: f64 = 0.75;
    pub const EMPTY_FILL: f64 = 0.60;
    /// Price stays within this band of the town's reference price.
    pub const PRICE_MIN_OF_REF: f64 = 0.6;
    pub const PRICE_MAX_OF_REF: f64 = 1.8;
    /// Facility upgrades bought per offseason at most, and how likely each facility is to be picked.
    pub const MAX_UPGRADES_PER_OFFSEASON: usize = 2;
    pub const FACILITY_WEIGHTS: [(Facility, f64); 4] = [(Facility::Seating, 3.0), (Facility::Concessions, 2.0), (Facility::Scoreboard, 2.0), (Facility::Lights, 1.0)];
    /// An upgrade must leave the reserve intact and cost no more than this share of spare cash.
    pub const UPGRADE_MAX_SHARE_OF_SPARE: f64 = 0.6;
    /// Owners take profits out too: each offseason this share of whatever is still spare (after upgrades) leaves the books
    /// as an owner distribution, so cash cannot pile up forever in big markets.
    pub const DIVIDEND_SHARE: f64 = 0.4;
    /// Share of home games an AI owner with spare cash runs a promo night for.
    pub const PROMO_SHARE: f64 = 0.4;
    /// A promo is only booked when its cost is below this share of spare cash.
    pub const PROMO_MAX_SHARE_OF_SPARE: f64 = 0.05;
}

/// Cash an AI owner wants to keep.
pub fn ai_reserve(rev_per_game: f64) -> f64 {
    rev_per_game * 14.0 * AiBusinessTuning::RESERVE_SEASONS
}

pub fn ai_in_austerity(rev_per_game: f64, cash: i64) -> bool {
    (cash as f64) < ai_reserve(rev_per_game) * AiBusinessTuning::AUSTERITY_BELOW
}

/// Per-game payroll budget for an AI team: a share of revenue, plus spare cash spread over a few seasons,
/// minus a haircut when the cash is thin.
pub fn ai_payroll_budget(rev_per_game: f64, cash: i64) -> f64 {
    let mut b = rev_per_game * 0.75;
    let spare = cash as f64 - ai_reserve(rev_per_game);
    if spare > 0.0 {
        b += spare / AiBusinessTuning::SURPLUS_SPEND_OVER;
    }
    if ai_in_austerity(rev_per_game, cash) {
        b *= AiBusinessTuning::AUSTERITY_BUDGET_MULT;
    }
    b
}

/// End of season, before the books roll: reprice tickets from last season's average crowd. Returns the new price.
pub fn ai_adjust_ticket_price(fin: &mut TeamFinance, r: &mut Rng) -> f64 {
    if !r.chance(AiBusinessTuning::REPRICE_CHANCE) {
        return fin.ticket_price;
    }
    let home: Vec<i64> = fin.games.iter().filter(|g| g.home).map(|g| g.attendance).collect();
    if home.is_empty() {
        return fin.ticket_price;
    }
    let fill = home.iter().sum::<i64>() as f64 / home.len() as f64 / fin.capacity() as f64;
    let t = AiBusinessTuning::PRICE_STEP;
    let mut delta = 0.0;
    if fill >= AiBusinessTuning::SELL_OUT_FILL {
        delta = t;
    } else if fill < AiBusinessTuning::EMPTY_FILL {
        delta = -2.0 * t;
    } else if fill < AiBusinessTuning::THIN_FILL {
        delta = -t;
    }
    let reference = TownEconomy::ref_price(&fin.town);
    let price = (fin.ticket_price + delta).clamp((reference * AiBusinessTuning::PRICE_MIN_OF_REF).max(1.0), (reference * AiBusinessTuning::PRICE_MAX_OF_REF).min(25.0));
    fin.ticket_price = (price * 4.0).round() / 4.0;
    fin.ticket_price
}

impl League {
    /// New season: an AI owner with spare cash fixes up the stadium and books promo nights.
    /// Call after the schedule for the new season exists.
    pub fn ai_spend_surplus(&mut self, i: usize, rev_per_game: f64, r: &mut Rng) {
        if self.finances[i].bankrupt {
            return;
        }
        let reserve = ai_reserve(rev_per_game);
        let mut spare = self.finances[i].cash as f64 - reserve;
        if spare <= 0.0 {
            return;
        }
        let season = self.season;
        let mut bought = 0;
        while bought < AiBusinessTuning::MAX_UPGRADES_PER_OFFSEASON {
            let fin = &self.finances[i];
            let options: Vec<(Facility, f64)> = AiBusinessTuning::FACILITY_WEIGHTS
                .iter()
                .filter(|(f, _)| (fin.upgrade_cost(*f).unwrap_or(1 << 40) as f64) <= spare * AiBusinessTuning::UPGRADE_MAX_SHARE_OF_SPARE)
                .copied()
                .collect();
            if options.is_empty() {
                break;
            }
            let f = r.weighted(&options);
            let cost = self.finances[i].upgrade_cost(f).expect("option was affordable");
            if !self.finances[i].upgrade(f, 0, season) {
                break;
            }
            spare -= cost as f64;
            bought += 1;
        }

        if spare > 0.0 {
            let pay = (spare * AiBusinessTuning::DIVIDEND_SHARE).round() as i64;
            let fin = &mut self.finances[i];
            fin.cash -= pay;
            fin.ledger.push(LedgerEntry::new(season, 0, LedgerKind::Other, -pay, "owner distribution"));
            spare -= pay as f64;
        }

        let town = self.finances[i].town.clone();
        let promos: Vec<&'static PromoTemplate> = promo_templates().iter().filter(|p| (p.cost(&town) as f64) <= spare * AiBusinessTuning::PROMO_MAX_SHARE_OF_SPARE).collect();
        if promos.is_empty() {
            return;
        }
        let home_weeks: Vec<i32> = self.schedule.iter().filter(|g| g.home == i).map(|g| g.week).collect();
        for week in home_weeks {
            if !r.chance(AiBusinessTuning::PROMO_SHARE) {
                continue;
            }
            // Prefer the safer ones; the risky ones are still on the menu.
            let weights: Vec<(usize, f64)> = promos.iter().enumerate().map(|(k, t)| (k, (1.0 - t.risk_chance * 2.0).max(0.2))).collect();
            let p = promos[r.weighted(&weights)];
            self.finances[i].promos.insert(week, p.id.clone());
        }
    }
}
