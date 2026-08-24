---
name: Release
about: Checklist for cutting a new release
---

<!-- Copy the "Copy into release PR" section below into the release PR description. -->

## Copy into release PR

- [ ] Decide the version bump (patch/minor/major per SemVer)
- [ ] Update `version` in `src-tauri/Cargo.toml`
- [ ] Update `version` in `src-tauri/tauri.conf.json` (must match)
- [ ] Run `towncrier build --version X.Y.Z` from the repo root
- [ ] Review the resulting `CHANGELOG.md` diff

## After the release PR is merged

- [ ] Tag the release: `git tag vX.Y.Z && git push origin vX.Y.Z`
- [ ] Wait for the Release CI job to finish and create the draft GitHub Release
- [ ] Check all expected assets are attached (Windows, macOS x2, Linux installers, `latest.json`)
- [ ] Copy the new version's section from `CHANGELOG.md` into the draft release's description, replacing the placeholder text (Releases page → Edit on the draft, or `gh release edit vX.Y.Z --notes-file path/to/notes.md`)
- [ ] Publish the release (Publish release button on the draft's edit page, or `gh release edit vX.Y.Z --draft=false`)
