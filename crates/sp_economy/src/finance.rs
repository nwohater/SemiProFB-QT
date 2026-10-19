use crate::Money;
use crate::business::{Facility, PromoTemplate, SponsorClause, SponsorMarket, SponsorOffer, promo_by_id, sponsor_clause_by_id, sponsor_clause_by_text};
use crate::contract::Contract;
use crate::council::{CouncilGoal, SeasonReport};
use crate::ledger::{GameFinance, LedgerEntry, LedgerKind};
use crate::pay_scale::PayScale;
use serde::{Deserialize, Serialize};
use sp_core::geo::town_miles;
use sp_core::{Rng, Team, Town, Weather};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SponsorDeal {
    pub name: String,
    pub per_game: Money,
    pub seasons_left: i32,
    pub clause: String,
    /// Performance clause: deal ends if the team wins less than this share.
    pub min_win_pct: f64,
    pub clause_id: String,
}

impl SponsorDeal {
    pub fn new(name: &str, per_game: Money, seasons_left: i32) -> Self {
        Self { name: name.to_string(), per_game, seasons_left, clause: String::new(), min_win_pct: 0.0, clause_id: String::new() }
    }

    pub fn from_offer(o: &SponsorOffer) -> Self {
        Self { name: o.name.clone(), per_game: o.per_game, seasons_left: o.seasons, clause: o.clause.clone(), min_win_pct: o.min_win_pct, clause_id: o.clause_id.clone() }
    }

    pub fn effects(&self) -> Option<&'static SponsorClause> {
        sponsor_clause_by_id(&self.clause_id).or_else(|| sponsor_clause_by_text(&self.clause))
    }
}

/// Difficulty knobs for money. All values are tuning seeds.
pub struct EconTuning;

impl EconTuning {
    /// Non-payroll costs (rent, insurance, travel, medical, misc) creep up each season, up to a ceiling.
    pub const INFLATION_PER_SEASON: f64 = 0.08;
    pub const INFLATION_CAP: f64 = 1.0;

    pub fn cost_index(season: i32) -> f64 {
        1.0 + Self::INFLATION_CAP.min(Self::INFLATION_PER_SEASON * 0.max(season - 1) as f64)
    }

    /// Price and fan acceptance: a town that loves its team bears a higher price and shrugs at price changes;
    /// a cold town wants cheap seats and walks away when prices rise. Both are neutral at interest 50.
    pub const TOLERANCE_AT_ZERO: f64 = 0.7;
    pub const TOLERANCE_SPAN: f64 = 0.6;
    pub const ELASTICITY_AT_ZERO: f64 = 1.6;
    pub const ELASTICITY_AT_FULL: f64 = 1.0;

    pub fn price_tolerance(interest: f64) -> f64 {
        Self::TOLERANCE_AT_ZERO + Self::TOLERANCE_SPAN * (interest / 100.0).clamp(0.0, 1.0)
    }

    pub fn price_elasticity(interest: f64) -> f64 {
        Self::ELASTICITY_AT_ZERO + (Self::ELASTICITY_AT_FULL - Self::ELASTICITY_AT_ZERO) * (interest / 100.0).clamp(0.0, 1.0)
    }

    /// Game-day crowd vs the price-curve mean. Promo nights are a known event, so they swing less;
    /// a quiet Saturday is walk-up weather and competing barbecues.
    pub const CROWD_SIGMA_PROMO: f64 = 0.08;
    pub const CROWD_MIN_PROMO: f64 = 0.78;
    pub const CROWD_MAX_PROMO: f64 = 1.18;
    pub const CROWD_SIGMA_PLAIN: f64 = 0.18;
    pub const CROWD_MIN_PLAIN: f64 = 0.55;
    pub const CROWD_MAX_PLAIN: f64 = 1.40;

    pub fn crowd_noise(rng: &mut Rng, promo: bool) -> f64 {
        let (sd, lo, hi) = if promo {
            (Self::CROWD_SIGMA_PROMO, Self::CROWD_MIN_PROMO, Self::CROWD_MAX_PROMO)
        } else {
            (Self::CROWD_SIGMA_PLAIN, Self::CROWD_MIN_PLAIN, Self::CROWD_MAX_PLAIN)
        };
        rng.gaussian(1.0, sd).clamp(lo, hi)
    }

    /// A new club starts a little cold: fans have to be won over before prices can rise (neutral is 50).
    pub const STARTING_INTEREST: f64 = 45.0;

