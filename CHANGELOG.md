# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Unreleased changes are tracked as fragments in [`changelog.d/`](changelog.d/README.md) rather than in this file directly, and are folded in here when a release is cut.

<!-- towncrier release notes start -->

## [0.1.0] - 2026-08-24

### Added

- Season schedule generation for flag football leagues, with a configurable start date, start time, break windows, time between games, and number of fields.
- Team list management: browse, edit, and add teams from a JSON file.
- Automatic bye-week handling and referee assignment for each game.
- Export of the generated schedule to an Excel (`.xlsx`) file.
- English and French language support, for both the app interface and the generated spreadsheet.
- Automatic update checking on launch.
