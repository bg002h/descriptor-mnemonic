# v0.11 Phase 1 — Module scaffold + bit-stream library

**Date:** 2026-04-30
**Scope:** Tasks 1.1, 1.2, 1.3 (and this task, 1.4, the phase review).

## Commits

- Task 1.1 — `e911d5d` — scaffold `v11` module + `V11Error` enum
- Task 1.2 — `2d4989e` — `BitWriter` (MSB-first)
- Task 1.3 — `c5c9a1f` — `BitReader` (MSB-first)

## Files added

- `crates/md-codec/src/v11/mod.rs`
- `crates/md-codec/src/v11/bitstream.rs`
- `crates/md-codec/src/v11/error.rs`
- `crates/md-codec/src/lib.rs` — single-line `pub mod v11;` addition

## Test count

7 unit tests in `v11::bitstream::tests`, all passing under `cargo test -p md-codec --lib v11`:

- BitWriter: `write_zero_bits_is_noop`, `write_5_bits_msb_first`, `write_8_bits_is_one_byte`, `write_two_5_bit_values_packs_into_one_and_a_bit`
- BitReader: `read_full_byte_aligned`, `round_trip_5_bit_values`, `read_past_end_errors`

## Spec coverage

- §4.6 (bit-packing convention): MSB-first, big-endian-within-byte, fully covered for the read+write primitives in scope.
- §13.1 (bit-stream library): partial — the core `BitWriter`/`BitReader` types and their bit-level read/write primitives are in place. Higher-level helpers (varint encoders, framed-chunk readers, etc.) are deferred to subsequent phases.

## Carryover cleanups completed mid-phase

- Task 1.3 added `#[derive(Default)]` to `BitWriter` (carryover from Task 1.2 — the derive was identified as appropriate after the type stabilized).
- Task 1.3 added a `// --- BitReader ---` section marker in `bitstream.rs` to delimit reader vs. writer code, since both now live in the same module.
- Task 1.1 originally intended to declare `pub mod bitstream;` in `v11/mod.rs`, but this was deferred into Task 1.2 to avoid a half-state where `mod.rs` referenced a non-existent submodule. The declaration landed alongside the actual `bitstream.rs` source in `2d4989e`.

## Deferred minor items (not blocking; logged for future cleanup)

- **`read_bits` `chunk == 8` overflow fix vs. plan text.** The implementation diverged slightly from the literal plan text to avoid a left-shift overflow when `chunk == 8`. Behavior matches the spec; the plan-vs-actual deviation is purely an implementation detail worth noting if the plan is ever re-derived from the code.
- **`read_past_end_errors` could be strengthened.** A code-quality review observation: the test confirms the error is raised but does not assert that reader state is preserved (i.e., that a failed read leaves the bit position unchanged so callers can recover or re-attempt). Deferred — not on the critical path for Phase 2, and the underlying behavior is correct; only the test coverage is thinner than ideal.

## Open items

None blocking phase exit.

## Next phase

Phase 2 — LP4-ext varint (encoder + decoder built on top of the bit-stream primitives landed here).
