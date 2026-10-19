use crate::game_state::{GameState, QUARTER_SECONDS, Side, Sides};
use crate::kicking::{KickConditions, Kicking};
use crate::play_event::{DriveSummary, InjuryRecord, KeyPlay, PlayEvent, PlayType, ScoringPlay};
use crate::stats::{PlayerStats, TeamStats};
use crate::team_ctx::{DefPackage, DefUnit, OffPackage, OffUnit, TeamCtx};
use sp_core::returners::ReturnRole;
use sp_core::strategy::{DefenseFront, DefenseStrategy, OffenseScheme, OffenseStrategy, PocketStyle};
use sp_core::{Attr, Player, Position, Rng, Team, Weather};
use std::collections::{BTreeMap, HashMap, HashSet};

fn sigmoid(z: f64) -> f64 {
    1.0 / (1.0 + (-z).exp())
}

/// Outside linebackers with a rush score above `EDGE_RUSH_FLOOR` join the pass rush on their own:
/// chance = (score - floor) / span, capped at `EDGE_RUSH_MAX`.
pub const EDGE_RUSH_FLOOR: f64 = 56.0;
pub const EDGE_RUSH_SPAN: f64 = 20.0;
pub const EDGE_RUSH_MAX: f64 = 0.8;

#[derive(Clone, Copy, Debug)]
pub struct GameConfig {
    pub weather: Weather,
    /// -1 headwind .. +1 tailwind for the home side's primary direction.
    pub wind_arrow: f64,
    pub playoff: bool,
    pub keep_events: bool,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self { weather: Weather::Clear, wind_arrow: 0.0, playoff: false, keep_events: false }
    }
}

#[derive(Clone, Debug)]
pub struct GameResult {
    pub home_score: i32,
    pub away_score: i32,
    pub overtime: bool,
    pub tie: bool,
    pub team_stats: Sides<TeamStats>,
    pub player_stats: Sides<BTreeMap<u32, PlayerStats>>,
    pub drives: Vec<DriveSummary>,
    pub events: Vec<PlayEvent>,
    pub injuries: Vec<InjuryRecord>,
    pub score_by_quarter: Sides<Vec<i32>>,
    pub weather: Weather,
    pub scoring_plays: Vec<ScoringPlay>,
    pub key_plays: Vec<KeyPlay>,
    pub players: Sides<HashMap<u32, Player>>,
    pub team_names: Sides<String>,
    /// Short names (the nickname, or the town when a team has none), for text that must stay short.
    pub team_nicks: Sides<String>,
    pub end_fatigue: Sides<BTreeMap<u32, f64>>,
}

impl GameResult {
    pub fn winner(&self) -> Option<Side> {
        if self.tie {
            None
        } else if self.home_score > self.away_score {
            Some(Side::Home)
        } else {
            Some(Side::Away)
        }
    }

