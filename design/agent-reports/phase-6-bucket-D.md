# Phase 6 bucket D — tests/ecc.rs (Tasks 6.19–6.20)

**Status:** DONE
**Commit:** `b937aee`
**File:** `crates/wdm-codec/tests/ecc.rs` (NEW)
**Tests added:** 2

## Summary

Two BCH error-correction stress tests: every-position single-substitution coverage and a 1000-iteration multi-substitution rejection test.

## Test details

- **Task 6.19 — `bch_single_substitution_at_every_position_corrects`**: iterates every data-part position, substitutes with the first different bech32 character, asserts exactly 1 correction in `report.corrections` and `DecodeOutcome::AutoCorrected`.
- **Task 6.20 — `many_substitutions_always_rejected`**: 1000 iterations with 5–9 random substitutions (seed `0xDEADBEEF`) using `common::corrupt_n`, asserts ≥95% rejection. Test runs in ~1.4s total.

## Empirical rejection rate for 6.20

100% rejection (1000/1000) with the `wsh(pk(@0/**))` policy and 5–9 errors — well above the 95% threshold. The policy encodes to a short string (Regular BCH code, ~18 data-part chars), so any 5+ error pattern is deep in uncorrectable territory; a false positive to a valid codeword would be astronomically unlikely.

## Follow-up items (added to FOLLOWUPS.md in `c64f66c`)

- `6d-rand-gen-keyword` (v0.1-nice-to-have): the `rng.r#gen()` workaround for the Rust 2024 `gen` reserved keyword is correct; if the crate migrates to a newer `rand` API (e.g., `rng.random::<u64>()`), that workaround can be cleaned up.
