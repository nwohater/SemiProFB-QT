# Semi-Pro Football (Linux native: Rust + Qt 6/QML)
Port of the Flutter/Dart game at `../Semi-ProFB`. **The Dart code is the source of truth** for logic and tuning (not the SlateVault specs).
Same rules/formulas/tables; outcomes need NOT match the Dart version number-for-number. Check statistical parity (distributions), not exact output.
**Fix clear Dart bugs instead of porting them** (user's explicit rule). Log each one under "Dart bugs fixed" below.
- Cargo workspace: `crates/{sp_core,sp_sim,sp_economy,sp_league,sp_tools}` (all done); `app/` = cxx-qt + QML UI (not started).
- All randomness via seeded `sp_core::Rng`. Tuning values live in data tables (`data/`), not scattered in code.
- `data/*.json|csv` are extracted from Dart by `tools/extract_core_data.py` (re-run if Dart tables change).
- Saves use serde JSON (fresh format; Dart saves are not migrated).
- Dark theme/layout prototype: `../qt-sim-demo` (cxx-qt 0.10 + QML; use `#[auto_cxx_name]` so QML sees camelCase).
- Art/audio: already copied into `assets/` (see below).
- Run: `cargo test --workspace`.
- Parity check vs Dart: `./target/release/calibrate 500 <seed>` here vs `dart run bin/calibrate.dart 500 <seed>` in `../Semi-ProFB/packages/sp_tools`; average 20+ seeds (one seed = 12 teams = noisy).

## Dart bugs fixed (intentional differences)
- Late-game clock (`sp_sim` burn_clock): Dart's `trailing` flag was true for the *leading* team, so leaders burned less clock and trailers more. Fixed. Effect: overtime ~5.2% of games vs Dart ~4.4%.
- Kept fumbles (run and pass): Dart moved the ball by the gain and then again in `after_gain` (yards double counted). Fixed: yards count once.
- Trades (`sp_league::trade`): Dart refused any deal if the partner was already short at a position (e.g. after retirements, before the AI refill), even a pure gift. Fixed: only refuse if the deal makes a shortage worse.
- AI refill / `ensure_minimum` (`sp_league::offseason`): when the free-agent pool had nobody at a position, Dart signed a random player from another position (AI) or left the hole (user), so a club could end up with no kicker. Fixed: a walk-on is generated at the missing position.
- Home "Runway" tile (`app/src/views.rs::runway`): Dart divided cash by a tiny average loss and showed things like "~15270 games". Now anything past 28 games (two seasons) reads "Healthy".

## Port status (update as you go)
- DONE + tested: sp_core (53 tests), sp_sim (+box score, 25 tests), sp_economy (42 tests), sp_tools (calibrate).
- sp_league (Dart: `../Semi-ProFB/packages/sp_league/lib/src`): DONE+tested (31 tests; a full season sims end to end): weather, schedule, standings, playoffs, ai_strategy, absence_log, text (data/*.json), inbox(+ops), league hub, ai_business, council, fan_favorites, awards, hall_of_fame, owner_career, surprise_bills, conduct, insights, gazette.
  Also DONE: draft, offseason, trade, persistence (serde JSON, `LeagueSave`; a loaded league plays on identically). Parity: `target/release/balance 8 8` vs Dart `dart run bin/balance.dart 8 8` (cash/net/age/facility curves match).
  Dart league tests: ALL ported (tests/league_port, draft_trade, inbox_conduct_council, awards_bills_gazette, persistence_port, franchise, season, persistence). Whole workspace: 298 tests, 0 failing (`cargo test --release --workspace`).
  NEXT: the Qt/QML UI crate `app/` (cxx-qt bridge exposing `League` + a game controller; theme/components from `../qt-sim-demo`; art/music from `assets/`). First slice: New Game -> Home -> Roster -> advance week.
- Rust notes: `gen` is a reserved word (edition 2024). serde_json uses `float_roundtrip`. Money is i64 (`sp_economy::Money`). Use BTreeMap where iteration order must be deterministic.
- Assets copied from the Flutter app (2026-10-18): `assets/` (101MB runtime art, music, CSVs), `packaging/` (Linux desktop/icon files), `art-src/` (full-res originals: design/helmets-original, artwork-src/stories). Nothing is wired into the UI yet.

## UI status (app/)
- Built with cxx-qt: `Game` QObject (`app/src/backend.rs`) returns JSON view-models (`app/src/views.rs`); QML parses them, `rev` bump refreshes bindings.
- Working: Player dialog (ratings, contract, jersey art with name/number, release, dress toggle), Free agents (filters, tryouts), Negotiation dialog (2 rounds), Title, New Game, shell (header + rail/tabs, responsive), Home (next game, alerts, last result, standings, phase button incl. offseason), Roster (filter/sort/active toggle), League, Inbox (read-only), Week results dialog. Autosave on advance.
- Also working: Offseason flow (progression, re-sign contracts, free agents, draft with confirm/pass/sim/auto, start next season — verified a full season→offseason→season 2 cycle), Finances (overview, tickets+promos, sponsors, facilities, payroll, loan, council; P&L 'Sheet' tab not ported yet).
- Also working: Game plan (offense/defense/special teams with fit chips), Depth chart (formation packages, reorder, add, reset; no drag/formation field), Trades (partner, asset picks, cash, their answer + the money, make the trade).
TODO: (no trophy animation: the team photo replaces it) depth-chart drag + formation art; install-test the pkg. Also done: Finances "Sheet" tab, content width cap (1560) in AppShell, Arch packaging (`packaging/linux/arch`, `makepkg -f --nodeps` works; needs `!lto` or the cxx-qt static libs fail to link). Done since the first slices: inbox pleas/conduct, League hub tabs (`--dev-ltab`), News/Gazette (page 10), box score dialog (`Store.openBox(i)`, `--dev-dialog box --dev-btab Players`), `StatGrid.qml` fed by `app/src/stats.rs`, champion/retirement/game-over overlays (`StoryOverlay.qml`, `--dev-story champion|retire|gameover`), Settings (page 11; music via `Music.qml` singleton + QtMultimedia, saves/backup, difficulty), Help (page 12, data in `assets/help.json`), tutorial overlay (first run; `--dev-tutorial`), Legacy (page 13, saves a PNG to ~/Downloads). New games set `keep_box_scores: true` (Dart does; without it no box scores exist). Gotchas: `pkill -x` fails (15-char comm limit) so use `pkill -f "release/semi-pro-footbal[l]"`; `data` is a reserved Item property; Loader-loaded rows do not fill the layout width (use inline `component`s instead); closures over `i` in AppShell dev hooks must copy the value.
- Dev: run with `SEMI_PRO_ASSETS=$PWD/assets`; `--dev-new`, `--dev-advance N`, `--dev-page N` (0 Home, 1 Offseason, 2 Roster, 3 Free agents, 4 League, 5 Inbox, 6 Finances, 7 Game plan, 8 Depth chart, 9 Trades), `--dev-trade`, `--dev-offseason`, `--dev-nextseason`, `--dev-otab`, `--dev-ftab`, `--dev-dialog player|neg` flags (use `$XDG_DATA_HOME` to isolate the save). QML errors go to journald (`journalctl --user | grep semi-pro`), not stderr. Qt here has no `font.families`.
- Jersey numbers: `Team::numbers` is filled lazily; `Game::bump` calls `ensure_numbers` for the user team after any change.
- Starting rosters: Dart never passed `nearbyTowns` to `TeamGenerator.generate`, so every starting player was a local (the 55% local split was dead code). Port passes the other division towns, giving ~55% local.
- Slider handles still use the default black Basic style (theme later).
- QML gotchas: an `id: gc` fails qmlcachegen (masks the global gc()); small PrimaryButtons need explicit leftPadding/rightPadding or the label collapses; Basic-style CheckBox/Slider are unthemed -> use Check.qml / Slide.qml.

## License
Code: MIT (`LICENSE`). Art, music, names and branding: all rights reserved (`LICENSE-ASSETS.md`).