    /// Fans: pull toward neutral each game and at season rollover; a hot streak is harder to hold at the top.
    pub const FAN_DRIFT_PER_GAME: f64 = 0.05;
    pub const FAN_COOLDOWN_PER_SEASON: f64 = 0.3;
    /// Each point of interest over 50 trims 1.2% off a win's gain.
    pub const WIN_DAMPING_ABOVE_50: f64 = 0.012;
    /// Loyalty softens losses by loyalty / this.
    pub const LOSS_LOYALTY_DIVISOR: f64 = 35.0;

    /// Debt: interest climbs with how much of the credit line is drawn; the line shrinks after a losing year.
    pub const LOAN_BASE_RATE: f64 = 0.010;
    pub const LOAN_UTIL_RATE: f64 = 0.020;
    pub const LOAN_MIN_PAY_PCT: f64 = 0.03;
    pub const CREDIT_WIN_MIN: f64 = 0.6;
    pub const CREDIT_WIN_SPAN: f64 = 0.8;
    /// No facility spending while the loan is over this share of the limit.
    pub const UPGRADE_DEBT_COVENANT: f64 = 0.5;

    /// Players know when you are flush: re-sign asks rise with cash on hand. Highest threshold reached wins.
    pub const FLUSH_PREMIUM: [(Money, f64); 3] = [(100_000, 0.08), (200_000, 0.15), (400_000, 0.25)];

    /// An established player (ovr 50+) who sat on the bench wants to play and asks for more to stay.
    pub const BENCHED_MIN_OVR: i32 = 50;
    pub const BENCHED_PREMIUM: f64 = 0.12;

    /// Each individual award a player has won raises his ask by this much (championships do not count), up to this many awards.
    pub const AWARD_PREMIUM: f64 = 0.20;
    pub const AWARD_PREMIUM_MAX: i32 = 5;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CashStatus {
    Ok,
    Low,
    Negative,
    FinalNotice,
    Bankrupt,
}

/// Everything money-related for one team. Cash in whole dollars.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamFinance {
    pub town: Town,
    pub cash: Money,
    /// 0..100, see `EconTuning::STARTING_INTEREST`.
    pub fan_interest: f64,
    pub ticket_price: f64,
    /// Player id -> contract.
    pub contracts: BTreeMap<u32, Contract>,
    pub sponsors: Vec<SponsorDeal>,
    pub ledger: Vec<LedgerEntry>,
    pub games: Vec<GameFinance>,
    pub overdue_games: i32,
    pub bankrupt: bool,
    /// Your levers (features/economy.md): promo nights by week, stadium upgrades, loans, sponsor offers.
    pub promos: BTreeMap<i32, String>,
    pub facilities: BTreeMap<Facility, i32>,
    pub offers: Vec<SponsorOffer>,
    pub loan_balance: Money,
    /// Fan interest earned (or lost) this season from ticket pricing, and from the most recent game.
    pub price_fan_this_season: f64,
    pub last_price_fan_delta: f64,
    pub last_season_revenue: Money,
    /// The Town Council's asks for this season, and its annual reviews of past ones.
    pub goals: Vec<CouncilGoal>,
    pub reports: Vec<SeasonReport>,
    pub last_win_pct: f64,
}

impl TeamFinance {
    pub fn new(town: Town, cash: Money, fan_interest: f64, ticket_price: Option<f64>) -> Self {
        let ticket_price = ticket_price.unwrap_or_else(|| TownEconomy::ref_price(&town));
        Self {
            town,
            cash,
            fan_interest,
            ticket_price,
            contracts: BTreeMap::new(),
            sponsors: Vec::new(),
            ledger: Vec::new(),
            games: Vec::new(),
            overdue_games: 0,
            bankrupt: false,
            promos: BTreeMap::new(),
            facilities: BTreeMap::new(),
            offers: Vec::new(),
            loan_balance: 0,
            price_fan_this_season: 0.0,
            last_price_fan_delta: 0.0,
            last_season_revenue: 0,
            goals: Vec::new(),
            reports: Vec::new(),
            last_win_pct: 0.5,
        }
    }

    pub fn level(&self, f: Facility) -> i32 {
        self.facilities.get(&f).copied().unwrap_or(0)
    }

    pub fn capacity(&self) -> i64 {
        self.town.capacity as i64 + 400 * self.level(Facility::Seating) as i64
    }

    pub fn concession_mult(&self) -> f64 {
        1.0 + 0.08 * self.level(Facility::Concessions) as f64
    }

    pub fn sponsor_mult(&self) -> f64 {
        1.0 + 0.06 * self.level(Facility::Scoreboard) as f64
    }

    pub fn lights_mult(&self) -> f64 {
        1.0 + 0.06 * self.level(Facility::Lights) as f64
    }

