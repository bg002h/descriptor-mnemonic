# v0.6 Phase 4 Review — Error variant rename

**Status:** DONE
**Commit:** `41f6c00` (feature/v0.6-strip-layer-3)
**File(s):**
- `crates/md-codec/src/error.rs`
- `crates/md-codec/src/bytecode/encode.rs`
- `crates/md-codec/src/bytecode/decode.rs`
- `crates/md-codec/src/vectors.rs`
- `crates/md-codec/tests/error_coverage.rs`
- `crates/md-codec/tests/conformance.rs`
- `crates/md-codec/tests/taproot.rs`
- `design/SPEC_v0_6_strip_layer_3.md`
- `design/MD_SCOPE_DECISION_2026-04-28.md`
**Role:** reviewer (mechanical-rename audit)

## Summary

Phase 4 (commit `41f6c00`) is mechanically clean and consistent. Rename `Error::TapLeafSubsetViolation` → `Error::SubsetViolation` uniformly applied across `error.rs`, encoder, decoder catch-all replacement, vectors generator, error_coverage mirror, and conformance test. Forward-pointing reference scrub of `design/` markdown correctly applied; historical references retained. `design/agent-reports/` left unchanged. error_coverage gate green. No defects of confidence ≥ 90.

## All checks pass

1. No surviving `TapLeafSubsetViolation` token in `crates/md-codec/src/` or `crates/md-codec/tests/*.rs`.
2. `tests/error_coverage.rs` mirror has `SubsetViolation` not `TapLeafSubsetViolation`.
3. `tests/conformance.rs` test renamed `rejects_tap_leaf_subset_violation` → `rejects_subset_violation`.
4. error_coverage gate passes.
5. `design/` markdown forward-pointing-reference scrub policy applied correctly. Spot-checked `SPEC_v0_6_strip_layer_3.md` and `MD_SCOPE_DECISION_2026-04-28.md` — past-tense / historical references KEEP `TapLeafSubsetViolation`; forward-pointing references READ `SubsetViolation`.
6. `design/agent-reports/` unchanged.
7. Compile clean.

## Pre-existing items noted (NOT Phase 4 defects — Phase 3 docstring leftovers)

These are decoder rustdoc references to v0.6 catch-all behaviour that were updated mechanically by Phase 4's sed but still describe pre-strip semantics:

- `crates/md-codec/src/bytecode/decode.rs:596-605` — `decode_tap_terminal` rustdoc still says "Out-of-subset tags surface `Error::SubsetViolation` immediately" but the v0.6 catch-all emits `BytecodeErrorKind::TagInvalidContext`, not `SubsetViolation`. (Already fixed by controller as part of Phase 5 cleanup.)
- `crates/md-codec/src/vectors.rs:1825` — `build_negative_taptree_inner_off_subset` rustdoc says "decode_tap_terminal which calls validate_tap_leaf_subset" but v0.6 catch-all uses TagInvalidContext, no validate call. (Already updated in Phase 5 audit-table fold.)

## Conclusion

Phase 4 mechanical rename: **DONE**. Phase 5 (corpus expansion) can proceed.
