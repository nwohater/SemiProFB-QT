use crate::awards::SeasonAwards;
use crate::inbox::*;
use crate::league::{League, LeaguePhase, WeekResult};
use crate::text::{commas, fill, inbox_text, money};
use sp_core::{Player, Rng};
use sp_economy::business::promo_by_id;
use sp_economy::{CashStatus, LedgerEntry, LedgerKind, SponsorDeal};
use std::collections::BTreeMap;

/// Sponsor deals at the moment the season rolls over (their seasons-left count is changed in place).
pub type SponsorSnapshot = Vec<(SponsorDeal, i32)>;

impl League {
    pub fn unread_inbox(&self, feed: Option<InboxFeed>) -> usize {
        self.inbox.iter().filter(|i| i.unread && feed.map(|f| f == i.feed).unwrap_or(true)).count()
    }

    pub fn mark_inbox_read(&mut self, feed: Option<InboxFeed>) {
        for i in &mut self.inbox {
            if feed.map(|f| f == i.feed).unwrap_or(true) {
                i.unread = false;
            }
        }
    }

    pub fn post_inbox(&mut self, feed: InboxFeed, kind: InboxKind, title: &str, text: &str, o: PostOpts) {
        let week = o.at_week.unwrap_or(if self.phase == LeaguePhase::Offseason { 0 } else { self.week });
        let id = self.inbox_seq;
        self.inbox_seq += 1;
        self.inbox.push(InboxItem {
            id,
            season: self.season,
            week,
            feed,
            kind,
            title: title.to_string(),
            text: text.to_string(),
            from: o.from,
            player_id: o.player_id,
            unread: true,
            plea_team: o.plea_team,
            plea_ask: o.plea_ask,
            plea_state: PleaState::Open,
            conduct_cost: o.conduct_cost,
            conduct_games: o.conduct_games,
            conduct_state: ConductState::Open,
        });
        if self.inbox.len() > MAX_INBOX_ITEMS {
            let cut = self.inbox.len() - MAX_INBOX_ITEMS;
            self.inbox.drain(0..cut);
        }
    }

    pub fn inbox_item_mut(&mut self, id: u32) -> Option<&mut InboxItem> {
        self.inbox.iter_mut().find(|i| i.id == id)
    }

