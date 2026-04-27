# Phase 6 bucket C — tests/chunking.rs (Tasks 6.15–6.18)

**Status:** DONE
**Commit:** `d98b51b`
**File(s):** `crates/wdm-codec/tests/chunking.rs` (NEW)
**Role:** implementer
**Tests added:** 4

## Summary

4 named integration tests covering chunking-specific behavior: cross-chunk hash mismatch rejection, correct in-order reassembly, out-of-order reassembly, and the natural long-code boundary (49–56 bytes).

## Test details

- **`chunk_hash_mismatch_rejects`** — uses `chunk_bytes` directly with a synthetic 60-byte bytecode (→ 2 Regular chunks), flips `chunks[1].fragment[0]` (payload, not hash region), calls `reassemble_chunks`, asserts `Error::CrossChunkHashMismatch`.
- **`chunk_hash_correct_reassembly`** — encodes `wsh(multi(5,...,@11/**))` (12 keys, reliably >56 bytes bytecode), decodes in-order, asserts structural equality.
- **`chunk_out_of_order_reassembly`** — same policy, reverses raw strings before `decode`, asserts success.
- **`natural_long_code_boundary`** — uses `wsh(multi(2,@0/**,@1/**,@2/**,@3/**))`, the same policy as the existing `encode_single_string_long_naturally` unit test. Confirmed to produce 49–56 byte bytecode. Asserts 1 chunk, `BchCode::Long`, `SingleString` header, and round-trip equality. A guarded fallback handles the (unlikely) case where encoder changes shift the size out of range.

## Follow-up items (added to FOLLOWUPS.md in `c64f66c`)

- `6c-encode-options-builder` (v0.1-nice-to-have): `EncodeOptions` is `#[non_exhaustive]`, preventing struct-update syntax `EncodeOptions { force_long_code: true, ..Default::default() }` from external integration tests. The `natural_long_code_boundary` test had to use a conditional check (`if bytecode.len() > 48 && bytecode.len() <= 56`) instead of explicit `force_long_code` testing. If `force_long_code` needs to be exercised from integration tests in future, `EncodeOptions` should expose a builder method (e.g., `EncodeOptions::default().with_force_long_code()`).

## Concerns at landing time (resolved by other buckets)

- Bucket F's `error_coverage.rs` had a pre-existing compile error: `include_str!("conformance.rs")` was failing because `conformance.rs` (bucket E) didn't exist yet. Resolved when bucket F switched to runtime `fs::read_to_string`.
