use crate::inbox::{InboxFeed, InboxKind, PostOpts};
use crate::league::League;
use crate::text::{fill, inbox_text, money};
use sp_core::Rng;
use sp_economy::{CouncilGoals, GoalResult, LedgerEntry, LedgerKind, SeasonOutcome, SeasonReport};
use std::collections::BTreeMap;

impl League {
    /// How the season looks right now (or how it ended), for judging goals.
    pub fn council_outcome(&self) -> SeasonOutcome {
        let u = self.user();
        let f = &self.finances[u];
        let rec = &self.standings.records[u];
        let home: Vec<i64> = f.games.iter().filter(|g| g.home).map(|g| g.attendance).collect();
        SeasonOutcome {
            wins: rec.wins,
            games_played: rec.games(),
            made_playoffs: self.playoffs.as_ref().map(|p| p.seeds.contains(&u)).unwrap_or(false),
            champion: self.champion() == Some(u),
            net: f.season_net(),
            avg_crowd: if home.is_empty() { 0 } else { (home.iter().sum::<i64>() as f64 / home.len() as f64).round() as i64 },
            fan_interest: f.fan_interest,
        }
    }

    /// First-season goals, or goals that follow last season's results.
    pub fn assign_council_goals(&mut self, rng: &mut Rng, last_win_pct: Option<f64>) {
        let Some(u) = self.user_team else { return };
        let town = self.teams[u].town.clone();
        let f = &self.finances[u];
        let last = f.reports.last();
        let goals = CouncilGoals::generate(
            &town,
            rng,
            if last.is_none() { None } else { last_win_pct },
            last.map(|l| l.finish != "Missed the playoffs").unwrap_or(false),
            last.map(|l| l.avg_crowd).unwrap_or(0),
            f.capacity(),
        );
        self.finances[u].goals = goals;
    }

    /// Judge the finished season, pay the grant, and file the report. Call before the books roll over.
    pub fn file_council_report(&mut self) {
        let Some(u) = self.user_team else { return };
        let o = self.council_outcome();
        let results: Vec<GoalResult> = self.finances[u].goals.iter().map(|g| GoalResult { goal: *g, met: g.is_met(&o) }).collect();
        let grant = CouncilGoals::grant(&results);

        let mut rev: BTreeMap<LedgerKind, i64> = BTreeMap::new();
        let mut exp: BTreeMap<LedgerKind, i64> = BTreeMap::new();
        for g in &self.finances[u].games {
            for (k, v) in &g.revenue {
                *rev.entry(*k).or_insert(0) += v;
            }
            for (k, v) in &g.expenses {
                *exp.entry(*k).or_insert(0) += v;
            }
        }
        let others: Vec<&sp_economy::TeamFinance> = (0..self.finances.len()).filter(|&i| i != u && !self.finances[i].games.is_empty()).map(|i| &self.finances[i]).collect();
        let lg_rev = if others.is_empty() { 0 } else { (others.iter().map(|x| x.games.iter().map(|g| g.total_revenue()).sum::<i64>()).sum::<i64>() as f64 / others.len() as f64).round() as i64 };
        let lg_net = if others.is_empty() { 0 } else { (others.iter().map(|x| x.season_net()).sum::<i64>() as f64 / others.len() as f64).round() as i64 };

        let season = self.season;
        let record = self.standings.records[u].to_string();
        let f = &mut self.finances[u];
        if grant > 0 {
            f.cash += grant;
            f.ledger.push(LedgerEntry::new(season, 0, LedgerKind::Other, grant, "Town Council grant"));
        }
        f.reports.push(SeasonReport {
            season,
            record,
            finish: if o.champion { "Champions" } else if o.made_playoffs { "Playoffs" } else { "Missed the playoffs" }.to_string(),
            revenue: rev,
            expenses: exp,
            avg_crowd: o.avg_crowd,
            capacity: f.capacity(),
            fan_interest: o.fan_interest,
            cash_end: f.cash,
            league_avg_revenue: lg_rev,
            league_avg_net: lg_net,
            goals: results.clone(),
            grant,
        });

        let met = results.iter().filter(|r| r.met).count();
        let verdict = if results.is_empty() {
            "The Council had no goals on file, which Carl says is a first.".to_string()
        } else if met == results.len() {
            format!("All {} goals met. The Council is delighted and has found money in a drawer: a grant of {}.", results.len(), money(grant))
        } else if met == 0 {
            "No goals met. The Council would like a word, and a smaller budget for the paint.".to_string()
        } else {
            format!("{met} of {} goals met. Grant: {}.", results.len(), money(grant))
        };
        let pool = if results.is_empty() {
            "councilReviewEmpty"
        } else if met == results.len() {
            "councilReviewAll"
        } else if met == 0 {
            "councilReviewNone"
        } else {
            "councilReviewPartial"
        };
        let mut r = Rng::new(season as u64 * 7919 + 41);
        let text = fill(inbox_text().pick(pool, &mut r), &[("verdict", verdict)]);
        self.post_inbox(InboxFeed::Money, InboxKind::Council, &format!("Town Council: Season {season} review"), &text, PostOpts::week(0));
    }

    /// Tells the user what the Council wants this season.
    pub fn write_council_goals_letter(&mut self) {
        let Some(u) = self.user_team else { return };
        if self.finances[u].goals.is_empty() {
            return;
        }
        let list = self.finances[u].goals.iter().map(|g| format!("{} ({})", g.label(), money(g.reward))).collect::<Vec<_>>().join("; ");
        let season = self.season;
        let mut r = Rng::new(season as u64 * 7919 + 43);
        let text = fill(inbox_text().pick("councilGoals", &mut r), &[("list", list)]);
        self.post_inbox(InboxFeed::Money, InboxKind::Council, &format!("Town Council: Season {season} goals"), &text, PostOpts::week(0));
    }
}
