use serde::{Deserialize, Serialize};

/// The two feeds in the owner's inbox.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InboxFeed {
    Money,
    Travel,
}

/// What kind of message it is (drives the icon and tone).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InboxKind {
    Report,
    Promo,
    Sponsor,
    Bank,
    Council,
    Fan,
    Injury,
    WorkInjury,
    Trip,
    Returned,
    Award,
    Rumor,
    Plea,
    Bill,
    Conduct,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PleaState {
    Open,
    Helped,
    Declined,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConductState {
    Open,
    Paid,
    Sits,
}

pub const MAX_INBOX_ITEMS: usize = 200;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InboxItem {
    pub id: u32,
    pub season: i32,
    /// 0 = preseason / offseason.
    pub week: i32,
    pub feed: InboxFeed,
    pub kind: InboxKind,
    pub title: String,
    pub text: String,
    /// Set when the message is written by a player (first person); shown as "From: ...".
    pub from: Option<String>,
    /// The player this is about, so the inbox can open his page.
    pub player_id: Option<u32>,
    pub unread: bool,
    /// Set on a plea for help: the club asking, how much, and how it ended.
    pub plea_team: Option<usize>,
    pub plea_ask: i64,
    pub plea_state: PleaState,
    /// Set on an off-field trouble letter: what it costs to settle, how many games he sits if you do not, and your answer.
    pub conduct_cost: i64,
    pub conduct_games: i32,
    pub conduct_state: ConductState,
}

/// Optional parts of a posted message.
#[derive(Clone, Debug, Default)]
pub struct PostOpts {
    pub from: Option<String>,
    pub at_week: Option<i32>,
    pub player_id: Option<u32>,
    pub plea_team: Option<usize>,
    pub plea_ask: i64,
    pub conduct_cost: i64,
    pub conduct_games: i32,
}

impl PostOpts {
    pub fn week(w: i32) -> Self {
        Self { at_week: Some(w), ..Default::default() }
    }

    pub fn player(mut self, id: u32) -> Self {
        self.player_id = Some(id);
        self
    }

    pub fn from(mut self, who: impl Into<String>) -> Self {
        self.from = Some(who.into());
        self
    }
}
