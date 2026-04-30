# v0.11 Phase 2 — LP4-ext varint

**Date:** 2026-04-30
**Scope:** Tasks 2.1 (LP4-ext varint encode/decode) and 2.2 (this phase review).

## Commits

- Task 2.1 — `a26a022` — LP4-ext varint encode/decode

## Files added

- `crates/md-codec/src/v11/varint.rs`
- `crates/md-codec/src/v11/mod.rs` — single-line `pub mod varint;` addition

## Test count

10 unit tests in `v11::varint::tests`, all passing under
`cargo test -p md-codec --lib v11::varint`:

- `varint_zero`, `varint_one`, `varint_84`, `varint_1024`
- `varint_16383_no_extension`, `varint_16384_uses_extension`
- `varint_max_u31`
- `varint_zero_costs_4_bits`, `varint_one_costs_5_bits`, `varint_84_costs_11_bits`

## Spec coverage

- §4.1 (LP4-ext varint): encode + decode, including the L=0 zero-length form,
  the in-band 4-bit-length form (L ∈ 1..=14), and the single-extension form
  (L=15 + 4-bit `L_high` + payload). Length-prefix-only edge cases are
  exercised alongside payload round-trips and exact-bit-cost assertions.

## Authorized deviation logged

The plan's `varint_max_u31` test asserted that `(1u32 << 31) - 1` round-trips,
but the documented LP4-ext single-extension form caps at
14 + max(`L_high` = 15) = 29 payload bits. The implementer reduced the test
value to `(1u32 << 29) - 1` (the actual cap) with a clarifying comment.
Recursive extension for values > 29 bits is deferred per plan §16 open
item #3.

## Deferred minor items (not blocking; logged for future cleanup)

- **`write_varint` cap is `debug_assert!` only.** The `debug_assert!(l_high <= 15)`
  guards the single-extension cap in debug builds; in release, values requiring
  more than 29 payload bits silently corrupt (only the low bits of `l_high` are
  written). Code-quality reviewer suggested promoting to `assert!` for hardened
  release-mode behavior. Deferred to phase-polish or a future hardening commit.
- **`read_varint` L=0 path lacks a hand-crafted test.** The L=0 decode path is
  exercised only via the `varint_zero` round-trip; it is not separately tested
  with a hand-crafted 4-bit-zero input stream. Code-quality reviewer flagged as
  a coverage gap; consider adding in a future test-coverage pass.

## Open items

None blocking phase exit.

## Next phase

Phase 3 — Header (3-bit version + 2 mode flags, per the locked D9 header
allocation; see commit `c30037b`).
