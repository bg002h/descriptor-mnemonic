# v0.7.0 Phase 3 review

**Status:** DONE_CLEAN
**Reviewer:** Claude Opus 4.7 (1M context)
**Date:** 2026-04-29
**Commit reviewed:** `f654d3b` on `feature/v0.7.0-development`
**Files reviewed:**
- `crates/md-codec/src/bytecode/encode.rs` (lines 617–756 + 811–867)
- `crates/md-codec/src/bytecode/decode.rs` (`tag_to_bip388_name`, lines 1017–1067)
- `crates/md-codec/tests/taproot.rs` (lines 130–170)
- `crates/md-codec/tests/v0_5_type_wiring.rs` (lines 1–80)
- `design/IMPLEMENTATION_PLAN_v0_7_0.md` §3 + §4
- `design/SPEC_v0_7_0.md` §4.4 + §9
- `design/agent-reports/v0-7-0-plan-review-1.md` (Concern 6)
**Role:** reviewer (Phase 3)

## Summary

Phase 3 is correct. The `HISTORICAL_COLDCARD_TAP_OPERATORS` constant is byte-identically equivalent to the prior hardcoded match arms; the depth-first leaf-first walker is exhaustive over all 30 `Terminal` variants; existing tests continue to pass under the new walk-order semantics; the back-compat shim is preserved. **No Critical, no Important findings.** Two nits worth tracking.

## Verification of the 6 specific concerns

### Concern 1 — Back-compat preservation (PASS, confidence 95)

`HISTORICAL_COLDCARD_TAP_OPERATORS = ["pk_k", "pk_h", "multi_a", "or_d", "and_v", "older", "c:", "v:"]` matches `tag_to_bip388_name` output for all 8 prior admit arms exactly.

### Concern 2 — Walk-order semantics change (PASS, confidence 90)

The new walker recurses into children FIRST, then checks the parent. Reviewer traced both existing tests:

- `taproot_rejects_out_of_subset_sha256`: `and_v(v:sha256(...), c:pk_k(...))` — both old and new walkers report "sha256". Same outcome.
- `taproot_rejects_wrapper_alt_outside_subset`: `thresh(2, c:pk_k, sc:pk_k, sc:pk_k)` — old reported "thresh", new reports "s:" (deeper child). Test asserts `operator == "thresh" || operator.starts_with("s:")` — defensively allows both. Strict diagnostic improvement (deepest violation = most actionable).
- `tap_leaf_subset_violation_has_leaf_index_field`: same fixture as above, asserts `contains("sha256")` — passes.

### Concern 3 — Exhaustive child recursion (PASS, confidence 95)

All 30 `Terminal` variants accounted for in the walker's match (no `_` catch-all). Future miniscript upgrades adding new variants produce a compile error pointing here — desirable.

### Concern 4 — `pub const HISTORICAL_COLDCARD_TAP_OPERATORS` visibility (NIT, see N-1)

### Concern 5 — Walk-order test coverage gap (NIT, see N-2)

### Concern 6 — Tap-illegal `Multi`/`SortedMulti` arms (PASS, confidence 90)

Walker treats them as leaves (no children to recurse). Defensive coverage for hand-AST inputs; reachable code path is dead via the parser (BIP 342 disables `OP_CHECKMULTISIG` in tapscript).

## Nits

### N-1. `HISTORICAL_COLDCARD_TAP_OPERATORS` visibility could be `pub(crate)` (Confidence: 80)

Plan §3.3 specified `const` (private); implementation uses `pub const`. The only in-tree consumer is the same-module back-compat shim. md-signer-compat (Phase 4) defines its own `COLDCARD_TAP.allowed_operators` array and does not reference this constant. `pub(crate)` would tighten the API surface.

### N-2. Walk-order semantics change has no regression test (Confidence: 85)

The "deepest violation reported" property is observable but not pinned. A ≤20-line hand-AST test asserting "thresh(1, sha256(H))" rejects with operator="sha256" (not "thresh") under an empty-allowlist would lock the new contract.

## FOLLOWUPS to add

1. **`v07-historical-coldcard-const-visibility`** (Tier: v0.7.x defensive cleanup) — tighten `pub const HISTORICAL_COLDCARD_TAP_OPERATORS` to `pub(crate) const`.
2. **`v07-walker-deepest-violation-pin-test`** (Tier: v0.7.x defensive testing) — add a regression test pinning the depth-first leaf-first contract.

## Verdict

**DONE_CLEAN.** Phase 3 faithfully implements the spec. Walk-order semantics change is intentional, documented in the rustdoc, doesn't break any existing test, and produces strictly better diagnostics. Two nits worth tracking, not blocking. Controller proceeds to Phase 5.
