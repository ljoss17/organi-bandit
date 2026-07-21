# Organi-Bandit

A desktop application that generates a flag football season schedule as a spreadsheet. Given a list of teams and a few scheduling parameters, it produces a `.xlsx` file listing every game of the season — round-robin group stage followed by a single-elimination playoff.

## What it does

- Reads the list of teams from a `.txt` file (one team name per line)
- Lets you configure the season through a desktop UI:
  - Start and end date
  - Number of bye weeks per team
  - Game days (e.g. Saturday, Sunday)
  - Maximum number of concurrent games (number of fields)
  - Number of sequential games per day (e.g. 2 for morning/afternoon)
- Generates a full round-robin group stage, followed by a single-elimination playoff bracket
- Exports the complete schedule to a `.xlsx` spreadsheet with a single click

The UI is in French.

## Tech stack

- **Rust** — application logic
- **Tauri** — desktop UI shell
- **chrono** — date and weekday handling
- **rust_xlsxwriter** — spreadsheet generation

## Status

This README describes the initial working version (MVP) only. Features such as editable team/restricted-day lists, additional tournament formats, multi-language support, and CSV export are planned but not yet part of this version.

## Getting started

```bash
cargo install create-tauri-app --locked
cargo create-tauri-app
cd <project-name>
cargo install tauri-cli --version "^2.0.0" --locked
cargo tauri dev
```

## Input files

- **Teams** — a `.txt` file with one team name per line.

## Output

- A `.xlsx` file listing all games of the season (group stage and playoffs) with date, time, field, and matchup for each game.