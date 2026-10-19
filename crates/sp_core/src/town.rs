use crate::region::town_abbreviation;
use serde::{Deserialize, Serialize};

/// Town profile (features/economy.md sec 5). Traits are 1-10 tuning seeds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Town {
    pub id: String,
    pub name: String,
    /// Division name within the region, e.g. North | Coastal | South.
    pub division: String,
    pub population: i32,
    pub wealth: i32,
    pub turnout: i32,
    pub loyalty: i32,
    pub culture: i32,
    pub biz: i32,
    pub capacity: i32,
    pub nickname: String,
    pub mascot: String,
    pub quirk: String,
}

impl Town {
    /// Short team code for tight spots like stat tables (e.g. FRE, SLO).
    pub fn abbr(&self) -> String {
        town_abbreviation(&self.id).unwrap_or_else(|| {
            let letters: String = self.name.chars().filter(|c| c.is_ascii_alphabetic()).collect();
            letters.chars().take(3).collect::<String>().to_uppercase()
        })
    }

    pub fn team_name(&self) -> String {
        if self.nickname.is_empty() { self.name.clone() } else { format!("{} {}", self.name, self.nickname) }
    }

    pub fn with_nickname(&self, nickname: &str) -> Town {
        Town { nickname: nickname.to_string(), ..self.clone() }
    }
}
