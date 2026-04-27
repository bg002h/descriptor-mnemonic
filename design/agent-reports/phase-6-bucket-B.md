# Phase 6 bucket B — tests/upstream_shapes.rs (Task 6.14)

**Status:** DONE
**Commit:** `860f3ee`
**File:** `crates/wdm-codec/tests/upstream_shapes.rs` (NEW)
**Tests added:** 9 (0 ignored)

## Summary

9 named `shape_*` tests covering the descriptor-codec operator coverage matrix rewritten in BIP 388 `@i` placeholder form. Each test calls `common::round_trip_assert` on the policy string.

## Shape coverage

| # | Test | Operator |
|---|---|---|
| 1 | `shape_pk` | `pk(K)` (basic key check) |
| 2 | `shape_pkh` | `wsh(c:pk_h(@0/**))` — top-level `pkh()` rejected by v0.1 D-4 scope; used inner-tree equivalent |
| 3 | `shape_multi` | `multi(k, ...)` |
| 4 | `shape_sortedmulti` | `sortedmulti(k, ...)` |
| 5 | `shape_and_v` | `and_v(V, T)` |
| 6 | `shape_or_d` | `or_d(B, T)` |
| 7 | `shape_or_i` | `or_i(B, B)` |
| 8 | `shape_andor` | `andor(B, T, T)` |
| 9 | `shape_thresh` | `thresh(k, ...)` (variable arity) |

## Shape adjustments made

- **Shape 2 (`pkh`)**: top-level `pkh(@0/**)` is rejected by v0.1 D-4 scope restriction (only `Wsh()` allowed). Used `wsh(c:pk_h(@0/**))` instead, which is the equivalent miniscript inner-tree form. Exercises `PkH` (0x1C) + `Check` (0x0C) tags.

All 9 shapes type-checked and round-tripped cleanly with no further adjustments needed.

## Test results

- `cargo test -p wdm-codec --test upstream_shapes`: 9 passed, 0 failed, 0 ignored
- `cargo clippy --workspace --all-targets -- -D warnings`: clean (this file)
- `rustfmt --check` on this file: clean

## Pre-existing failures in OTHER files at the time of this commit (not caused by this work, resolved by other parallel buckets)

- `corpus_encode_decode_encode_idempotency` in `tests/corpus.rs` (bucket A's file)
- `cargo fmt --check` reported diffs in `tests/corpus.rs` and `tests/common/mod.rs` (other agents' files)

These were transient parallel-batch states; resolved by the time the full batch landed.

## Follow-up items

None from this bucket.
