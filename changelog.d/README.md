# Changelog fragments

Every user-facing PR (a fix, a feature, a behavior change) should add one file here describing it, instead of editing `CHANGELOG.md` directly. [towncrier](https://towncrier.readthedocs.io/) collects these into `CHANGELOG.md` when a release is cut.

## Adding a fragment

Create a file named `<issue-number>.<type>.md`, for example `42.feature.md` for issue #42. The content is the entry text as it should appear in the changelog:

```
Multi-language support for the UI and generated spreadsheet
```

`<type>` must be one of:

- `feature`: a new capability, renders under Added
- `bugfix`: a bug fix, renders under Fixed
- `change`: a change to existing behavior, renders under Changed

## Cutting a release

Run `towncrier build --version X.Y.Z` from the repository root. It reads every fragment in this directory, groups them into `CHANGELOG.md` under a new version heading, and deletes the fragment files it used. Review the resulting diff, then commit and tag the release.