    /// Multiplier on offseason rating growth (decline is untouched).
    pub fn training_mult(&self) -> f64 {
        1.0 + 0.10 * self.level(Facility::Training) as f64
    }

    pub fn upkeep_per_game(&self) -> Money {
        Facility::ALL.iter().map(|&f| self.level(f) as Money * f.spec().upkeep_per_level).sum()
    }

    /// Cost of the next level, or `None` if maxed.
    pub fn upgrade_cost(&self, f: Facility) -> Option<Money> {
        let spec = f.spec();
        if self.level(f) >= spec.max_level { None } else { Some(spec.cost_at(self.level(f))) }
    }

    /// Buy the next level out of cash. Returns false if maxed or you can't afford it.
    pub fn upgrade(&mut self, f: Facility, week: i32, season: i32) -> bool {
        let Some(cost) = self.upgrade_cost(f) else { return false };
        if self.cash < cost || self.under_debt_covenant() {
            return false;
        }
        self.cash -= cost;
        let l = self.level(f) + 1;
        self.facilities.insert(f, l);
        self.ledger.push(LedgerEntry::new(season, week, LedgerKind::Facilities, -cost, &f.spec().name));
        true
    }

    /// Tear down the top level to stop paying its upkeep. No refund, and building it back costs full price. False if there is nothing to remove.
    pub fn downgrade(&mut self, f: Facility) -> bool {
        if self.level(f) <= 0 {
            return false;
        }
        let l = self.level(f) - 1;
        self.facilities.insert(f, l);
        true
    }

    /// The bank trusts a winner more: the line moves with last season's record.
    pub fn credit_limit(&self) -> Money {
        ((40_000.0 + self.last_season_revenue as f64 * 0.4) * (EconTuning::CREDIT_WIN_MIN + EconTuning::CREDIT_WIN_SPAN * self.last_win_pct)).round() as Money
    }

    /// Per-game interest rate: climbs as more of the credit line is drawn.
    pub fn loan_rate(&self) -> f64 {
        EconTuning::LOAN_BASE_RATE + EconTuning::LOAN_UTIL_RATE * (self.loan_balance as f64 / self.credit_limit().max(1) as f64).clamp(0.0, 1.5)
    }

    /// The bank blocks stadium spending while you are deep in debt.
    pub fn under_debt_covenant(&self) -> bool {
        self.loan_balance as f64 > self.credit_limit() as f64 * EconTuning::UPGRADE_DEBT_COVENANT
    }

    /// Borrow up to the credit limit. Interest accrues every game and a minimum payment is taken automatically.
    pub fn take_loan(&mut self, amount: Money, week: i32, season: i32) -> bool {
        if amount <= 0 || self.loan_balance + amount > self.credit_limit() {
            return false;
        }
        self.loan_balance += amount;
        self.cash += amount;
        self.ledger.push(LedgerEntry::new(season, week, LedgerKind::LoanProceeds, amount, "loan"));
        true
    }

    pub fn repay_loan(&mut self, amount: Money, week: i32, season: i32) -> bool {
        let pay = amount.clamp(0, self.loan_balance);
        if pay <= 0 || pay > self.cash {
            return false;
        }
        self.loan_balance -= pay;
        self.cash -= pay;
        self.ledger.push(LedgerEntry::new(season, week, LedgerKind::LoanPayment, -pay, "repayment"));
        true
    }

