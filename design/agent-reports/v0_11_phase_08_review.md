# v0.11 Phase 8 Review Report

- **Date:** 2026-04-30
- **Branch:** `feature/v0.11-impl-phase-1`
- **Commits:**
  - Task 8.1: `eb0ef03` — Tree Class 2 variable-arity dispatch (Multi, SortedMulti, MultiA, SortedMultiA), 2 tests (`sortedmulti_2of3_round_trip`, `sortedmulti_2of3_bit_cost`)
  - Task 8.2: `065293f` — Thresh round-trip test (`thresh_2of3_with_pk_children`)

## Files modified

- `crates/md-codec/src/v11/tree.rs` — filled in Class 2 dispatch arms in `write_node`/`read_node` (variable-arity threshold-`k` + child-count-`n` encoding with KeyArg children for the four multisig variants and arbitrary sub-Node children for Thresh); added 3 unit tests and the associated error variants.

## Test results

- `cargo test -p md-codec --lib v11::tree` → **7 passed** (4 from Phase 7 + 3 from Phase 8: `sortedmulti_2of3_round_trip`, `sortedmulti_2of3_bit_cost`, `thresh_2of3_with_pk_children`).
- Cumulative `cargo test -p md-codec --lib v11` → **51 passed** (48 prior + 3 from Phase 8).

## Spec coverage (§6.2, Class 2: variable-arity)

Class 2 tags now dispatched in `tree.rs`:

- **Multisig (KeyArg children):** Multi, SortedMulti, MultiA, SortedMultiA — encoded as `(k, n, KeyArg×n)`.
- **Thresh (Node children):** Thresh — encoded as `(k, n, Node×n)`.

### Bit-cost confirmation

For 2-of-3 sortedmulti with `@1`/`@2`/`@3` key-args (per §6.2):

- 5 bits tag (SortedMulti)
- 5 bits k=2
- 5 bits n=3
- 3 × (5-bit Node head + 2-bit KeyArg N-index) = 3 × 7 = 21 bits children

Total = **36 bits**, asserted by `sortedmulti_2of3_bit_cost`.

## Error variants added

The variable-arity dispatch surfaces three new structured errors for malformed inputs / out-of-range fields:

- `ThresholdOutOfRange` — k outside `[1, n]` for Multi/SortedMulti/MultiA/SortedMultiA/Thresh.
- `ChildCountOutOfRange` — n outside the spec-permitted child-count window.
- `KGreaterThanN` — explicit guard for `k > n` (distinct from the generic `ThresholdOutOfRange` to aid diagnosis).

## Deferred to later phases

The `_ => unimplemented!()` arms in `write_node`/`read_node` continue to close out as:

- **Phase 9:** Tr (taproot internal-key + optional TapTree)
- **Phase 10:** Terminals — After, Older, Sha256, Hash160
- **Phase 11:** Extension space — Hash256, Ripemd160, RawPkH, False, True

## Carry-forward deferred items (Phases 1–7)

- **Phase 1:** `read_past_end_errors` state-preservation; unused `BitStreamExhausted` variant
- **Phase 2:** `write_varint` `debug_assert!` → `assert!` upgrade; L=0 hand-crafted decode test
- **Phase 4:** `PathDecl::write` `# Errors` rustdoc gap
- **Phase 5:** `UseSitePath::write` `# Errors` rustdoc gap
- **Phase 7:** arity-2/3 explicit unit-test coverage (defer to Phase 14 smoke)

## Next

Phase 9 — Tree Class 3 (Tr).