    /// Everything the owner hears about after a week or playoff round. Uses its own Rng so game sims are unaffected.
    /// `injured_before` is the injury map from before the games were played.
    pub fn write_inbox(&mut self, r: &WeekResult, injured_before: &BTreeMap<u32, i32>, reason_before: &BTreeMap<u32, String>) {
        let Some(u) = self.user_team else { return };
        let mut nr = Rng::new(self.season as u64 * 7919 + r.week as u64 * 104 + 17);
        let t = inbox_text();
        let wk = r.week;
        let me_name = self.teams[u].town.team_name();

        // ---- Game & money
        for g in r.games.iter().filter(|g| g.home == u || g.away == u) {
            let home = g.home == u;
            let gf = if home { g.home_finance.as_ref() } else { g.away_finance.as_ref() };
            let (my, their) = if home { (g.home_score, g.away_score) } else { (g.away_score, g.home_score) };
            let opp = self.teams[if home { g.away } else { g.home }].town.team_name();
            let margin = my - their;
            let res = if margin > 0 { "W" } else if margin < 0 { "L" } else { "T" };
            if let Some(gf) = gf {
                let pool = if gf.net() < 0 {
                    "reportLosing"
                } else if !home {
                    "reportAway"
                } else if margin == 0 {
                    "reportTie"
                } else if margin >= 14 {
                    "reportWinBig"
                } else if margin > 0 {
                    "reportWin"
                } else if margin <= -14 {
                    "reportLossBig"
                } else {
                    "reportLoss"
                };
                let tail = fill(t.pick(pool, &mut nr), &[("t", me_name.clone()), ("o", opp.clone())]);
                let crowd = if home {
                    format!(" Crowd {} of {} ({}% full).", commas(gf.attendance), commas(gf.capacity), if gf.capacity == 0 { 0 } else { (gf.attendance as f64 * 100.0 / gf.capacity as f64).round() as i64 })
                } else {
                    String::new()
                };
                let mut night = String::new();
                if home && !g.playoff {
                    match promo_by_id(gf.promo_id.as_deref().unwrap_or("")) {
                        None => night = format!(" {}", t.pick("reportNoPromo", &mut nr)),
                        Some(ran) if gf.promo_mishap => {
                            night = format!(" {}", fill(t.pick("reportPromoMishap", &mut nr), &[("promo", ran.name.clone()), ("why", ran.risk_text.clone())]));
                        }
                        Some(_) => {}
                    }
                }
                self.post_inbox(
                    InboxFeed::Money,
                    InboxKind::Report,
                    &format!("{res} {my}-{their} {} {opp}", if home { "vs" } else { "at" }),
                    &format!("Revenue {}, costs {}, net {}{}.{crowd}{night} {tail}", money(gf.total_revenue()), money(gf.total_expenses()), if gf.net() >= 0 { "+" } else { "" }, money(gf.net())),
                    PostOpts::week(wk),
                );
            }
            // Promo night result (home games only).
            let promo = if home && !g.playoff { promo_by_id(gf.and_then(|f| f.promo_id.as_deref()).unwrap_or("")) } else { None };
            if let (Some(promo), Some(gf)) = (promo, gf) {
                let base = promo.cost(&self.teams[u].town);
                let cost = gf.expenses.get(&LedgerKind::Promo).copied().unwrap_or(0);
                let wrong = gf.promo_mishap;
                let text = if wrong {
                    format!("{} Cost {} (planned {}).", fill(t.pick("promoBad", &mut nr), &[("promo", promo.name.clone()), ("why", promo.risk_text.clone())]), money(cost), money(base))
                } else {
                    format!("{} Cost {}.", fill(t.pick("promoGood", &mut nr), &[("promo", promo.name.clone())]), money(cost))
                };
                self.post_inbox(InboxFeed::Money, InboxKind::Promo, &format!("{}: {}", promo.name, if wrong { "trouble" } else { "success" }), &text, PostOpts::week(wk));
            }
        }

        // Sponsor news from this advance (mid-season offers).
        for s in self.sponsor_news.clone() {
            self.post_inbox(InboxFeed::Money, InboxKind::Sponsor, "Sponsor offer", &s, PostOpts::week(wk));
        }

        // Sponsor events: win bonuses, win-rate warnings, and the odd clause flavor.
        let my_game = r.games.iter().find(|g| g.home == u || g.away == u);
        let sponsors = self.finances[u].sponsors.clone();
        if let Some(mg) = my_game {
            if !sponsors.is_empty() {
                let won = mg.winner() == Some(u);
                let home = mg.home == u;
                let rec = self.standings.records[u].clone();
                for d in &sponsors {
                    let c = d.effects();
                    if let (true, Some(c)) = (won, c) {
                        if c.win_bonus > 0 {
                            self.post_inbox(InboxFeed::Money, InboxKind::Sponsor, &format!("{}: win bonus", d.name), &fill(t.pick("sponsorBonus", &mut nr), &[("s", d.name.clone()), ("amt", commas(c.win_bonus))]), PostOpts::week(wk));
                        }
                    }
                    if !mg.playoff && d.min_win_pct > 0.0 && rec.games() >= 6 && wk % 4 == 2 && rec.pct() < d.min_win_pct {
                        self.post_inbox(
                            InboxFeed::Money,
                            InboxKind::Sponsor,
                            &format!("{} is watching", d.name),
                            &fill(t.pick("sponsorWatch", &mut nr), &[("s", d.name.clone()), ("need", ((d.min_win_pct * 100.0).round() as i64).to_string()), ("pct", ((rec.pct() * 100.0).round() as i64).to_string())]),
                            PostOpts::week(wk),
                        );
                    }
                }
                if home && nr.chance(0.35) {
                    let d = nr.pick(&sponsors).clone();
                    let lines = t.sponsor_clause_events.get(&d.clause_id).or_else(|| d.effects().and_then(|e| t.sponsor_clause_events.get(&e.id)));
                    let line = match lines {
                        Some(l) => nr.pick(l).clone(),
                        None => nr.pick(t.list("sponsorClauseGeneric")).clone(),
                    };
                    self.post_inbox(InboxFeed::Money, InboxKind::Sponsor, &d.name, &fill(&line, &[("s", d.name.clone())]), PostOpts::week(wk));
                }
            }
        }

        // Bank notices.
        let status = self.finances[u].status(&self.teams[u]);
        match status {
            CashStatus::Low => self.post_inbox(InboxFeed::Money, InboxKind::Bank, "The bank has noticed", t.pick("bankLow", &mut nr), PostOpts::week(wk)),
            CashStatus::Negative => self.post_inbox(InboxFeed::Money, InboxKind::Bank, "Overdrawn", t.pick("bankNegative", &mut nr), PostOpts::week(wk)),
            CashStatus::FinalNotice => self.post_inbox(InboxFeed::Money, InboxKind::Bank, "FINAL NOTICE", t.pick("bankFinal", &mut nr), PostOpts::week(wk)),
            _ => {}
        }

        self.write_rival_money_rumors(&mut nr, wk);

        // Town council after the title game.
        if self.champion().is_some() && r.phase_after == LeaguePhase::Complete {
            let town = self.teams[u].town.name.clone();
            if self.champion() == Some(u) {
                self.post_inbox(InboxFeed::Money, InboxKind::Council, "Letter from the Town Council", &fill(t.pick("councilTitle", &mut nr), &[("town", town)]), PostOpts::week(wk));
            }
        }
        if wk == 14 && r.phase_after != LeaguePhase::RegularSeason {
            let rec = self.standings.records[u].clone();
            let pool = if rec.pct() < 0.5 { "councilWarn" } else { "councilOk" };
            let town = self.teams[u].town.name.clone();
            self.post_inbox(InboxFeed::Money, InboxKind::Council, "Letter from the Town Council", &fill(t.pick(pool, &mut nr), &[("town", town), ("rec", rec.to_string())]), PostOpts::week(wk));
        }

        // ---- Injuries & travel (your players)
        let roster: Vec<Player> = self.teams[u].roster.clone();
        for p in &roster {
            let out = self.injured_games.get(&p.id).copied();
            let was = injured_before.get(&p.id).copied();
            if let (Some(out), None) = (out, was) {
                // Newly out. The reason tells us whether it was a game injury, a job injury or a work trip.
                match self.out_reason.get(&p.id).cloned() {
                    None => {
                        let minor = out <= 1;
                        if p.ovr() >= 45 || out >= 3 {
                            let line = fill(t.pick(if minor { "gameInjuryMinor" } else { "gameInjuryPlayer" }, &mut nr), &[("p", p.name.clone()), ("g", out.to_string()), ("body", t.pick("bodyParts", &mut nr).to_string())]);
                            self.post_inbox(InboxFeed::Travel, InboxKind::Injury, &format!("{} {}: hurt in the game", p.position.label(), p.name), &line, PostOpts::week(wk).player(p.id).from(format!("{} ({})", p.name, p.position.label())));
                        }
                    }
                    Some(why) if !why.starts_with("Suspended") => {
                        let job = p.day_job.clone();
                        let (injury, trip_lines) = job_lines(&job);
                        let trip = why.starts_with("Stuck");
                        let pool = if trip { trip_lines } else { injury };
                        let line = fill(&nr.pick(&pool).clone(), &[("p", p.name.clone()), ("job", job.to_lowercase()), ("g", out.to_string())]);
                        let (kind, title) = if trip { (InboxKind::Trip, format!("{} {}: stuck at work", p.position.label(), p.name)) } else { (InboxKind::WorkInjury, format!("{} {}: hurt at work", p.position.label(), p.name)) };
                        self.post_inbox(InboxFeed::Travel, kind, &title, &line, PostOpts::week(wk).player(p.id).from(format!("{} ({})", p.name, job)));
                    }
                    Some(_) => {}
                }
            }
            // Back after sitting out.
            if was.is_some() && out.is_none() && p.ovr() >= 45 {
                let why = reason_before.get(&p.id);
                let from_trip = why.map(|w| w.starts_with("Stuck")).unwrap_or(false);
                let from_conduct = why.map(|w| w.starts_with("Suspended")).unwrap_or(false);
                let from_work = why.is_some() && !from_trip && !from_conduct;
                let pool = if from_conduct {
                    "returnedConduct"
                } else if from_trip {
                    "returnedTrip"
                } else if from_work {
                    "returnedWorkInjury"
                } else {
                    "returned"
                };
                let text = fill(t.pick(pool, &mut nr), &[("p", p.name.clone()), ("pos", p.position.label().to_string()), ("job", p.day_job.to_lowercase())]);
                self.post_inbox(InboxFeed::Travel, InboxKind::Returned, &format!("{} {} is back", p.position.label(), p.name), &text, PostOpts::week(wk).player(p.id));
            }
        }
    }