    pub fn promo_for(&self, week: i32) -> Option<&'static PromoTemplate> {
        self.promos.get(&week).and_then(|id| promo_by_id(id))
    }

    pub fn sponsor_att_mult(&self) -> f64 {
        self.sponsors.iter().fold(1.0, |a, s| a * s.effects().map(|e| e.att_mult).unwrap_or(1.0))
    }

    pub fn sponsor_fan_per_game(&self) -> f64 {
        self.sponsors.iter().map(|s| s.effects().map(|e| e.fan_per_game).unwrap_or(0.0)).sum()
    }

    pub fn sponsor_cost_per_home(&self) -> Money {
        self.sponsors.iter().map(|s| s.effects().map(|e| e.cost_per_game).unwrap_or(0)).sum()
    }

    /// Sponsor income for one game, including per-win bonuses.
    pub fn sponsor_income(&self, won: bool) -> Money {
        let sum: Money = self.sponsors.iter().map(|s| s.per_game + if won { s.effects().map(|e| e.win_bonus).unwrap_or(0) } else { 0 }).sum();
        (sum as f64 * self.sponsor_mult()).round() as Money
    }

    /// Sponsor slots the town's interest in the team supports. Deals you already hold stay if interest falls.
    pub fn sponsor_slots(&self) -> usize {
        SponsorMarket::slots_for(self.fan_interest)
    }

    /// Fan interest needed for one more slot, or `None` at the maximum.
    pub fn next_slot_interest(&self) -> Option<f64> {
        SponsorMarket::SLOT_THRESHOLDS.iter().find(|(slots, _)| *slots == self.sponsor_slots() + 1).map(|(_, need)| *need)
    }

    /// Accept a sponsor offer into a free slot.
    pub fn accept_offer(&mut self, index: usize) -> bool {
        if index >= self.offers.len() || self.sponsors.len() >= self.sponsor_slots() {
            return false;
        }
        let o = self.offers.remove(index);
        self.sponsors.push(SponsorDeal::from_offer(&o));
        true
    }

    /// Walk away from a deal; costs one game of its pay.
    pub fn cancel_sponsor(&mut self, index: usize, week: i32, season: i32) -> bool {
        if index >= self.sponsors.len() {
            return false;
        }
        let s = self.sponsors.remove(index);
        self.cash -= s.per_game;
        self.ledger.push(LedgerEntry::new(season, week, LedgerKind::Other, -s.per_game, &format!("cancel {}", s.name)));
        true
    }

    pub fn season_net(&self) -> Money {
        self.games.iter().map(|g| g.net()).sum()
    }

    /// Per-game cost of dressing this active list plus retainers for everyone else with a contract.
    pub fn projected_payroll(&self, team: &Team) -> Money {
        team.roster
            .iter()
            .filter_map(|p| self.contracts.get(&p.id).map(|c| (p, c)))
            .map(|(p, c)| if team.active_ids.contains(&p.id) { c.active_pay() } else { PayScale::retainer(c.ppg) })
            .sum()
    }

    /// New season: clear per-game books, trim old ledger, fans cool, sponsors age (performance clauses enforced).
    /// AI teams auto-renew expired sponsors; for the player, new offers appear on the market instead.
    pub fn roll_season(&mut self, rng: &mut Rng, win_pct: f64, auto_renew: bool) {
        self.last_season_revenue = self.games.iter().map(|g| g.total_revenue()).sum();
        self.last_win_pct = win_pct;
        self.games.clear();
        self.promos.clear();
        self.price_fan_this_season = 0.0;
        self.last_price_fan_delta = 0.0;
        if self.ledger.len() > 4000 {
            let cut = self.ledger.len() - 4000;
            self.ledger.drain(0..cut);
        }
        self.fan_interest += (50.0 - self.fan_interest) * EconTuning::FAN_COOLDOWN_PER_SEASON;
        for s in &mut self.sponsors {
            s.seasons_left -= 1;
        }
        let before = self.sponsors.len();
        self.sponsors.retain(|s| !(s.seasons_left <= 0 || (s.min_win_pct > 0.0 && win_pct < s.min_win_pct)));
        let lost = before - self.sponsors.len();
        if auto_renew {
            for _ in 0..lost {
                let names: Vec<String> = self.sponsors.iter().map(|s| s.name.clone()).collect();
                let clauses: Vec<String> = self.sponsors.iter().map(|s| s.clause_id.clone()).collect();
                let o = SponsorMarket::generate(&self.town, rng, 1, 0, &names, &clauses).remove(0);
                self.sponsors.push(SponsorDeal::from_offer(&o));
            }
        } else {
            let names: Vec<String> = self.sponsors.iter().map(|s| s.name.clone()).collect();
            let clauses: Vec<String> = self.sponsors.iter().map(|s| s.clause_id.clone()).collect();
            self.offers = SponsorMarket::generate(&self.town, rng, 3, 0, &names, &clauses);
        }
        self.overdue_games = 0;
    }

    pub fn status(&self, team: &Team) -> CashStatus {
        if self.bankrupt {
            CashStatus::Bankrupt
        } else if self.overdue_games >= 2 {
            CashStatus::FinalNotice
        } else if self.cash < 0 {
            CashStatus::Negative
        } else if self.cash < self.projected_payroll(team) * 2 {
            CashStatus::Low
        } else {
            CashStatus::Ok
        }
    }

    /// A copy with the ledger trimmed to the most recent `keep` entries, to keep saves small.
    pub fn for_save(&self, keep: usize) -> TeamFinance {
        let mut f = self.clone();
        if f.ledger.len() > keep {
            let cut = f.ledger.len() - keep;
            f.ledger.drain(0..cut);
        }
        f
    }
}

