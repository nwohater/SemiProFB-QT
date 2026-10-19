use sp_core::strategy::*;
use sp_core::{Attr, Player, Position, Team};
use std::collections::BTreeMap;

/// Always shown as a word (Good / Okay / Risky), never color alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fit {
    Good,
    Okay,
    Risky,
}

impl Fit {
    pub fn label(self) -> &'static str {
        match self {
            Fit::Good => "Good",
            Fit::Okay => "Okay",
            Fit::Risky => "Risky",
        }
    }
}

#[derive(Clone, Debug)]
pub struct FitPart {
    /// e.g. "QB arm 71"
    pub text: String,
    pub fit: Fit,
}

#[derive(Clone, Debug)]
pub struct FitResult {
    pub fit: Fit,
    pub reason: String,
    pub parts: Vec<FitPart>,
}

#[derive(Clone, Debug)]
pub struct Matchup {
    pub label: String,
    pub ours: i32,
    pub theirs: i32,
    pub fit: Fit,
}

#[derive(Clone, Debug)]
pub struct OpponentReport {
    pub summary: String,
    pub tips: Vec<String>,
    /// Short tags for the next-game card: front, blitz, run/pass lean, weakest unit.
    pub chips: Vec<String>,
}

/// Roster-derived numbers the Home and Strategy screens show: unit ratings, strategy fit, opponent notes.
pub struct Insights;

// Baselines are the league-typical values for a starter; fit bands are +/- around them.
const GOOD: f64 = 4.0;
const RISKY: f64 = -4.0;

fn band(value: f64, baseline: f64) -> Fit {
    let d = value - baseline;
    if d >= GOOD {
        Fit::Good
    } else if d <= RISKY {
        Fit::Risky
    } else {
        Fit::Okay
    }
}

fn top<'a>(t: &'a Team, pos: &[Position], n: usize) -> Vec<&'a Player> {
    let mut pool: Vec<&Player> = t.active().filter(|p| pos.contains(&p.position)).collect();
    pool.sort_by_key(|p| std::cmp::Reverse(p.ovr()));
    pool.into_iter().take(n).collect()
}

fn avg(xs: impl Iterator<Item = f64>) -> f64 {
    let v: Vec<f64> = xs.collect();
    if v.is_empty() { 40.0 } else { v.iter().sum::<f64>() / v.len() as f64 }
}

fn attr(t: &Team, pos: &[Position], a: Attr, n: usize) -> f64 {
    avg(top(t, pos, n).iter().map(|p| p.ratings[a] as f64))
}

struct Inputs {
    qb_arm: f64,
    qb_acc: f64,
    qb_in: f64,
    qb_speed: f64,
    wr_speed: f64,
    wr_hands: f64,
    ol_str: f64,
    ol_pass_pro: f64,
    rb_skill: f64,
    rb_str: f64,
    endurance: f64,
    dl_burst: f64,
    dl_str: f64,
    lb_in: f64,
    db_speed: f64,
    db_hands: f64,
}

fn inputs(t: &Team) -> Inputs {
    use Position::*;
    let ol = [Ot, Og, C];
    Inputs {
        qb_arm: attr(t, &[Qb], Attr::Strength, 1),
        qb_acc: attr(t, &[Qb], Attr::Discipline, 1),
        qb_in: attr(t, &[Qb], Attr::Intelligence, 1),
        qb_speed: attr(t, &[Qb], Attr::Speed, 1),
        wr_speed: attr(t, &[Wr], Attr::Speed, 3),
        wr_hands: attr(t, &[Wr, Te], Attr::Hands, 4),
        ol_str: attr(t, &ol, Attr::Strength, 5),
        ol_pass_pro: (attr(t, &ol, Attr::Agility, 5) + attr(t, &ol, Attr::Intelligence, 5)) / 2.0,
        rb_skill: (attr(t, &[Rb], Attr::Acceleration, 1) + attr(t, &[Rb], Attr::Speed, 1) + attr(t, &[Rb], Attr::Agility, 1)) / 3.0,
        rb_str: attr(t, &[Rb, Fb], Attr::Strength, 2),
        endurance: avg(t.active().map(|p| p.ratings[Attr::Endurance] as f64)),
        dl_burst: attr(t, &[De], Attr::Acceleration, 2),
        dl_str: attr(t, &[De, Dt], Attr::Strength, 4),
        lb_in: attr(t, &[Mlb, Olb], Attr::Intelligence, 3),
        db_speed: attr(t, &[Cb, S], Attr::Speed, 4),
        db_hands: attr(t, &[Cb, S], Attr::Hands, 4),
    }
}

