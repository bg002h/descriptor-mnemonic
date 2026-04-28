# Phase 4 (identifier mass-rename) — agent reports

**Phase**: identifier mass-rename across Rust codebase — `wdm` → `md`, v0.3.0
**Branch**: `rename/v0.3-wdm-to-md`
**Commits**: 7 implementer + 2 oversight + 1 controller-cleanup
**Date**: 2026-04-27

---

## Implementer commits

- `08cd1af` — 4a: `use wdm_codec` → `use md_codec` across 20 files; `#[ignore]` on SHA-lock test (PRE-STEP)
- `ca6fcbf` — 4b: `WdmBackup` → `MdBackup`, `WdmKey` → `MdKey` across 8 files
- `fba767c` — 4c: `WDM_REGULAR_CONST` → `MD_REGULAR_CONST`, `WDM_LONG_CONST` → `MD_LONG_CONST` (~24 references)
- `74a28da` — 4d: 21 test fn renames (cli.rs ×18, policy.rs ×2, encoding.rs ×1)
- `cfd45ab` — CONTEXTUAL: doc-comment rewrites (24 files), `cargo_bin("wdm")` → `cargo_bin("md")`
- `6c303c0` — oversight fix #1: 3 files the broad sed missed (tests/upstream_shapes.rs, tests/common/mod.rs, src/bin/md/json.rs)
- `2c9d720` — oversight fix #2 (comprehensive): 9 more files swept (tests/corpus.rs, tests/conformance.rs, src/bytecode/path.rs, tests/ecc.rs, tests/vectors_schema.rs, src/error.rs, src/encoding/bch_decode.rs, tests/cli.rs, tests/bch_correction.rs)
- (controller cleanup) — cli.rs doc-comment fix lines 42, 241-243

**Final state**: 36 files changed, 328 insertions / 326 deletions — almost-perfect symmetry confirms no logic added/removed.

---

## Spec compliance review #1 (Sonnet) — COMPLIANT-WITH-CAVEATS

Adjudicated implementer's 5 deferral claims:
1. **upstream_shapes.rs / common/mod.rs WDM doc comments** → Phase 4 oversight, MUST FIX (every occurrence is `///` or `//!`, not string literals)
2. **bin/md/json.rs `wdm` CLI references in doc comments** → Phase 4 oversight, MUST FIX (3 lines in doc comments; 1 string literal at line 245 correctly Phase 5)
3. **hrp_expand_md_matches_spec test body "wdm" literals** → Phase 5, deferral OK (with `// TODO Phase 5` inline marker)
4. **encoding.rs:93 HRP constant value** → Phase 5, deferral OK (analogous to GENERATOR_FAMILY)
5. **vectors_schema.rs:112 `chunk.starts_with("wdm1")`** → Phase 6, deferral OK (coupled to vector regen)

Resulted in oversight fix dispatch.

---

## Spec compliance re-review (Sonnet) — ✅ SPEC COMPLIANT

After two oversight-fix commits, all 8 verifications passed:
- No comment-context WDM remains (all 16 remaining matches are string literals — Phase 5/6)
- No PascalCase `Wdm[A-Z]` remains
- No `wdm_codec::` paths remain
- No `WDM_*_CONST` remain
- No `fn wdm_*` test functions remain
- All gates: build PASS, 564 passing + 1 ignored, clippy clean, fmt clean
- Phase 5/6 deferrals intact and unchanged
- No HISTORICAL files rewritten

---

## Code quality review (superpowers:code-reviewer, Sonnet) — APPROVED

**Strengths**:
- Line-count balance excellent (328+/326-) — no logic change
- Per-commit scope discipline (bisect-friendly)
- No old identifiers remain in src/
- policy.rs cascaded coherently (struct doc, method docs, doctest use paths all in sync)
- Rustdoc clean (zero warnings/errors from `cargo doc --no-deps`)
- HRP constant value correctly left untouched at line 93
- Clap `name = "wdm"` left untouched at line 33
- No new files created
- `cargo_bin("wdm")` → `cargo_bin("md")` correctly treated as functional rename

**Issues**:
- Critical: None
- **Important #1**: Stale `"wdm1"` string literals in `tests/cli.rs` (lines 42, 50, 112-113, 215-217, 252-253, 415) without `// TODO Phase 5` markers. Risk that Phase 5 misses them. → Resolved via FOLLOWUPS entry `phase-5-cli-wdm1-assertion-sweep` + controller doc-comment cleanup on lines 42, 241-243.
- **Important #2**: Test name `hrp_expand_md_matches_spec` misleading because body still tests `"wdm"`. Recommended `hrp_expand_preimage_wdm_pending_phase5`. → Accepted as-is (Phase 5 will fix when body flips); test has TODO Phase 5 inline marker at line 979.
- **Minor**: Workflow doc should note that broad sed must enumerate `src/**`, `tests/**`, `src/bin/**` separately — `tests/` tree is a common blind spot. → FOLLOWUPS entry `rename-workflow-broad-sed-enumeration-lesson`.
- **Minor**: SHA-lock `#[ignore]` placement verified clean at `tests/vectors_schema.rs:220` with `// TODO Phase 6: re-enable after vector regen` comment correctly above `#[test]`.

**Assessment**: APPROVED. Phase 4 is a clean, complete identifier rename. The Important issues are Phase 5 setup risk rather than current defects.

---

## Phase 4 closure

✅ Both reviews passed (after 2 oversight fix passes). Two FOLLOWUPS items filed:
- `phase-5-cli-wdm1-assertion-sweep` (Phase 5 prep)
- `rename-workflow-broad-sed-enumeration-lesson` (process improvement)

Phase 5 (string literal sweep) unblocked. Build green; 564 passing + 1 ignored; clippy + fmt clean.
