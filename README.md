# Organi-Bandit

[![Tests](https://github.com/ljoss17/organi-bandit/actions/workflows/tests.yml/badge.svg)](https://github.com/ljoss17/organi-bandit/actions/workflows/tests.yml)
[![Latest release](https://img.shields.io/github/v/release/ljoss17/organi-bandit)](https://github.com/ljoss17/organi-bandit/releases)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)

A desktop application that generates a flag football season schedule as a spreadsheet. Configure your teams and a few scheduling parameters, and it produces a `.xlsx` file listing every game of the season — a round-robin group stage followed by a single-elimination playoff.

## What it does

- **Team management** — browse to a `teams.json` file, and add, rename, re-seed, or remove teams directly in the app. Saving writes the changes back to the same file.
- **Season configuration**, set through the desktop UI:
  - Start date and which weekdays count as game days
  - Number of fields (how many games can run at the same time)
  - Start time, time between games, and a start/end break window (e.g. a lunch break with no games scheduled)
- **Round-robin group stage** — every team plays every other team twice (once as home, once as away), with a bye week for one team per round when the team count is odd, and a referee automatically assigned to each game from the teams not already playing that time slot.
- **Single-elimination playoffs** — the top 8 teams from the group stage advance to a knockout bracket.
- **Excel export** — the full schedule (group stage and playoffs) is written to a `.xlsx` file in a folder you choose.
- **English and French UI**, switchable at runtime — the exported spreadsheet's own labels (column headers, day titles) follow whichever language is selected.

## Tech stack

- **Rust** — application and scheduling logic
- **Tauri 2** — desktop shell, wrapping a plain HTML/CSS/JS frontend (no bundler or framework)
- **chrono** / **chrono-tz** — date, weekday, and timezone handling
- **rust_xlsxwriter** — spreadsheet generation
- **rust-i18n** — translated labels in the generated spreadsheet
- **tauri-plugin-dialog** — native file and folder pickers

## Status

Round-robin and single-elimination are currently the only supported tournament formats, and there's no CSV export — only `.xlsx`. Contributions adding either are welcome.

## Getting started

Requires a recent [Rust toolchain](https://www.rust-lang.org/tools/install) and the [Tauri CLI](https://tauri.app/):

```bash
cargo install tauri-cli --version "^2.0.0" --locked
```

Tauri's Linux backend depends on WebKitGTK. On Debian/Ubuntu, install the system packages it needs before building:

```bash
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

(On other platforms, see [Tauri's own prerequisites guide](https://v2.tauri.app/start/prerequisites/).)

Then, from the repository root:

```bash
git clone https://github.com/ljoss17/organi-bandit.git
cd organi-bandit
cargo tauri dev
```

## Input files

Teams are read from a JSON file — one object per team, with an optional seed used to arrange the single-elimination bracket:

```json
[
  { "name": "Riviera Saints", "seed": null },
  { "name": "Fribourg Cardinals", "seed": null },
  { "name": "Morges Bandits", "seed": null }
]
```

A sample file is included at `src-tauri/resources/teams.json`. The app can also browse to and edit any other file in this format directly from its Teams panel.

## Output

A `.xlsx` file listing every game of the season — group stage and playoffs — with date, time, field, matchup, and assigned referee for each game, saved to the folder you select before generating.

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache License, Version 2.0](LICENSE-APACHE) at your option.

The Morges Bandits logo (the application icon and in-app branding) is the property of the Morges Bandits club and is used with their permission. It is not covered by the license above — please don't reuse it outside this project without the club's consent.
