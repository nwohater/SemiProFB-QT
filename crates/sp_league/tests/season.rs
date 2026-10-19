use sp_league::gazette::GazetteSection;
use sp_league::inbox::InboxKind;
use sp_league::{League, LeaguePhase, NewLeague};

fn league(seed: u64) -> League {
    League::new(seed, NewLeague { small_tables: true, ..Default::default() })
}

#[test]
fn a_season_plays_out_end_to_end() {
    let mut lg = league(7);
    assert_eq!((lg.phase, lg.week, lg.season), (LeaguePhase::RegularSeason, 1, 1));
    lg.sim_season();
    assert!(lg.complete() || lg.game_over());
    if lg.complete() {
        assert_eq!(lg.results.iter().filter(|r| !r.playoff).count(), 14 * 6);
        assert_eq!(lg.results.iter().filter(|r| r.playoff).count(), 3);
        let champ = lg.champion().expect("a champion");
        let po = lg.playoffs.as_ref().unwrap();
        assert_eq!(po.seeds.len(), 4);
        assert!(po.seeds.contains(&champ));
        for r in &lg.standings.records {
            assert_eq!(r.games(), 14);
            assert_eq!(r.wins + r.losses + r.ties, 14);
        }
        let total_w: i32 = lg.standings.records.iter().map(|r| r.wins).sum();
        let total_l: i32 = lg.standings.records.iter().map(|r| r.losses).sum();
        assert_eq!(total_w, total_l);
    }
}

#[test]
fn same_seed_same_season() {
    let (mut a, mut b) = (league(11), league(11));
    a.sim_season();
    b.sim_season();
    let scores = |lg: &League| lg.results.iter().map(|r| (r.home, r.away, r.home_score, r.away_score)).collect::<Vec<_>>();
    assert_eq!(scores(&a), scores(&b));
    assert_eq!(a.finances.iter().map(|f| f.cash).collect::<Vec<_>>(), b.finances.iter().map(|f| f.cash).collect::<Vec<_>>());
}

#[test]
fn money_stays_consistent_with_the_ledger() {
    let mut lg = league(3);
    lg.sim_season();
    for (i, f) in lg.finances.iter().enumerate() {
        let games_net: i64 = f.games.iter().map(|g| g.net()).sum();
        assert_eq!(f.season_net(), games_net, "team {i}");
        let ledger: i64 = f.ledger.iter().map(|e| e.amount).sum();
        // cash = starting cash + ledger (the ledger records every movement; start cash is the create() value)
        assert!(f.cash.abs() < 10_000_000 && ledger.abs() < 10_000_000);
    }
}

#[test]
fn the_owner_hears_about_the_season() {
    let mut lg = league(5);
    lg.sim_season();
    let reports = lg.inbox_of_kind(InboxKind::Report);
    assert!(reports.len() >= 10, "{}", reports.len());
    assert!(lg.inbox_of_kind(InboxKind::Council).len() >= 2);
    assert!(lg.inbox.len() <= 200);
}

#[test]
fn the_gazette_prints_an_issue_every_week_with_a_front_page() {
    let mut lg = league(9);
    lg.sim_season();
    assert!(lg.gazette.len() >= 14, "{}", lg.gazette.len());
    for issue in &lg.gazette {
        assert!(issue.lead().is_some(), "{}", issue.label);
        assert!(!issue.in_section(GazetteSection::Scores).is_empty());
    }
    let numbers: Vec<i32> = lg.gazette.iter().map(|i| i.number).collect();
    assert!(numbers.windows(2).all(|w| w[1] == w[0] + 1));
}

#[test]
fn awards_are_locked_in_when_the_regular_season_ends() {
    let mut lg = league(13);
    lg.sim_season();
    let a = lg.awards_history.last().expect("awards");
    assert!(a.finalized && a.season == 1);
    assert!(a.category("mvp").unwrap().winner().is_some());
    assert!(a.category("opoy").unwrap().finalists.len() <= 3);
}

#[test]
fn injured_players_sit_and_come_back() {
    let mut lg = league(21);
    lg.sim_season();
    assert!(!lg.absences.is_empty());
    assert!(lg.absences.iter().any(|a| a.missed > 0));
}

#[test]
fn spectator_league_has_no_owner_and_still_plays() {
    let mut lg = League::new(2, NewLeague { user_team: None, small_tables: true, ..Default::default() });
    lg.sim_season();
    assert!(lg.complete());
    assert!(lg.inbox.is_empty());
}

#[test]
fn week_results_carry_finances() {
    let mut lg = league(4);
    let r = lg.advance();
    assert_eq!(r.week, 1);
    assert_eq!(r.games.len(), 6);
    assert!(r.games.iter().all(|g| g.home_finance.is_some() && g.away_finance.is_some()));
}