    /// Rumors when another club's cash slides under the thresholds. Each stage is told once per slide (it resets if they recover).
    fn write_rival_money_rumors(&mut self, nr: &mut Rng, wk: i32) {
        let t = inbox_text();
        for i in 0..self.teams.len() {
            if Some(i) == self.user_team {
                continue;
            }
            let (cash, bankrupt) = (self.finances[i].cash, self.finances[i].bankrupt);
            let stage = if bankrupt || cash < 0 { 3 } else if cash < 1000 { 2 } else if cash < 3000 { 1 } else { 0 };
            let prev = self.rival_cash_stage.get(&i).copied().unwrap_or(0);
            if stage == prev {
                continue;
            }
            self.rival_cash_stage.insert(i, stage);
            if stage < prev || stage == 0 {
                continue;
            }
            let name = self.teams[i].town.team_name();
            let town = self.teams[i].town.name.clone();
            let (title, pool) = match stage {
                1 => (format!("Rumors: {name} short on cash"), "rumorLowCash"),
                2 => (format!("{name} is nearly broke"), "rumorVeryLow"),
                _ => (format!("{name} is likely doomed"), "rumorDoomed"),
            };
            self.post_inbox(InboxFeed::Money, InboxKind::Rumor, &title, &fill(t.pick(pool, nr), &[("t", name.clone()), ("town", town.clone())]), PostOpts::week(wk));
            // A club in real trouble writes to the owner directly, once a season, and only while it can still be saved.
            let season = self.season;
            if stage >= 2 && !bankrupt && !self.inbox.iter().any(|m| m.kind == InboxKind::Plea && m.plea_team == Some(i) && m.season == season) {
                let ask = self.plea_ask_for(i);
                let text = fill(t.pick("pleas", nr), &[("t", name.clone()), ("town", town), ("amt", commas(ask))]);
                self.post_inbox(InboxFeed::Money, InboxKind::Plea, &format!("Plea from the {name}"), &text, PostOpts { at_week: Some(wk), from: Some(format!("Owner, {name}")), plea_team: Some(i), plea_ask: ask, ..Default::default() });
            }
        }
    }

