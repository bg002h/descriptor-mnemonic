# v0.11 Phase 13 Review — Validation Invariants

- **Date:** 2026-04-30
- **Branch:** `feature/v0.11-impl-phase-1`
- **Phase commit:** `e509580` (Task 13.1)
- **Spec refs:** §7 (validation invariants), §6.3.1 (tap-script-tree leaf restriction)

## Scope

Phase 13 lands the three structural validators that enforce v0.11 wire-format
invariants beyond what the bit-level decoder catches. This phase also resolves
the Phase 9 carry-forward item (tap-script-tree leaf validation).

## Deliverables (commit `e509580`)

Three validators, 7 unit tests, 4 new error variants:

1. **`validate_placeholder_usage(root, n)`** — BIP 388 well-formedness
   (§7). Every `@i` for `i ∈ 0..n` must appear at least once in the
   descriptor body, and the *first* occurrence of each placeholder must
   appear in canonical ascending order during a pre-order traversal of the
   AST. Rejects unreferenced placeholders and out-of-order first
   occurrences.

2. **`validate_multipath_consistency(shared, overrides)`** — all
   `MultiPath` declarations across the shared path and any per-`@N`
   overrides must agree on alt-count (§7). Mismatched alt-counts surface
   as a structured error rather than a silent wire-format ambiguity.

3. **`validate_tap_script_tree(node)`** — enforces §6.3.1: tap-script-tree
   leaves may not carry top-level constructors (`Wpkh`, `Tr`, `Wsh`, `Sh`,
   `Pkh`) nor non-tap-context multisig (`Multi`, `SortedMulti`). Only
   tap-context fragments and `pk_k`/`pk_h`/policy combinators are admissible
   leaves.

## Test verification

```
$ cargo test -p md-codec --lib v11::validate
running 7 tests
test v11::validate::tests::multipath_consistency_rejects_mismatched_alt_counts ... ok
test v11::validate::tests::multipath_consistency_ok_when_all_match ... ok
test v11::validate::tests::placeholder_usage_rejects_unreferenced ... ok
test v11::validate::tests::placeholder_usage_rejects_out_of_order_first_occurrences ... ok
test v11::validate::tests::placeholder_usage_ok_for_2_of_3 ... ok
test v11::validate::tests::tap_tree_leaf_accepts_pk_k ... ok
test v11::validate::tests::tap_tree_leaf_rejects_wsh ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 549 filtered out
```

**Cumulative v11 tests:** 72 (65 prior + 7 new).

## Carry-forward resolution

- **P9 — tap-script-tree leaf validation.** Deferred from Phase 9 with a
  spec-cited TODO; now shipped as `validate_tap_script_tree` with
  `ForbiddenTapTreeLeaf` rejecting the §6.3.1-prohibited tag set. Item is
  closed.

## Minor non-blocking review notes (deferred)

1. **`ForbiddenTapTreeLeaf { tag: String }`** uses Rust `Debug` formatting
   to render the offending tag. This is convenient but non-stable across
   refactors of the `Tag` enum. Prefer `Display` impl or carrying the
   raw `u8` discriminant for stability. Defer.

2. **Placeholder bounds check** — `(*index as usize) < seen.len()`
   silently ignores out-of-range placeholder indices rather than asserting
   the structural invariant established earlier in the pipeline. Harden
   with `debug_assert!` to catch upstream regressions in tests. Defer.

## Updated deferred-items carry-forward list

- **P1:** `read_past_end_errors` state-preservation; unused
  `BitStreamExhausted` variant.
- **P2:** `write_varint` `debug_assert` → `assert` audit; `L=0` hand-crafted
  test.
- **P4:** `PathDecl::write` `# Errors` rustdoc gap.
- **P5:** `UseSitePath::write` `# Errors` rustdoc gap.
- **P7:** arity-2/3 dispatch coverage (deferred to Phase 14 smoke).
- **P9:** ✅ **RESOLVED** in P13 (`validate_tap_script_tree` shipped).
- **P12:** TLV decoder loop `>= 5` workaround; proper bit-bounded reader
  in P14, rollback in P19.
- **P13a:** `ForbiddenTapTreeLeaf` `Debug` formatting → `Display`/`u8`.
- **P13b:** placeholder bounds check `debug_assert!` hardening.

## Verdict

Phase 13 **PASS**. Validators implement §7 + §6.3.1, tests pass cleanly,
P9 carry-forward closed, only minor cosmetic/hardening notes remain.

**Next:** Phase 14 — end-to-end encoder + decoder integration (and
arity-2/3 dispatch smoke from P7).
