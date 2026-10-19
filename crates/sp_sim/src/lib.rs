//! Game simulation engine. Ported from the Dart `sp_sim`.

pub mod box_score;
pub mod game_sim;
pub mod game_state;
pub mod kicking;
pub mod play_event;
pub mod stats;
pub mod team_ctx;


pub use box_score::{BoxScore, BoxScoreBuilder};
pub use game_sim::{GameConfig, GameResult, GameSim};
pub use game_state::{GameState, Side, Sides};
pub use kicking::{KickConditions, Kicking};
pub use stats::{PlayerStats, TeamStats};
pub use team_ctx::{DefPackage, OffPackage, TeamCtx};