    pub fn score(&self, side: Side) -> i32 {
        match side {
            Side::Home => self.home_score,
            Side::Away => self.away_score,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum End {
    Touchdown,
    FieldGoal,
    Safety,
    Turnover,
    Punt,
    Downs,
    MissedFg,
}

impl End {
    fn name(self) -> &'static str {
        match self {
            End::Touchdown => "touchdown",
            End::FieldGoal => "fieldGoal",
            End::Safety => "safety",
            End::Turnover => "turnover",
            End::Punt => "punt",
            End::Downs => "downs",
            End::MissedFg => "missedFg",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FourthChoice {
    Go,
    Punt,
    FieldGoal,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ClockType {
    Run,
    Completion,
    Incomplete,
    Sack,
    Kick,
    Score,
}

#[derive(Clone, Copy)]
struct DriveEnd {
    kind: End,
    scorer: Option<Side>,
    end_half: bool,
    end_game: bool,
}

impl DriveEnd {
    fn new(kind: End) -> Self {
        Self { kind, scorer: None, end_half: false, end_game: false }
    }

    fn scored(kind: End, scorer: Side) -> Self {
        Self { kind, scorer: Some(scorer), end_half: false, end_game: false }
    }
}

struct Step {
    end: Option<DriveEnd>,
    clock_type: ClockType,
    hit: bool,
}

impl Step {
    fn new(clock_type: ClockType) -> Self {
        Self { end: None, clock_type, hit: false }
    }


    fn ending(end: DriveEnd, clock_type: ClockType, hit: bool) -> Self {
        Self { end: Some(end), clock_type, hit }
    }
}

type Roles<'a> = Vec<(&'static str, &'a str)>;

/// A weighted pick list that keeps insertion order and overwrites on a repeated key (like a Dart map).
struct Weights<'a> {
    items: Vec<(&'a Player, f64)>,
}

impl<'a> Weights<'a> {
    fn new() -> Self {
        Self { items: Vec::new() }
    }

    fn set(&mut self, p: &'a Player, w: f64) {
        match self.items.iter_mut().find(|(x, _)| x.id == p.id) {
            Some(slot) => slot.1 = w,
            None => self.items.push((p, w)),
        }
    }

    fn remove(&mut self, p: &Player) {
        self.items.retain(|(x, _)| x.id != p.id);
    }

    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn pick(&self, rng: &mut Rng) -> &'a Player {
        rng.weighted(&self.items)
    }
}

// Scoring formulas shared by several plays. They only read a team context, so they never fight the borrow checker.
fn run_block_score(c: &TeamCtx, p: &Player) -> f64 {
    0.45 * c.ev(p, Attr::Strength) + 0.2 * c.ev(p, Attr::Intelligence) + 0.15 * c.ev(p, Attr::Agility) + 0.1 * c.ev(p, Attr::Acceleration) + 0.1 * c.ev(p, Attr::Discipline)
}

fn run_front_score(c: &TeamCtx, p: &Player) -> f64 {
    0.4 * c.ev(p, Attr::Strength) + 0.25 * c.ev(p, Attr::Intelligence) + 0.15 * c.ev(p, Attr::Acceleration) + 0.1 * c.ev(p, Attr::Agility) + 0.1 * c.ev(p, Attr::Speed)
}

fn pass_block_score(c: &TeamCtx, p: &Player) -> f64 {
    0.35 * c.ev(p, Attr::Strength) + 0.2 * c.ev(p, Attr::Agility) + 0.15 * c.ev(p, Attr::Acceleration) + 0.3 * c.ev(p, Attr::Intelligence)
}

fn pass_rush_score(c: &TeamCtx, p: &Player) -> f64 {
    0.3 * c.ev(p, Attr::Acceleration) + 0.2 * c.ev(p, Attr::Agility) + 0.3 * c.ev(p, Attr::Strength) + 0.2 * c.ev(p, Attr::Intelligence)
}

fn sep_score(c: &TeamCtx, p: &Player) -> f64 {
    0.3 * c.ev(p, Attr::Speed) + 0.25 * c.ev(p, Attr::Acceleration) + 0.3 * c.ev(p, Attr::Agility) + 0.15 * c.ev(p, Attr::Strength)
}

fn cov_score(c: &TeamCtx, p: &Player) -> f64 {
    0.25 * c.ev(p, Attr::Speed) + 0.2 * c.ev(p, Attr::Acceleration) + 0.25 * c.ev(p, Attr::Agility) + 0.2 * c.ev(p, Attr::Intelligence) + 0.1 * c.ev(p, Attr::Strength)
}

fn mean(xs: impl Iterator<Item = f64>) -> f64 {
    let (mut sum, mut n) = (0.0, 0usize);
    for x in xs {
        sum += x;
        n += 1;
    }
    sum / n as f64
}

/// Drive-level 11v11 sim. Deterministic by `Rng`. Control is strategy only (D9).
pub struct GameSim<'a> {
    rng: Rng,
    config: GameConfig,
    s: GameState,
    t: [TeamCtx<'a>; 2],
    drives: Vec<DriveSummary>,
    events: Vec<PlayEvent>,
    injuries: Vec<InjuryRecord>,
    scoring_plays: Vec<ScoringPlay>,
    key_plays: Vec<KeyPlay>,
    score_by_quarter: Sides<Vec<i32>>,
    kc: KickConditions,
}

impl<'a> GameSim<'a> {
    pub fn new(rng: Rng, home: &'a Team, away: &'a Team, config: GameConfig) -> Self {
        Self::with_unavailable(rng, home, away, config, &HashSet::new(), &HashSet::new())
    }

    pub fn with_unavailable(rng: Rng, home: &'a Team, away: &'a Team, config: GameConfig, home_unavailable: &HashSet<u32>, away_unavailable: &HashSet<u32>) -> Self {
        Self {
            rng,
            config,
            s: GameState::default(),
            t: [TeamCtx::new(home, Side::Home, home_unavailable), TeamCtx::new(away, Side::Away, away_unavailable)],
            drives: Vec::new(),
            events: Vec::new(),
            injuries: Vec::new(),
            scoring_plays: Vec::new(),
            key_plays: Vec::new(),
            score_by_quarter: Sides::new(vec![0; 6], vec![0; 6]),
            kc: KickConditions { weather: config.weather, wind_arrow: config.wind_arrow },
        }
    }

    fn c(&self, s: Side) -> &TeamCtx<'a> {
        &self.t[s.idx()]
    }

    fn st(&mut self, s: Side, p: &Player) -> &mut PlayerStats {
        self.t[s.idx()].st(p)
    }

    fn ev(&self, s: Side, p: &Player, a: Attr) -> f64 {
        self.t[s.idx()].ev(p, a)
    }

    // ---------------------------------------------------------------- driver

    pub fn simulate(mut self) -> GameResult {
        let opening = if self.rng.chance(0.5) { Side::Home } else { Side::Away };
        self.kickoff(opening.other());
        let mut first_half_done = false;
        let opening_receiver = opening;

        loop {
            let result = self.run_drive(false);
            self.after_drive(result);
            if result.end_game {
                break;
            }
            if result.end_half && !first_half_done {
                first_half_done = true;
                for c in &mut self.t {
                    c.halftime_recovery();
                }
                self.kickoff(opening_receiver);
            }
        }

        let mut overtime = false;
        if self.s.home_score == self.s.away_score {
            overtime = true;
            self.overtime();
        }

        let [home, away] = &self.t;
        GameResult {
            home_score: self.s.home_score,
            away_score: self.s.away_score,
            overtime,
            tie: self.s.home_score == self.s.away_score,
            team_stats: Sides::new(home.team_stats.clone(), away.team_stats.clone()),
            player_stats: Sides::new(home.stats.clone(), away.stats.clone()),
            drives: self.drives.clone(),
            events: self.events.clone(),
            injuries: self.injuries.clone(),
            score_by_quarter: self.score_by_quarter.clone(),
            weather: self.config.weather,
            scoring_plays: self.scoring_plays.clone(),
            key_plays: self.key_plays.clone(),
            players: Sides::new(
                home.pool.iter().map(|&p| (p.id, p.clone())).collect(),
                away.pool.iter().map(|&p| (p.id, p.clone())).collect(),
            ),
            team_names: Sides::new(home.team.town.team_name(), away.team.town.team_name()),
            team_nicks: Sides::new(Self::nick(home.team), Self::nick(away.team)),
            end_fatigue: Sides::new(home.fatigue.clone(), away.fatigue.clone()),
        }
    }

    fn nick(team: &Team) -> String {
        if team.town.nickname.is_empty() { team.town.name.clone() } else { team.town.nickname.clone() }
    }

    fn after_drive(&mut self, d: DriveEnd) {
        let restart = !d.end_half && !d.end_game;
        match d.kind {
            End::Touchdown => {
                self.after_touchdown(d.scorer.unwrap());
                if restart {
                    self.kickoff(d.scorer.unwrap());
                }
            }
            End::FieldGoal => {
                if restart {
                    self.kickoff(d.scorer.unwrap());
                }
            }
            End::Safety => {
                if restart {
                    self.kickoff(d.scorer.unwrap().other());
                }
            }
            End::Turnover | End::Punt | End::Downs | End::MissedFg => {} // possession already set
        }
    }

    fn score(&mut self, side: Side, pts: i32, desc: Option<String>) {
        self.s.add_score(side, pts);
        let q = (self.s.quarter - 1).clamp(0, 5) as usize;
        self.score_by_quarter[side][q] += pts;
        if let Some(text) = desc {
            self.scoring_plays.push(ScoringPlay { quarter: self.s.quarter, clock: self.s.clock, side, points: pts, text, home_score: self.s.home_score, away_score: self.s.away_score });
        }
    }

    fn key(&mut self, side: Side, text: String, flair: &[&str]) {
        let extra = if !flair.is_empty() && self.rng.chance(0.35) { format!(" {}", self.rng.pick(flair)) } else { String::new() };
        self.key_plays.push(KeyPlay { quarter: self.s.quarter, clock: self.s.clock, side, text: format!("{text}{extra}") });
    }

    fn overtime(&mut self) {
        // D20: each team starts at own 30, both get a possession, then next score wins. One 15-min period for
        // regular season (tie if still level); playoffs continue until a winner.
        self.s.overtime = true;
        let first = if self.rng.chance(0.5) { Side::Home } else { Side::Away };
        let max_periods = if self.config.playoff { 8 } else { 1 };
        for period in 1..=max_periods {
            self.s.quarter = 4 + period;
            self.s.clock = QUARTER_SECONDS;
            let mut side = if period % 2 == 1 { first } else { first.other() };
            let mut possessions = 0;
            while self.s.clock > 0 {
                self.s.possession = side;
                self.s.yard_line = 30;
                self.s.first_down();
                self.run_drive(true);
                possessions += 1;
                if possessions >= 2 && self.s.home_score != self.s.away_score {
                    return;
                }
                side = side.other();
            }
            if self.s.home_score != self.s.away_score {
                return;
            }
        }
    }

    // ------------------------------------------------------------- kickoff

    fn kickoff(&mut self, kicking: Side) {
        let (ki, ri) = (kicking.idx(), kicking.other().idx());
        let receiving = kicking.other();
        let kdiff = self.s.lead(kicking);
        let desperate = !self.s.overtime && self.s.quarter == 4 && self.s.clock < 240 && kdiff < 0 && kdiff >= -16;
        let onside_tendency = self.t[ki].team.strategy.special_teams.onside_tendency;
        let onside = if desperate { self.rng.chance(0.75) } else { self.rng.chance(onside_tendency) };
        let kicker = self.t[ki].kicker();
        if onside {
            let recover = self.rng.chance(0.14 + 0.002 * (kicker.ratings[Attr::Discipline] as f64 - 55.0));
            self.t[ki].st(kicker).inc("onside_att");
            if recover {
                self.t[ki].st(kicker).inc("onside_rec");
                self.s.possession = kicking;
                self.s.yard_line = 52;
                let town = self.t[ki].team.town.name.clone();
                self.key(kicking, format!("Onside kick recovered by {town}!"), &["Nobody on the other sideline saw it coming. Everybody else did."]);
            } else {
                self.s.possession = receiving;
                self.s.yard_line = 48;
            }
            self.s.first_down();
            self.t[ki].apply_fatigue(&[kicker], 0.3);
            return;
        }
        let ret = self.t[ri].returner_for(ReturnRole::Kick);
        let res = Kicking::kickoff(&mut self.rng, kicker, Some(ret), &self.kc);
        self.t[ki].st(kicker).inc("kickoffs");
        if res.touchback {
            self.t[ki].st(kicker).inc("kick_tb");
        } else {
            let st = self.t[ri].st(ret);
            st.inc("kr");
            st.add("kr_yds", res.return_yards as f64);
            st.set_max("kr_long", res.return_yards as f64);
        }
        self.s.possession = receiving;
        self.s.yard_line = res.spot;
        self.s.first_down();
        self.t[ki].apply_fatigue(&[kicker], 0.3);
        if res.td {
            self.t[ri].st(ret).inc("kr_td");
            self.score(receiving, 6, Some(format!("{} {}-yd kickoff return", ret.name, res.return_yards)));
            self.key(receiving, format!("{} takes the kickoff {} yards to the house.", ret.name, res.return_yards), &["The coverage team is still looking for him."]);
            self.extra_point(receiving);
            self.kickoff(receiving);
        }
    }

    /// After a touchdown: XP or 2-point attempt (late-game situations).
    fn after_touchdown(&mut self, side: Side) {
        let diff = self.s.lead(side);
        let go2 = !self.s.overtime && self.s.quarter == 4 && self.s.clock < 600 && [-2, -5, -8, 1, 5, 8].contains(&diff);
        if !go2 {
            self.extra_point(side);
            return;
        }
        let ou = self.c(side).offense(OffPackage::Heavy);
        let qb = ou.qb;
        let line_str = ou.ol.iter().map(|x| self.ev(side, x, Attr::Strength)).sum::<f64>() / ou.ol.len() as f64;
        let p = (0.46 + 0.002 * (self.ev(side, qb, Attr::Discipline) - 55.0) + 0.002 * (line_str - 60.0)).clamp(0.2, 0.7);
        self.st(side, qb).inc("twopt_att");
        if self.rng.chance(p) {
            self.st(side, qb).inc("twopt_made");
            self.score(side, 2, Some(format!("{} two-point conversion", qb.name)));
        }
    }

    fn extra_point(&mut self, side: Side) {
        let kicker = self.c(side).kicker();
        let good = self.rng.chance(Kicking::xp_probability(kicker, &self.kc));
        self.t[side.idx()].team_stats.xp_att += 1;
        self.st(side, kicker).inc("xpa");
        if good {
            self.t[side.idx()].team_stats.xp_made += 1;
            self.st(side, kicker).inc("xpm");
            self.score(side, 1, Some(format!("{} extra point", kicker.name)));
        }
    }

    // ---------------------------------------------------------- drive loop

    fn run_drive(&mut self, ot: bool) -> DriveEnd {
        let off = self.s.possession;
        let mut drive = DriveSummary::new(off, self.s.quarter, self.s.clock, self.s.yard_line);
        self.t[off.idx()].team_stats.drives += 1;
        let ts0 = &self.t[off.idx()].team_stats;
        let base = ts0.total_yards() - ts0.sack_yards_lost;
        let end = self.drive_loop(&mut drive, off, base, ot);
        self.drives.push(drive);
        end
    }

    fn drive_loop(&mut self, drive: &mut DriveSummary, off: Side, base: i32, ot: bool) -> DriveEnd {
        let mut plays = 0;
        loop {
            let tally = |sim: &Self| {
                let ts = &sim.t[off.idx()].team_stats;
                ts.total_yards() - ts.sack_yards_lost - base
            };
            if plays > 40 {
                drive.yards = tally(self);
                drive.result = "runaway".into();
                self.s.change_possession(100 - self.s.yard_line);
                return DriveEnd::new(End::Turnover);
            }
            let q0 = self.s.quarter;
            let clock_before = self.s.clock;
            let step = self.play_once(ot);
            plays += 1;
            drive.plays += 1;
            // time of possession
            let burned = if q0 == self.s.quarter { clock_before - self.s.clock } else { clock_before };
            self.t[off.idx()].team_stats.seconds_of_possession += burned.max(0);

            if let Some(end) = step.end {
                drive.yards = tally(self);
                drive.result = end.kind.name().into();
                return self.apply_clock(end, ot, q0);
            }
            if self.clock_expired_past(q0, ot) {
                drive.yards = tally(self);
                drive.result = if q0 == 2 { "half" } else { "end" }.into();
                return DriveEnd { kind: End::Turnover, scorer: None, end_half: q0 == 2, end_game: q0 >= 4 || ot };
            }
        }
    }

    fn clock_expired_past(&self, q0: i32, ot: bool) -> bool {
        if ot {
            return self.s.clock <= 0;
        }
        (q0 == 2 && self.s.quarter == 3) || (q0 == 4 && self.s.quarter == 5)
    }

    fn apply_clock(&self, e: DriveEnd, ot: bool, q0: i32) -> DriveEnd {
        let half = !ot && q0 == 2 && self.s.quarter == 3;
        let end = (!ot && q0 == 4 && self.s.quarter == 5) || (ot && self.s.clock <= 0);
        DriveEnd { kind: e.kind, scorer: e.scorer, end_half: half, end_game: end }
    }

    // --------------------------------------------------------- one snap

    fn play_once(&mut self, _ot: bool) -> Step {
        let off = self.s.possession;
        let def = off.other();
        let strat = self.c(off).team.strategy;

        // Victory formation.
        if self.should_kneel() {
            return self.kneel(off);
        }

        // 4th down decision.
        if self.s.down == 4 {
            match self.fourth_down(off, &strat.offense) {
                FourthChoice::FieldGoal => return self.field_goal(off),
                FourthChoice::Punt => return self.punt(off, def),
                FourthChoice::Go => {}
            }
        }

        // Pre-snap penalty check.
        if let Some(pen) = self.penalty_check(off, def) {
            return pen;
        }

        let pass_prob = self.pass_probability(&strat.offense);
        let is_pass = self.rng.chance(pass_prob);
        let off_pkg = self.off_package(&strat.offense, is_pass);
        let def_strategy = self.c(def).team.strategy.defense;
        let def_pkg = self.def_package(&def_strategy, is_pass);
        let mut ou = self.c(off).offense(off_pkg);
        let du = self.c(def).defense(def_pkg);

        let step = if is_pass {
            self.pass_play(off, def, &ou, &du, &strat.offense, &def_strategy)
        } else {
            self.run_play(off, def, &mut ou, &du, &strat.offense, &def_strategy)
        };

        // Fatigue, snaps, injuries.
        let intensity = if is_pass { 1.0 } else { 1.1 };
        let on_off = ou.all();
        let on_def = du.all();
        self.t[off.idx()].apply_fatigue(&on_off, intensity);
        self.t[def.idx()].apply_fatigue(&on_def, intensity);
        for p in &on_off {
            self.st(off, p).inc("snaps_off");
        }
        for p in &on_def {
            self.st(def, p).inc("snaps_def");
        }
        self.injury_rolls(off, &on_off, step.hit);
        self.injury_rolls(def, &on_def, step.hit);

        // Clock
        self.burn_clock(step.clock_type, &strat.offense);
        step
    }

    fn burn_clock(&mut self, ty: ClockType, o: &OffenseStrategy) {
        let mut secs = match ty {
            ClockType::Run => self.rng.gaussian(31.0, 6.0),
            ClockType::Completion => self.rng.gaussian(27.0, 6.0),
            ClockType::Incomplete => self.rng.gaussian(8.0, 2.0),
            ClockType::Sack => self.rng.gaussian(29.0, 6.0),
            ClockType::Kick => self.rng.gaussian(10.0, 3.0),
            ClockType::Score => self.rng.gaussian(12.0, 4.0),
        };
        // Late in the game a trailing offense hurries (shorter plays) and a leading one runs the clock down.
        // (The Dart sim had this backwards: its `trailing` flag was true for the *leading* team.)
        let trailing = self.s.lead(self.s.possession) < 0;
        let late = self.s.quarter >= 4 && self.s.clock < 300;
        if late && trailing && ty != ClockType::Incomplete {
            secs *= 0.6;
        }
        if late && !trailing && self.s.home_score != self.s.away_score && ty != ClockType::Incomplete {
            secs *= 1.25;
        }
        secs *= 1.25 - 0.5 * o.tempo; // tempo 0.5 -> 1.0x
        self.s.run_clock((secs.round() as i32).clamp(2, 45));
    }

    fn should_kneel(&self) -> bool {
        if self.s.overtime || self.s.quarter != 4 || self.s.down > 3 {
            return false;
        }
        let lead = self.s.lead(self.s.possession);
        if lead <= 0 {
            return false;
        }
        self.s.clock <= 40 * (4 - self.s.down) + 20
    }

    fn kneel(&mut self, off: Side) -> Step {
        let qb = self.c(off).offense(OffPackage::Base).qb;
        self.st(off, qb).inc("kneels");
        self.event(PlayType::Kneel, -1, "kneel", &[("qb", &qb.name)]);
        self.s.yard_line = (self.s.yard_line - 1).clamp(1, 99);
        self.s.to_go += 1;
        self.s.down += 1;
        self.s.run_clock(40);
        self.check_downs(ClockType::Run, false)
    }

    // ----------------------------------------------------- play selection

    fn pass_probability(&self, o: &OffenseStrategy) -> f64 {
        let s = &self.s;
        let mut p = 0.08 + 0.78 * o.run_pass_bias;
        p += match o.scheme {
            OffenseScheme::AirRaid => 0.05,
            OffenseScheme::Spread => 0.04,
            OffenseScheme::GroundAndPound => -0.12,
            OffenseScheme::Option => -0.08,
            _ => 0.0,
        };
        if s.to_go >= 8 {
            p += 0.15;
        }
        if s.to_go >= 4 && s.to_go < 8 {
            p += 0.04;
        }
        if s.to_go <= 2 {
            p -= 0.25;
        }
        let diff = s.lead(s.possession);
        let late = s.quarter >= 4 && s.clock < 420;
        if late && diff < -3 {
            p += 0.25;
        }
        if late && diff > 3 {
            p -= 0.25;
        }
        if s.yards_to_end_zone() <= 3 {
            p -= 0.2;
        }
        p.clamp(0.08, 0.92)
    }

    fn off_package(&mut self, o: &OffenseStrategy, is_pass: bool) -> OffPackage {
        if self.s.yards_to_end_zone() <= 3 && !is_pass {
            return OffPackage::Heavy;
        }
        if o.scheme == OffenseScheme::AirRaid && is_pass && self.rng.chance(0.5) {
            return OffPackage::Spread;
        }
        if o.scheme == OffenseScheme::GroundAndPound && self.rng.chance(0.45) {
            return if self.rng.chance(0.4) { OffPackage::Heavy } else { OffPackage::TwoTe };
        }
        if !is_pass && self.rng.chance(0.25) {
            return OffPackage::TwoTe;
        }
        OffPackage::Base
    }

    fn def_package(&self, d: &DefenseStrategy, is_pass: bool) -> DefPackage {
        let s = &self.s;
        if s.yards_to_end_zone() <= 3 {
            return DefPackage::GoalLine;
        }
        if d.front == DefenseFront::ThreeFour {
            return DefPackage::ThreeFour;
        }
        if d.front == DefenseFront::Nickel || (is_pass && s.to_go >= 6 && s.down >= 2) {
            return DefPackage::Nickel;
        }
        DefPackage::FourThree
    }

    fn fourth_down(&self, off: Side, strat: &OffenseStrategy) -> FourthChoice {
        let st = self.c(off).team.strategy;
        let s = &self.s;
        let to_ez = s.yards_to_end_zone();
        let fg_dist = to_ez + 17;
        let diff = s.lead(s.possession);
        let late = s.quarter >= 4 && s.clock < 300;
        let aggr = strat.fourth_down_aggression;
        let go_for = s.to_go <= 1 + (aggr * 3.0).round() as i32 && s.yard_line >= 40;
        let in_range = fg_dist as f64 <= st.special_teams.field_goal_range_limit;
        let desperate = late && diff < 0 && s.to_go <= 8 && !(in_range && diff >= -3);
        if desperate {
            return FourthChoice::Go;
        }
        if in_range {
            if go_for && s.to_go <= 1 && aggr > 0.6 {
                return FourthChoice::Go;
            }
            return FourthChoice::FieldGoal;
        }
        if go_for { FourthChoice::Go } else { FourthChoice::Punt }
    }

    // ------------------------------------------------------------ kicks

    fn field_goal(&mut self, off: Side) -> Step {
        let kicker = self.c(off).kicker();
        let res = Kicking::field_goal(&mut self.rng, kicker, self.s.yards_to_end_zone(), &self.kc);
        self.t[off.idx()].team_stats.fg_att += 1;
        self.st(off, kicker).inc("fga");
        self.t[off.idx()].apply_fatigue(&[kicker], 0.3);
        let dist = res.distance.to_string();
        self.event(PlayType::FieldGoal, 0, if res.good { "fg_good" } else { "fg_miss" }, &[("kicker", &kicker.name), ("dist", &dist)]);
        let o = self.c(off).team.strategy.offense;
        if res.good {
            self.t[off.idx()].team_stats.fg_made += 1;
            let st = self.st(off, kicker);
            st.inc("fgm");
            st.set_max("fg_long", res.distance as f64);
            self.score(off, 3, Some(format!("{} {}-yd field goal", kicker.name, res.distance)));
            if res.distance >= 50 {
                self.key(off, format!("{} hits from {}.", kicker.name, res.distance), &["The chain gang applauded. Politely."]);
            }
            self.burn_clock(ClockType::Kick, &o);
            return Step::ending(DriveEnd::scored(End::FieldGoal, off), ClockType::Kick, false);
        }
        self.burn_clock(ClockType::Kick, &o);
        self.s.change_possession((100 - self.s.yard_line).clamp(20, 80));
        Step::ending(DriveEnd::new(End::MissedFg), ClockType::Kick, false)
    }

    fn punt(&mut self, off: Side, def: Side) -> Step {
        let punter = self.c(off).punter();
        let dist = Kicking::punt_distance(&mut self.rng, punter, &self.kc);
        let o = self.c(off).team.strategy.offense;
        {
            let ts = &mut self.t[off.idx()].team_stats;
            ts.punts += 1;
            ts.punt_yards += dist;
        }
        let st = self.st(off, punter);
        st.inc("punts");
        st.add("punt_yds", dist as f64);
        self.t[off.idx()].apply_fatigue(&[punter], 0.3);
        let landing = self.s.yard_line + dist;
        let new_spot;
        if landing >= 100 {
            new_spot = 20;
            self.st(off, punter).inc("punt_tb");
        } else {
            let land_opp = 100 - landing;
            let ret = self.c(def).returner_for(ReturnRole::Punt);
            let speed = ret.ratings[Attr::Speed] as f64;
            let ret_yards = (self.rng.gaussian(5.0 + (speed - 50.0) * 0.08, 5.0).round() as i32).clamp(-2, 40);
            new_spot = (land_opp + ret_yards).clamp(1, 99);
            if land_opp <= 20 {
                self.st(off, punter).inc("punt_in20");
            }
            self.st(def, ret).inc("pr");
            // A rare breakaway: the returner goes the whole way.
            if self.rng.chance(0.004 + 0.00015 * (speed - 55.0).clamp(-20.0, 40.0)) {
                let td_yards = 100 - land_opp;
                let st = self.st(def, ret);
                st.add("pr_yds", td_yards as f64);
                st.set_max("pr_long", td_yards as f64);
                st.inc("pr_td");
                self.event(PlayType::Punt, dist, "punt", &[("punter", &punter.name)]);
                self.burn_clock(ClockType::Kick, &o);
                self.score(def, 6, Some(format!("{} {}-yd punt return", ret.name, td_yards)));
                self.key(
                    def,
                    format!("{} takes the punt {} yards to the house.", ret.name, td_yards),
                    &["The punt team is still looking for him.", "The punter would like a word with his gunners."],
                );
                self.s.change_possession(50);
                self.after_touchdown(def);
                self.s.possession = off; // the punting team gets the ball back on the kickoff
                self.kickoff(def);
                return Step::ending(DriveEnd::new(End::Punt), ClockType::Kick, false);
            }
            let st = self.st(def, ret);
            st.add("pr_yds", ret_yards as f64);
            st.set_max("pr_long", ret_yards as f64);
        }
        self.event(PlayType::Punt, dist, "punt", &[("punter", &punter.name)]);
        self.burn_clock(ClockType::Kick, &o);
        self.s.change_possession(new_spot);
        Step::ending(DriveEnd::new(End::Punt), ClockType::Kick, false)
    }

    // -------------------------------------------------------- penalties

    fn penalty_check(&mut self, off: Side, def: Side) -> Option<Step> {
        let avg_di = |ps: &[&Player]| if ps.is_empty() { 50.0 } else { ps.iter().map(|p| p.ratings[Attr::Discipline] as f64).sum::<f64>() / ps.len() as f64 };
        let o_line: Vec<&'a Player> = self.c(off).pool.iter().copied().filter(|p| matches!(p.position, Position::Ot | Position::Og | Position::C)).collect();
        let d_front: Vec<&'a Player> = self.c(def).pool.iter().copied().filter(|p| matches!(p.position, Position::De | Position::Dt | Position::Cb | Position::S)).collect();
        let p_off = 0.043 * (1.0 + (55.0 - avg_di(&o_line)) / 90.0);
        let p_def = 0.034 * (1.0 + (55.0 - avg_di(&d_front)) / 90.0);
        let r = self.rng.next_f64();
        if r < p_off {
            let holding = self.rng.chance(0.45);
            let yards = if holding { 10 } else { 5 };
            let culprit = self.weighted_by_low_di(&o_line);
            Some(self.apply_penalty(off, culprit, if holding { "holding" } else { "false_start" }, -yards, false))
        } else if r < p_off + p_def {
            let culprit = self.weighted_by_low_di(&d_front);
            let kind = self.rng.weighted(&[("offside", 40.0), ("def_holding", 30.0), ("pass_interference", 22.0), ("roughing", 8.0)]);
            let yards = match kind {
                "offside" | "def_holding" => 5,
                "roughing" => 15,
                _ => self.rng.range(8, 25),
            };
            Some(self.apply_penalty(def, culprit, kind, yards, kind != "offside"))
        } else {
            None
        }
    }

    fn weighted_by_low_di(&mut self, ps: &[&'a Player]) -> &'a Player {
        if ps.is_empty() {
            return self.c(self.s.possession).pool[0];
        }
        let m: Vec<(&'a Player, f64)> = ps.iter().map(|&p| (p, 110.0 - p.ratings[Attr::Discipline] as f64)).collect();
        self.rng.weighted(&m)
    }

    fn apply_penalty(&mut self, culprit_team: Side, culprit: &Player, kind: &str, yards: i32, auto_first: bool) -> Step {
        let off = self.s.possession;
        let offense_penalty = culprit_team == off;
        let mut gain = yards;
        if !offense_penalty {
            let to_ez = self.s.yards_to_end_zone();
            let upper = if to_ez / 2 == 0 { 1 } else { (to_ez as f64 / 2.0).ceil() as i32 };
            gain = yards.clamp(0, upper);
        }
        {
            let ts = &mut self.t[culprit_team.idx()].team_stats;
            ts.penalties += 1;
            ts.penalty_yards += gain.abs();
        }
        let st = self.st(culprit_team, culprit);
        st.inc("penalties");
        st.add("pen_yds", gain.abs() as f64);
        self.event(PlayType::Penalty, gain, kind, &[("on", &culprit.name)]);
        self.s.yard_line = (self.s.yard_line + gain).clamp(1, 99);
        if offense_penalty {
            self.s.to_go -= gain;
            if self.s.to_go > 40 {
                self.s.to_go = 40;
            }
        } else {
            self.s.to_go -= gain;
            if auto_first || self.s.to_go <= 0 {
                self.s.first_down();
            }
        }
        let secs = self.rng.range(5, 10);
        self.s.run_clock(secs);
        Step::new(ClockType::Incomplete)
    }

    // -------------------------------------------------------- run play

    fn run_play(&mut self, o: Side, d: Side, ou: &mut OffUnit<'a>, du: &DefUnit<'a>, os: &OffenseStrategy, ds: &DefenseStrategy) -> Step {
        let (oi, di) = (o.idx(), d.idx());
        let is_qb_run = os.scheme == OffenseScheme::Option && self.rng.chance(0.18);
        let carrier = if is_qb_run { ou.qb } else { self.pick_rusher(o, ou) };

        let mut front: Vec<&'a Player> = du.dl.clone();
        front.extend(&du.lb);
        let mut block_avg = mean(ou.ol.iter().map(|p| run_block_score(&self.t[oi], p)));
        if !ou.tes.is_empty() {
            let te_avg = mean(ou.tes.iter().map(|p| run_block_score(&self.t[oi], p)));
            block_avg += 0.8 * (te_avg - block_avg) / 5.0;
        }
        let front_avg = mean(front.iter().map(|p| run_front_score(&self.t[di], p)));
        let commit = ds.run_stop_commitment;
        let lane = (block_avg - front_avg) - 3.0 * (commit - 0.5);

        // Matchups: pair OL i vs front i for pancakes / penetration credit.
        let pairs = ou.ol.len().min(du.dl.len() + du.lb.len());
        let mut penetrations = 0;
        let mut penetrator: Option<&'a Player> = None;
        for i in 0..pairs {
            let b = ou.ol[i];
            let f = front[i % front.len()];
            let margin = run_block_score(&self.t[oi], b) - run_front_score(&self.t[di], f) + self.rng.gaussian(0.0, 8.0);
            if margin > 15.0 {
                self.t[oi].st(b).inc("pancakes");
            } else if margin < -15.0 {
                penetrations += 1;
                penetrator.get_or_insert(f);
                self.t[di].st(f).inc("blocks_beaten");
            }
        }

        let skill = (self.ev(o, carrier, Attr::Acceleration) + self.ev(o, carrier, Attr::Agility) + self.ev(o, carrier, Attr::Speed)) / 3.0;
        let mut mean_yards = 3.2 + 0.075 * lane + 0.04 * (skill - 55.0);
        mean_yards -= 0.9 * penetrations as f64;
        if os.scheme == OffenseScheme::GroundAndPound {
            mean_yards += 0.5;
        }
        if os.scheme == OffenseScheme::Option {
            mean_yards += 0.5;
        }
        let mut yards = self.rng.gaussian(mean_yards, 3.4);
        let breakaway = 0.035 + 0.002 * (self.ev(o, carrier, Attr::Speed) - 55.0).clamp(-20.0, 40.0) - 0.04 * (commit - 0.5);
        if self.rng.chance(breakaway) {
            yards += self.rng.gaussian(15.0, 8.0).abs();
        }
        let broke_p = 0.14 + 0.004 * (self.ev(o, carrier, Attr::Strength) + self.ev(o, carrier, Attr::Agility) - 100.0).clamp(-30.0, 50.0);
        let broke_tackle = self.rng.chance(broke_p);
        if broke_tackle && yards > 0.0 {
            yards += self.rng.gaussian(2.5, 1.5).abs();
            self.st(o, carrier).inc("broken_tackles");
        }
        let y = (yards.round() as i32).clamp(-9, 99);

        // Fumble
        let mut fumble_p = 0.017 + (55.0 - self.ev(o, carrier, Attr::Hands)) * 0.0003 + 0.006 * (ds.turnover_hunting - 0.3);
        if self.config.weather == Weather::Rain {
            fumble_p *= 1.6;
        }
        if self.config.weather == Weather::Snow {
            fumble_p *= 1.3;
        }
        let tackler = self.pick_tackler(d, du, true, y < 0);

        let mut ev: Roles<'a> = vec![("ball_carrier", carrier.name.as_str()), ("tackler", tackler.name.as_str())];
        if let Some(p) = penetrator {
            ev.push(("blocked_by_beaten", p.name.as_str()));
        }

        self.t[oi].team_stats.plays += 1;
        self.t[oi].team_stats.rush_plays += 1;
        self.st(o, carrier).inc("rush_att");

        let mut kept_fumble = false;
        if self.rng.chance(fumble_p.clamp(0.002, 0.06)) {
            self.st(o, carrier).inc("fumbles");
            self.st(d, tackler).inc("ff");
            let lost = self.rng.chance(0.5);
            if lost {
                self.st(o, carrier).add("rush_yds", y as f64);
                self.t[oi].team_stats.rush_yards += y;
                self.s.yard_line = (self.s.yard_line + y).clamp(1, 99);
                self.event(PlayType::Run, y, "fumble_lost", &ev);
                self.st(o, carrier).inc("fumbles_lost");
                self.t[oi].team_stats.fumbles_lost += 1;
                self.t[oi].team_stats.turnovers += 1;
                let all = du.all();
                let recoverer = *self.rng.pick(&all);
                self.st(d, recoverer).inc("fr");
                self.track_tackle(d, tackler, false, None);
                self.s.change_possession((100 - self.s.yard_line).clamp(1, 99));
                return Step::ending(DriveEnd::new(End::Turnover), ClockType::Run, true);
            }
            // Recovered by the offense: the play carries on as a normal gain (counted once below).
            self.event(PlayType::Run, y, "fumble_kept", &ev);
            kept_fumble = true;
        }

        let assist = if self.rng.chance(0.3) {
            let all = du.all();
            Some(*self.rng.pick(&all))
        } else {
            None
        };
        self.track_tackle(d, tackler, y < 0, assist);
        let st = self.st(o, carrier);
        st.add("rush_yds", y as f64);
        self.t[oi].team_stats.rush_yards += y;
        let st = self.st(o, carrier);
        st.set_max("rush_long", y as f64);
        if y <= 0 {
            st.inc("stuffs");
        }
        self.after_gain(o, y, ClockType::Run, &ev, carrier, true, None, y > 6 || y < 0 || kept_fumble, !kept_fumble)
    }

    fn pick_rusher(&mut self, o: Side, ou: &mut OffUnit<'a>) -> &'a Player {
        let mut m = Weights::new();
        for &p in &ou.rbs {
            m.set(p, if p.position == Position::Fb { 0.12 } else { 1.0 });
        }
        // Committee: the starter doesn't take every carry; the top backups share the rest.
        let backs = self.c(o).rotation_backs(ou);
        for (i, &b) in backs.iter().take(2).enumerate() {
            m.set(b, if i == 0 { 0.4 } else { 0.15 });
        }
        let pick = m.pick(&mut self.rng);
        // A committee back subs in for the starter: he's the one on the field, so snaps, wear and injury exposure follow him.
        if !ou.rbs.iter().any(|p| p.id == pick.id) {
            match ou.rbs.iter().position(|p| p.position != Position::Fb) {
                Some(i) => ou.rbs[i] = pick,
                None => ou.rbs.push(pick),
            }
        }
        pick
    }

    fn pick_tackler(&mut self, d: Side, du: &DefUnit<'a>, run: bool, tfl: bool) -> &'a Player {
        let c = &self.t[d.idx()];
        let mut m = Weights::new();
        for &p in &du.dl {
            m.set(p, (if run { 0.9 } else { 0.3 }) + (if tfl { 0.6 } else { 0.0 }) + c.ev(p, Attr::Strength) / 120.0);
        }
        for &p in &du.lb {
            m.set(p, (if run { 1.6 } else { 1.0 }) + c.ev(p, Attr::Intelligence) / 80.0);
        }
        for &p in &du.db {
            m.set(p, (if run { 0.8 } else { 1.3 }) + c.ev(p, Attr::Speed) / 160.0);
        }
        m.pick(&mut self.rng)
    }

    fn track_tackle(&mut self, d: Side, tackler: &Player, tfl: bool, assist: Option<&Player>) {
        match assist {
            Some(a) if a.id != tackler.id => {
                self.st(d, tackler).inc("tackles_ast");
                self.st(d, a).inc("tackles_ast");
            }
            _ => self.st(d, tackler).inc("tackles_solo"),
        }
        if tfl {
            self.st(d, tackler).inc("tfl");
        }
        if tfl && assist.is_none() {
            self.st(d, tackler).inc("run_stops");
        }
    }

    // ------------------------------------------------------- pass play

    fn pass_play(&mut self, o: Side, d: Side, ou: &OffUnit<'a>, du: &DefUnit<'a>, os: &OffenseStrategy, ds: &DefenseStrategy) -> Step {
        let (oi, di) = (o.idx(), d.idx());
        let qb = ou.qb;
        self.t[oi].team_stats.plays += 1;
        self.t[oi].team_stats.pass_plays += 1;

        // Pass rush vs protection.
        let mut rushers: Vec<&'a Player> = du.dl.clone();
        let mut blitzers: Vec<&'a Player> = Vec::new();
        if self.rng.chance(ds.blitz_frequency) {
            let extra = if self.rng.chance(0.3) { 2 } else { 1 };
            // Linebackers blitz most (outside backers with a real pass rush most of all); a safety or corner comes only now and then.
            let mut pool = Weights::new();
            for &p in &du.lb {
                pool.set(p, (if p.position == Position::Olb { 2.5 } else { 0.5 }) * (0.4 + pass_rush_score(&self.t[di], p) / 60.0));
            }
            for &p in &du.db {
                pool.set(p, (if p.position == Position::S { 0.25 } else { 0.12 }) * (0.4 + pass_rush_score(&self.t[di], p) / 60.0));
            }
            for _ in 0..extra {
                if pool.is_empty() {
                    break;
                }
                let pick = pool.pick(&mut self.rng);
                pool.remove(pick);
                blitzers.push(pick);
            }
            rushers.extend(&blitzers);
        }
        // A true edge rusher at outside linebacker (the rare Lawrence Taylor) is sent after the quarterback on his own, no blitz call needed.
        for &p in &du.lb {
            if p.position != Position::Olb || rushers.iter().any(|r| r.id == p.id) {
                continue;
            }
            let chance = ((pass_rush_score(&self.t[di], p) - EDGE_RUSH_FLOOR) / EDGE_RUSH_SPAN).clamp(0.0, EDGE_RUSH_MAX);
            if self.rng.chance(chance) {
                rushers.push(p);
            }
        }
        let mut blockers: Vec<&'a Player> = ou.ol.clone();
        blockers.extend(&ou.tes);
        blockers.extend(ou.rbs.iter().take(1));
        let depth_base = (os.pass_depth + if os.scheme == OffenseScheme::AirRaid { 0.08 } else { 0.0 } - if os.scheme == OffenseScheme::Spread { 0.05 } else { 0.0 }).clamp(0.0, 1.0);

        let mut winners: Vec<&'a Player> = Vec::new();
        let mut primary: Option<&'a Player> = None;
        let quick_release = (self.ev(o, qb, Attr::Intelligence) - 50.0) * 0.004;
        for (i, &rusher) in rushers.iter().enumerate() {
            let blocker = blockers[i % blockers.len()];
            let bs = pass_block_score(&self.t[oi], blocker) * if matches!(blocker.position, Position::Rb | Position::Te) { 0.75 } else { 1.0 };
            let is_blitz = blitzers.iter().any(|b| b.id == rusher.id);
            let z = (pass_rush_score(&self.t[di], rusher) - bs) / 16.0 - 2.25 + if is_blitz { 0.25 } else { 0.0 } + 0.95 * (depth_base - 0.5) - quick_release;
            if self.rng.chance(sigmoid(z)) {
                winners.push(rusher);
                primary.get_or_insert(rusher);
                self.st(d, rusher).inc("pressures");
                self.st(o, blocker).inc("pressures_allowed");
            }
        }
        let pressured = !winners.is_empty();
        let blitz_penalty = blitzers.len() as f64 * 0.35; // fewer defenders in coverage

        // Sack?
        let escape = (self.ev(o, qb, Attr::Agility) + self.ev(o, qb, Attr::Intelligence)) / 2.0;
        let mut sack_p = if pressured { 0.17 + 0.04 * (winners.len() as f64 - 1.0) + 0.22 * (depth_base - 0.5) - 0.003 * (escape - 50.0) } else { 0.0 };
        if os.pocket == PocketStyle::Scramble {
            sack_p -= 0.03;
        }
        if pressured && self.rng.chance(sack_p.clamp(0.04, 0.6)) {
            return self.sack(o, d, ou, du, &winners, qb);
        }

        // Scramble?
        let scramble_base = if pressured { 0.14 } else { 0.01 };
        let scramble_p = scramble_base
            + match os.pocket {
                PocketStyle::Scramble => 0.14,
                PocketStyle::Rolling => 0.05,
                _ => 0.0,
            }
            + 0.004 * ((self.ev(o, qb, Attr::Speed) + self.ev(o, qb, Attr::Acceleration)) / 2.0 - 45.0);
        if self.rng.chance(scramble_p.clamp(0.0, 0.5)) {
            return self.scramble(o, d, ou, du, qb);
        }

        // Target selection.
        let mut weights = Weights::new();
        for p in ou.targets() {
            let base = match p.position {
                Position::Wr => 1.0,
                Position::Te => 0.9,
                Position::Rb => 0.4,
                _ => 0.2,
            };
            let skill = (self.ev(o, p, Attr::Hands) + self.ev(o, p, Attr::Speed) + self.ev(o, p, Attr::Agility)) / 3.0;
            // A good tight end is a featured target, not a blocker who catches the odd checkdown: ability counts for more at TE.
            weights.set(p, base * if p.position == Position::Te { ((skill - 45.0) / 18.0).exp() } else { 0.4 + skill / 50.0 });
        }
        // QB reads: intelligence skews toward the best-open option.
        let target = weights.pick(&mut self.rng);
        self.st(o, target).inc("targets");
        self.st(o, qb).inc("pass_att");
        self.t[oi].team_stats.pass_att += 1;

        // Air yards.
        let max_depth = 15.0 + self.ev(o, qb, Attr::Strength) * 0.5;
        let is_screen_like = target.position == Position::Rb;
        let mut air = if is_screen_like { self.rng.gaussian(0.0, 2.0) } else { self.rng.gaussian(3.5 + depth_base * 9.0 + os.play_action * 2.0, 4.5) };
        air = air.clamp(-3.0, max_depth);
        if air > self.s.yards_to_end_zone() as f64 {
            air = self.s.yards_to_end_zone() as f64;
        }

        // Coverage defender.
        let cov_pool: Vec<&'a Player> = match target.position {
            Position::Wr => {
                let mut v = du.db.clone();
                v.extend(&du.db);
                v.extend(du.lb.iter().take(1));
                v
            }
            Position::Te => {
                let mut v = du.lb.clone();
                v.extend(du.db.iter().skip(2));
                v
            }
            _ => {
                let mut v = du.lb.clone();
                v.extend(du.db.iter().skip(du.db.len().saturating_sub(1)));
                v
            }
        };
        let cover = cov_pool[self.rng.next_int(cov_pool.len())];
        self.st(d, cover).inc("tgt_allowed");
        let sep_edge = sep_score(&self.t[oi], target) - cov_score(&self.t[di], cover) + blitz_penalty * 6.0;

        // Accuracy: DI-driven, harder with depth, pressure, bad weather.
        let acc = self.ev(o, qb, Attr::Discipline);
        let mut z_acc = 1.8 + 0.045 * (acc - 55.0) - 0.07 * (air - 8.0) - if pressured { 0.75 } else { 0.0 } + 0.01 * (escape - 50.0) * if pressured { 1.0 } else { 0.0 };
        if os.scheme == OffenseScheme::AirRaid {
            z_acc += 0.05; // a whole system built around the deep throw
        }
        match self.config.weather {
            Weather::Rain => z_acc -= 0.2,
            Weather::Snow => z_acc -= 0.4,
            Weather::Wind if air > 12.0 => z_acc -= 0.3,
            _ => {}
        }
        let accurate = self.rng.chance(sigmoid(z_acc));

        let mut ev: Roles<'a> = vec![("qb", qb.name.as_str()), ("target", target.name.as_str()), ("defender", cover.name.as_str())];
        if pressured {
            ev.push(("pass_rusher", primary.unwrap().name.as_str()));
        }

        if !accurate {
            // Incompletion or INT.
            let int_p = 0.11 + 0.004 * (air - 8.0) + 0.003 * (55.0 - self.ev(o, qb, Attr::Intelligence)) + if pressured { 0.05 } else { 0.0 } + 0.002 * (self.ev(d, cover, Attr::Hands) - 50.0) + 0.04 * (ds.turnover_hunting - 0.3);
            if self.rng.chance(int_p.clamp(0.02, 0.4)) {
                return self.interception(o, d, qb, cover, &ev, air);
            }
            if self.rng.chance(0.3) {
                self.st(d, cover).inc("pd");
            }
            self.st(o, qb).add("throwaways", 0.0);
            self.event(PlayType::Pass, 0, "incomplete", &ev);
            return self.incomplete();
        }

        let open = self.rng.chance(sigmoid(1.15 - 0.03 * (air - 8.0) + 0.05 * sep_edge - if self.config.weather == Weather::Snow { 0.15 } else { 0.0 }));
        if !open {
            let int_p = 0.07 + 0.003 * (self.ev(d, cover, Attr::Hands) - 50.0);
            if self.rng.chance(int_p.clamp(0.01, 0.3)) {
                return self.interception(o, d, qb, cover, &ev, air);
            }
            if self.rng.chance(0.55) {
                self.st(d, cover).inc("pd");
            }
            self.event(PlayType::Pass, 0, "incomplete", &ev);
            return self.incomplete();
        }

        let mut drop_p = 0.05 - 0.0012 * (self.ev(o, target, Attr::Hands) - 50.0);
        if self.config.weather == Weather::Rain {
            drop_p += 0.04;
        }
        if self.config.weather == Weather::Snow {
            drop_p += 0.05;
        }
        if self.rng.chance(drop_p.clamp(0.01, 0.25)) {
            self.st(o, target).inc("drops");
            self.event(PlayType::Pass, 0, "drop", &ev);
            return self.incomplete();
        }

        // Completion + YAC.
        let yac_skill = (self.ev(o, target, Attr::Acceleration) + self.ev(o, target, Attr::Agility) + self.ev(o, target, Attr::Speed)) / 3.0;
        let base_yac = self.rng.gaussian(2.2 + if air < 8.0 { 1.5 } else { 0.0 } + 0.07 * (yac_skill - 55.0), 2.6).clamp(0.0, 40.0);
        let big_yac = if self.rng.chance(0.045 + 0.002 * (yac_skill - 55.0).clamp(-15.0, 30.0)) { self.rng.gaussian(18.0, 8.0).abs() } else { 0.0 };
        let yac = base_yac + big_yac;
        let total = ((air + yac).round() as i32).clamp(-3, 99);
        let tackler = self.pick_tackler(d, du, false, false);
        self.t[oi].team_stats.pass_cmp += 1;
        self.t[oi].team_stats.pass_yards += total;
        let st = self.st(o, qb);
        st.inc("pass_cmp");
        st.add("pass_yds", total as f64);
        let st = self.st(o, target);
        st.inc("rec");
        st.add("rec_yds", total as f64);
        st.add("yac", yac.round());
        let st = self.st(d, cover);
        st.inc("cmp_allowed");
        st.add("yds_allowed", total as f64);
        if total >= 30 {
            let f1 = format!("{} has been informed.", cover.name);
            self.key(o, format!("{} to {} for {}.", qb.name, target.name, total), &[&f1, "Replay confirms: wide open."]);
        }
        self.st(o, target).set_max("rec_long", total as f64);
        self.st(o, qb).set_max("pass_long", total as f64);
        ev.push(("tackler", tackler.name.as_str()));

        let mut fumble_p = 0.014 + (55.0 - self.ev(o, target, Attr::Hands)) * 0.0003;
        if self.config.weather == Weather::Rain {
            fumble_p *= 1.6;
        }
        if self.rng.chance(fumble_p.clamp(0.002, 0.05)) {
            self.st(o, target).inc("fumbles");
            self.st(d, tackler).inc("ff");
            if self.rng.chance(0.5) {
                self.s.yard_line = (self.s.yard_line + total).clamp(1, 99);
                self.st(o, target).inc("fumbles_lost");
                self.t[oi].team_stats.fumbles_lost += 1;
                self.t[oi].team_stats.turnovers += 1;
                let all = du.all();
                let recoverer = *self.rng.pick(&all);
                self.st(d, recoverer).inc("fr");
                self.event(PlayType::Pass, total, "fumble_lost", &ev);
                self.s.change_possession((100 - self.s.yard_line).clamp(1, 99));
                return Step::ending(DriveEnd::new(End::Turnover), ClockType::Completion, true);
            }
        }
        let assist = if self.rng.chance(0.28) {
            let all = du.all();
            Some(*self.rng.pick(&all))
        } else {
            None
        };
        self.track_tackle(d, tackler, false, assist);
        self.event(PlayType::Pass, total, "complete", &ev);
        self.after_gain(o, total, ClockType::Completion, &ev, target, false, Some(qb), total > 12, true)
    }

    fn incomplete(&mut self) -> Step {
        self.s.down += 1;
        self.check_downs(ClockType::Incomplete, false)
    }

    fn sack(&mut self, o: Side, d: Side, ou: &OffUnit<'a>, du: &DefUnit<'a>, winners: &[&'a Player], qb: &'a Player) -> Step {
        let oi = o.idx();
        // Any rusher who beat his man can get the sack, so it is not always the first (best) lineman in the list.
        let sacker = if winners.len() > 1 { winners[self.rng.next_int(winners.len())] } else { winners[0] };
        let shared = winners.len() > 1 && self.rng.chance(0.35);
        let partner = if shared { winners.iter().copied().find(|w| w.id != sacker.id) } else { None };
        let lost = (self.rng.gaussian(6.5, 2.5).round() as i32).clamp(1, 15);
        self.t[d.idx()].team_stats.sacks_made += 1;
        self.t[oi].team_stats.sacks_taken += 1;
        self.t[oi].team_stats.sack_yards_lost += lost;
        self.st(o, qb).inc("sacks_taken");
        self.st(o, qb).add("sack_yds", lost as f64);
        if shared {
            self.st(d, sacker).add("sacks", 0.5);
            self.st(d, partner.unwrap()).add("sacks", 0.5);
        } else {
            self.st(d, sacker).inc("sacks");
        }
        self.st(d, sacker).inc("qb_hits");
        self.st(d, sacker).add("tfl", if shared { 0.5 } else { 1.0 });
        self.st(d, sacker).inc("tackles_solo");
        // Credit sack allowed to a blocker beaten (first OL).
        let blocked = ou.ol[self.rng.next_int(ou.ol.len())];
        self.st(o, blocked).inc("sacks_allowed");
        let mut fumble_p = 0.04 + (55.0 - self.ev(o, qb, Attr::Hands)) * 0.0006;
        if self.config.weather == Weather::Rain {
            fumble_p *= 1.5;
        }
        self.s.yard_line -= lost;
        let ev: Roles<'a> = vec![("qb", qb.name.as_str()), ("pass_rusher", sacker.name.as_str()), ("beaten", blocked.name.as_str())];
        if self.s.yard_line <= 0 {
            self.event(PlayType::Sack, -lost, "safety", &ev);
            self.score(d, 2, Some(format!("Safety: {} sack", sacker.name)));
            self.s.yard_line = 1;
            return Step::ending(DriveEnd::scored(End::Safety, d), ClockType::Sack, true);
        }
        if self.rng.chance(fumble_p.clamp(0.01, 0.2)) {
            self.st(o, qb).inc("fumbles");
            self.st(d, sacker).inc("ff");
            if self.rng.chance(0.55) {
                self.st(o, qb).inc("fumbles_lost");
                self.t[oi].team_stats.fumbles_lost += 1;
                self.t[oi].team_stats.turnovers += 1;
                let all = du.all();
                let recoverer = *self.rng.pick(&all);
                self.st(d, recoverer).inc("fr");
                self.event(PlayType::Sack, -lost, "fumble_lost", &ev);
                self.s.change_possession((100 - self.s.yard_line).clamp(1, 99));
                return Step::ending(DriveEnd::new(End::Turnover), ClockType::Sack, true);
            }
        }
        self.event(PlayType::Sack, -lost, "sack", &ev);
        self.s.to_go += lost;
        self.s.down += 1;
        self.check_downs(ClockType::Sack, true)
    }

    fn scramble(&mut self, o: Side, d: Side, _ou: &OffUnit<'a>, du: &DefUnit<'a>, qb: &'a Player) -> Step {
        let oi = o.idx();
        let skill = (self.ev(o, qb, Attr::Speed) + self.ev(o, qb, Attr::Acceleration) + self.ev(o, qb, Attr::Agility)) / 3.0;
        let y = (self.rng.gaussian(4.5 + 0.1 * (skill - 45.0), 4.0).round() as i32).clamp(-4, 40);
        let tackler = self.pick_tackler(d, du, false, false);
        self.t[oi].team_stats.pass_plays -= 1;
        self.t[oi].team_stats.rush_plays += 1;
        let st = self.st(o, qb);
        st.inc("rush_att");
        st.inc("scrambles");
        st.add("rush_yds", y as f64);
        st.set_max("rush_long", y as f64);
        self.t[oi].team_stats.rush_yards += y;
        self.track_tackle(d, tackler, false, None);
        let fumble_p = 0.012 + (55.0 - self.ev(o, qb, Attr::Hands)) * 0.0004 + if self.config.weather == Weather::Rain { 0.01 } else { 0.0 };
        if self.rng.chance(fumble_p.clamp(0.002, 0.08)) {
            self.st(o, qb).inc("fumbles");
            self.st(d, tackler).inc("ff");
            if self.rng.chance(0.5) {
                self.st(o, qb).inc("fumbles_lost");
                self.t[oi].team_stats.fumbles_lost += 1;
                self.t[oi].team_stats.turnovers += 1;
                self.s.yard_line = (self.s.yard_line + y).clamp(1, 99);
                self.event(PlayType::Scramble, y, "fumble_lost", &[("qb", &qb.name)]);
                self.s.change_possession((100 - self.s.yard_line).clamp(1, 99));
                return Step::ending(DriveEnd::new(End::Turnover), ClockType::Run, true);
            }
        }
        self.event(PlayType::Scramble, y, "scramble", &[("qb", &qb.name), ("tackler", &tackler.name)]);
        self.after_gain(o, y, ClockType::Run, &vec![("qb", qb.name.as_str())], qb, true, None, true, false)
    }

    fn interception(&mut self, o: Side, d: Side, qb: &'a Player, defender: &'a Player, ev: &Roles<'a>, air: f64) -> Step {
        let oi = o.idx();
        self.st(o, qb).inc("pass_int");
        self.t[oi].team_stats.interceptions_thrown += 1;
        self.t[oi].team_stats.turnovers += 1;
        self.st(d, defender).inc("int");
        self.key(
            d,
            format!("{} picks off {}.", defender.name, qb.name),
            &["That one was tipped by the wind. Or the QB.", "The sideline reaction was not recorded for family audiences."],
        );
        let spot = (self.s.yard_line + air.round() as i32).clamp(1, 120);
        let ret_yards = (self.rng.gaussian(10.0 + (self.ev(d, defender, Attr::Speed) - 55.0) * 0.2, 9.0).round() as i32).clamp(0, 60);
        self.st(d, defender).add("int_yds", ret_yards as f64);
        self.event(PlayType::Pass, 0, "int", ev);
        // Defense's own yard line = 100 - spot (if spot > 100 it's a touchback to 20)
        let new_spot = if spot >= 100 { 20 } else { 100 - spot + ret_yards };
        if spot < 100 && new_spot >= 100 {
            self.st(d, defender).inc("int_td");
            self.score(d, 6, Some(format!("{} {}-yd interception return", defender.name, ret_yards)));
            let f = format!("{} would like to discuss that throw with himself.", qb.name);
            self.key(d, format!("{} takes one back for six.", defender.name), &[&f]);
            self.s.change_possession(50);
            // Defense scored: after TD they kick extra point then kickoff; simplest: treat as defensive TD ending drive.
            self.after_touchdown(d);
            self.s.possession = o; // offense gets ball back via kickoff below
            self.kickoff(d);
            return Step::ending(DriveEnd::new(End::Turnover), ClockType::Completion, true);
        }
        self.s.change_possession(new_spot.clamp(1, 99));
        Step::ending(DriveEnd::new(End::Turnover), ClockType::Completion, true)
    }

    // ----------------------------------------------- after a yardage play

    #[allow(clippy::too_many_arguments)]
    fn after_gain(&mut self, o: Side, yards: i32, clock: ClockType, ev: &Roles<'a>, carrier: &Player, rush: bool, passer: Option<&Player>, hit: bool, count_play: bool) -> Step {
        let oi = o.idx();
        if rush && count_play {
            self.event(PlayType::Run, yards, "run", ev);
        }
        let new_line = self.s.yard_line + yards;
        if new_line >= 100 {
            // Touchdown
            self.st(o, carrier).inc(if rush { "rush_td" } else { "rec_td" });
            if let Some(p) = passer {
                self.st(o, p).inc("pass_td");
            }
            let len = 100 - self.s.yard_line;
            let desc = if rush { format!("{} {}-yd run", carrier.name, len) } else { format!("{} {}-yd pass from {}", carrier.name, len, passer.map(|p| p.name.as_str()).unwrap_or("null")) };
            self.score(o, 6, Some(desc));
            if len >= 30 {
                if rush {
                    self.key(o, format!("{} breaks free for a {}-yard score.", carrier.name, len), &["The defense has requested a moment of silence.", "Somebody was supposed to be there."]);
                } else {
                    self.key(o, format!("{} finds {} for {}.", passer.map(|p| p.name.as_str()).unwrap_or("null"), carrier.name, len), &["The defense has requested a moment of silence.", "Somebody was supposed to be there."]);
                }
            }
            return Step::ending(DriveEnd::scored(End::Touchdown, o), ClockType::Score, hit);
        }
        if new_line <= 0 {
            // Tackled in own end zone (safety)
            let d = o.other();
            self.score(d, 2, Some("Safety: ball carrier tackled in the end zone".to_string()));
            self.s.yard_line = 1;
            return Step::ending(DriveEnd::scored(End::Safety, d), clock, true);
        }
        self.s.yard_line = new_line;
        self.s.to_go -= yards;
        let first_down = self.s.to_go <= 0;
        if self.s.down == 3 {
            self.t[oi].team_stats.third_att += 1;
            if first_down {
                self.t[oi].team_stats.third_conv += 1;
            }
        } else if self.s.down == 4 {
            self.t[oi].team_stats.fourth_att += 1;
            if first_down {
                self.t[oi].team_stats.fourth_conv += 1;
            }
        }
        if first_down {
            self.t[oi].team_stats.first_downs += 1;
            self.st(o, carrier).inc(if rush { "rush_fd" } else { "rec_fd" });
            self.s.first_down();
            return Step { end: None, clock_type: clock, hit };
        }
        self.s.down += 1;
        self.check_downs(clock, hit)
    }

    fn check_downs(&mut self, clock: ClockType, hit: bool) -> Step {
        if self.s.down > 4 {
            // Turnover on downs
            self.s.change_possession((100 - self.s.yard_line).clamp(1, 99));
            return Step::ending(DriveEnd::new(End::Downs), clock, hit);
        }
        Step { end: None, clock_type: clock, hit }
    }

    // ---------------------------------------------------------- injuries

    fn injury_rolls(&mut self, side: Side, on_field: &[&'a Player], hit: bool) {
        let ci = side.idx();
        for &p in on_field {
            if self.t[ci].injured.contains(&p.id) {
                continue;
            }
            let en = p.ratings[Attr::Endurance] as f64 / 100.0;
            let fatigue = self.t[ci].fatigue.get(&p.id).copied().unwrap_or(0.0);
            let mut p0 = 0.0003 * (1.65 - en) * if hit { 2.0 } else { 1.0 } * (1.0 + 0.5 * fatigue);
            if matches!(p.position, Position::K | Position::P) {
                p0 *= 0.2;
            }
            if self.rng.next_f64() < p0 {
                let sev = self.rng.weighted(&[("shaken", 55.0), ("minor", 28.0), ("multiWeek", 13.0), ("seasonEnding", 4.0 + if en < 0.4 { 3.0 } else { 0.0 })]);
                let games = match sev {
                    "shaken" => 0,
                    "minor" => 1,
                    "multiWeek" => self.rng.range(2, 6),
                    _ => 20,
                };
                self.t[ci].injured.insert(p.id);
                self.t[ci].st(p).inc("injuries");
                self.injuries.push(InjuryRecord { side, player_id: p.id, name: p.name.clone(), severity: sev.to_string(), games_out: games });
            }
        }
    }

    fn event(&mut self, ty: PlayType, yards: i32, result: &str, roles: &[(&str, &str)]) {
        if !self.config.keep_events {
            return;
        }
        self.events.push(PlayEvent {
            quarter: self.s.quarter,
            clock: self.s.clock,
            offense: self.s.possession,
            down: self.s.down,
            to_go: self.s.to_go,
            yard_line: self.s.yard_line,
            play_type: ty,
            yards,
            result: result.to_string(),
            roles: roles.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect::<BTreeMap<_, _>>(),
        });
    }
}
