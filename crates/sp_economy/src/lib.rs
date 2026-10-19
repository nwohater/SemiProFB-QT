//! Money: contracts, pay scale, gate, sponsors, loans, council goals. Ported from the Dart `sp_economy`.

pub mod business;
pub mod contract;
pub mod council;
pub mod finance;
pub mod ledger;
pub mod negotiation;
pub mod pay_scale;
pub mod rookie_scale;

pub use business::{Facility, PromoTemplate, SponsorClause, SponsorMarket, SponsorOffer};
pub use contract::Contract;
pub use council::{CouncilGoal, CouncilGoals, GoalKind, GoalResult, SeasonOutcome, SeasonReport};
pub use finance::{CashStatus, EconTuning, EconomyEngine, GameInput, SponsorDeal, TeamFinance, TownEconomy};
pub use ledger::{GameFinance, LedgerEntry, LedgerKind};
pub use negotiation::{Negotiation, OfferOutcome, OfferResponse, ResignPremium};
pub use pay_scale::PayScale;

/// Whole dollars.
pub type Money = i64;