fn combine(parts: &[FitPart]) -> Fit {
    let risky = parts.iter().filter(|p| p.fit == Fit::Risky).count();
    let good = parts.iter().filter(|p| p.fit == Fit::Good).count();
    if risky >= 2 || (risky == 1 && good == 0) {
        Fit::Risky
    } else if good >= 2 && risky == 0 {
        Fit::Good
    } else {
        Fit::Okay
    }
}

fn with_parts(base: &str, parts: &[FitPart]) -> String {
    let worst: Vec<&str> = parts.iter().filter(|p| p.fit == Fit::Risky).map(|p| p.text.as_str()).collect();
    let best: Vec<&str> = parts.iter().filter(|p| p.fit == Fit::Good).map(|p| p.text.as_str()).collect();
    if !worst.is_empty() {
        return format!("{base} {} {} weak spot{}.", worst.join(" and "), if worst.len() == 1 { "is a" } else { "are" }, if worst.len() == 1 { "" } else { "s" });
    }
    if !best.is_empty() {
        return format!("{base} {} help.", best.join(" and "));
    }
    base.to_string()
}

struct Parts(Vec<FitPart>);

impl Parts {
    fn add(&mut self, label: &str, v: f64, base: f64) {
        self.0.push(FitPart { text: format!("{label} {}", v.round() as i64), fit: band(v, base) });
    }
}

