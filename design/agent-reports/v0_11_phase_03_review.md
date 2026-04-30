# v0.11 Phase 3 — Payload header (5-bit)

**Date:** 2026-04-30
**Scope:** Tasks 3.1, 3.2 (this phase review).

## Commits

- Task 3.1 — `ec3fdab` — `Header` struct (5-bit payload header) + 2 error variants + 5 tests

## Files added

- `crates/md-codec/src/v11/header.rs` (new)
- `crates/md-codec/src/v11/mod.rs` — `pub mod header;` addition
- `crates/md-codec/src/v11/error.rs` — 2 new error variants (wrong-version, reserved-bit-set)

## Test count

5 unit tests in `v11::header::tests`, all passing under `cargo test -p md-codec --lib v11::header`:

- `header_common_case_byte_value`
- `header_round_trip_shared`
- `header_round_trip_divergent`
- `header_rejects_wrong_version`
- `header_rejects_reserved_bit`

Cumulative `v11` test count: **22** (7 bitstream + 10 varint + 5 header), all passing under `cargo test -p md-codec --lib v11`.

## Spec coverage

- §3.3 (5-bit payload header layout): version (3 bits), reserved bit 3, divergent-paths flag (bit 4). Encoder writes spec-compliant bytes; decoder rejects non-conformant inputs.

## Implementation notes

- **Field name `divergent_paths` (not `all_keys_same_path`).** The Header struct exposes the flag as `divergent_paths: bool` so the field name matches the bit's value semantically: bit 4 = 0 means shared (`divergent_paths == false`), bit 4 = 1 means divergent (`divergent_paths == true`). This avoids the double-negative read pattern that an `all_keys_same_path` naming would force on every callsite.
- **Reserved bit 3.** Encoder writes 0 unconditionally. Decoder rejects payload-headers with bit 3 = 1. Note that the eventual decoder dispatch on bit 3 to distinguish payload-header from chunk-header framing is Phase 19/20 territory; the Phase 3 payload-header decoder simply rejects bit-3=1 inputs at this layer, which is the correct behavior for the payload-header reader in isolation.

## Authorized minor deviations

- **Dropped `| 0u64`.** The implementer omitted a `| 0u64` term flagged by `clippy::identity_op`. No behavior change; the bit-pack expression evaluates identically.
- **`V0_11_VERSION` as associated const.** Declared as `Header::V0_11_VERSION` rather than a free `const` in the module. The spec/plan did not specify which form; both are valid. Associated-const placement keeps the constant scoped to the type it parameterizes.

## Deferred items carrying forward

- **Phase 1 — `read_past_end_errors` state-preservation assertion.** Test could additionally assert that a failed read leaves bit position unchanged (recoverability). Behavior is correct; only test coverage is thinner than ideal.
- **Phase 2 — `write_varint` debug-assert promotion.** The `debug_assert!` guard on varint input bounds could be promoted to an unconditional `assert!` for release-mode safety.

Both items are non-blocking and worth consolidating in a single future polish commit rather than spot-fixing across phases.

## Open items

None blocking phase exit.

## Next phase

Phase 4 — first wire-format encoder slice consuming `Header` + varint primitives.
