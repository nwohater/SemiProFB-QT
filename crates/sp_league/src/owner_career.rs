use crate::hall_of_fame::HallEntry;
use crate::league::League;
use serde::{Deserialize, Serialize};
use sp_economy::LedgerKind;

/// How the user's club did in one season (finished seasons only).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnerSeason {
    pub season: i32,
    pub wins: i32,
    pub losses: i32,
    pub ties: i32,
    pub division_title: bool,
    /// 1-4, 0 = missed the playoffs.
    pub seed: usize,
    /// Champions | Runner-up | Semifinal exit | Playoffs | Missed the playoffs
    pub finish: String,
    pub playoff_wins: i32,
    pub playoff_losses: i32,
    /// False for seasons rebuilt from older saves: the playoff round (and so the playoff record) is not known.
    pub exact: bool,
}

impl OwnerSeason {
    pub fn made_playoffs(&self) -> bool {
        self.seed > 0
    }

    pub fn champion(&self) -> bool {
        self.finish == "Champions"
    }

    pub fn record(&self) -> String {
        format!("{}-{}{}", self.wins, self.losses, if self.ties > 0 { format!("-{}", self.ties) } else { String::new() })
    }

    pub fn pct(&self) -> f64 {
        let g = self.wins + self.losses + self.ties;
        if g == 0 { 0.0 } else { (self.wins as f64 + self.ties as f64 * 0.5) / g as f64 }
    }
}

/// One gift from the user's own cash to a struggling club.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClubHelp {
    pub season: i32,
    pub team: String,
    pub amount: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnerAward {
    pub season: i32,
    pub award: String,
    pub player: String,
    pub pos: String,
}

/// Everything the user has done as an owner, put together from the league's records.
#[derive(Clone, Debug)]
pub struct OwnerCareer {
    pub team_name: String,
    pub town: String,
    pub seasons: Vec<OwnerSeason>,
    pub help: Vec<ClubHelp>,
    pub awards: Vec<OwnerAward>,
    pub hall_of_famers: Vec<HallEntry>,
}

impl OwnerCareer {
    pub fn wins(&self) -> i32 {
        self.seasons.iter().map(|s| s.wins).sum()
    }

    pub fn losses(&self) -> i32 {
        self.seasons.iter().map(|s| s.losses).sum()
    }

    pub fn ties(&self) -> i32 {
        self.seasons.iter().map(|s| s.ties).sum()
    }

    pub fn pct(&self) -> f64 {
        let g = self.wins() + self.losses() + self.ties();
        if g == 0 { 0.0 } else { (self.wins() as f64 + self.ties() as f64 * 0.5) / g as f64 }
    }

    pub fn record(&self) -> String {
        format!("{}-{}{}", self.wins(), self.losses(), if self.ties() > 0 { format!("-{}", self.ties()) } else { String::new() })
    }

    pub fn titles(&self) -> usize {
        self.seasons.iter().filter(|s| s.champion()).count()
    }

    pub fn runner_ups(&self) -> usize {
        self.seasons.iter().filter(|s| s.finish == "Runner-up").count()
    }

    pub fn division_titles(&self) -> usize {
        self.seasons.iter().filter(|s| s.division_title).count()
    }

    pub fn playoff_appearances(&self) -> usize {
        self.seasons.iter().filter(|s| s.made_playoffs()).count()
    }

    pub fn playoff_wins(&self) -> i32 {
        self.seasons.iter().map(|s| s.playoff_wins).sum()
    }

    pub fn playoff_losses(&self) -> i32 {
        self.seasons.iter().map(|s| s.playoff_losses).sum()
    }

    pub fn playoff_record_exact(&self) -> bool {
        self.seasons.iter().all(|s| s.exact || !s.made_playoffs())
    }

    pub fn best_season(&self) -> Option<&OwnerSeason> {
        self.seasons.iter().max_by(|a, b| a.pct().total_cmp(&b.pct()))
    }

    pub fn clubs_helped(&self) -> usize {
        self.help.iter().map(|h| &h.team).collect::<std::collections::HashSet<_>>().len()
    }

    pub fn total_given(&self) -> i64 {
        self.help.iter().map(|h| h.amount).sum()
    }

    pub fn times_helped(&self) -> usize {
        self.help.len()
    }
}

