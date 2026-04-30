# v0.11 Phase 7 Review Report

- **Date:** 2026-04-30
- **Branch:** `feature/v0.11-impl-phase-1`
- **Commits:**
  - Task 7.1: `67cb1f5` — Tree Class 1 dispatch (KeyArg + Children for arity 0/1/2/3 fixed-arity tags), 3 tests
  - Task 7.2: `a1b284d` — `wrapper_chain_v_c_pk_round_trip` test (1 test)

## Files modified

- `crates/md-codec/src/v11/tree.rs` — filled in Class 1 dispatch arms in `write_node`/`read_node` (KeyArg encoding + fixed-arity child recursion); added 4 unit tests.

## Test results

- `cargo test -p md-codec --lib v11::tree` → 4 passed (`key_arg_n1_zero_bits`, `key_arg_n3_two_bits`, `key_arg_round_trip`, `wrapper_chain_v_c_pk_round_trip`).
- `cargo test -p md-codec --lib v11` → **48 passed** cumulative (44 prior + 3 from Task 7.1 + 1 from Task 7.2).

## Spec coverage (§6.1, Class 1: fixed-arity, no body fields)

Class 1 tags now dispatched in `tree.rs`:

- **Key-arg leaves:** Pkh, Wpkh, PkK, PkH
- **Script-wrapping (Sh, Wsh):** 1 child (sub-script)
- **Wrappers (all 7):** Alt, Swap, Check, DupIf, Verify, NonZero, ZeroNotEqual — arity 1
- **Binary combinators:** AndV, AndB, OrB, OrC, OrD, OrI — arity 2
- **Ternary:** AndOr — arity 3
- **TapTree internal:** arity 2

## Deferred to later phases

The `_ => unimplemented!()` arms in `write_node`/`read_node` will close out as:

- **Phase 8:** Tree Class 2 — Multi, SortedMulti, MultiA, SortedMultiA, Thresh (variable-arity multisig + thresh)
- **Phase 9:** Tr (taproot internal-key + optional TapTree)
- **Phase 10:** Terminals — After, Older, Sha256, Hash160
- **Phase 11:** Extension space — Hash256, Ripemd160, RawPkH, False, True

## TDD discipline observation

Task 7.1 implementer landed tests + impl together (no explicit red step recorded). Code came out correct on the first pass. Same pattern as Task 4.2 — flagging for trend-watching but not blocking.

## Code quality observations

The arity-1 path is exercised by `wrapper_chain_v_c_pk_round_trip` (Verify/Check around PkK). Arity-2 (AndV/AndB/OrB/OrC/OrD/OrI) and arity-3 (AndOr) paths are structurally implemented and dispatched but lack dedicated unit-test coverage. Recommend deferring full per-tag coverage to **Phase 14 smoke tests** rather than backfilling now — the recursive structure is uniform across these arms and the dispatch itself is exercised.

## Carry-forward deferred items (Phases 1–6)

- **Phase 1:** `read_past_end_errors` state-preservation; unused `BitStreamExhausted` variant
- **Phase 2:** `write_varint` `debug_assert!` → `assert!` upgrade; L=0 hand-crafted decode test
- **Phase 4:** `PathDecl::write` `# Errors` rustdoc gap
- **Phase 5:** `UseSitePath::write` `# Errors` rustdoc gap
- **Phase 7 (new):** arity-2/3 explicit unit-test coverage (defer to Phase 14 smoke)

## Next

Phase 8 — Tree Class 2 (variable-arity multisig + thresh).
