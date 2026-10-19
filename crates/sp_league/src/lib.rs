//! League, schedule, offseason and AI teams. Ported from the Dart `sp_league`.

pub mod absence_log;
pub mod ai_business;
pub mod ai_strategy;
pub mod awards;
pub mod conduct;
pub mod draft;
pub mod council;
pub mod fan_favorites;
pub mod gazette;
pub mod hall_of_fame;
pub mod inbox;
pub mod inbox_ops;
pub mod insights;
pub mod league;
pub mod offseason;
pub mod owner_career;
pub mod persistence;
pub mod playoffs;
pub mod schedule;
pub mod standings;
pub mod surprise_bills;
pub mod trade;
pub mod text;
pub mod weather;

pub use league::{League, LeaguePhase, NewLeague, WeekResult};
