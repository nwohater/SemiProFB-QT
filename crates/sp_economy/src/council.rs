use crate::Money;
use crate::ledger::LedgerKind;
use serde::{Deserialize, Serialize};
use sp_core::{Rng, Town};
use std::collections::BTreeMap;

/// What the Town Council can ask of you in a season.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GoalKind {
    Wins,
    Playoffs,
    Title,
    BreakEven,
    AvgCrowd,
    FanInterest,
}

/// Tuning for each goal: base grant (scaled by stadium size) and the lane it sits in (field | books | crowd).
pub struct GoalSpec {
    pub base_reward: Money,
    pub lane: &'static str,
}

pub fn goal_spec(k: GoalKind) -> GoalSpec {
    match k {
        GoalKind::Wins => GoalSpec { base_reward: 1500, lane: "field" },
        GoalKind::Playoffs => GoalSpec { base_reward: 3000, lane: "field" },
        GoalKind::Title => GoalSpec { base_reward: 8000, lane: "field" },
        GoalKind::BreakEven => GoalSpec { base_reward: 1500, lane: "books" },
        GoalKind::AvgCrowd => GoalSpec { base_reward: 1500, lane: "crowd" },
        GoalKind::FanInterest => GoalSpec { base_reward: 1500, lane: "crowd" },
    }
}

/// Everything a goal can be judged on, for a season in progress or finished.
#[derive(Clone, Copy, Debug)]
pub struct SeasonOutcome {
    pub wins: i32,
    pub games_played: i32,
    pub made_playoffs: bool,
    pub champion: bool,
    pub net: Money,
    pub avg_crowd: i64,
    pub fan_interest: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CouncilGoal {
    pub kind: GoalKind,
    /// Wins, crowd, or fan interest; unused for the rest.
    pub target: i64,
    pub reward: Money,
}

impl CouncilGoal {
    pub fn label(&self) -> String {
        match self.kind {
            GoalKind::Wins => format!("Win {} games", self.target),
            GoalKind::Playoffs => "Make the playoffs".into(),
            GoalKind::Title => "Win the title".into(),
            GoalKind::BreakEven => "Finish the season in the black".into(),
            GoalKind::AvgCrowd => format!("Average {} fans at home", self.target),
            GoalKind::FanInterest => format!("Fan interest of {} or better", self.target),
        }
    }

    pub fn is_met(&self, o: &SeasonOutcome) -> bool {
        match self.kind {
            GoalKind::Wins => o.wins as i64 >= self.target,
            GoalKind::Playoffs => o.made_playoffs,
            GoalKind::Title => o.champion,
            GoalKind::BreakEven => o.net >= 0,
            GoalKind::AvgCrowd => o.avg_crowd >= self.target,
            GoalKind::FanInterest => o.fan_interest >= self.target as f64,
        }
    }

