use crate::Money;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LedgerKind {
    Gate,
    Concessions,
    Merch,
    Sponsors,
    LeagueShare,
    PlayoffBonus,
    PlayerPay,
    Retainers,
    Rent,
    Travel,
    Insurance,
    Medical,
    Other,
    Promo,
    Upkeep,
    Interest,
    LoanPayment,
    LoanProceeds,
    Facilities,
    SponsorCost,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub season: i32,
    pub week: i32,
    pub kind: LedgerKind,
    /// + revenue, - expense.
    pub amount: Money,
    pub memo: String,
}

impl LedgerEntry {
    pub fn new(season: i32, week: i32, kind: LedgerKind, amount: Money, memo: &str) -> Self {
        Self { season, week, kind, amount, memo: memo.to_string() }
    }
}

/// One team's money for one game.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameFinance {
    pub week: i32,
    pub home: bool,
    pub attendance: i64,
    pub capacity: i64,
    pub revenue: BTreeMap<LedgerKind, Money>,
    pub expenses: BTreeMap<LedgerKind, Money>,
    pub cash_after: Money,
    /// Home promo that ran this week, if any, and whether its risk fired.
    pub promo_id: Option<String>,
    pub promo_mishap: bool,
}

impl GameFinance {
    pub fn total_revenue(&self) -> Money {
        self.revenue.values().sum()
    }

    pub fn total_expenses(&self) -> Money {
        self.expenses.values().sum()
    }

    pub fn net(&self) -> Money {
        self.total_revenue() - self.total_expenses()
    }
}
