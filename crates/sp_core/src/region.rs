use crate::rng::Rng;
use crate::town::Town;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

const REGIONS_JSON: &str = include_str!("../../../data/regions.json");
const NICKNAMES_JSON: &str = include_str!("../../../data/nicknames.json");
const TOWN_META_JSON: &str = include_str!("../../../data/town_meta.json");

/// A playable league region: 12 starting towns in 3 divisions of 4, plus a reserve pool of 5 more per division.
/// Loaded from `data/regions.json` (extracted from the Dart tables).
#[derive(Clone, Debug, Deserialize)]
pub struct Region {
    pub id: String,
    pub name: String,
    pub divisions: Vec<String>,
    /// The division on the water (a little wetter and windier). Empty if none.
    pub coastal: String,
    /// Cold winters: snow games, more so late in the season.
    pub cold: bool,
    pub starting: Vec<Town>,
    pub replacement: Vec<Town>,
    pub rivalries: Vec<(String, String)>,
}

#[derive(Deserialize)]
struct NicknameData {
    nicknames: Vec<String>,
    regions: HashMap<String, Vec<String>>,
    mascots: Vec<String>,
}

#[derive(Deserialize)]
struct TownMeta {
    abbreviations: HashMap<String, String>,
    coords: HashMap<String, (f64, f64)>,
}

fn regions() -> &'static Vec<Region> {
    static R: OnceLock<Vec<Region>> = OnceLock::new();
    R.get_or_init(|| {
        let map: HashMap<String, Region> = serde_json::from_str(REGIONS_JSON).expect("regions.json");
        ["ca", "tx", "fl", "pa"].iter().map(|id| map[*id].clone()).collect()
    })
}

fn nickname_data() -> &'static NicknameData {
    static D: OnceLock<NicknameData> = OnceLock::new();
    D.get_or_init(|| serde_json::from_str(NICKNAMES_JSON).expect("nicknames.json"))
}

fn town_meta() -> &'static TownMeta {
    static D: OnceLock<TownMeta> = OnceLock::new();
    D.get_or_init(|| serde_json::from_str(TOWN_META_JSON).expect("town_meta.json"))
}

pub fn town_abbreviation(id: &str) -> Option<String> {
    town_meta().abbreviations.get(id).cloned()
}

pub(crate) fn town_coords(lowercase_name: &str) -> Option<(f64, f64)> {
    town_meta().coords.get(lowercase_name).copied()
}

impl Region {
    pub fn all() -> &'static [Region] {
        regions()
    }

    pub fn california() -> &'static Region {
        Self::by_id(Some("ca"))
    }

    /// The region with this id, or California when unknown.
    pub fn by_id(id: Option<&str>) -> &'static Region {
        let all = regions();
        all.iter().find(|r| Some(r.id.as_str()) == id).unwrap_or(&all[0])
    }

    /// The region a set of town ids belongs to (California if unknown).
    pub fn of_towns(ids: &[String]) -> &'static Region {
        let all = regions();
        ids.first().and_then(|first| all.iter().find(|r| r.all_towns().any(|t| &t.id == first))).unwrap_or(&all[0])
    }

    pub fn all_towns(&self) -> impl Iterator<Item = &Town> {
        self.starting.iter().chain(self.replacement.iter())
    }

    pub fn town(&self, id: &str) -> Option<&Town> {
        self.all_towns().find(|t| t.id == id)
    }

    /// The fixed original league: starting towns, division by division.
    pub fn default_league_towns(&self) -> Vec<String> {
        self.starting.iter().map(|t| t.id.clone()).collect()
    }

    /// A random league: 4 of the 9 towns in each division, alphabetical within a division. The other 5 per division
    /// become the replacement pool for bankrupt clubs.
    pub fn deal_league_towns(&self, rng: &mut Rng) -> Vec<String> {
        let mut out = Vec::new();
        for d in &self.divisions {
            let mut pool: Vec<&Town> = self.all_towns().filter(|t| &t.division == d).collect();
            rng.shuffle(&mut pool);
            let mut picked: Vec<&Town> = pool.into_iter().take(4).collect();
            picked.sort_by(|a, b| a.name.cmp(&b.name));
            out.extend(picked.iter().map(|t| t.id.clone()));
        }
        out
    }
}

/// True when two towns are rivals, in any region.
pub fn is_rivalry(a: &str, b: &str) -> bool {
    regions().iter().flat_map(|r| r.rivalries.iter()).any(|(x, y)| (x == a && y == b) || (x == b && y == a))
}

/// Managed club nicknames. Dealt uniquely at new game; retired names stay unused until the table is exhausted.
pub fn nickname_table() -> &'static [String] {
    &nickname_data().nicknames
}

pub fn mascot_table() -> &'static [String] {
    &nickname_data().mascots
}

/// The nicknames that suit `region_id` (all of them when `None`). A name with no region tags fits anywhere.
pub fn nicknames_for(region_id: Option<&str>) -> Vec<String> {
    let d = nickname_data();
    d.nicknames
        .iter()
        .filter(|n| match (region_id, d.regions.get(*n)) {
            (Some(r), Some(tags)) => tags.iter().any(|t| t == r),
            _ => true,
        })
        .cloned()
        .collect()
}