    /// What a struggling club asks for: clear the hole, cover its average losses for the rest of the season, plus a few games
    /// into the next one so it comes back instead of folding at the final whistle. Round hundreds.
    pub fn plea_ask_for(&self, team: usize) -> i64 {
        let f = &self.finances[team];
        let played = f.games.len() as i64;
        let avg_net = if played == 0 { 0.0 } else { f.season_net() as f64 / played as f64 };
        let burn = if avg_net < 0.0 { -avg_net } else { 0.0 };
        let per_season = crate::schedule::ScheduleGenerator::WEEKS as i64;
        let left = (per_season - played).clamp(0, per_season);
        let need = (-f.cash).clamp(0, 1 << 30) as f64 + burn * (left + 3) as f64 + 1500.0;
        ((need.clamp(2500.0, (1i64 << 30) as f64)) / 500.0).ceil() as i64 * 500
    }

    /// Give a struggling club `amount` out of your own cash. Returns the thank-you text, or `None` if it can't be done.
    pub fn help_club(&mut self, plea_id: u32, amount: i64) -> Option<String> {
        let u = self.user();
        let plea = self.inbox.iter().find(|i| i.id == plea_id)?.clone();
        let t = plea.plea_team?;
        if plea.plea_state != PleaState::Open || amount <= 0 || self.finances[u].cash < amount || (self.finances[t].bankrupt && self.finances[t].cash + amount < 0) {
            return None;
        }
        let (season, week) = (self.season, self.week);
        let team_name = self.teams[t].town.team_name();
        let my_name = self.teams[u].town.team_name();
        let town = self.teams[t].town.name.clone();
        {
            let mine = &mut self.finances[u];
            mine.cash -= amount;
            mine.ledger.push(LedgerEntry::new(season, week, LedgerKind::Other, -amount, &format!("gift to {team_name}")));
        }
        self.log_club_help(&team_name, amount);
        {
            let f = &mut self.finances[t];
            f.cash += amount;
            f.ledger.push(LedgerEntry::new(season, week, LedgerKind::Other, amount, &format!("gift from the {my_name}")));
            if f.cash >= 0 {
                f.overdue_games = 0;
                f.bankrupt = false;
            }
        }
        self.rival_cash_stage.remove(&t);
        if let Some(p) = self.inbox_item_mut(plea_id) {
            p.plea_state = PleaState::Helped;
            p.unread = false;
        }
        let f = &mut self.finances[u];
        f.fan_interest = (f.fan_interest + 1.5).clamp(0.0, 100.0);
        let mut r = Rng::new(season as u64 * 6151 + t as u64 * 31 + amount as u64);
        let text = fill(inbox_text().pick("pleaThanks", &mut r), &[("t", team_name.clone()), ("town", town), ("amt", commas(amount))]);
        self.post_inbox(InboxFeed::Money, InboxKind::Plea, &format!("Thank you from the {team_name}"), &text, PostOpts { from: Some(format!("Owner, {team_name}")), plea_team: Some(t), ..Default::default() });
        if let Some(last) = self.inbox.last_mut() {
            last.plea_state = PleaState::Helped;
        }
        Some(text)
    }

