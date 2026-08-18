# Contributing to Organi-Bandit

Thanks for considering a contribution. This is a small, spare-time project. Response times on issues and PRs may vary, but they won't be ignored.

## Getting set up

Full setup steps (Rust toolchain, Tauri CLI, system dependencies) are in the [README](README.md#getting-started). Once `cargo tauri dev` runs the app locally, you're set up correctly.

## Project layout

- `src/`: the frontend. Plain HTML, CSS, and JavaScript, loaded directly by Tauri with no bundler or framework. If you're adding a script, it goes in `src/main.js`; there's no build step to run.
- `src-tauri/`: the Rust backend. Scheduling logic, Tauri commands, and the Excel export.
- `src/locales/strings/en.json` / `fr.json`: frontend UI text. Static HTML text is tagged with a `data-i18n="key"` (or `data-i18n-placeholder="key"` for input placeholders) attribute in `index.html`, matched against a key in these files. Both files need the same set of keys; a key missing from one logs a console warning at runtime rather than failing to build, so it's easy to miss without checking both.
- `src-tauri/locales/excel.yml`: labels used in the generated `.xlsx` file (column headers, day titles), loaded via `rust-i18n`. This is a separate system from the frontend JSON files, with no shared keys. Translating a UI label doesn't translate the spreadsheet, and vice versa.

If you're adding a new piece of user-facing text, check which of these two systems it belongs to (frontend UI vs. generated spreadsheet) and add the key to *all* the language files for that system, not just one.

## Before opening a PR

CI runs `cargo fmt`, `cargo clippy`, and `cargo test` on every PR. Running them locally first saves a round-trip:

```bash
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-features --all-targets -- -D warnings
cargo test --all-features
```

`clippy` is run with warnings denied, so a clean local run means CI will pass on that front too.

## Making changes

- Keep dependencies minimal. This project avoids pulling in a library where a small amount of hand-written code does the job just as well. If you're adding a dependency, it's worth explaining in the PR why the functionality couldn't reasonably be hand-rolled.
- Match the existing style rather than introducing a new pattern for something already done elsewhere in the codebase (e.g. how existing Tauri commands are structured, how existing tests are written).
- Avoid unrelated formatting or refactoring changes mixed into a functional PR. It makes it much harder to review what actually changed.

## Reporting bugs / suggesting features

Open an issue with what you expected to happen, what actually happened, and, for a bug, enough detail to reproduce it (team/schedule configuration, if relevant: the scheduling logic is sensitive to team counts, byes, and time-slot configuration in ways that aren't always obvious from the symptom alone).