/// Shuffle unique nicknames onto `towns`. `reserved` (town id -> nick) is applied first.
pub fn deal_nicknames(towns: &[Town], rng: &mut Rng, reserved: &HashMap<String, String>, region_id: Option<&str>) -> Vec<Town> {
    let mut out = towns.to_vec();
    let mut taken = std::collections::HashSet::new();
    let mut kept = vec![false; out.len()];
    for (i, t) in out.iter_mut().enumerate() {
        let nick = reserved.get(&t.id).map(|s| s.trim()).unwrap_or("");
        if nick.is_empty() || !taken.insert(nick.to_string()) {
            continue;
        }
        t.nickname = nick.to_string();
        kept[i] = true;
    }
    let mut pool: Vec<String> = nicknames_for(region_id).into_iter().filter(|n| !taken.contains(n)).collect();
    rng.shuffle(&mut pool);
    let mut p = 0;
    for (i, t) in out.iter_mut().enumerate() {
        if kept[i] {
            continue;
        }
        t.nickname = pool[p].clone();
        p += 1;
    }
    out
}

/// Next identity from the table. Prefers names never used in this franchise, then names not currently in the league.
pub fn pick_unused_nickname(rng: &mut Rng, used_ever: &std::collections::HashSet<String>, in_use: &std::collections::HashSet<String>, region_id: Option<&str>) -> String {
    let table = nicknames_for(region_id);
    let fresh: Vec<&String> = table.iter().filter(|n| !used_ever.contains(*n)).collect();
    if !fresh.is_empty() {
        return rng.pick(&fresh).to_string();
    }
    let free: Vec<&String> = table.iter().filter(|n| !in_use.contains(*n)).collect();
    if free.is_empty() { rng.pick(&table).clone() } else { rng.pick(&free).to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geo::known_town;

    #[test]
    fn four_regions_each_12_plus_15_with_unique_ids() {
        assert_eq!(Region::all().len(), 4);
        for r in Region::all() {
            assert_eq!(r.starting.len(), 12, "{}", r.id);
            assert_eq!(r.replacement.len(), 15, "{}", r.id);
            let ids: std::collections::HashSet<_> = r.all_towns().map(|t| &t.id).collect();
            assert_eq!(ids.len(), 27, "{}", r.id);
            for d in &r.divisions {
                assert_eq!(r.starting.iter().filter(|t| &t.division == d).count(), 4, "{} {d}", r.id);
                assert_eq!(r.replacement.iter().filter(|t| &t.division == d).count(), 5, "{} {d}", r.id);
            }
        }
    }

    #[test]
    fn every_town_has_coordinates_and_an_abbreviation() {
        for r in Region::all() {
            for t in r.all_towns() {
                assert!(known_town(&t.name), "no coords for {}", t.name);
                assert!(town_abbreviation(&t.id).is_some(), "no abbr for {}", t.id);
            }
        }
    }

    #[test]
    fn rivalries_reference_real_towns() {
        for r in Region::all() {
            for (a, b) in &r.rivalries {
                assert!(r.town(a).is_some() && r.town(b).is_some(), "{} {a}-{b}", r.id);
            }
        }
        assert!(is_rivalry("fresno", "visalia") && is_rivalry("visalia", "fresno"));
        assert!(!is_rivalry("fresno", "erie"));
    }

    #[test]
    fn deal_gives_four_per_division_sorted() {
        let r = Region::california();
        let ids = r.deal_league_towns(&mut Rng::new(5));
        assert_eq!(ids.len(), 12);
        for (d, chunk) in r.divisions.iter().zip(ids.chunks(4)) {
            let names: Vec<&str> = chunk.iter().map(|id| r.town(id).unwrap().name.as_str()).collect();
            assert!(chunk.iter().all(|id| &r.town(id).unwrap().division == d));
            assert!(names.windows(2).all(|w| w[0] <= w[1]), "{names:?}");
        }
    }

    #[test]
    fn nicknames_are_unique_and_respect_regions() {
        let r = Region::by_id(Some("pa"));
        let towns = deal_nicknames(&r.starting, &mut Rng::new(2), &HashMap::new(), Some("pa"));
        let nicks: std::collections::HashSet<_> = towns.iter().map(|t| &t.nickname).collect();
        assert_eq!(nicks.len(), 12);
        let allowed = nicknames_for(Some("pa"));
        assert!(towns.iter().all(|t| allowed.contains(&t.nickname)));
        assert!(!allowed.contains(&"Gatormen".to_string()));
    }

    #[test]
    fn reserved_nicknames_stick() {
        let r = Region::california();
        let reserved: HashMap<String, String> = [("fresno".to_string(), "Firehawks".to_string())].into();
        let towns = deal_nicknames(&r.starting, &mut Rng::new(2), &reserved, Some("ca"));
        assert_eq!(towns[0].nickname, "Firehawks");
        assert_eq!(towns.iter().filter(|t| t.nickname == "Firehawks").count(), 1);
    }

    #[test]
    fn pick_unused_prefers_fresh_names() {
        let table = nicknames_for(None);
        let used: std::collections::HashSet<String> = table.iter().skip(1).cloned().collect();
        let n = pick_unused_nickname(&mut Rng::new(1), &used, &Default::default(), None);
        assert_eq!(n, table[0]);
    }
}
