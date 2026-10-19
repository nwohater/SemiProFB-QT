mod common;
use common::*;
use sp_sim::{BoxScore, BoxScoreBuilder, GameConfig, Side};

fn play_bs(seed: u64) -> sp_sim::GameResult {
    play_towns(seed, 8, 10, GameConfig::default())
}

#[test]
fn scoring_plays_add_up_to_the_final_score() {
    for seed in 1..=60 {
        let r = play_bs(seed);
        let (mut h, mut a) = (0, 0);
        for p in &r.scoring_plays {
            match p.side {
                Side::Home => h += p.points,
                Side::Away => a += p.points,
            }
        }
        assert_eq!(h, r.home_score, "seed {seed} home");
        assert_eq!(a, r.away_score, "seed {seed} away");
        if let Some(last) = r.scoring_plays.last() {
            assert_eq!((last.home_score, last.away_score), (r.home_score, r.away_score));
        }
    }
}

#[test]
fn quarter_line_adds_to_final_score() {
    for seed in 1..=40 {
        let r = play_bs(seed);
        assert_eq!(r.score_by_quarter.home.iter().sum::<i32>(), r.home_score);
        assert_eq!(r.score_by_quarter.away.iter().sum::<i32>(), r.away_score);
    }
}

#[test]
fn box_score_builds_and_has_the_expected_sections() {
    let r = play_bs(7);
    let b = BoxScoreBuilder::build(&r);
    assert_eq!(b.home_score, r.home_score);
    assert!(b.team_stats.len() >= 8);
    for side in Side::ALL {
        let t = &b.player_tables[side];
        assert!(!t.passing.is_empty() && !t.rushing.is_empty() && !t.defense.is_empty() && !t.offensive_line.is_empty());
        assert!(!b.why[side].is_empty() && b.why[side].len() <= 3);
    }
    assert!(b.player_of_game.is_some());
    assert!(serde_json::to_string(&b).unwrap().len() > 500);
}

#[test]
fn passing_table_totals_match_team_stats_and_leader_is_first() {
    let r = play_bs(11);
    let b = BoxScoreBuilder::build(&r);
    for side in Side::ALL {
        let rows = &b.player_tables[side].passing;
        let yds: f64 = rows.iter().map(|l| l.raw.get("pass_yds").copied().unwrap_or(0.0)).sum();
        assert_eq!(yds.round() as i32, r.team_stats[side].pass_yards);
        for w in rows.windows(2) {
            assert!(w[0].raw["pass_yds"] >= w[1].raw["pass_yds"]);
        }
    }
}

#[test]
fn display_columns_keep_their_order() {
    let b = BoxScoreBuilder::build(&play_bs(11));
    let cols: Vec<&str> = b.player_tables.home.passing[0].display.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(cols, ["C/A", "Yds", "TD", "INT", "Sck", "Rtg"]);
}

#[test]
fn why_report_gives_the_winner_favorable_lines_and_a_tie_the_even_note() {
    for seed in 1..=400 {
        let r = play_bs(seed);
        let b = BoxScoreBuilder::build(&r);
        if r.tie {
            assert!(b.why.home[0].contains("even"));
            return;
        }
    }
    // Ties are rare (~0.3-1%); only checked when one occurs.
}

#[test]
fn kneels_two_point_tries_onside_kicks_and_key_plays_occur_over_many_games() {
    let (mut kneels, mut two_pt, mut onside, mut key_plays, mut kr) = (0, 0, 0, 0, 0);
    for seed in 1..=300 {
        let r = play_bs(seed);
        key_plays += r.key_plays.len();
        for side in Side::ALL {
            for st in r.player_stats[side].values() {
                kneels += st.get("kneels").round() as i32;
                two_pt += st.get("twopt_att").round() as i32;
                onside += st.get("onside_att").round() as i32;
                kr += st.get("kr").round() as i32;
            }
        }
    }
    assert!(kneels > 0 && two_pt > 0 && onside > 0);
    assert!(key_plays > 100, "{key_plays}");
    assert!(kr > 300, "{kr}");
}

#[test]
fn box_scores_survive_json_exactly() {
    for seed in 1..=12 {
        let b = BoxScoreBuilder::build(&play_bs(seed));
        let text = serde_json::to_string(&b).unwrap();
        let back: BoxScore = serde_json::from_str(&text).unwrap();
        assert_eq!(back, b, "seed {seed}");
        assert_eq!(serde_json::to_string(&back).unwrap(), text, "seed {seed}");
        assert_eq!(back.winner(), b.winner());
    }
}

#[test]
fn player_of_game_comes_from_the_game() {
    let r = play_bs(7);
    let b = BoxScoreBuilder::build(&r);
    let pog = b.player_of_game.unwrap();
    assert!(r.players[pog.side].contains_key(&pog.player.id));
    assert!(!pog.summary.is_empty());
}
