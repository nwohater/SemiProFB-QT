use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use sp_core::Region;
use sp_league::persistence::LeagueSave;
use sp_league::{League, LeaguePhase, NewLeague};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::views;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(i32, rev)]
        #[qproperty(bool, has_game)]
        #[qproperty(QString, assets_url)]
        #[qproperty(QString, notice)]
        type Game = super::GameRust;

        // ---- reads (JSON view-models)
        #[qinvokable]
        fn regions_json(self: &Game) -> QString;
        #[qinvokable]
        fn deal_json(self: &Game, region: &QString, seed: i32) -> QString;
        #[qinvokable]
        fn nicknames_json(self: &Game, region: &QString) -> QString;
        #[qinvokable]
        fn header_json(self: &Game) -> QString;
        #[qinvokable]
        fn home_json(self: &Game) -> QString;
        #[qinvokable]
        fn roster_json(self: &Game) -> QString;
        #[qinvokable]
        fn standings_json(self: &Game) -> QString;
        #[qinvokable]
        fn inbox_json(self: &Game) -> QString;
        #[qinvokable]
        fn story_json(self: &Game, kind: &QString) -> QString;
        #[qinvokable]
        fn leaders_json(self: &Game, category: &QString, playoffs: bool, career: bool, active_only: bool) -> QString;
        #[qinvokable]
        fn team_stats_json(self: &Game, team: i32, playoffs: bool) -> QString;
        #[qinvokable]
        fn box_score_json(self: &Game, index: i32) -> QString;
        #[qinvokable]
        fn news_json(self: &Game, issue: i32) -> QString;
        #[qinvokable]
        fn league_tab_json(self: &Game, tab: &QString) -> QString;
        #[qinvokable]
        fn offseason_json(self: Pin<&mut Game>) -> QString;
        #[qinvokable]
        fn has_save(self: &Game) -> bool;
        #[qinvokable]
        fn trade_desk_json(self: &Game, partner: i32) -> QString;
        #[qinvokable]
        fn trade_eval_json(self: &Game, offer_json: &QString) -> QString;
        #[qinvokable]
        fn depth_json(self: &Game) -> QString;
        #[qinvokable]
        fn depth_candidates_json(self: &Game, key: &QString) -> QString;
        #[qinvokable]
        fn strategy_json(self: &Game) -> QString;
        #[qinvokable]
        fn finances_json(self: &Game) -> QString;
        #[qinvokable]
        fn draft_json(self: &Game) -> QString;
        #[qinvokable]
        fn jerseys_json(self: &Game) -> QString;
        #[qinvokable]
        fn negotiation_json(self: Pin<&mut Game>, player_id: i32, seasons: i32) -> QString;
        #[qinvokable]
        fn free_agents_json(self: Pin<&mut Game>) -> QString;
        #[qinvokable]
        fn player_json(self: Pin<&mut Game>, player_id: i32) -> QString;

        // ---- actions
        #[qinvokable]
        fn new_game(self: Pin<&mut Game>, region: &QString, town_ids_json: &QString, user_id: &QString, nicknames_json: &QString) -> bool;
        #[qinvokable]
        fn advance(self: Pin<&mut Game>) -> QString;
        #[qinvokable]
        fn begin_offseason(self: Pin<&mut Game>);
        #[qinvokable]
        fn finish_offseason(self: Pin<&mut Game>) -> QString;
        #[qinvokable]
        fn set_active(self: Pin<&mut Game>, player_id: i32, active: bool) -> bool;
        #[qinvokable]
        fn auto_set_active(self: Pin<&mut Game>);
        #[qinvokable]
        fn set_auto_roster(self: Pin<&mut Game>, on: bool);
        #[qinvokable]
        fn set_auto_depth(self: Pin<&mut Game>, on: bool);
        #[qinvokable]
        fn mark_inbox_read(self: Pin<&mut Game>);
        #[qinvokable]
        fn mark_item_read(self: Pin<&mut Game>, item_id: i32);
        #[qinvokable]
        fn save(self: Pin<&mut Game>) -> bool;
        #[qinvokable]
        fn load(self: Pin<&mut Game>) -> bool;
        #[qinvokable]
        fn quit_to_title(self: Pin<&mut Game>);
        #[qinvokable]
        fn music_json(self: &Game) -> QString;
        #[qinvokable]
        fn finance_sheet_json(self: &Game) -> QString;
        #[qinvokable]
        fn legacy_json(self: &Game) -> QString;
        #[qinvokable]
        fn downloads_dir(self: &Game) -> QString;
        #[qinvokable]
        fn help_json(self: &Game) -> QString;
        #[qinvokable]
        fn settings_json(self: &Game) -> QString;
        #[qinvokable]
        fn set_setting(self: &Game, key: &QString, value_json: &QString);
        #[qinvokable]
        fn backup_save(self: Pin<&mut Game>) -> bool;
        #[qinvokable]
        fn restore_backup(self: Pin<&mut Game>) -> bool;
        #[qinvokable]
        fn abandon_franchise(self: Pin<&mut Game>);
        #[qinvokable]
        fn propose_trade(self: Pin<&mut Game>, offer_json: &QString) -> QString;
        #[qinvokable]
        fn set_depth(self: Pin<&mut Game>, key: &QString, ids_json: &QString);
        #[qinvokable]
        fn auto_sort(self: Pin<&mut Game>, key: &QString);
        #[qinvokable]
        fn auto_sort_all(self: Pin<&mut Game>);
        #[qinvokable]
        fn set_strategy(self: Pin<&mut Game>, strategy_json: &QString);
        #[qinvokable]
        fn help_club(self: Pin<&mut Game>, item_id: i32, amount: i32) -> bool;
        #[qinvokable]
        fn decline_plea(self: Pin<&mut Game>, item_id: i32);
        #[qinvokable]
        fn pay_conduct(self: Pin<&mut Game>, item_id: i32) -> bool;
        #[qinvokable]
        fn let_him_sit(self: Pin<&mut Game>, item_id: i32) -> bool;
        #[qinvokable]
        fn set_ticket_price(self: Pin<&mut Game>, cents: i32);
        #[qinvokable]
        fn schedule_promo(self: Pin<&mut Game>, week: i32, promo_id: &QString) -> bool;
        #[qinvokable]
        fn cancel_promo(self: Pin<&mut Game>, week: i32) -> bool;
        #[qinvokable]
        fn accept_sponsor(self: Pin<&mut Game>, index: i32) -> bool;
        #[qinvokable]
        fn cancel_sponsor(self: Pin<&mut Game>, index: i32) -> bool;
        #[qinvokable]
        fn upgrade_facility(self: Pin<&mut Game>, key: &QString) -> bool;
        #[qinvokable]
        fn downgrade_facility(self: Pin<&mut Game>, key: &QString) -> bool;
        #[qinvokable]
        fn take_loan(self: Pin<&mut Game>, amount: i32) -> bool;
        #[qinvokable]
        fn repay_loan(self: Pin<&mut Game>, amount: i32) -> bool;
        #[qinvokable]
        fn draft_player(self: Pin<&mut Game>, player_id: i32) -> QString;
        #[qinvokable]
        fn pass_pick(self: Pin<&mut Game>) -> QString;
        #[qinvokable]
        fn sim_draft_to_user(self: Pin<&mut Game>);
        #[qinvokable]
        fn sim_draft_all(self: Pin<&mut Game>);
        #[qinvokable]
        fn offer(self: Pin<&mut Game>, player_id: i32, ppg: i32, seasons: i32, bonus: i32, round: i32) -> QString;
        #[qinvokable]
        fn change_position(self: Pin<&mut Game>, player_id: i32, pos: &QString) -> bool;
        #[qinvokable]
        fn release_player(self: Pin<&mut Game>, player_id: i32) -> i32;
        #[qinvokable]
        fn hold_tryouts(self: Pin<&mut Game>, position: &QString) -> i32;
    }
}