/// Options for the gate model; the defaults are a neutral night.
#[derive(Clone, Copy, Debug)]
pub struct AttOpts {
    pub opponent_win_pct: f64,
    pub rivalry: bool,
    pub weather: Weather,
    pub noise: f64,
    pub playoff_boost: f64,
    pub extra_mult: f64,
    pub capacity: Option<i64>,
    /// Only for `attendance_band`: use the tighter promo swing.
    pub promo: bool,
}

impl Default for AttOpts {
    fn default() -> Self {
        Self { opponent_win_pct: 0.5, rivalry: false, weather: Weather::Clear, noise: 1.0, playoff_boost: 1.0, extra_mult: 1.0, capacity: None, promo: false }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AttendanceBand {
    pub lo: i64,
    pub mid: i64,
    pub hi: i64,
}

/// Town demand model (features/economy.md sec 2, 5).
pub struct TownEconomy;

impl TownEconomy {
    /// Price a town will bear at neutral demand; wealthier towns tolerate more.
    pub fn ref_price(t: &Town) -> f64 {
        3.0 + 0.9 * t.wealth as f64
    }

    pub fn concession_per_cap(t: &Town) -> f64 {
        1.2 + 0.35 * t.wealth as f64
    }

    pub fn base_demand(t: &Town) -> f64 {
        (t.population as f64).powf(0.6) * t.turnout as f64 * 0.30
    }

    /// Loyalty sets a floor under attendance when the team stinks (still show up).
    pub fn hype(t: &Town, interest: f64) -> f64 {
        let floor = t.loyalty as f64 / 10.0 * 0.6;
        let eff = floor + (1.0 - floor) * (interest / 100.0).powf(1.2);
        0.35 + eff
    }

    pub fn weather_factor(w: Weather) -> f64 {
        match w {
            Weather::Clear => 1.0,
            Weather::Wind => 0.95,
            Weather::Rain => 0.85,
            Weather::Snow => 0.7,
        }
    }

    pub fn attendance(t: &Town, interest: f64, price: f64, o: &AttOpts) -> i64 {
        let cap = o.capacity.unwrap_or(t.capacity as i64);
        ((Self::mean_demand(t, interest, price, o) * o.noise).round() as i64).clamp(0, cap)
    }

    /// Uncapped expected crowd (price curve, town, opponent, weather). Game day multiplies this by `EconTuning::crowd_noise` then caps it.
    pub fn mean_demand(t: &Town, interest: f64, price: f64, o: &AttOpts) -> f64 {
        let opp = 0.9 + 0.2 * o.opponent_win_pct + if o.rivalry { 0.2 } else { 0.0 };
        let culture = 1.0 + (t.culture as f64 - 5.0) * 0.02;
        let price_f = (Self::ref_price(t) * EconTuning::price_tolerance(interest) / price).powf(EconTuning::price_elasticity(interest));
        Self::base_demand(t) * Self::hype(t, interest) * price_f * Self::weather_factor(o.weather) * opp * culture * o.playoff_boost * o.extra_mult
    }

    /// Typical band around the mean for the Tickets tab. `o.promo` uses the tighter promo swing.
    pub fn attendance_band(t: &Town, interest: f64, price: f64, o: &AttOpts) -> AttendanceBand {
        let cap = o.capacity.unwrap_or(t.capacity as i64);
        let raw = Self::mean_demand(t, interest, price, o);
        let (lo_m, hi_m) = if o.promo { (EconTuning::CROWD_MIN_PROMO, EconTuning::CROWD_MAX_PROMO) } else { (EconTuning::CROWD_MIN_PLAIN, EconTuning::CROWD_MAX_PLAIN) };
        AttendanceBand { lo: ((raw * lo_m).round() as i64).clamp(0, cap), mid: (raw.round() as i64).clamp(0, cap), hi: ((raw * hi_m).round() as i64).clamp(0, cap) }
    }
}

#[derive(Clone, Debug)]
pub struct GameInput {
    pub week: i32,
    pub playoff: bool,
    /// False also for ties.
    pub home_team_won: bool,
    pub tie: bool,
    pub margin: i32,
    pub weather: Weather,
    pub rivalry: bool,
    pub opponent_win_pct: f64,
    /// For this team.
    pub injury_severities: Vec<String>,
    /// Fan favorites in the lineup (home team's input), 1.0 = none.
    pub attendance_boost: f64,
}

impl GameInput {
    #[allow(clippy::too_many_arguments)]
    pub fn new(week: i32, playoff: bool, home_team_won: bool, tie: bool, margin: i32, weather: Weather, rivalry: bool, opponent_win_pct: f64) -> Self {
        Self { week, playoff, home_team_won, tie, margin, weather, rivalry, opponent_win_pct, injury_severities: Vec::new(), attendance_boost: 1.0 }
    }
}

/// Settles one game's money for both teams, updates cash, fan interest and bankruptcy.
pub struct EconomyEngine;

impl EconomyEngine {
    pub const LEAGUE_SHARE_PER_GAME: Money = 350;

    /// Revenue sharing: small stadiums get a bigger cut so tiny towns are hard, not hopeless.
    pub const SMALL_MARKET_SHARE_BELOW_CAPACITY: i32 = 4500;
    pub const SMALL_MARKET_SHARE_PER_SEAT: f64 = 1.4;
    pub const VISITOR_GATE_SHARE: f64 = 0.08;

    pub fn league_share_for(t: &Town) -> Money {
        Self::LEAGUE_SHARE_PER_GAME + ((Self::SMALL_MARKET_SHARE_BELOW_CAPACITY - t.capacity).max(0) as f64 * Self::SMALL_MARKET_SHARE_PER_SEAT).round() as Money
    }

    pub fn medical_cost(severity: &str) -> Money {
        match severity {
            "shaken" => 25,
            "minor" => 120,
            "multiWeek" => 500,
            "seasonEnding" => 1400,
            _ => 0,
        }
    }

    /// Sets up starting finances: contracts for every rostered player, two local sponsors, starting cash.
    pub fn create(team: &Team, rng: &mut Rng) -> TeamFinance {
        let t = &team.town;
        let mut fin = TeamFinance::new(t.clone(), 40_000 + (t.wealth + t.biz) as Money * 4000, EconTuning::STARTING_INTEREST, None);
        for p in &team.roster {
            let seasons = rng.weighted(&[(1, 30.0), (2, 35.0), (3, 35.0)]);
            let miles = town_miles(&p.hometown, &t.name);
            let ask = (PayScale::ask(p, t, seasons) as f64 * rng.gaussian(1.0, 0.05)).round() as Money;
            fin.contracts.insert(p.id, Contract { player_id: p.id, ppg: 20.max(ask), seasons_left: seasons, win_bonus: (ask as f64 * 0.1).round() as Money, commute: PayScale::commute(miles) });
        }
        let names = ["Gus's Tire & Muffler", "Valley Feed & Seed", "Big Don Insurance", "Pete's Pawn & Loan", "Sunrise Dental", "Delta Bail Bonds"];
        for _ in 0..2 {
            let per = (t.biz as f64 * 55.0 * rng.gaussian(1.0, 0.1)).round() as Money;
            let name = *rng.pick(&names);
            let seasons = rng.range(1, 3);
            fin.sponsors.push(SponsorDeal::new(name, per, seasons));
        }
        fin
    }

    fn travel_cost(team: &Team, opponent_town: &str) -> Money {
        let miles = town_miles(&team.town.name, opponent_town);
        let riders = 52.0; // players + a coach and a guy named Dale
        (150.0 + miles * 0.35 + riders * 2.0).round() as Money
    }

    /// Returns (home, away) finances.
    #[allow(clippy::too_many_arguments)]
    pub fn settle(rng: &mut Rng, home: &Team, away: &Team, home_fin: &mut TeamFinance, away_fin: &mut TeamFinance, hi: &GameInput, ai: &GameInput, season: i32) -> (GameFinance, GameFinance) {
        // ---------------------------------------------------------- home revenue
        // Promo night (home games only): costs money, lifts the crowd, and might go wrong.
        let promo = if hi.playoff { None } else { home_fin.promo_for(hi.week) };
        let (mut promo_att, mut promo_cap, mut promo_merch, mut promo_cost) = (1.0, 1.0, 1.0, 0);
        let mut promo_mishap = false;
        if let Some(p) = promo {
            promo_mishap = rng.chance(p.risk_chance);
            promo_att = p.attendance_mult * if promo_mishap { p.mishap_att_mult } else { 1.0 };
            promo_cap = p.per_cap_mult;
            promo_merch = p.merch_mult;
            promo_cost = p.cost(&home.town) + if promo_mishap { (p.cost(&home.town) as f64 * p.mishap_fine_mult).round() as Money } else { 0 };
        }
        let noise = EconTuning::crowd_noise(rng, promo.is_some() || hi.playoff);
        let att = TownEconomy::attendance(
            &home.town,
            home_fin.fan_interest,
            home_fin.ticket_price,
            &AttOpts {
                opponent_win_pct: hi.opponent_win_pct,
                rivalry: hi.rivalry,
                weather: hi.weather,
                noise,
                playoff_boost: if hi.playoff { 1.25 } else { 1.0 },
                capacity: Some(home_fin.capacity()),
                extra_mult: home_fin.lights_mult() * promo_att * home_fin.sponsor_att_mult() * hi.attendance_boost,
                promo: false,
            },
        );
        let gate_full = (att as f64 * home_fin.ticket_price).round() as Money;
        let visitor_share = (gate_full as f64 * Self::VISITOR_GATE_SHARE).round() as Money;
        let mut rev: BTreeMap<LedgerKind, Money> = BTreeMap::new();
        rev.insert(LedgerKind::Gate, gate_full - visitor_share);
        rev.insert(LedgerKind::Concessions, (att as f64 * TownEconomy::concession_per_cap(&home.town) * home_fin.concession_mult() * promo_cap * if hi.weather == Weather::Rain { 0.9 } else { 1.0 }).round() as Money);
        rev.insert(LedgerKind::Merch, (att as f64 * 0.35 * promo_merch * (0.5 + home_fin.fan_interest / 100.0)).round() as Money);
        rev.insert(LedgerKind::Sponsors, home_fin.sponsor_income(hi.home_team_won));
        rev.insert(LedgerKind::LeagueShare, Self::league_share_for(&home.town));
        if hi.playoff {
            rev.insert(LedgerKind::PlayoffBonus, 600);
        }
        let mut home_exp = Self::team_costs(home, home_fin, hi, true, &away.town.name, hi.home_team_won, season);
        if promo_cost > 0 {
            home_exp.insert(LedgerKind::Promo, promo_cost);
        }
        if home_fin.sponsor_cost_per_home() > 0 {
            home_exp.insert(LedgerKind::SponsorCost, home_fin.sponsor_cost_per_home());
        }

        // ---------------------------------------------------------- away revenue
        let away_won = !hi.home_team_won && !hi.tie;
        let mut arev: BTreeMap<LedgerKind, Money> = BTreeMap::new();
        arev.insert(LedgerKind::Gate, visitor_share);
        arev.insert(LedgerKind::Sponsors, away_fin.sponsor_income(away_won));
        arev.insert(LedgerKind::LeagueShare, Self::league_share_for(&away.town));
        if ai.playoff {
            arev.insert(LedgerKind::PlayoffBonus, 600);
        }
        let away_exp = Self::team_costs(away, away_fin, ai, false, &home.town.name, away_won, season);

        let hf = Self::book(home_fin, hi.week, true, att, rev, home_exp, season, promo.map(|p| p.id.clone()), promo_mishap);
        let af = Self::book(away_fin, ai.week, false, 0, arev, away_exp, season, None, false);

        Self::update_fans(home_fin, hi, hi.home_team_won);
        // Pricing only matters after a home game actually played at that price.
        home_fin.last_price_fan_delta = Self::price_fan_effect(&home.town, home_fin.ticket_price, att, home_fin.capacity(), home_fin.price_fan_this_season);
        home_fin.price_fan_this_season += home_fin.last_price_fan_delta;
        home_fin.fan_interest = (home_fin.fan_interest + home_fin.last_price_fan_delta).clamp(0.0, 100.0);
        away_fin.last_price_fan_delta = 0.0;
        Self::update_fans(away_fin, ai, away_won);
        (hf, af)
    }

    fn team_costs(team: &Team, fin: &TeamFinance, gi: &GameInput, is_home: bool, opponent_town: &str, won: bool, season: i32) -> BTreeMap<LedgerKind, Money> {
        let (mut pay, mut retain) = (0, 0);
        for p in &team.roster {
            let Some(c) = fin.contracts.get(&p.id) else { continue };
            if team.active_ids.contains(&p.id) {
                pay += c.active_pay() + if won { c.win_bonus } else { 0 };
            } else {
                retain += PayScale::retainer(c.ppg);
            }
        }
        let cap = team.town.capacity as f64;
        let ix = EconTuning::cost_index(season);
        let inflate = |v: f64| (v * ix).round() as Money;
        let mut m = BTreeMap::new();
        m.insert(LedgerKind::PlayerPay, pay);
        m.insert(LedgerKind::Retainers, retain);
        if is_home {
            m.insert(LedgerKind::Rent, inflate(150.0 + cap * 0.085));
        } else {
            m.insert(LedgerKind::Travel, inflate(Self::travel_cost(team, opponent_town) as f64));
        }
        m.insert(LedgerKind::Insurance, inflate(160.0 + (team.roster.len() * 3) as f64));
        m.insert(LedgerKind::Medical, inflate(gi.injury_severities.iter().map(|s| Self::medical_cost(s)).sum::<Money>() as f64));
        m.insert(LedgerKind::Other, inflate(180.0 + if is_home { 120.0 } else { 0.0 }));
        if fin.upkeep_per_game() > 0 {
            m.insert(LedgerKind::Upkeep, fin.upkeep_per_game());
        }
        if fin.loan_balance > 0 {
            m.insert(LedgerKind::Interest, (fin.loan_balance as f64 * fin.loan_rate()).round() as Money);
            m.insert(LedgerKind::LoanPayment, ((fin.loan_balance as f64 * EconTuning::LOAN_MIN_PAY_PCT).round() as Money).clamp(1, fin.loan_balance));
        }
        m
    }

    #[allow(clippy::too_many_arguments)]
    fn book(fin: &mut TeamFinance, week: i32, home: bool, att: i64, rev: BTreeMap<LedgerKind, Money>, exp: BTreeMap<LedgerKind, Money>, season: i32, promo_id: Option<String>, promo_mishap: bool) -> GameFinance {
        for (&k, &v) in &rev {
            if v != 0 {
                fin.ledger.push(LedgerEntry::new(season, week, k, v, ""));
            }
        }
        for (&k, &v) in &exp {
            if v != 0 {
                fin.ledger.push(LedgerEntry::new(season, week, k, -v, ""));
            }
        }
        let net: Money = rev.values().sum::<Money>() - exp.values().sum::<Money>();
        fin.cash += net;
        let lp = exp.get(&LedgerKind::LoanPayment).copied().unwrap_or(0);
        if lp > 0 {
            fin.loan_balance = (fin.loan_balance - lp).clamp(0, 1 << 40);
        }
        if fin.cash < 0 {
            fin.overdue_games += 1;
            if fin.overdue_games >= 3 {
                fin.bankrupt = true;
            }
        } else {
            fin.overdue_games = 0;
        }
        let g = GameFinance { week, home, attendance: att, capacity: fin.capacity(), revenue: rev, expenses: exp, cash_after: fin.cash, promo_id, promo_mishap };
        fin.games.push(g.clone());
        g
    }

    /// Fan-interest change from ticket pricing after a home game. Cheap seats (25%+ under the town's reference price) win goodwill
    /// when the park is at least 80% full; gouging (25%+ over) costs a little. Capped per season so it can't be farmed.
    pub fn price_fan_effect(t: &Town, price: f64, attendance: i64, capacity: i64, earned_this_season: f64) -> f64 {
        let ratio = price / TownEconomy::ref_price(t);
        let loyalty = 1.0 + (t.loyalty as f64 - 5.0) * 0.05;
        if ratio <= 0.75 {
            if capacity == 0 || (attendance as f64 / capacity as f64) < 0.8 {
                return 0.0;
            }
            let depth = ((0.75 - ratio) / 0.5).clamp(0.0, 1.0);
            let gain = 0.5 + depth * 1.0;
            let room = (6.0 - earned_this_season).clamp(0.0, 6.0);
            return (gain * loyalty).clamp(0.0, room);
        }
        if ratio >= 1.25 {
            let depth = ((ratio - 1.25) / 0.5).clamp(0.0, 1.0);
            let room = (earned_this_season + 4.0).clamp(0.0, 4.0);
            return -(0.3 + depth * 0.7).clamp(0.0, room);
        }
        0.0
    }

    fn update_fans(fin: &mut TeamFinance, gi: &GameInput, won: bool) {
        let culture = 1.0 + (fin.town.culture as f64 - 5.0) * 0.04;
        let d;
        if gi.tie {
            d = 0.0;
        } else if won {
            // A fan base that is already fired up gets less from one more win.
            let damp = (1.0 - EconTuning::WIN_DAMPING_ABOVE_50 * (fin.fan_interest - 50.0).max(0.0)).clamp(0.4, 1.0);
            d = (2.0 + (gi.margin.abs() as f64 / 7.0).min(3.0) + if gi.rivalry { 2.0 } else { 0.0 } + if gi.playoff { 2.0 } else { 0.0 }) * culture * damp;
        } else {
            d = -(2.0 + (gi.margin.abs() as f64 / 7.0).min(3.0) + if gi.rivalry { 1.5 } else { 0.0 }) * (2.0 - culture.clamp(0.8, 1.2)) * (1.0 - fin.town.loyalty as f64 / EconTuning::LOSS_LOYALTY_DIVISOR);
        }
        fin.fan_interest = (fin.fan_interest + d + fin.sponsor_fan_per_game() + (50.0 - fin.fan_interest) * EconTuning::FAN_DRIFT_PER_GAME).clamp(0.0, 100.0);
    }
}
