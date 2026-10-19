use crate::inbox::{InboxFeed, InboxKind, PostOpts};
use crate::league::League;
use crate::text::{commas, fill, surprise_bills, welfare_bills};
use sp_economy::{LedgerEntry, LedgerKind};

/// Tuning for the surprise bill that can land on a club sitting on a pile of cash. All values are tuning seeds.
pub struct SurpriseBillTuning;

impl SurpriseBillTuning {
    /// Cash on hand when the week is booked -> chance a bill shows up. Highest threshold reached wins.
    pub const CHANCES: [(i64, f64); 4] = [(100_000, 0.15), (200_000, 0.25), (300_000, 0.35), (400_000, 0.45)];

    /// Regular-season week when bills can first land (skip the opener; this is a midseason event).
    pub const FIRST_WEEK: i32 = 5;

    /// The bill is this share of the cash on hand, rolled between the two.
    pub const MIN_SHARE: f64 = 0.20;
    pub const MAX_SHARE: f64 = 0.50;

    /// Rounded to the nearest...
    pub const ROUND_TO: i64 = 100;

    /// Share of bills that are a league "relief payment": the money goes to the poorest club in the league instead of away.
    pub const WELFARE_CHANCE: f64 = 0.15;
}

impl League {
    /// Chance of a bill given the cash on hand (0 when below the lowest threshold).
    pub fn surprise_bill_chance(cash: i64) -> f64 {
        let mut p = 0.0;
        for (threshold, chance) in SurpriseBillTuning::CHANCES {
            if cash >= threshold {
                p = chance;
            }
        }
        p
    }

    /// After a regular-season week: one attempt per season, once cash is over the line and the week is far enough in.
    /// Returns the amount (0 = none, too early, already tried, or the dice missed).
    pub fn maybe_roll_surprise_bill(&mut self, played_week: i32) -> i64 {
        if played_week < SurpriseBillTuning::FIRST_WEEK {
            return 0;
        }
        self.roll_surprise_bill(Some(played_week))
    }

    /// A club with cash to spare may get a big unexpected bill. Returns the amount (0 = none).
    /// Marks the season as attempted when cash is high enough, hit or miss, so the weekly hook only rolls once.
    pub fn roll_surprise_bill(&mut self, at_week: Option<i32>) -> i64 {
        let Some(u) = self.user_team else { return 0 };
        if self.surprise_bill_attempted {
            return 0;
        }
        let cash = self.finances[u].cash;
        let p = Self::surprise_bill_chance(cash);
        if p <= 0.0 {
            return 0;
        }
        self.surprise_bill_attempted = true;
        let week_no = at_week.unwrap_or(self.week);
        let mut r = self.rng.fork(self.season as u64 * 6217 + week_no as u64 * 91 + 53);
        if !r.chance(p) {
            return 0;
        }
        let share = SurpriseBillTuning::MIN_SHARE + r.next_f64() * (SurpriseBillTuning::MAX_SHARE - SurpriseBillTuning::MIN_SHARE);
        let round_to = SurpriseBillTuning::ROUND_TO;
        let amount = ((cash as f64 * share) / round_to as f64).round() as i64 * round_to;
        if amount <= 0 {
            return 0;
        }
        // Sometimes the bill is the league making you share with the poorest club (never a bankrupt one: it is past saving).
        let mut poorest: Option<usize> = None;
        for i in 0..self.teams.len() {
            if i == u || self.finances[i].bankrupt {
                continue;
            }
            if poorest.map(|p| self.finances[i].cash < self.finances[p].cash).unwrap_or(true) {
                poorest = Some(i);
            }
        }
        let welfare = poorest.is_some() && r.chance(SurpriseBillTuning::WELFARE_CHANCE);
        let bill = if welfare { r.pick(welfare_bills()).clone() } else { r.pick(surprise_bills()).clone() };
        let poor_name = if welfare { self.teams[poorest.unwrap()].town.team_name() } else { String::new() };
        let season = self.season;
        if welfare {
            let pi = poorest.unwrap();
            self.finances[pi].cash += amount;
            self.finances[pi].ledger.push(LedgerEntry::new(season, week_no, LedgerKind::Other, amount, "league relief payment"));
        }
        let f = &mut self.finances[u];
        f.cash -= amount;
        let memo = if welfare { format!("league relief payment to {poor_name}") } else { bill.title.to_lowercase() };
        f.ledger.push(LedgerEntry::new(season, week_no, LedgerKind::Other, -amount, &memo));
        let me = self.teams[u].town.clone();
        let title = fill(&bill.title, &[("poor", poor_name.clone())]);
        let body = fill(&bill.body, &[("amt", format!("${}", commas(amount))), ("team", me.team_name()), ("town", me.name.clone()), ("poor", poor_name)]);
        self.post_inbox(InboxFeed::Money, InboxKind::Bill, &title, &body, PostOpts::week(week_no));
        amount
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chance_climbs_with_cash() {
        assert_eq!(League::surprise_bill_chance(99_999), 0.0);
        assert_eq!(League::surprise_bill_chance(100_000), 0.15);
        assert_eq!(League::surprise_bill_chance(250_000), 0.25);
        assert_eq!(League::surprise_bill_chance(1_000_000), 0.45);
    }
}
