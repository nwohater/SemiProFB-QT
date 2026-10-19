use sp_core::strategy::*;
use sp_core::{Attr, Position, Rng, Team};

/// Picks a sane strategy from a roster's strengths so AI teams play differently.
pub struct AiStrategy;

fn avg(t: &Team, pos: &[Position], a: Attr, top: usize) -> f64 {
    let mut ps: Vec<_> = t.roster.iter().filter(|p| pos.contains(&p.position)).collect();
    ps.sort_by_key(|p| std::cmp::Reverse(p.ovr()));
    let use_: Vec<_> = ps.into_iter().take(top).collect();
    if use_.is_empty() {
        return 40.0;
    }
    use_.iter().map(|p| p.ratings[a] as f64).sum::<f64>() / use_.len() as f64
}

impl AiStrategy {
    pub fn for_team(t: &Team, rng: &mut Rng) -> Strategy {
        use Position::*;
        let qb_arm = avg(t, &[Qb], Attr::Strength, 1);
        let qb_acc = avg(t, &[Qb], Attr::Discipline, 1);
        let qb_speed = avg(t, &[Qb], Attr::Speed, 1);
        let wr_speed = avg(t, &[Wr], Attr::Speed, 3);
        let wr_hands = avg(t, &[Wr, Te], Attr::Hands, 3);
        let ol_str = avg(t, &[Ot, Og, C], Attr::Strength, 5);
        let ol_in = avg(t, &[Ot, Og, C], Attr::Intelligence, 5);
        let rb_skill = (avg(t, &[Rb], Attr::Acceleration, 1) + avg(t, &[Rb], Attr::Speed, 1)) / 2.0;
        let dl_str = avg(t, &[De, Dt], Attr::Strength, 4);
        let dl_burst = avg(t, &[De], Attr::Acceleration, 2);
        let lb_in = avg(t, &[Mlb, Olb], Attr::Intelligence, 3);
        let db_speed = avg(t, &[Cb, S], Attr::Speed, 4);

        let pass_score = (qb_arm + qb_acc) / 2.0 * 0.5 + wr_speed * 0.25 + wr_hands * 0.15 + ol_in * 0.1;
        let run_score = ol_str * 0.5 + rb_skill * 0.35 + (qb_speed - 40.0) * 0.05;
        let bias = (0.5 + (pass_score - run_score) * 0.008 + rng.gaussian(0.0, 0.03)).clamp(0.32, 0.60);

        let scheme = if bias > 0.6 {
            if qb_arm > 62.0 { OffenseScheme::AirRaid } else { OffenseScheme::Spread }
        } else if bias < 0.45 {
            if ol_str > 62.0 { OffenseScheme::GroundAndPound } else { OffenseScheme::Option }
        } else if rng.chance(0.5) {
            OffenseScheme::Balanced
        } else {
            OffenseScheme::ProStyle
        };

        let depth = (0.35 + (qb_arm - 50.0) * 0.007 + (wr_speed - 55.0) * 0.005).clamp(0.22, 0.62);
        let pocket = if qb_speed > 58.0 && scheme == OffenseScheme::Option {
            PocketStyle::Scramble
        } else if qb_speed > 55.0 {
            PocketStyle::Rolling
        } else {
            PocketStyle::Pocket
        };

        let front = if dl_str > 62.0 && lb_in > 52.0 {
            DefenseFront::ThreeFour
        } else if db_speed > 60.0 {
            DefenseFront::Nickel
        } else {
            DefenseFront::FourThree
        };
        let blitz = (0.12 + (dl_burst - 55.0) * 0.006 + (lb_in - 50.0) * 0.004).clamp(0.05, 0.4);
        let commit = (0.5 + (dl_str - 60.0) * 0.01 - (db_speed - 58.0) * 0.005).clamp(0.2, 0.8);

        let tempo = 0.45 + rng.gaussian(0.0, 0.08);
        let fourth = (0.25 + rng.gaussian(0.0, 0.1)).clamp(0.05, 0.7);
        let coverage = if db_speed > 62.0 {
            Coverage::Man
        } else if rng.chance(0.4) {
            Coverage::Zone
        } else {
            Coverage::Mix
        };

        Strategy {
            offense: OffenseStrategy { scheme, run_pass_bias: bias, pass_depth: depth, tempo, pocket, fourth_down_aggression: fourth, play_action: 0.3 },
            defense: DefenseStrategy { front, coverage, blitz_frequency: blitz, run_stop_commitment: commit, turnover_hunting: 0.3 },
            special_teams: SpecialTeamsStrategy::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sp_core::team_generator::TeamGenerator;
    use sp_core::{PlayerGenerator, Region};

    #[test]
    fn strategies_stay_in_range_and_differ_between_teams() {
        let mut tg = TeamGenerator::new(PlayerGenerator::built_in(Rng::new(1)));
        let ca = Region::california();
        let mut schemes = std::collections::HashSet::new();
        for t in &ca.starting {
            let team = tg.generate(t, &[]);
            for seed in 0..10 {
                let s = AiStrategy::for_team(&team, &mut Rng::new(seed));
                assert!((0.32..=0.60).contains(&s.offense.run_pass_bias));
                assert!((0.22..=0.62).contains(&s.offense.pass_depth));
                assert!((0.05..=0.4).contains(&s.defense.blitz_frequency));
                assert!((0.05..=0.7).contains(&s.offense.fourth_down_aggression));
                schemes.insert(format!("{:?}", s.offense.scheme));
            }
        }
        assert!(schemes.len() >= 3, "{schemes:?}");
    }
}
