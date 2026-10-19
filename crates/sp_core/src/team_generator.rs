use crate::generator::{GenOptions, PlayerGenerator};
use crate::rng::Rng;
use crate::team::{ROSTER_MIX, Team, pick_active_list};
use crate::tiers::Tier;
use crate::town::Town;

/// Builds a 50-man roster from the position mix and a pyramid of quality tiers,
/// shifted by town wealth, and picks a default 46-man active list.
pub struct TeamGenerator {
    pub players: PlayerGenerator,
}

impl TeamGenerator {
    pub fn new(players: PlayerGenerator) -> Self {
        Self { players }
    }

    pub fn from_seed(seed: u64) -> Self {
        Self::new(PlayerGenerator::new(Rng::new(seed)))
    }

    pub fn rng(&mut self) -> &mut Rng {
        &mut self.players.rng
    }

    pub fn generate(&mut self, town: &Town, nearby_towns: &[String]) -> Team {
        let mut built = Vec::new();
        for &(pos, count) in &ROSTER_MIX {
            for _ in 0..count {
                let rng = &mut self.players.rng;
                let tier = rng.weighted(&[
                    (Tier::ExPro, 3.0),
                    (Tier::ExCollege, 7.0 + (town.wealth - 5) as f64),
                    (Tier::Regular, 50.0),
                    (Tier::LocalAmateur, 25.0),
                    (Tier::WalkOn, 15.0),
                ]);
                let hometown = if rng.chance(0.55) || nearby_towns.is_empty() { town.name.clone() } else { rng.pick(nearby_towns).clone() };
                // A brand-new league: nobody has a pro history yet.
                built.push(self.players.generate(pos, GenOptions::default().tier(tier).hometown(&hometown).years_pro(0)));
            }
        }
        let mut team = Team::new(town.clone(), built);
        team.active_ids = pick_active_list(&team, &Default::default());
        team
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enums::Position;
    use crate::region::Region;
    use crate::returners::ReturnRole;
    use crate::team::{ACTIVE_ROSTER_SIZE, min_active_coverage};

    fn team(seed: u64) -> Team {
        TeamGenerator::new(PlayerGenerator::built_in(Rng::new(seed))).generate(&Region::california().starting[0], &[])
    }

    #[test]
    fn roster_is_50_with_a_46_man_active_list_covering_every_position() {
        let t = team(4);
        assert_eq!(t.roster.len(), 50);
        assert_eq!(t.active_ids.len(), ACTIVE_ROSTER_SIZE);
        for p in Position::ALL {
            assert!(t.active().filter(|x| x.position == p).count() >= min_active_coverage(p), "{}", p.label());
        }
    }

    #[test]
    fn everyone_gets_a_unique_number_in_a_legal_band() {
        let mut t = team(5);
        t.ensure_numbers();
        let nums: std::collections::HashSet<i32> = t.numbers.values().copied().collect();
        assert_eq!(nums.len(), 50);
        assert!(t.roster.iter().all(|p| p.jersey == t.numbers.get(&p.id).copied()));
    }

    #[test]
    fn depth_order_puts_listed_active_players_first_then_the_rest_by_rating() {
        let mut t = team(4);
        let qbs: Vec<u32> = t.roster.iter().filter(|p| p.position == Position::Qb).map(|p| p.id).collect();
        let auto: Vec<u32> = t.depth_order(Position::Qb).iter().map(|p| p.id).collect();
        let ovrs: Vec<i32> = t.depth_order(Position::Qb).iter().map(|p| p.ovr()).collect();
        assert!(ovrs.windows(2).all(|w| w[0] >= w[1]));
        let reversed: Vec<u32> = auto.iter().rev().copied().collect();
        t.set_depth(Position::Qb, reversed.clone());
        assert_eq!(t.depth_order(Position::Qb).iter().map(|p| p.id).collect::<Vec<_>>(), reversed);
        t.auto_sort(Position::Qb);
        assert_eq!(t.depth_order(Position::Qb).iter().map(|p| p.id).collect::<Vec<_>>(), auto);
        assert!(qbs.len() >= auto.len());
    }

    #[test]
    fn return_order_is_automatic_until_you_set_it() {
        let mut t = team(6);
        let auto: Vec<u32> = t.return_order(ReturnRole::Kick).iter().map(|p| p.id).collect();
        assert!(!auto.is_empty());
        t.set_return_depth(ReturnRole::Kick, vec![*auto.last().unwrap()]);
        assert_eq!(t.return_order(ReturnRole::Kick)[0].id, *auto.last().unwrap());
        t.auto_sort_all();
        assert_eq!(t.return_order(ReturnRole::Kick).iter().map(|p| p.id).collect::<Vec<_>>(), auto);
    }

    #[test]
    fn team_survives_json_round_trip_including_depth_and_numbers() {
        let mut t = team(4);
        t.ensure_numbers();
        let ids: Vec<u32> = t.depth_order(Position::Wr).iter().map(|p| p.id).rev().collect();
        t.set_depth(Position::Wr, ids.clone());
        t.set_return_depth(ReturnRole::Punt, ids.clone());
        let json = serde_json::to_string(&t).unwrap();
        let back: Team = serde_json::from_str(&json).unwrap();
        assert_eq!(back.roster, t.roster);
        assert_eq!(back.depth[&Position::Wr], ids);
        assert_eq!(back.numbers, t.numbers);
        assert_eq!(back.active_ids, t.active_ids);
        assert_eq!(back.strategy, t.strategy);
    }

    #[test]
    fn wealthier_towns_field_better_teams() {
        let ca = Region::california();
        let rich = ca.starting.iter().find(|t| t.id == "thousand_oaks").unwrap();
        let poor = ca.starting.iter().find(|t| t.id == "stockton").unwrap();
        let avg = |town: &Town| {
            let mut g = TeamGenerator::new(PlayerGenerator::built_in(Rng::new(99)));
            (0..40).map(|_| g.generate(town, &[]).roster.iter().map(|p| p.ovr() as f64).sum::<f64>() / 50.0).sum::<f64>() / 40.0
        };
        assert!(avg(rich) > avg(poor));
    }

    #[test]
    fn players_are_deterministic_across_a_whole_team() {
        let (a, b) = (team(42), team(42));
        assert_eq!(a.roster, b.roster);
    }
}