    pub fn decline_plea(&mut self, plea_id: u32) {
        if let Some(p) = self.inbox_item_mut(plea_id) {
            if p.plea_state == PleaState::Open {
                p.plea_state = PleaState::Declined;
            }
        }
    }

    /// Day-one letters.
    pub fn write_inbox_welcome(&mut self) {
        let Some(u) = self.user_team else { return };
        let mut r = Rng::new(self.season as u64 * 31 + 5);
        let t = inbox_text();
        let town = self.teams[u].town.name.clone();
        self.post_inbox(InboxFeed::Money, InboxKind::Council, "Letter from the Town Council", &fill(t.pick("councilWelcome", &mut r), &[("town", town)]), PostOpts::week(0));
        let n = self.finances[u].offers.len();
        if n > 0 {
            self.post_inbox(InboxFeed::Money, InboxKind::Sponsor, "Sponsors are knocking", &fill(t.pick("sponsorsWaiting", &mut r), &[("n", n.to_string())]), PostOpts::week(0));
        }
    }

    /// Fan-favorite news and the new-season letter (offseason).
    pub fn write_inbox_offseason(&mut self, season_start: bool) {
        let Some(u) = self.user_team else { return };
        let mut r = Rng::new(self.season as u64 * 9973 + if season_start { 11 } else { 3 });
        let t = inbox_text();
        for n in self.fan_news.clone() {
            if self.inbox.iter().any(|i| i.kind == InboxKind::Fan && i.season == self.season && i.text == n.text) {
                continue;
            }
            let mut o = PostOpts::week(0);
            o.player_id = n.player_id;
            self.post_inbox(InboxFeed::Money, InboxKind::Fan, "Fan favorites", &n.text, o);
        }
        if season_start {
            let n = self.finances[u].offers.len();
            if n > 0 {
                self.post_inbox(InboxFeed::Money, InboxKind::Sponsor, "Sponsor offers", &fill(t.pick("sponsorsWaiting", &mut r), &[("n", n.to_string())]), PostOpts::week(0));
            }
            let season = self.season;
            self.post_inbox(InboxFeed::Money, InboxKind::Council, "Letter from the Town Council", &fill(t.pick("councilSeason", &mut r), &[("s", season.to_string())]), PostOpts::week(0));
        }
    }

    pub fn snapshot_sponsors(&self) -> SponsorSnapshot {
        self.my_finance().sponsors.iter().map(|d| (d.clone(), d.seasons_left)).collect()
    }

    /// After the rollover: who left (clause) or ended (term), by comparing with `before`.
    pub fn write_sponsor_rollover(&mut self, before: &SponsorSnapshot, win_pct: f64) {
        let Some(u) = self.user_team else { return };
        let mut r = Rng::new(self.season as u64 * 4421 + 9);
        let t = inbox_text();
        let still: Vec<String> = self.finances[u].sponsors.iter().map(|s| s.name.clone()).collect();
        for (d, seasons_left) in before {
            if still.contains(&d.name) {
                continue;
            }
            let ended = seasons_left - 1 <= 0;
            if !ended && d.min_win_pct > 0.0 && win_pct < d.min_win_pct {
                self.post_inbox(InboxFeed::Money, InboxKind::Sponsor, &format!("{} has left", d.name), &fill(t.pick("sponsorLeft", &mut r), &[("s", d.name.clone()), ("need", ((d.min_win_pct * 100.0).round() as i64).to_string())]), PostOpts::week(0));
            } else {
                self.post_inbox(InboxFeed::Money, InboxKind::Sponsor, &format!("{}: deal ended", d.name), &fill(t.pick("sponsorExpired", &mut r), &[("s", d.name.clone())]), PostOpts::week(0));
            }
        }
    }

