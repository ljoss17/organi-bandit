# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Unreleased changes are tracked as fragments in [`changelog.d/`](changelog.d/README.md) rather than in this file directly, and are folded in here when a release is cut.

<!-- towncrier release notes start -->

## [0.2.0] - 2026-09-07

### Added

- The generated spreadsheet now shows a break row between games scheduled before and after the season's break window. ([#1](https://github.com/ljoss17/organi-bandit/issues/1))
- Specific dates can now be excluded from the season, so the scheduler skips them when placing games. The generated spreadsheet lists them alongside the schedule, with consecutive dates shown as a single range. ([#3](https://github.com/ljoss17/organi-bandit/issues/3))
- A season can now be limited to one game a week even when several game days are selected. The day is then left open: each week's date cell in the generated spreadsheet becomes a dropdown of that week's eligible days, so it can be picked there. ([#5](https://github.com/ljoss17/organi-bandit/issues/5))

### Fixed

- Round Robin schedules no longer place a team in two back-to-back games. Its two games each day are now always separated by the season's break. ([#2](https://github.com/ljoss17/organi-bandit/issues/2))

### Changed

- Game duration is now configured separately from the time between games, so how long a game lasts and how much rest teams get between games can be adjusted independently. ([#4](https://github.com/ljoss17/organi-bandit/issues/4))


## [0.1.0] - 2026-08-28

### Added

- Season schedule generation for flag football leagues, with a configurable start date, start time, break windows, time between games, and number of fields.
- Team list management: browse, edit, and add teams from a JSON file.
- Automatic bye-week handling and referee assignment for each game.
- Export of the generated schedule to an Excel (`.xlsx`) file.
- English and French language support, for both the app interface and the generated spreadsheet.
- Automatic update checking on launch.
