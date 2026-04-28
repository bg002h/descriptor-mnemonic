# Phases 8+9 Implementer Report — v0.4.0 Cargo.lock + Docs

**Date**: 2026-04-27
**Branch**: feature/v0.4-bip388-modern-surface
**Phases**: 8 (Cargo.lock refresh) + 9 (CHANGELOG + MIGRATION)

## Status: COMPLETE

Both phases executed cleanly with no issues.

---

## Phase 8 — Cargo.lock refresh

**Command**: `RUSTUP_TOOLCHAIN=stable cargo update --workspace`

**Cargo.lock diff scope**: Single change only — local `md-codec` package
version `0.3.0` → `0.4.0`. No unrelated dep churn. Three deps were noted as
"behind latest" (pass `--verbose` to see) but left at their pinned Rust 1.85
compatible versions as expected.

**Build**: `cargo build --workspace --all-targets` — Finished in 0.05s (cached).

**Test count**: 609 passing, 0 failed, 0 ignored (matches expected).

**Commit**: `c47c628` — "cargo: refresh Cargo.lock for v0.4.0 release"

---

## Phase 9 — CHANGELOG + MIGRATION

### Task 9.1 — CHANGELOG.md

- New `[0.4.0]` entry inserted at top (after file header), before `[0.3.0]`.
- Covers: added descriptor types, wire format additive expansion, test vector
  SHAs (v0.1: `bb2bcc78...`, v0.2: `caddad36...`), family token reset, CLI
  additions, FOLLOWUPS closures + new deferred items.
- Link reference `[0.4.0]` added at bottom of references block.
- Historical entries (v0.3.0, v0.2.x, v0.1.x) untouched — verified by diff.

### Task 9.3 — MIGRATION.md

- New `v0.3.x → v0.4.0` section (6 numbered points) inserted ABOVE the
  existing `v0.2.x → v0.3.0` section, separated by `---`.
- Covers: Cargo dep bump, CLI surface, vector SHA migration (both files),
  no public API changes, `--path bip48-nested`, restriction matrix normative.
- Historical sections untouched.

**Commit**: `624417a` — "docs: CHANGELOG + MIGRATION for v0.4.0 — BIP 388 modern surface"

**Push**: Succeeded — `313a790..624417a` → `origin/feature/v0.4-bip388-modern-surface`

---

## Files changed

- `Cargo.lock` — version string bump only (Phase 8)
- `CHANGELOG.md` — 52-line `[0.4.0]` entry + link ref (Phase 9)
- `MIGRATION.md` — 30-line `v0.3.x → v0.4.0` section (Phase 9)

## Concerns

None. All changes matched plan exactly. Test count held at 609.
