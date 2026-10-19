# Semi-Pro Football

A small-town football management sim for Linux. You own a semi-pro club: the payroll, the roster, the stadium, the sponsors and the problems. Players have day jobs, the town has opinions, and every week the *Gazette* tells the story.

This is the native Linux version, written in **Rust** with a **Qt 6 / QML** interface (via [cxx-qt](https://github.com/KDAB/cxx-qt)). It is a port of the Flutter/Dart mobile game.

## Features

- Pick a state (California, Texas, Florida or Pennsylvania) and a town from a dealt league of 12 clubs in 3 divisions, with a map of where everyone is
- Full seasons: weekly games, playoffs, offseason with progression, re-signing, free agency and a draft
- Roster, depth chart (formation packages) and game plan, with optional auto roster and auto depth chart
- Money that matters: tickets and promos, sponsors, facilities, payroll, loans and a town council
- Negotiated contracts, trades with the AI clubs, and the odd off-field problem
- Box scores, league leaders, team stats, awards, Hall of Fame and your own owner Legacy page (shareable as a picture)
- A weekly *Gazette*, an inbox full of pleas, bills and rumors, and a built-in tutorial and help
- Autosaves on every advance

## Requirements

- Rust (stable, edition 2024) and `cargo`
- Qt 6 with Declarative (QML), Multimedia (FFmpeg backend) and the Qt tools (`qmake6`/`qt6-tools`)
- `imagemagick` only when building the Arch package

On Arch: `sudo pacman -S rust qt6-base qt6-declarative qt6-multimedia qt6-multimedia-ffmpeg qt6-tools`

## Build and run

```sh
cargo build --release -p semi-pro-football
SEMI_PRO_ASSETS=$PWD/assets ./target/release/semi-pro-football
```

The game looks for its art and music in `$SEMI_PRO_ASSETS`, then `./assets`, then next to the repo when run from `target/`, then the installed copy in `/usr/share/semi-pro-football/assets`.

Saves and settings live in `~/.local/share/semi-pro-football/`. Set `XDG_DATA_HOME` to use a separate save while testing.

### Arch package

```sh
cd packaging/linux/arch
makepkg -si
```

## Tests

```sh
cargo test --release --workspace
```

Two tools check that the simulation still behaves like the original:

```sh
./target/release/calibrate 500 <seed>   # game statistics, compare with the Dart calibrate tool
./target/release/balance 8 8            # multi-season cash, age and facility curves
```

One seed is only 12 teams and is noisy, so average 20 or more seeds when comparing.

## Layout

| Path | What it is |
| --- | --- |
| `crates/sp_core` | players, towns, ratings, rosters, seeded RNG |
| `crates/sp_sim` | the game simulation and box scores |
| `crates/sp_economy` | contracts, finances, sponsors, facilities |
| `crates/sp_league` | seasons, schedule, draft, offseason, trades, inbox, Gazette, saves |
| `crates/sp_tools` | `calibrate` and `balance` command line tools |
| `app/` | the cxx-qt bridge (`src/`) and the QML interface (`qml/`) |
| `data/` | tuning tables and text, extracted from the Dart game by `tools/` |
| `assets/` | art, music and help text used at runtime |
| `art-src/` | full-resolution art originals |
| `packaging/` | desktop file, icons and the Arch `PKGBUILD` |

All randomness goes through a seeded RNG, so a franchise plays out the same way from the same choices. Tuning values live in `data/`, not in the code.

## Differences from the mobile game

The rules, formulas and tables are the same, but a few clear bugs in the Dart version were fixed rather than ported (for example, a late-game clock that favored the wrong team and double-counted yards on fumbles). Outcomes match statistically, not number for number. The full list is in `CLAUDE.md`.

## License

The code is released under the [MIT License](LICENSE). The art, music, names and branding are **all rights reserved**; see [LICENSE-ASSETS.md](LICENSE-ASSETS.md).
