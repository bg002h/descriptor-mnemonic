# Phase 6 bucket F — tests/error_coverage.rs (Task 6.22)

**Status:** DONE_WITH_CONCERNS (at landing time; concerns resolved by integrated state)
**Commit:** `6c00eba`
**File(s):** `crates/wdm-codec/tests/error_coverage.rs` (NEW); read `crates/wdm-codec/src/error.rs` for variant enumeration; read `crates/wdm-codec/Cargo.toml` to confirm strum dev-dep
**Role:** implementer
**Tests added:** 5 (1 main exhaustiveness test + 4 helper unit tests)

## Summary

`tests/error_coverage.rs` implements the strum-based exhaustiveness gate that ensures every `wdm_codec::Error` variant has a corresponding `rejects_<snake_case>` test in `tests/conformance.rs`. The `ErrorVariantName` mirror enum (25 variants) derives `EnumIter`. The main test iterates all variants at runtime, reads `tests/conformance.rs` via `fs::read_to_string`, and asserts each has a matching `rejects_*` test name (with `rejects_invalid_bytecode_` as a prefix match for the multi-sub-variant case).

## Key deviation from the prompt's `include_str!` approach

Switched to `std::fs::read_to_string` at runtime so the file compiles independently of whether `conformance.rs` exists — avoids a compile-time failure during the parallel-batch race when conformance.rs hadn't yet landed.

## State at landing time

The 4 helper tests (`pascal_to_snake_*`) passed immediately. The main exhaustiveness test failed at the moment of this commit because conformance.rs (bucket E) had not yet landed with `rejects_miniscript`. **This was correct behavior** — the gate was working as designed.

After the integrated batch landed (with E's `afc4564` providing `rejects_miniscript` + the other 33 `rejects_*` tests), all 5 tests in this file pass.

## Concerns from this bucket's report (resolved at integration time)

The bucket F report listed the following as outstanding when the file was committed; at integration time, these were all resolved:

1. **Missing `rejects_miniscript` test** in conformance.rs — bucket E added it (`afc4564`).
2. **`cargo fmt` failures in conformance.rs** — bucket E's commit was fmt-clean by the time it landed.
3. **`rejects_invalid_bytecode_missing_children` test failure** — bucket E `#[ignore]`d this test (see `6e-missing-children-unreachable` in FOLLOWUPS.md).
4. **`rejects_unsupported_version` test failure** — resolved by bucket E using the correct version-nibble shift in the test fixture.

## Follow-up items

None NEW from this bucket; all the items it flagged at landing were already-known or were resolved by bucket E's integrated commit.