    /// Where you stand right now, in the goal's own units.
    pub fn progress(&self, o: &SeasonOutcome) -> String {
        match self.kind {
            GoalKind::Wins => format!("{} of {} wins", o.wins, self.target),
            GoalKind::Playoffs => if o.champion || o.made_playoffs { "In" } else { "Not yet" }.into(),
            GoalKind::Title => if o.champion { "Champions" } else { "Not yet" }.into(),
            GoalKind::BreakEven => format!("{}${} so far", if o.net >= 0 { "+" } else { "-" }, o.net.abs()),
            GoalKind::AvgCrowd => format!("{} of {}", o.avg_crowd, self.target),
            GoalKind::FanInterest => format!("{} of {}", o.fan_interest.round() as i64, self.target),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalResult {
    pub goal: CouncilGoal,
    pub met: bool,
}

pub struct CouncilGoals;

impl CouncilGoals {
    /// Standard season length, used to scale win targets.
    pub const GAMES_PER_SEASON: i32 = 14;
    pub const ALL_MET_BONUS_PCT: f64 = 0.25;

    pub fn reward_for(k: GoalKind, t: &Town) -> Money {
        let scaled = goal_spec(k).base_reward as f64 * (t.capacity as f64 / 2500.0).sqrt();
        (scaled / 100.0).round() as Money * 100
    }

    /// Three goals, one per lane. Targets follow how last season went so they stay reachable but not free.
    pub fn generate(t: &Town, rng: &mut Rng, last_win_pct: Option<f64>, last_made_playoffs: bool, last_avg_crowd: i64, capacity: i64) -> Vec<CouncilGoal> {
        let cap = if capacity > 0 { capacity } else { t.capacity as i64 };
        let pct = last_win_pct.unwrap_or(0.4);
        let wins = ((pct * Self::GAMES_PER_SEASON as f64 + 0.5).round() as i64).clamp(4, 11);

        let field = if pct >= 0.75 || (last_made_playoffs && pct >= 0.6) {
            *rng.pick(&[GoalKind::Title, GoalKind::Playoffs])
        } else if pct >= 0.5 {
            *rng.pick(&[GoalKind::Playoffs, GoalKind::Wins])
        } else {
            GoalKind::Wins
        };
        let crowd = *rng.pick(&[GoalKind::AvgCrowd, GoalKind::FanInterest]);

        let make = |k: GoalKind, target: i64| CouncilGoal { kind: k, target, reward: Self::reward_for(k, t) };
        let crowd_target = if last_avg_crowd > 0 {
            ((cap as f64 * 0.95).round() as i64).min(((last_avg_crowd as f64 * 1.05).round() as i64).max((cap as f64 * 0.5).round() as i64))
        } else {
            (cap as f64 * 0.55).round() as i64
        };
        vec![
            if field == GoalKind::Wins { make(field, 11.min(wins + if pct >= 0.5 { 1 } else { 0 })) } else { make(field, 0) },
            make(GoalKind::BreakEven, 0),
            if crowd == GoalKind::AvgCrowd { make(crowd, (crowd_target as f64 / 50.0).round() as i64 * 50) } else { make(crowd, 55) },
        ]
    }

    /// Grant for a finished season: each goal met pays its reward, plus a bonus for the clean sweep.
    pub fn grant(results: &[GoalResult]) -> Money {
        let met: Money = results.iter().filter(|r| r.met).map(|r| r.goal.reward).sum();
        let sweep = !results.is_empty() && results.iter().all(|r| r.met);
        if sweep { (met as f64 * (1.0 + Self::ALL_MET_BONUS_PCT) / 100.0).round() as Money * 100 } else { met }
    }
}

/// The Council's annual review of the books and the field.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeasonReport {
    pub season: i32,
    pub record: String,
    /// Champions | Playoffs | Missed the playoffs.
    pub finish: String,
    pub revenue: BTreeMap<LedgerKind, Money>,
    pub expenses: BTreeMap<LedgerKind, Money>,
    pub avg_crowd: i64,
    pub capacity: i64,
    pub fan_interest: f64,
    pub cash_end: Money,
    pub league_avg_revenue: Money,
    pub league_avg_net: Money,
    pub goals: Vec<GoalResult>,
    pub grant: Money,
}

impl SeasonReport {
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

#[cfg(test)]
mod tests {
    use super::*;
    use sp_core::Region;

    fn outcome(wins: i32, net: Money) -> SeasonOutcome {
        SeasonOutcome { wins, games_played: 14, made_playoffs: false, champion: false, net, avg_crowd: 2000, fan_interest: 60.0 }
    }

    #[test]
    fn goals_are_one_per_lane_and_rewards_scale_with_the_stadium() {
        let ca = Region::california();
        let small = ca.starting.iter().find(|t| t.id == "monterey").unwrap();
        let big = ca.starting.iter().find(|t| t.id == "bakersfield").unwrap();
        for seed in 0..40 {
            let goals = CouncilGoals::generate(big, &mut Rng::new(seed), Some(0.6), false, 3000, 0);
            let lanes: Vec<&str> = goals.iter().map(|g| goal_spec(g.kind).lane).collect();
            assert_eq!(lanes, ["field", "books", "crowd"]);
            assert!(goals.iter().all(|g| g.reward % 100 == 0 && g.reward > 0));
        }
        assert!(CouncilGoals::reward_for(GoalKind::Title, big) > CouncilGoals::reward_for(GoalKind::Title, small));
    }

    #[test]
    fn win_targets_follow_last_season_and_stay_in_range() {
        let t = &Region::california().starting[0];
        for pct in [0.0, 0.2, 0.5, 0.8, 1.0] {
            for seed in 0..20 {
                for g in CouncilGoals::generate(t, &mut Rng::new(seed), Some(pct), false, 0, 0) {
                    if g.kind == GoalKind::Wins {
                        assert!((4..=11).contains(&g.target), "{pct} -> {}", g.target);
                    }
                }
            }
        }
    }

    #[test]
    fn goals_judge_the_outcome() {
        let win = CouncilGoal { kind: GoalKind::Wins, target: 7, reward: 1000 };
        assert!(win.is_met(&outcome(7, 0)) && !win.is_met(&outcome(6, 0)));
        let black = CouncilGoal { kind: GoalKind::BreakEven, target: 0, reward: 1000 };
        assert!(black.is_met(&outcome(0, 0)) && !black.is_met(&outcome(0, -1)));
        assert_eq!(black.progress(&outcome(0, -250)), "-$250 so far");
        assert_eq!(win.label(), "Win 7 games");
    }

    #[test]
    fn grant_pays_met_goals_and_a_sweep_bonus() {
        let g = |r, met| GoalResult { goal: CouncilGoal { kind: GoalKind::Wins, target: 5, reward: r }, met };
        assert_eq!(CouncilGoals::grant(&[g(1500, true), g(1500, false), g(3000, true)]), 4500);
        assert_eq!(CouncilGoals::grant(&[g(1500, true), g(1500, true), g(3000, true)]), 7500); // 6000 * 1.25
        assert_eq!(CouncilGoals::grant(&[]), 0);
    }
}
