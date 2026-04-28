# Phase 3 (Cargo + lib/bin renames) — agent reports

**Phase**: Cargo manifest rename + crate dir move + Cargo.lock refresh — `wdm` → `md`, v0.3.0
**Branch**: `rename/v0.3-wdm-to-md`
**Commit**: `418e75d` — `cargo: rename wdm-codec → md-codec; lib wdm_codec → md_codec; bin wdm → md`
**Date**: 2026-04-27

---

## Implementer report (Sonnet)

**Status**: DONE

**What was implemented**: Phase 3 complete. Renamed crate package, library, CLI binary; moved directories; bumped version 0.2.3→0.3.0; refreshed Cargo.lock with no unrelated dep churn.

**Verification results** (all 6 PASS):
1. cargo metadata: `name=md-codec version=0.3.0 lib_name=md_codec`
2. Bin name + path: `md -> .../crates/md-codec/src/bin/md/main.rs` AND `gen_vectors -> .../crates/md-codec/src/bin/gen_vectors.rs`
3. Directory layout: `md-codec` present, no `wdm-codec`; `md/` and `gen_vectors.rs` in bin, no `wdm/`
4. Cargo.lock: `md-codec v0.3.0` present, 0 occurrences of `wdm-codec`
5. Cargo.lock diff: 42 lines (21+/21−), only local crate package entry, no unrelated dep churn
6. Git rename detection: 41 source files show as `crates/{wdm-codec => md-codec}/...` renames at high similarity

**Files changed**: 43 files (30 ins, 30 del) — Cargo.toml (root + crate), Cargo.lock, 41 renamed source/test/vector files.

**Concerns**: None. `gen_vectors` bin left untouched as specified. `cargo build` is RED (expected — `use wdm_codec::` imports remain in source; Phase 4 handles those).

---

## Spec compliance review (Sonnet)

**Verdict**: ✅ SPEC COMPLIANT — all 12 verifications passed.

Highlights:
- V7 git rename: `crates/{wdm-codec => md-codec}/...` rename entries present; `git mv` correctly used.
- V8 history: `git log --follow` on `crates/md-codec/src/lib.rs` surfaces pre-rename commits `0993dc0` and `fbbe6ec`.
- V9 lock scope: only `-name = "wdm-codec"` / `+name = "md-codec"` deltas; zero unrelated dep churn.
- V10 source untouched: no `.rs`/`.md`/`.json`/`.yml` files modified (only renamed); Phase 4 territory clean.
- V12 build-RED-as-expected: errors are all `unresolved import wdm_codec` style; manifest is parse-valid.

**Concerns flagged**: None.

---

## Code quality review (superpowers:code-reviewer, Sonnet)

**Assessment**: APPROVED

**Strengths**:
- All 41 file renames are clean `git mv` operations. 40 are R100 (byte-identical); only `Cargo.toml` is R081 reflecting exactly the fields that should change.
- Cargo.lock diff is surgically correct: one `wdm-codec 0.2.3` swapped for one `md-codec 0.3.0`, same 16 deps, no ghost entries.
- Filesystem layout exactly right: `src/lib.rs`, `src/bin/md/main.rs`, `src/bin/md/json.rs`, `src/bin/gen_vectors.rs` all present.
- `cargo build` fails ONLY with `unresolved import/module wdm_codec` errors — no manifest-parse errors, no missing-dep errors, no other surprises.
- Version bump `0.2.3 → 0.3.0` is correct pre-1.0 semver step (minor bump for wire-format-breaking change).
- `[patch."https://github.com/apoelstra/rust-miniscript"]` block intact; relative path `../rust-miniscript-fork` still valid (both crate dirs at same depth).

**Issues**:
- Critical: None.
- Important: None.
- Minor: `crates/md-codec/Cargo.toml` has no `keywords`, `categories`, `documentation`, or `homepage` fields. Not a regression (these were absent before Phase 3), but since the package is being renamed and versioned for a potential crates.io publish, a future cleanup pass should add at least `keywords` and `categories`. → **FOLLOWUPS item `cargo-toml-crates-io-metadata-fields`**.

**Verdict**: APPROVED. Textbook filesystem + manifest rename.

---

## Phase 3 closure

✅ Both reviews passed. One minor item filed in FOLLOWUPS (`cargo-toml-crates-io-metadata-fields`). Phase 4 (identifier mass-rename) unblocked. Build is RED as expected — Phase 4 fixes it.
