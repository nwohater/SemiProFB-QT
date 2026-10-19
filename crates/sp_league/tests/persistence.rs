use sp_league::persistence::LeagueSave;
use sp_league::{League, NewLeague};

fn league(seed: u64) -> League {
    League::new(seed, NewLeague { keep_box_scores: true, ..Default::default() })
}

fn scores(lg: &League) -> Vec<(usize, usize, i32, i32)> {
    lg.results.iter().map(|r| (r.home, r.away, r.home_score, r.away_score)).collect()
}

#[test]
fn a_saved_league_loads_and_keeps_playing_exactly_like_the_original() {
    let mut a = league(31);
    for _ in 0..6 {
        a.advance();
    }
    let text = LeagueSave::encode(&a);
    let mut b = LeagueSave::decode(&text).expect("loads");
    assert_eq!(b.week, a.week);
    assert_eq!(b.standings.records, a.standings.records);
    assert_eq!(b.finances.iter().map(|f| f.cash).collect::<Vec<_>>(), a.finances.iter().map(|f| f.cash).collect::<Vec<_>>());
    assert_eq!(b.inbox.len(), a.inbox.len());
    assert_eq!(b.gazette.len(), a.gazette.len());
    // keep playing both: identical results
    a.sim_season();
    b.sim_season();
    assert_eq!(scores(&a), scores(&b));
    assert_eq!(a.champion(), b.champion());
    assert_eq!(a.awards_history, b.awards_history);
}

#[test]
fn the_offseason_survives_a_save_too() {
    let mut a = league(32);
    a.sim_season();
    a.begin_offseason();
    a.sim_draft_to_user();
    let text = LeagueSave::encode(&a);
    let mut b = LeagueSave::decode(&text).expect("loads");
    assert!(b.draft.is_some());
    assert_eq!(b.draft.as_ref().unwrap().made(), a.draft.as_ref().unwrap().made());
    assert_eq!(b.free_agents.len(), a.free_agents.len());
    assert_eq!(b.pending_expired, a.pending_expired);
    a.sim_draft_all();
    b.sim_draft_all();
    a.finish_offseason();
    b.finish_offseason();
    a.sim_season();
    b.sim_season();
    assert_eq!(scores(&a), scores(&b));
    assert_eq!(a.teams.iter().map(|t| t.roster.len()).collect::<Vec<_>>(), b.teams.iter().map(|t| t.roster.len()).collect::<Vec<_>>());
}

#[test]
fn saves_keep_box_scores_for_the_owners_games_only() {
    let mut a = league(33);
    a.sim_season();
    let b = LeagueSave::decode(&LeagueSave::encode(&a)).unwrap();
    let u = a.user();
    for r in &b.results {
        let mine = r.home == u || r.away == u;
        assert_eq!(r.box_score.is_some(), mine, "week {} {}-{}", r.week, r.home, r.away);
    }
}

#[test]
fn bad_or_old_saves_are_refused() {
    assert!(LeagueSave::decode("{not json").is_err());
    let a = league(34);
    let text = LeagueSave::encode(&a).replacen("\"v\":1", "\"v\":99", 1);
    match LeagueSave::decode(&text) {
        Err(e) => assert!(e.contains("Unsupported save version"), "{e}"),
        Ok(_) => panic!("an unknown save version must be refused"),
    }
}

#[test]
fn saves_are_reasonably_small() {
    let mut a = league(35);
    a.sim_season();
    let n = LeagueSave::encode(&a).len();
    assert!(n < 12_000_000, "{n} bytes");
}