pub struct GameRust {
    league: Option<League>,
    rev: i32,
    has_game: bool,
    assets_url: QString,
    notice: QString,
}

impl Default for GameRust {
    fn default() -> Self {
        Self { league: None, rev: 0, has_game: false, assets_url: QString::from(&assets_url()), notice: QString::default() }
    }
}

/// Where the art and music live: `$SEMI_PRO_ASSETS`, `./assets`, next to the repo when run from `target/`, or the installed copy.
fn assets_dir() -> PathBuf {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(p) = std::env::var("SEMI_PRO_ASSETS") {
        candidates.push(p.into());
    }
    candidates.push("assets".into());
    if let Ok(exe) = std::env::current_exe() {
        for up in 1..=4 {
            if let Some(d) = exe.ancestors().nth(up) {
                candidates.push(d.join("assets"));
            }
        }
    }
    candidates.push("/usr/share/semi-pro-football/assets".into());
    candidates.into_iter().find(|p| p.join("artwork").is_dir()).unwrap_or_else(|| "assets".into())
}

fn assets_url() -> String {
    let d = assets_dir();
    let d = d.canonicalize().unwrap_or(d);
    format!("file://{}/", d.display())
}

fn save_path() -> PathBuf {
    let base = std::env::var("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into())).join(".local/share"));
    base.join("semi-pro-football").join("save.json")
}

fn backup_path() -> PathBuf {
    save_path().with_file_name("save.backup.json")
}

fn settings_path() -> PathBuf {
    save_path().with_file_name("settings.json")
}

fn read_settings() -> serde_json::Map<String, serde_json::Value> {
    std::fs::read_to_string(settings_path()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn file_info(p: &std::path::Path) -> serde_json::Value {
    match std::fs::metadata(p) {
        Ok(m) => {
            let secs = m.modified().ok().and_then(|t| t.elapsed().ok()).map(|d| d.as_secs()).unwrap_or(0);
            serde_json::json!({ "bytes": m.len(), "secsAgo": secs })
        }
        Err(_) => serde_json::Value::Null,
    }
}

fn facility_by_key(key: &str) -> Option<sp_economy::business::Facility> {
    sp_economy::business::Facility::ALL.iter().copied().find(|f| f.spec().key == key)
}

fn q(s: impl AsRef<str>) -> QString {
    QString::from(s.as_ref())
}

fn json_q(v: serde_json::Value) -> QString {
    q(v.to_string())
}

fn seed_now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1)
}

impl qobject::Game {
    fn read<T>(&self, f: impl FnOnce(&League) -> T, empty: T) -> T {
        match self.rust().league.as_ref() {
            Some(lg) => f(lg),
            None => empty,
        }
    }

    fn bump(mut self: Pin<&mut Self>) {
        // Jersey numbers follow the roster; refresh them after anything that could have changed it.
        let inner: &mut GameRust = self.as_mut().rust_mut().get_mut();
        if let Some(lg) = inner.league.as_mut() {
            let u = lg.user();
            if lg.teams[u].numbers.len() != lg.teams[u].roster.len() {
                lg.teams[u].ensure_numbers();
            }
        }
        let r = self.rev() + 1;
        self.as_mut().set_rev(r);
    }

    fn lg(self: Pin<&mut Self>) -> &mut League {
        self.get_mut_league()
    }

    fn get_mut_league(self: Pin<&mut Self>) -> &mut League {
        // SAFETY of the unwrap: every caller checks `has_game` in QML; a missing league is a programming error.
        let inner: &mut GameRust = self.rust_mut().get_mut();
        inner.league.as_mut().expect("no game loaded")
    }

    fn regions_json(&self) -> QString {
        json_q(views::regions())
    }

    fn deal_json(&self, region: &QString, seed: i32) -> QString {
        let s = if seed == 0 { seed_now() } else { seed as u64 };
        json_q(views::deal(&region.to_string(), s))
    }

    fn nicknames_json(&self, region: &QString) -> QString {
        json_q(views::nicknames(&region.to_string()))
    }

    fn header_json(&self) -> QString {
        self.read(|lg| json_q(views::header(lg)), q("{}"))
    }

    fn home_json(&self) -> QString {
        self.read(|lg| json_q(views::home(lg)), q("{}"))
    }

    fn roster_json(&self) -> QString {
        self.read(|lg| json_q(views::roster(lg)), q("{}"))
    }

    fn standings_json(&self) -> QString {
        self.read(|lg| json_q(views::standings(lg)), q("{}"))
    }

    fn league_tab_json(&self, tab: &QString) -> QString {
        let tab = tab.to_string();
        self.read(|lg| json_q(views::league_tab(lg, &tab)), q("{}"))
    }

    fn news_json(&self, issue: i32) -> QString {
        self.read(|lg| json_q(views::news(lg, issue as i64)), q("{}"))
    }

    fn leaders_json(&self, category: &QString, playoffs: bool, career: bool, active_only: bool) -> QString {
        let c = category.to_string();
        self.read(|lg| json_q(views::leaders(lg, &c, playoffs, career, active_only)), q("{}"))
    }

    fn team_stats_json(&self, team: i32, playoffs: bool) -> QString {
        self.read(|lg| json_q(views::team_stats(lg, team as i64, playoffs)), q("{}"))
    }

    fn box_score_json(&self, index: i32) -> QString {
        self.read(|lg| json_q(views::box_score(lg, index.max(0) as usize)), q("{}"))
    }

    fn story_json(&self, kind: &QString) -> QString {
        let k = kind.to_string();
        self.read(|lg| json_q(views::story(lg, &k)), q("{}"))
    }

    fn inbox_json(&self) -> QString {
        self.read(|lg| json_q(views::inbox(lg)), q("{}"))
    }

    fn negotiation_json(mut self: Pin<&mut Self>, player_id: i32, seasons: i32) -> QString {
        json_q(views::negotiation(self.as_mut().lg(), player_id as u32, seasons))
    }

    fn free_agents_json(mut self: Pin<&mut Self>) -> QString {
        json_q(views::free_agents(self.as_mut().lg()))
    }

    fn player_json(mut self: Pin<&mut Self>, player_id: i32) -> QString {
        json_q(views::player_detail(self.as_mut().lg(), player_id as u32))
    }

    /// One negotiation round. An accepted offer signs the player; the answer carries the outcome, his line and any counter.
    fn offer(mut self: Pin<&mut Self>, player_id: i32, ppg: i32, seasons: i32, bonus: i32, round: i32) -> QString {
        use sp_economy::negotiation::OfferOutcome;
        let id = player_id as u32;
        let lg = self.as_mut().lg();
        let r = lg.negotiate(id, ppg as i64, seasons, bonus as i64, round);
        let signed = r.outcome == OfferOutcome::Accept && lg.sign_player(id, ppg as i64, seasons, bonus as i64);
        let out = serde_json::json!({
            "outcome": match r.outcome { OfferOutcome::Accept => "accept", OfferOutcome::Counter => "counter", OfferOutcome::Reject => "reject" },
            "counter": r.counter_ppg, "line": r.line, "signed": signed,
        });
        if signed {
            self.as_mut().save();
        }
        self.bump();
        json_q(out)
    }

    fn change_position(self: Pin<&mut Self>, player_id: i32, pos: &QString) -> bool {
        let label = pos.to_string();
        let Some(p) = sp_core::Position::ALL.iter().copied().find(|p| p.label() == label) else { return false };
        self.act(|lg| lg.change_position(player_id as u32, p))
    }

    fn release_player(mut self: Pin<&mut Self>, player_id: i32) -> i32 {
        let cost = self.as_mut().lg().release_player(player_id as u32);
        self.as_mut().save();
        self.bump();
        cost as i32
    }

    fn hold_tryouts(mut self: Pin<&mut Self>, position: &QString) -> i32 {
        let label = position.to_string();
        let pos = views::position_groups().into_iter().find(|(l, _)| *l == label).map(|(_, ps)| ps[0]);
        let n = self.as_mut().lg().hold_tryouts(pos, 8).len();
        self.as_mut().save();
        self.bump();
        n as i32
    }

    /// Jersey art on disk: team slug -> file name (`slug_numbercolor.png`).
    fn jerseys_json(&self) -> QString {
        let mut map = serde_json::Map::new();
        if let Ok(rd) = std::fs::read_dir(assets_dir().join("artwork/jerseys")) {
            for e in rd.flatten() {
                let file = e.file_name().to_string_lossy().to_string();
                if let (Some(stem), true) = (file.strip_suffix(".png"), file.contains('_')) {
                    if let Some(i) = stem.rfind('_') {
                        map.insert(stem[..i].to_string(), serde_json::Value::String(file.clone()));
                    }
                }
            }
        }
        json_q(serde_json::Value::Object(map))
    }

    fn draft_json(&self) -> QString {
        self.read(|lg| json_q(views::draft(lg)), q("{}"))
    }

    fn draft_player(mut self: Pin<&mut Self>, player_id: i32) -> QString {
        let r = self.as_mut().lg().draft_player(player_id as u32);
        self.as_mut().save();
        self.bump();
        match r { Ok(_) => q(""), Err(e) => q(e) }
    }

    fn pass_pick(mut self: Pin<&mut Self>) -> QString {
        let r = self.as_mut().lg().pass_pick();
        self.as_mut().save();
        self.bump();
        match r { Ok(_) => q(""), Err(e) => q(e) }
    }

    fn sim_draft_to_user(mut self: Pin<&mut Self>) {
        self.as_mut().lg().sim_draft_to_user();
        self.as_mut().save();
        self.bump();
    }

    fn sim_draft_all(mut self: Pin<&mut Self>) {
        self.as_mut().lg().sim_draft_all();
        self.as_mut().save();
        self.bump();
    }

    fn finances_json(&self) -> QString {
        self.read(|lg| json_q(views::finances(lg)), q("{}"))
    }

    /// Runs a money action, saves and refreshes the views.
    fn act<R>(mut self: Pin<&mut Self>, f: impl FnOnce(&mut League) -> R) -> R {
        let r = f(self.as_mut().lg());
        self.as_mut().save();
        self.bump();
        r
    }

    fn help_club(self: Pin<&mut Self>, item_id: i32, amount: i32) -> bool {
        self.act(|lg| lg.help_club(item_id as u32, amount as i64).is_some())
    }

    fn decline_plea(self: Pin<&mut Self>, item_id: i32) {
        self.act(|lg| lg.decline_plea(item_id as u32));
    }

    fn pay_conduct(self: Pin<&mut Self>, item_id: i32) -> bool {
        self.act(|lg| lg.pay_conduct(item_id as u32))
    }

    fn let_him_sit(self: Pin<&mut Self>, item_id: i32) -> bool {
        self.act(|lg| lg.let_him_sit(item_id as u32))
    }

    fn set_ticket_price(self: Pin<&mut Self>, cents: i32) {
        self.act(|lg| lg.set_ticket_price(cents as f64 / 100.0));
    }

    fn schedule_promo(self: Pin<&mut Self>, week: i32, promo_id: &QString) -> bool {
        let id = promo_id.to_string();
        self.act(|lg| lg.schedule_promo(week, &id))
    }

    fn cancel_promo(self: Pin<&mut Self>, week: i32) -> bool {
        self.act(|lg| lg.cancel_promo(week))
    }

    fn accept_sponsor(self: Pin<&mut Self>, index: i32) -> bool {
        self.act(|lg| lg.accept_sponsor(index as usize))
    }

    fn cancel_sponsor(self: Pin<&mut Self>, index: i32) -> bool {
        self.act(|lg| lg.cancel_sponsor_deal(index as usize))
    }

    fn upgrade_facility(self: Pin<&mut Self>, key: &QString) -> bool {
        let Some(fac) = facility_by_key(&key.to_string()) else { return false };
        self.act(|lg| lg.upgrade_facility(fac))
    }

    fn downgrade_facility(self: Pin<&mut Self>, key: &QString) -> bool {
        let Some(fac) = facility_by_key(&key.to_string()) else { return false };
        self.act(|lg| lg.downgrade_facility(fac))
    }

    fn take_loan(self: Pin<&mut Self>, amount: i32) -> bool {
        self.act(|lg| lg.take_loan(amount as i64))
    }

    fn repay_loan(self: Pin<&mut Self>, amount: i32) -> bool {
        self.act(|lg| lg.repay_loan(amount as i64))
    }

    fn strategy_json(&self) -> QString {
        self.read(|lg| json_q(views::strategy(lg)), q("{}"))
    }

    /// Slider drags call this many times a second, so it does not save; QML calls `save` when the drag ends.
    fn set_strategy(mut self: Pin<&mut Self>, strategy_json: &QString) {
        let Ok(st) = serde_json::from_str::<sp_core::strategy::Strategy>(&strategy_json.to_string()) else { return };
        let lg = self.as_mut().lg();
        let u = lg.user();
        lg.teams[u].strategy = st;
        self.bump();
    }

    fn depth_json(&self) -> QString {
        self.read(|lg| json_q(views::depth(lg)), q("{}"))
    }

    fn depth_candidates_json(&self, key: &QString) -> QString {
        let k = key.to_string();
        self.read(|lg| json_q(views::depth_candidates(lg, &k)), q("[]"))
    }

    fn set_depth(self: Pin<&mut Self>, key: &QString, ids_json: &QString) {
        let k = key.to_string();
        let Ok(ids) = serde_json::from_str::<Vec<u32>>(&ids_json.to_string()) else { return };
        self.act(|lg| {
            let u = lg.user();
            if let Some(pos) = sp_core::Position::ALL.iter().copied().find(|p| p.label() == k) {
                lg.teams[u].set_depth(pos, ids);
            } else if let Some(role) = sp_core::returners::ReturnRole::ALL.iter().copied().find(|r| r.tag() == k) {
                lg.teams[u].set_return_depth(role, ids);
            }
        });
    }

    fn auto_sort(self: Pin<&mut Self>, key: &QString) {
        let k = key.to_string();
        self.act(|lg| {
            let u = lg.user();
            if let Some(pos) = sp_core::Position::ALL.iter().copied().find(|p| p.label() == k) {
                lg.teams[u].auto_sort(pos);
            } else if let Some(role) = sp_core::returners::ReturnRole::ALL.iter().copied().find(|r| r.tag() == k) {
                lg.teams[u].auto_sort_return(role);
            }
        });
    }

    fn auto_sort_all(self: Pin<&mut Self>) {
        self.act(|lg| {
            let u = lg.user();
            lg.teams[u].auto_sort_all();
        });
    }

    fn trade_desk_json(&self, partner: i32) -> QString {
        self.read(|lg| json_q(views::trade_desk(lg, if partner < 0 { None } else { Some(partner as usize) })), q("{}"))
    }

    fn trade_eval_json(&self, offer_json: &QString) -> QString {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&offer_json.to_string()) else { return q("{}") };
        let offer = views::parse_offer(&v);
        self.read(|lg| json_q(views::trade_eval(lg, &offer)), q("{}"))
    }

    fn propose_trade(self: Pin<&mut Self>, offer_json: &QString) -> QString {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&offer_json.to_string()) else { return q("") };
        let offer = views::parse_offer(&v);
        self.act(|lg| {
            let r = lg.propose_trade(&offer);
            if r.accepted { q("") } else { q(r.line) }
        })
    }

    fn offseason_json(mut self: Pin<&mut Self>) -> QString {
        json_q(views::offseason(self.as_mut().lg()))
    }

    fn has_save(&self) -> bool {
        save_path().exists()
    }

    fn new_game(mut self: Pin<&mut Self>, region: &QString, town_ids_json: &QString, user_id: &QString, nicknames_json: &QString) -> bool {
        let Ok(ids) = serde_json::from_str::<Vec<String>>(&town_ids_json.to_string()) else { return false };
        let Ok(nicks) = serde_json::from_str::<HashMap<String, String>>(&nicknames_json.to_string()) else { return false };
        let uid = user_id.to_string();
        let Some(slot) = ids.iter().position(|i| *i == uid) else { return false };
        let reg = Region::by_id(Some(&region.to_string()));
        let lg = League::new(seed_now(), NewLeague { user_team: Some(slot), keep_box_scores: true, town_ids: Some(ids), nicknames: nicks, region: Some(reg), ..Default::default() });
        {
            let inner: &mut GameRust = self.as_mut().rust_mut().get_mut();
            inner.league = Some(lg);
        }
        self.as_mut().set_has_game(true);
        self.as_mut().save();
        self.bump();
        true
    }

    fn advance(mut self: Pin<&mut Self>) -> QString {
        let lg = self.as_mut().lg();
        if lg.game_over() || !matches!(lg.phase, LeaguePhase::RegularSeason | LeaguePhase::Semifinals | LeaguePhase::Championship) {
            return q("{}");
        }
        let r = lg.advance();
        let v = views::week_result(lg, &r);
        self.as_mut().save();
        self.bump();
        json_q(v)
    }

    fn begin_offseason(mut self: Pin<&mut Self>) {
        let lg = self.as_mut().lg();
        if lg.phase == LeaguePhase::Complete {
            lg.begin_offseason();
        }
        self.as_mut().save();
        self.bump();
    }

    fn finish_offseason(mut self: Pin<&mut Self>) -> QString {
        let lg = self.as_mut().lg();
        if lg.phase != LeaguePhase::Offseason {
            return q("{}");
        }
        let res = lg.finish_offseason();
        let v = serde_json::json!({ "season": res.new_season, "replaced": res.replaced, "short": res.user_roster_short, "autoSigned": res.auto_signed.iter().map(|p| p.name.clone()).collect::<Vec<_>>() });
        self.as_mut().save();
        self.bump();
        json_q(v)
    }

    fn set_active(mut self: Pin<&mut Self>, player_id: i32, active: bool) -> bool {
        let lg = self.as_mut().lg();
        let u = lg.user();
        let id = player_id as u32;
        let t = &mut lg.teams[u];
        if !t.roster.iter().any(|p| p.id == id) {
            return false;
        }
        let ok = if active {
            if t.active_ids.len() >= sp_core::team::ACTIVE_ROSTER_SIZE && !t.active_ids.contains(&id) {
                false
            } else {
                t.active_ids.insert(id);
                true
            }
        } else {
            t.active_ids.remove(&id);
            true
        };
        if ok {
            self.bump();
        }
        ok
    }

    fn auto_set_active(mut self: Pin<&mut Self>) {
        self.as_mut().lg().auto_set_active();
        self.bump();
    }

    fn set_auto_roster(mut self: Pin<&mut Self>, on: bool) {
        let lg = self.as_mut().lg();
        lg.auto_set_roster = on;
        if on {
            lg.auto_set_active();
        }
        self.bump();
    }

    fn set_auto_depth(mut self: Pin<&mut Self>, on: bool) {
        let lg = self.as_mut().lg();
        lg.auto_depth = on;
        if on {
            if let Some(u) = lg.user_team {
                lg.teams[u].auto_sort_all();
            }
        }
        self.bump();
    }

    fn mark_item_read(mut self: Pin<&mut Self>, item_id: i32) {
        let changed = match self.as_mut().lg().inbox_item_mut(item_id as u32) {
            Some(i) if i.unread => {
                i.unread = false;
                true
            }
            _ => false,
        };
        if changed {
            self.as_mut().save();
            self.bump();
        }
    }

    fn mark_inbox_read(mut self: Pin<&mut Self>) {
        self.as_mut().lg().mark_inbox_read(None);
        self.bump();
    }

    fn save(mut self: Pin<&mut Self>) -> bool {
        let text = match self.as_mut().rust().league.as_ref() {
            Some(lg) => LeagueSave::encode(lg),
            None => return false,
        };
        let path = save_path();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, text).and_then(|_| std::fs::rename(&tmp, &path)).is_ok()
    }

    fn load(mut self: Pin<&mut Self>) -> bool {
        let Ok(text) = std::fs::read_to_string(save_path()) else { return false };
        match LeagueSave::decode(&text) {
            Ok(lg) => {
                {
                    let inner: &mut GameRust = self.as_mut().rust_mut().get_mut();
                    inner.league = Some(lg);
                }
                self.as_mut().set_has_game(true);
                self.bump();
                true
            }
            Err(e) => {
                self.as_mut().set_notice(q(format!("Could not load the save: {e}")));
                false
            }
        }
    }

    /// Every `.mp3` in assets/music, so dropping a new track in the folder is enough.
    fn music_json(&self) -> QString {
        let mut names: Vec<String> = std::fs::read_dir(assets_dir().join("music"))
            .map(|rd| rd.flatten().map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| n.to_lowercase().ends_with(".mp3")).collect())
            .unwrap_or_default();
        names.sort();
        json_q(serde_json::json!(names))
    }

    fn finance_sheet_json(&self) -> QString {
        self.read(|lg| json_q(views::finance_sheet(lg)), q("{}"))
    }

    fn legacy_json(&self) -> QString {
        self.read(|lg| json_q(views::legacy(lg)), q("{}"))
    }

    /// Where saved pictures go: ~/Downloads if it exists, else the home folder.
    fn downloads_dir(&self) -> QString {
        let home = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()));
        let d = home.join("Downloads");
        q(if d.is_dir() { d } else { home }.to_string_lossy())
    }

    fn help_json(&self) -> QString {
        q(std::fs::read_to_string(assets_dir().join("help.json")).unwrap_or_else(|_| "{\"categories\":[],\"topics\":[]}".into()))
    }

    fn settings_json(&self) -> QString {
        let mut v = serde_json::Value::Object(read_settings());
        let league = self.read(|lg| Some(views::settings(lg)), None);
        v["franchise"] = league.unwrap_or(serde_json::Value::Null);
        v["save"] = file_info(&save_path());
        v["backup"] = file_info(&backup_path());
        v["version"] = serde_json::Value::String(env!("CARGO_PKG_VERSION").to_string());
        json_q(v)
    }

    fn set_setting(&self, key: &QString, value_json: &QString) {
        let mut m = read_settings();
        if let Ok(v) = serde_json::from_str(&value_json.to_string()) {
            m.insert(key.to_string(), v);
            let path = settings_path();
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(path, serde_json::Value::Object(m).to_string());
        }
    }

    fn backup_save(mut self: Pin<&mut Self>) -> bool {
        if !self.as_mut().save() {
            return false;
        }
        std::fs::copy(save_path(), backup_path()).is_ok()
    }

    fn restore_backup(mut self: Pin<&mut Self>) -> bool {
        let Ok(text) = std::fs::read_to_string(backup_path()) else { return false };
        match LeagueSave::decode(&text) {
            Ok(lg) => {
                {
                    let inner: &mut GameRust = self.as_mut().rust_mut().get_mut();
                    inner.league = Some(lg);
                }
                self.as_mut().save();
                self.bump();
                true
            }
            Err(_) => false,
        }
    }

    /// Deletes the current franchise. A backup copy, if there is one, stays.
    fn abandon_franchise(mut self: Pin<&mut Self>) {
        let _ = std::fs::remove_file(save_path());
        {
            let inner: &mut GameRust = self.as_mut().rust_mut().get_mut();
            inner.league = None;
        }
        self.as_mut().set_has_game(false);
        self.bump();
    }

    fn quit_to_title(mut self: Pin<&mut Self>) {
        self.as_mut().save();
        {
            let inner: &mut GameRust = self.as_mut().rust_mut().get_mut();
            inner.league = None;
        }
        self.as_mut().set_has_game(false);
        self.bump();
    }
}