    pub fn write_offer_lapsed(&mut self, names: &[String]) {
        if self.user_team.is_none() || names.is_empty() {
            return;
        }
        let mut r = Rng::new(self.season as u64 * 6131 + self.week as u64);
        for n in names {
            self.post_inbox(InboxFeed::Money, InboxKind::Sponsor, &format!("{n} offer expired"), &fill(inbox_text().pick("sponsorOfferLapsed", &mut r), &[("s", n.clone())]), PostOpts::default());
        }
    }

    /// Announces the winners, plus a note when one of your players took an award.
    pub fn write_inbox_awards(&mut self, a: &SeasonAwards) {
        let Some(u) = self.user_team else { return };
        let mine: std::collections::HashSet<u32> = self.teams[u].roster.iter().map(|p| p.id).collect();
        let mut lines = Vec::new();
        for c in &a.categories {
            let Some(w) = c.winner() else { continue };
            lines.push(format!("{}: {} ({}, {}), {}.", c.title, w.name, w.pos, w.team, w.line));
            if mine.contains(&w.player_id) {
                let mut r = Rng::new(a.season as u64 * 17 + w.player_id as u64);
                let text = fill(inbox_text().pick("awardMine", &mut r), &[("p", w.name.clone()), ("a", c.title.clone())]);
                self.post_inbox(InboxFeed::Money, InboxKind::Award, &format!("Your player won {}", c.title), &text, PostOpts::week(0).player(w.player_id));
            }
        }
        if lines.is_empty() {
            return;
        }
        self.post_inbox(InboxFeed::Money, InboxKind::Award, &format!("Season {} awards", a.season), &format!("{}\nFull finalists are in League > Awards.", lines.join("\n")), PostOpts::week(0));
    }

    /// Signing and cancelling sponsors: do it and write the letter.
    pub fn accept_sponsor(&mut self, index: usize) -> bool {
        let u = self.user();
        let Some(o) = self.finances[u].offers.get(index).cloned() else { return false };
        if !self.finances[u].accept_offer(index) {
            return false;
        }
        let mut r = Rng::new(self.season as u64 * 313 + o.name.len() as u64 * 7 + index as u64);
        let text = fill(
            inbox_text().pick("sponsorSigned", &mut r),
            &[("s", o.name.clone()), ("amt", commas(o.per_game)), ("n", o.seasons.to_string()), ("seasons", if o.seasons == 1 { "season" } else { "seasons" }.to_string()), ("clause", o.clause.clone())],
        );
        self.post_inbox(InboxFeed::Money, InboxKind::Sponsor, &format!("{}: deal signed", o.name), &text, PostOpts::default());
        true
    }

    pub fn cancel_sponsor_deal(&mut self, index: usize) -> bool {
        let u = self.user();
        let Some(d) = self.finances[u].sponsors.get(index).cloned() else { return false };
        let (week, season) = (self.week, self.season);
        if !self.finances[u].cancel_sponsor(index, week, season) {
            return false;
        }
        let mut r = Rng::new(season as u64 * 419 + d.name.len() as u64 * 11 + index as u64);
        let text = fill(inbox_text().pick("sponsorCancelled", &mut r), &[("s", d.name.clone()), ("amt", commas(d.per_game))]);
        self.post_inbox(InboxFeed::Money, InboxKind::Sponsor, &format!("{}: deal cancelled", d.name), &text, PostOpts::default());
        true
    }
}

/// Injury and work-trip lines for a day job, matched by keyword in the job name.
fn job_lines(job: &str) -> (Vec<String>, Vec<String>) {
    let j = job.to_lowercase();
    let t = inbox_text();
    for (_, c) in &t.job_categories {
        if c.keys.iter().any(|k| j.contains(k.as_str())) {
            return (c.injury.clone(), c.trip.clone());
        }
    }
    (t.list("genericInjury").to_vec(), t.list("genericTrip").to_vec())
}