impl League {
    /// Writes the finished season into the owner's log. Call once, after the title game.
    pub fn log_owner_season(&mut self) {
        let Some(u) = self.user_team else { return };
        let Some(po) = &self.playoffs else { return };
        if self.owner_seasons.iter().any(|s| s.season == self.season) {
            return;
        }
        let rec = &self.standings.records[u];
        let seed = po.seed_of(u); // 0 when not in the field
        let div_order = self.standings.division_order(self.division_of(u));
        let semi = po.semifinals.iter().find(|g| g.home() == u || g.away() == u);
        let (mut pw, mut pl) = (0, 0);
        let mut finish = "Missed the playoffs";
        if seed > 0 {
            if semi.and_then(|g| g.winner()) == Some(u) {
                pw += 1;
                if po.champion == Some(u) {
                    pw += 1;
                    finish = "Champions";
                } else {
                    pl += 1;
                    finish = "Runner-up";
                }
            } else {
                pl += 1;
                finish = "Semifinal exit";
            }
        }
        self.owner_seasons.push(OwnerSeason {
            season: self.season,
            wins: rec.wins,
            losses: rec.losses,
            ties: rec.ties,
            division_title: div_order[0] == u,
            seed,
            finish: finish.to_string(),
            playoff_wins: pw,
            playoff_losses: pl,
            exact: true,
        });
    }

    /// Records a gift from your own cash to a struggling club.
    pub fn log_club_help(&mut self, team: &str, amount: i64) {
        self.club_help.push(ClubHelp { season: self.season, team: team.to_string(), amount });
    }

    /// Seasons for the career page: the exact log, plus any older finished seasons rebuilt from the league history.
    pub fn owner_season_list(&self) -> Vec<OwnerSeason> {
        let Some(u) = self.user_team else { return vec![] };
        let re = regex::Regex::new(r"(\d+)-(\d+)-(\d+)$").unwrap();
        let mut out = Vec::new();
        for h in &self.history {
            if let Some(known) = self.owner_seasons.iter().find(|s| s.season == h.season) {
                out.push(known.clone());
                continue;
            }
            let Some(m) = re.captures(&h.records[u]) else { continue };
            let pct_of = |t: usize| -> f64 {
                let Some(x) = re.captures(&h.records[t]) else { return 0.0 };
                let (w, l, ti): (i32, i32, i32) = (x[1].parse().unwrap(), x[2].parse().unwrap(), x[3].parse().unwrap());
                if w + l + ti == 0 { 0.0 } else { (w as f64 + ti as f64 * 0.5) / (w + l + ti) as f64 }
            };
            let mine = pct_of(u);
            let div = self.division_of(u);
            let div_title = (0..self.teams.len()).filter(|&t| t != u && self.division_of(t) == div).all(|t| pct_of(t) < mine);
            let seed = h.playoff_seeds.iter().position(|&s| s == u).map(|i| i + 1).unwrap_or(0);
            let champ = h.champion == u;
            let finish = if champ { "Champions" } else if seed > 0 { "Playoffs" } else { "Missed the playoffs" };
            out.push(OwnerSeason {
                season: h.season,
                wins: m[1].parse().unwrap(),
                losses: m[2].parse().unwrap(),
                ties: m[3].parse().unwrap(),
                division_title: div_title,
                seed,
                finish: finish.to_string(),
                playoff_wins: if champ { 2 } else { 0 }, // history keeps no playoff games: a champion went 2-0, any other playoff team lost at least once
                playoff_losses: if seed > 0 && !champ { 1 } else { 0 },
                exact: false,
            });
        }
        out
    }

    /// Gifts to other clubs: the log, or (for older saves) whatever the ledger still remembers.
    pub fn club_help_list(&self) -> Vec<ClubHelp> {
        if !self.club_help.is_empty() {
            return self.club_help.clone();
        }
        let Some(u) = self.user_team else { return vec![] };
        self.finances[u]
            .ledger
            .iter()
            .filter(|e| e.kind == LedgerKind::Other && e.memo.starts_with("gift to ") && e.amount < 0)
            .map(|e| ClubHelp { season: e.season, team: e.memo[8..].to_string(), amount: -e.amount })
            .collect()
    }

    pub fn owner_career(&self) -> OwnerCareer {
        let (name, abbr, town) = match self.user_team {
            Some(u) => (self.teams[u].town.team_name(), self.teams[u].town.abbr(), self.teams[u].town.name.clone()),
            None => (String::new(), String::new(), String::new()),
        };
        let mut awards = Vec::new();
        for a in &self.awards_history {
            for c in &a.categories {
                if let Some(w) = c.winner() {
                    if w.team == name {
                        awards.push(OwnerAward { season: a.season, award: c.title.clone(), player: w.name.clone(), pos: w.pos.clone() });
                    }
                }
            }
        }
        let hof = self.hall_of_fame.iter().filter(|e| e.inducted && e.abbr == abbr).cloned().collect();
        OwnerCareer { team_name: name, town, seasons: self.owner_season_list(), help: self.club_help_list(), awards, hall_of_famers: hof }
    }
}
