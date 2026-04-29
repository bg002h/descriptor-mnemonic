# v0.6 Phase 5 Review Report

**Status:** DONE_WITH_CONCERNS
**Commit:** `fe0e4a0` (corpus expansion + negative-vector audit)
**File(s):**
- `crates/md-codec/src/vectors.rs`
- `crates/md-codec/src/bytecode/decode.rs`
- `crates/md-codec/src/bytecode/encode.rs`
- `design/FOLLOWUPS.md`
**Role:** reviewer (corpus + audit)

## Summary

Phase 5 successfully landed the 17 new positive corpus fixtures, applied the negative-vector audit table per CRIT-3 + IMP-8, filed the deferred or_c entry to FOLLOWUPS, and updated the doc-staleness sites in the bytecode encode/decode module rustdoc. The audit-table application is wire-correct — runtime-emitted descriptions and provenance reference `TagInvalidContext` and reflect v0.6 behavior.

One stale-doc concern surfaced (Important, not Critical): the **rustdoc and one inline reasoning comment** on `build_negative_taptree_inner_off_subset` were not updated to match v0.6 catch-all behavior, even though the *output strings* it produces were updated. Doc-only gap with no functional impact.

## All checks pass except one

PASS: 17 new fixtures present; (id, description, policy_str) shape; n_tap_leaf_subset DELETED; n_taptree_inner_* family expected_error_variant updated to "InvalidBytecode"; n_taptree_inner_* description references TagInvalidContext; n_sh_bare/n_top_bare removed; decode.rs/encode.rs doc-staleness fixes correct; existing v0.5 fixtures preserved; or_c FOLLOWUPS entry filed.

FAIL (Important): `build_negative_taptree_inner_off_subset` rustdoc + inline comment at vectors.rs:1762-1792 still describe v0.5 behavior (validate_tap_leaf_subset, SubsetViolation). Runtime contract correct but developer-facing docs stale.

## Findings

### Important: stale rustdoc on `build_negative_taptree_inner_off_subset`

**Location:** `crates/md-codec/src/vectors.rs:1762-1765` (rustdoc) and 1786-1792 (inline comment).

**What's stale:** Both describe v0.5 behavior — "validate_tap_leaf_subset fires" / "We assert at the variant family level (SubsetViolation for recognised-but-off-subset tags)". In v0.6, validate_tap_leaf_subset is no longer called; decoder catch-all produces InvalidBytecode { kind: TagInvalidContext }.

**Severity:** Important per the dispatch brief's check "build_negative_taptree_inner_off_subset rustdoc updated to reflect v0.6 catch-all behaviour"; that line wasn't satisfied. Runtime contract is correct, but a future maintainer reading the rustdoc would be misled.

**Suggested fix:** Update both blocks to describe the v0.6 catch-all + cite the historical v0.5 transition.

## Notes

- 17 fixtures all use template-form policies (`@N/**` placeholders) consistent with the corpus.
- `tr_complex_recovery_path_md_v0_6` and `tr_pkh_in_tap_leaf_md_v0_6` correctly include `pkh()` since rust-miniscript desugars at parse.
- The `cfg!(debug_assertions)` sanity check at line 1795-1799 remains valid under v0.6 (catch-all still produces InvalidBytecode, just different kind).

End of review.
