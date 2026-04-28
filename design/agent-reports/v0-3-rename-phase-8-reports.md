# Phase 8 (documentation sweep) — agent reports

**Phase**: README + MIGRATION + CHANGELOG + FOLLOWUPS — `wdm` → `md` rename, v0.3.0
**Branch**: `rename/v0.3-wdm-to-md`
**Commits**: `ae66c2a` (READMEs), `84684ff` (MIGRATION), `63be1e8` (CHANGELOG + slip-0173 followup)
**Date**: 2026-04-27

---

## Implementer report (Sonnet)

**Status**: DONE

**Files edited**:
- `README.md` — title (Mnemonic Descriptor (MD)), "Renamed from" admonition, tree-art, all crate/BIP/CLI references, prose, test count, status section
- `crates/md-codec/README.md` — heading (md-codec), format name, BIP link, Cargo dep snippet 0.1→0.3, docs.rs link, CLI table, gen_vectors paths, status section
- `bip/README.md` — format name, BIP filename, all wdm-codec→md-codec paths, test count
- `MIGRATION.md` — header updated; new `## v0.2.x → v0.3.0` section with all 6 points; historical v0.1.x→v0.2.0 retained verbatim
- `CHANGELOG.md` — new `## [0.3.0]` entry at top; file header updated wdm-codec→md-codec; zero deletions from historical entries
- `design/FOLLOWUPS.md` — added `slip-0173-register-md-hrp` entry (post-release Phase 11)

**Judgment call (acknowledged)**: MIGRATION.md historical v0.1.x→v0.2.0 section retains references to `crates/wdm-codec/` paths and `wdm1` in code examples. Correctly preserved as historical record per spec.

---

## Combined spec + quality review (Sonnet) — APPROVED

All verifications passed:
- README title `# Mnemonic Descriptor (MD)`; admonition cites both old/new names + links
- Crate README + bip/README clean (zero `wdm`/`WDM`)
- MIGRATION §1-§6 all present; SHAs match Phase 6 source-of-truth character-for-character
- CHANGELOG v0.3.0 at top above v0.2.3; zero deletions from historical entries; covers all 6 breaking categories; family-stable promise reset noted; HRP collision vet documented; `[patch]` block status carried forward
- Cross-link `MIGRATION.md#v02x--v030` resolves correctly (GitHub markdown anchor convention)
- `slip-0173-register-md-hrp` FOLLOWUP entry well-formed
- Diff scope: only 6 doc files touched; no code touched
- Test count: 565 passing + 0 ignored (verified by summing 16 suite outputs)
- Build/clippy/fmt: clean

**Quality observations**:
- CHANGELOG entry well-structured, follows v0.2.x conventions (same `### Breaking` / `### Notes` headings, inline rationale, backtick-fenced SHAs, cross-links to MIGRATION.md)
- "HRP collision vet" + "Workspace `[patch]` block" notes preempt reviewer questions
- MIGRATION §5 correctly references the OLD family-stable SHA `b403073b…` so conformance-suite owners know what they were pinning and what to update to
- Implementer's judgment call on MIGRATION historical preservation is correct

**Concerns flagged**: None.

---

## Phase 8 closure

✅ APPROVED with 0 issues. Documentation aligned with v0.3.0 rename:
- All user-facing READMEs renamed
- New MIGRATION section guides v0.2.x → v0.3.0 upgraders
- CHANGELOG documents all breaking changes
- FOLLOWUPS captures defensive SLIP-0173 registration for post-release

Phase 9 (auto-memory updates) unblocked.

---

## Open release blockers (carried forward to release)

- **`bch-known-vector-repin-with-md-hrp` (v0.3-BLOCKER)** — Phase 5 code-quality reviewer elevated this. Round-trip-only tests can't catch polymod-constant drift if both create + verify drift together. **Controller must decide before tag**: (a) compute Python-reference values + hardcode (proper fix), (b) accept rationale that Phase 6 vector SHAs provide equivalent independent-verification protection at the SHA-pin level + downgrade to v0.3-nice-to-have, (c) ship with documented risk.
