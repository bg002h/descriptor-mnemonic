# Phase 10 Implementer Report — CHANGELOG + MIGRATION + Release Notes

**Phase**: 10 — CHANGELOG + MIGRATION + release notes draft
**Branch**: `feature/v0.5-multi-leaf-taptree`
**Commit**: `eca7d3c` ("docs(v0.5 phase 10): CHANGELOG + MIGRATION for v0.5.0 multi-leaf TapTree")
**Status**: DONE

---

## Tasks completed

### Task 10.1 — CHANGELOG.md

Inserted new `[0.5.0] — 2026-04-28` entry immediately after the `# Changelog` header,
before the `[0.4.1]` entry. Added reference link at the bottom of the link table.

**SHA resolution**: Phase 6's concern about `<NEW_SHA_FROM_PHASE_6>` placeholders was
resolved by reading actual on-disk SHAs via `sha256sum`:

- `v0.1.json`: `bb2bcc78835d519c7f7595994c6113ef62c379cee99e4d62288772834d4f1c26` — UNCHANGED
  from v0.4.1 (v0.1.json was not regenerated in Phase 6; no multi-leaf fixtures use schema 1)
- `v0.2.json`: `7d801228ab3529f2df786c50ff269142fae2d8e896a7766fb8eb9fcf080e328d` — changed
  from v0.4.1's `caddad36...` (Phase 6 regeneration added multi-leaf TapTree fixtures to schema 2)

The vectors_schema.rs `V0_2_SHA256` constant confirms `7d801228...` as the locked value.
No `<NEW_SHA_FROM_PHASE_6>` placeholder remains in the CHANGELOG.

**Note from final reviewer (M5) applied**: test count stated as "634 passing + 0 ignored
(was 609 at v0.4.1; +25 net)" per actual phase 9 count, not the stale "≥638" from the
plan text.

**Note on family token**: The CHANGELOG entry documents that the family generator token
remains `"md-codec 0.4"` at this commit and explicitly states Phase 11 will re-bump.
This is accurate per Phase 6 implementer's concern.

**All spec §6 CHANGELOG items covered**:
- tr(KEY, TREE) admittance
- Tag::TapTree (0x08) active
- BIP 341 depth-128 enforcement
- DecodeReport.tap_leaves + TapLeafReport
- Error::TapLeafSubsetViolation gains leaf_index + #[non_exhaustive]
- validate_tap_leaf_subset signature change
- Dispatcher message update
- v0.4 single-leaf-nonzero-depth rejection removed
- Wire format compatibility statement

### Task 10.2 — MIGRATION.md

New `## v0.4.x → v0.5.0` section inserted at the top (before `## v0.3.x → v0.4.0`).
Sections: What changed, What didn't change, How to upgrade, destructure-pattern update,
validate_tap_leaf_subset call-site update, New encoder behavior, New decoder behavior,
Test vector SHAs. Format mirrors the existing `## v0.3.x → v0.4.0` section style.

### Task 10.3 — Release notes draft

Written to `/tmp/v0.5.0-release-notes.md`. Covers:
- Pre-release status banner
- Summary (multi-leaf TapTree admittance, wire-format-additive)
- Added/Changed/Removed sections mirroring CHANGELOG
- Wire format framing table
- Quality gates (634 tests, CI status, MSRV 1.85)
- Family-stable design note
- FOLLOWUPS state (2 non-blocking open items + apoelstra#1)
- Dependency notes
- Links

### Task 10.4 — Commit

Committed `CHANGELOG.md` + `MIGRATION.md` as `eca7d3c`. The release notes file is at
`/tmp/v0.5.0-release-notes.md` (not committed — per plan, it's a `/tmp/` draft for
Phase 11 to paste into the GitHub release form).

---

## Self-review

- [ ] No `<NEW_SHA_FROM_PHASE_6>` placeholder appears in CHANGELOG — verified by substitution
- [ ] Existing CHANGELOG format preserved — `[0.5.0]` entry uses same headings
  (`### Added`, `### Changed`, `### Removed`, `### Wire format`, `### Notes`) as prior entries
- [ ] Earlier CHANGELOG entries undisturbed — `[0.4.1]` entry and all preceding entries unchanged
- [ ] MIGRATION.md prior sections undisturbed — `## v0.3.x → v0.4.0` and later sections intact
- [ ] v0.1.json SHA cross-checked: matches `bb2bcc78...` which is what v0.4.1 CHANGELOG shows
  as "UNCHANGED" (first v0.4.x patch validated family-stable promise)
- [ ] v0.2.json SHA cross-checked: `7d801228...` matches the `V0_2_SHA256` constant in
  `crates/md-codec/tests/vectors_schema.rs:252`
- [ ] Test count `634` matches Phase 9 final reviewer report (M5)

---

## Deferred items / concerns

None. All plan tasks completed. The one documentation concern (family generator token)
is explicitly noted inline in both CHANGELOG and MIGRATION.
