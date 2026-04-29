# v0.6 plan review (round 2)

**Status:** DONE_WITH_CONCERNS
**Commit:** `90d196b` — review of plan at `design/IMPLEMENTATION_PLAN_v0_6_strip_layer_3.md`
**File(s):**
- `design/IMPLEMENTATION_PLAN_v0_6_strip_layer_3.md`
- `design/agent-reports/v0-6-plan-review-1.md`
- `design/agent-reports/README.md`
- `design/FOLLOWUPS.md`
- `crates/md-codec/src/error.rs`
- `crates/md-codec/tests/vectors/v0.2.json`
**Role:** reviewer (plan, round 2)

## Summary

Round-1 fold-in is substantially complete and correctly addresses 8 of 8 findings plus the §6.3 spec-coverage concern. Audit table at Step 5.2 was independently cross-checked against `tests/vectors/v0.2.json` and is byte-accurate. One fold-in defect was introduced by the revision: two stale references to "Step 3.0" inside Phase 3's Task 3.2 that should now point at Task 2.0 (Phase 2) where the variant addition was actually pre-pinned. The defect is cosmetic and self-correcting at execution time (the implementer will notice the `BytecodeErrorKind::TagInvalidContext` variant already exists from Phase 2), but it shows that the IMP-7 pre-pin moved the work cross-phase without updating Phase 3's prose. Plan is executable; status `DONE_WITH_CONCERNS`.

## Round-1 fold-in audit

### CRIT-1 (SHA-pin window) — ADDRESSED
The failing-test whitelist appears in BOTH phase reviewer briefs (Phase 5 + Phase 6) and Step 5.2.3 commit body.

### CRIT-2 (sed scope) — ADDRESSED
Step 4.1.5 exists with the design/-markdown scrub policy. Past-tense vs forward-pointing distinction is explicit. `design/agent-reports/` correctly skipped as durable historical record.

### CRIT-3 (negative-vector audit) — ADDRESSED, INDEPENDENTLY VERIFIED

**In-tree v0.2.json verification:** Read `crates/md-codec/tests/vectors/v0.2.json` and confirmed all 8 vectors exist with the claimed `expected_error_variant`:
- `n_tap_leaf_subset` → `TapLeafSubsetViolation` ✓
- `n_taptree_inner_wpkh` → `TapLeafSubsetViolation` ✓
- `n_taptree_inner_sh` → `TapLeafSubsetViolation` ✓
- `n_taptree_inner_wsh` → `TapLeafSubsetViolation` ✓
- `n_taptree_inner_tr` → `TapLeafSubsetViolation` ✓
- `n_taptree_inner_pkh` → `TapLeafSubsetViolation` ✓
- `n_sh_bare` → `PolicyScopeViolation` ✓
- `n_top_bare` → `PolicyScopeViolation` ✓

Audit table matches reality byte-for-byte.

### IMP-4 (conformance.rs rename) — ADDRESSED
Step 4.2.3 present including the verification grep.

### IMP-5 (Phase 2 reviewer brief) — ADDRESSED
All four explicit checks present (option (a) tap-illegal arm comments; v0.6 byte-form cross-check; hash internal-byte-order verification; `validate_tap_leaf_subset` body unchanged).

### IMP-6 (rolled reviews) — ADDRESSED
Phase 5 covers Phase 4; Phase 6 covers Phase 7.

### IMP-7 (catch-all error kind) — ADDRESSED with one fold-in defect
Task 2.0 introduces `BytecodeErrorKind::TagInvalidContext`. Variant placement adjacent to `UnknownTag` per recommendation. Variant referenced in Phase 3 catch-all + Step 5.2 audit table.

**Defect (NEW-1)**: Phase 3 lines 959 and 971 reference "Step 3.0" but the variant addition is in **Task 2.0** (Phase 2), not Phase 3. Cosmetic; implementer will notice the variant exists and proceed.

### IMP-8 (Tag::Bare ripple) — ADDRESSED
`n_sh_bare` and `n_top_bare` in audit table with KEEP-with-input-rebase decisions.

## Spec coverage fold-in

### §6.3 byte-order test — ADDRESSED
Step 5.1.6 has defensive byte-pin assertion. References encode.rs:316-319 + spec §6.3.

### TDD framing — ADDRESSED accurately
Architecture summary correctly characterizes per-phase test discipline (Phase 1 TDD; Phases 2/3 regen-and-verify; Phase 4 mechanical; Phase 5 corpus-as-test-artifact; Phase 6+ doc-only).

## New defects introduced by the revision

### NEW-1: Phase 3 prose has stale "Step 3.0" references after IMP-7 pre-pin moved variant addition to Phase 2

**Locations:** Plan lines 959 and 971.

**What it says:**
- "Use the new BytecodeErrorKind::TagInvalidContext variant introduced for this purpose (see Step 3.0 below)."
- "The new BytecodeErrorKind::TagInvalidContext { tag: u8, context: &'static str } variant is added in Step 3.0 (below) before adding the new tap-leaf arms."

**What's actually true:** The variant is added in **Task 2.0** (Phase 2), not Phase 3. There is no Step 3.0 anywhere in the plan.

**Severity:** Cosmetic. Search-and-replace `Step 3.0 below` and `Step 3.0 (below)` with `Task 2.0 of Phase 2`.

Confidence: 95.

## Other defect checks

- **"Step 5.2.4" stale reference**: CLEAN (none).
- **Old Task 2.7 brief expansion**: CLEAN.
- **FOLLOWUPS entry placement**: CLEAN.

## Phase/step numbering coherence

Walked every Phase. Clean numbering throughout. Task 2.0's commit step makes the new task self-contained as required. Phase 3 has no Step 3.0 / Task 3.0 — see NEW-1.

## Cross-phase ordering check

Task 2.0 (Phase 2) introduces variant → Phase 3 catch-all references → Phase 5b audit table references. Dependency chain honored.

## Findings

**Critical:** None.
**Important:** NEW-1 (cosmetic prose).
**Nits:** None new.

## Concerns / deviations summary

Fold-in is high-fidelity. Every round-1 finding traces to a concrete plan revision; audit table is byte-accurate against in-tree state; architecture-summary TDD framing is honest. NEW-1 is the only revision-introduced defect and is cosmetic. Plan can execute as-is.

End of review.