impl Insights {
    /// Overall (1-99) for each unit, from active players at their listed position.
    pub fn unit_ratings(t: &Team) -> BTreeMap<&'static str, i32> {
        use Position::*;
        let unit = |pos: &[Position], n: usize| avg(top(t, pos, n).iter().map(|p| p.ovr() as f64)).round() as i32;
        BTreeMap::from([
            ("QB", unit(&[Qb], 1)),
            ("RB", unit(&[Rb, Fb], 2)),
            ("WR", unit(&[Wr], 3)),
            ("TE", unit(&[Te], 1)),
            ("OL", unit(&[Ot, Og, C], 5)),
            ("DL", unit(&[De, Dt], 4)),
            ("LB", unit(&[Mlb, Olb], 3)),
            ("DB", unit(&[Cb, S], 4)),
            ("K", unit(&[K], 1)),
            ("P", unit(&[P], 1)),
        ])
    }

    /// Head-to-head unit matchups for the next-game card.
    pub fn matchups(ours: &Team, theirs: &Team) -> Vec<Matchup> {
        let (a, b) = (Self::unit_ratings(ours), Self::unit_ratings(theirs));
        let m = |label: &str, x: i32, y: i32| Matchup { label: label.into(), ours: x, theirs: y, fit: band((x - y) as f64, 0.0) };
        vec![m("Our OL vs their DL", a["OL"], b["DL"]), m("Our WR vs their DB", a["WR"], b["DB"]), m("Our DL vs their OL", a["DL"], b["OL"]), m("Our DB vs their WR", a["DB"], b["WR"])]
    }

    pub fn offense_scheme(t: &Team, s: OffenseScheme) -> FitResult {
        let i = inputs(t);
        let mut p = Parts(Vec::new());
        match s {
            OffenseScheme::GroundAndPound => {
                p.add("RB speed/burst", i.rb_skill, 58.0);
                p.add("OL strength", i.ol_str, 58.0);
                p.add("RB/FB strength", i.rb_str, 52.0);
            }
            OffenseScheme::AirRaid => {
                p.add("QB arm", i.qb_arm, 56.0);
                p.add("QB accuracy", i.qb_acc, 56.0);
                p.add("WR speed", i.wr_speed, 60.0);
                p.add("OL pass protection", i.ol_pass_pro, 46.0);
            }
            OffenseScheme::Spread => {
                p.add("WR speed", i.wr_speed, 60.0);
                p.add("WR hands", i.wr_hands, 55.0);
                p.add("QB reads", i.qb_in, 54.0);
            }
            OffenseScheme::Option => {
                p.add("QB speed", i.qb_speed, 52.0);
                p.add("RB speed/burst", i.rb_skill, 58.0);
                p.add("OL strength", i.ol_str, 58.0);
            }
            OffenseScheme::ProStyle => {
                p.add("QB reads", i.qb_in, 54.0);
                p.add("RB speed/burst", i.rb_skill, 58.0);
                p.add("WR hands", i.wr_hands, 55.0);
            }
            OffenseScheme::Balanced => {
                p.add("QB accuracy", i.qb_acc, 56.0);
                p.add("RB speed/burst", i.rb_skill, 58.0);
                p.add("WR speed", i.wr_speed, 60.0);
            }
        }
        let parts = p.0;
        let combined = combine(&parts);
        let fit = if s == OffenseScheme::Balanced && combined == Fit::Risky { Fit::Okay } else { combined };
        let best: Vec<&str> = parts.iter().filter(|x| x.fit == Fit::Good).map(|x| x.text.as_str()).collect();
        let worst: Vec<&str> = parts.iter().filter(|x| x.fit == Fit::Risky).map(|x| x.text.as_str()).collect();
        let reason = if !worst.is_empty() {
            format!("{} {} weak spots for this plan.", worst.join(" and "), if worst.len() == 1 { "is" } else { "are" })
        } else if !best.is_empty() {
            format!("{} back this up.", best.join(" and "))
        } else {
            "Nothing special, nothing broken.".to_string()
        };
        FitResult { fit, reason, parts }
    }

    /// Slider fits: "runPass", "depth", "tempo", "pocket".
    pub fn offense_slider(t: &Team, slider: &str, value: f64, pocket: Option<PocketStyle>) -> FitResult {
        let i = inputs(t);
        let mut p = Parts(Vec::new());
        let reason: String = match slider {
            "depth" => {
                if value >= 0.6 {
                    p.add("QB arm", i.qb_arm, 56.0);
                    p.add("WR speed", i.wr_speed, 60.0);
                    p.add("OL pass protection", i.ol_pass_pro, 46.0);
                    "Deep throws need arm, speed and time to develop.".into()
                } else if value <= 0.35 {
                    p.add("QB reads", i.qb_in, 54.0);
                    p.add("WR hands", i.wr_hands, 55.0);
                    "Short throws lean on quick reads and sure hands.".into()
                } else {
                    p.add("QB accuracy", i.qb_acc, 56.0);
                    "A middle-of-the-field plan works for most rosters.".into()
                }
            }
            "runPass" => {
                if value <= 0.4 {
                    p.add("RB speed/burst", i.rb_skill, 58.0);
                    p.add("OL strength", i.ol_str, 58.0);
                    "Running more needs a back and a line that win.".into()
                } else if value >= 0.6 {
                    p.add("QB accuracy", i.qb_acc, 56.0);
                    p.add("WR hands", i.wr_hands, 55.0);
                    p.add("OL pass protection", i.ol_pass_pro, 46.0);
                    "Throwing more needs a QB, receivers, and time.".into()
                } else {
                    p.add("RB speed/burst", i.rb_skill, 58.0);
                    p.add("QB accuracy", i.qb_acc, 56.0);
                    "Balanced plans keep defenses honest.".into()
                }
            }
            "tempo" => {
                p.add("Team endurance", i.endurance, 55.0);
                if value >= 0.6 { "Hurry-up wears everyone out faster.".into() } else { "Slow play saves legs and burns clock.".into() }
            }
            "pocket" => {
                let style = pocket.unwrap_or(PocketStyle::Pocket);
                if style == PocketStyle::Pocket {
                    p.add("QB reads", i.qb_in, 54.0);
                    p.add("OL pass protection", i.ol_pass_pro, 46.0);
                    "Staying put needs protection and a QB who sees the field.".into()
                } else {
                    p.add("QB speed", i.qb_speed, 52.0);
                    p.add("QB agility", attr(t, &[Position::Qb], Attr::Agility, 1), 50.0);
                    "Moving the QB works when he can run and survive hits.".into()
                }
            }
            _ => String::new(),
        };
        let parts = p.0;
        FitResult { fit: combine(&parts), reason: with_parts(&reason, &parts), parts }
    }

    pub fn defense_front(t: &Team, f: DefenseFront) -> FitResult {
        let i = inputs(t);
        let mut p = Parts(Vec::new());
        match f {
            DefenseFront::FourThree => {
                p.add("DL strength", i.dl_str, 60.0);
                p.add("LB reads", i.lb_in, 52.0);
            }
            DefenseFront::ThreeFour => {
                p.add("DL strength", i.dl_str, 62.0);
                p.add("LB reads", i.lb_in, 54.0);
            }
            DefenseFront::Nickel => {
                p.add("DB speed", i.db_speed, 58.0);
                p.add("DB hands", i.db_hands, 48.0);
            }
        }
        let parts = p.0;
        FitResult { fit: combine(&parts), reason: with_parts("Your front is only as good as the people in it.", &parts), parts }
    }

    pub fn blitz(t: &Team, freq: f64) -> FitResult {
        let i = inputs(t);
        let mk = |label: &str, v: f64, base: f64| FitPart { text: format!("{label} {}", v.round() as i64), fit: band(v, base) };
        let parts = vec![mk("DE burst", i.dl_burst, 56.0), mk("LB reads", i.lb_in, 52.0), mk("DB speed", i.db_speed, 58.0)];
        if freq < 0.15 {
            return FitResult { fit: Fit::Okay, reason: "Rarely blitzing keeps everyone in coverage.".into(), parts };
        }
        FitResult { fit: combine(&parts), reason: with_parts("Blitzing leaves fewer people in coverage, so burst and speed matter.", &parts), parts }
    }

    pub fn opponent(opp: &Team) -> OpponentReport {
        let s = opp.strategy;
        let units = Self::unit_ratings(opp);
        let mut tips: Vec<String> = Vec::new();
        let front = match s.defense.front {
            DefenseFront::FourThree => "4-3 base front",
            DefenseFront::ThreeFour => "3-4 front",
            DefenseFront::Nickel => "nickel front",
        };
        let blitz_txt = if s.defense.blitz_frequency >= 0.25 {
            "blitzes often"
        } else if s.defense.blitz_frequency <= 0.12 {
            "rarely blitzes"
        } else {
            "blitzes sometimes"
        };
        let lean = if s.offense.run_pass_bias >= 0.6 {
            "throws a lot"
        } else if s.offense.run_pass_bias <= 0.42 {
            "runs a lot"
        } else {
            "mixes run and pass"
        };
        let mut ranked: Vec<(&&str, &i32)> = units.iter().filter(|(k, _)| !["K", "P", "TE"].contains(*k)).collect();
        ranked.sort_by_key(|(_, v)| **v);
        let weakest = ranked[0];
        let mut thin: Vec<String> = Vec::new();
        if opp.active().filter(|p| p.position == Position::S).count() < 3 {
            thin.push("thin at safety".into());
        }
        let thin_txt = if thin.is_empty() {
            String::new()
        } else {
            let j = thin.join(", ");
            let mut c = j.chars();
            format!("{}{}. ", c.next().unwrap().to_uppercase(), c.as_str())
        };
        let summary = format!("{front}, {blitz_txt}, {lean}. {thin_txt}Weakest unit: {} ({}).", weakest.0, weakest.1);
        if s.defense.blitz_frequency >= 0.25 {
            tips.push("Quick throws and screens hurt a blitzing defense.".into());
        }
        if units["DB"] < 48 {
            tips.push("Their secondary is soft. Deep passing can pay off.".into());
        }
        if units["DL"] < 48 || s.defense.run_stop_commitment < 0.4 {
            tips.push("They can be run on.".into());
        }
        if units["OL"] < 48 {
            tips.push("Their line is shaky. Bring pressure.".into());
        }
        if tips.is_empty() {
            tips.push("No obvious weakness. Play your own game.".into());
        }
        let cap = |t: &str| {
            let mut c = t.chars();
            format!("{}{}", c.next().unwrap().to_uppercase(), c.as_str())
        };
        OpponentReport { summary, tips, chips: vec![cap(front), cap(blitz_txt), cap(lean), format!("Weakest unit: {} {}", weakest.0, weakest.1)] }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sp_core::team_generator::TeamGenerator;
    use sp_core::{PlayerGenerator, Region, Rng};

    fn team(seed: u64) -> Team {
        TeamGenerator::new(PlayerGenerator::built_in(Rng::new(seed))).generate(&Region::california().starting[0], &[])
    }

    #[test]
    fn unit_ratings_cover_every_unit_in_range() {
        let u = Insights::unit_ratings(&team(1));
        assert_eq!(u.len(), 10);
        assert!(u.values().all(|v| (1..=99).contains(v)));
    }

    #[test]
    fn every_scheme_and_front_gives_a_reason() {
        let t = team(2);
        for s in [OffenseScheme::ProStyle, OffenseScheme::Spread, OffenseScheme::AirRaid, OffenseScheme::GroundAndPound, OffenseScheme::Option, OffenseScheme::Balanced] {
            let f = Insights::offense_scheme(&t, s);
            assert!(!f.reason.is_empty() && !f.parts.is_empty());
        }
        for f in [DefenseFront::FourThree, DefenseFront::ThreeFour, DefenseFront::Nickel] {
            assert!(!Insights::defense_front(&t, f).parts.is_empty());
        }
        assert_eq!(Insights::blitz(&t, 0.05).fit, Fit::Okay);
    }

    #[test]
    fn opponent_report_has_summary_tips_and_chips() {
        let r = Insights::opponent(&team(3));
        assert!(r.summary.contains("Weakest unit"));
        assert!(!r.tips.is_empty());
        assert_eq!(r.chips.len(), 4);
    }

    #[test]
    fn matchups_are_symmetric_in_inputs() {
        let (a, b) = (team(4), team(5));
        let m = Insights::matchups(&a, &b);
        assert_eq!(m.len(), 4);
        assert_eq!(m[0].ours, Insights::unit_ratings(&a)["OL"]);
    }
}
