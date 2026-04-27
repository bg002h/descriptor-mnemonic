# Phase 6 bucket E — tests/conformance.rs (Task 6.21)

**Status:** DONE
**Commit:** `afc4564`
**File(s):** `crates/wdm-codec/tests/conformance.rs` (NEW); read `crates/wdm-codec/src/error.rs` for variant enumeration
**Role:** implementer
**Tests added:** 34 (33 pass, 1 ignored)

## Summary

`assert_decode_rejects!` macro + 34 named `rejects_*` tests covering every rejection path across all public API layers.

## Test organization by layer

- **Layer 1 (codex32 / string)**: 5 tests
  - `rejects_invalid_hrp`, `rejects_mixed_case`, `rejects_invalid_string_length`, `rejects_invalid_char`, `rejects_bch_uncorrectable`
- **Layer 2 (ChunkHeader)**: 6 tests
  - `rejects_unsupported_version`, `rejects_unsupported_card_type`, `rejects_reserved_wallet_id_bits_set`, `rejects_invalid_chunk_count`, `rejects_invalid_chunk_index`, `rejects_chunk_header_truncated`
- **Layer 3 (reassembly)**: 9 tests
  - `rejects_empty_chunk_list`, `rejects_single_string_with_multiple_chunks`, `rejects_mixed_chunk_types`, `rejects_wallet_id_mismatch`, `rejects_total_chunks_mismatch`, `rejects_chunk_index_out_of_range`, `rejects_duplicate_chunk_index`, `rejects_missing_chunk_index`, `rejects_cross_chunk_hash_mismatch`
- **Layer 4 (bytecode)**: 10 tests
  - `rejects_invalid_bytecode_unknown_tag`, `rejects_invalid_bytecode_truncated`, `rejects_invalid_bytecode_varint_overflow`, **`rejects_invalid_bytecode_missing_children`** (`#[ignore]`d — see follow-up), `rejects_invalid_bytecode_unexpected_end`, `rejects_invalid_bytecode_trailing_bytes`, `rejects_invalid_bytecode_reserved_bits_set`, `rejects_invalid_bytecode_unexpected_tag`, `rejects_invalid_bytecode_type_check_failed`, `rejects_invalid_bytecode_invalid_path_component`
- **Layer 5 (policy scope)**: 2 tests
  - `rejects_policy_scope_violation`, `rejects_policy_parse`
- **Layer 6 (chunking)**: 1 test
  - `rejects_policy_too_large`
- **Plus**: `rejects_miniscript` (variant constructibility smoke test)

Total: 34 named tests, well above the 18-test minimum spec'd in `IMPLEMENTATION_TASKS_v0.1.md`.

## Follow-up items (added to FOLLOWUPS.md in `c64f66c`)

- `6e-missing-children-unreachable` (v0.1-nice-to-have): `BytecodeErrorKind::MissingChildren` is defined in the error enum but never emitted by any v0.1 code path. Truncation in `Multi`/`Thresh` surfaces as `UnexpectedEnd` instead. The `rejects_invalid_bytecode_missing_children` test is `#[ignore]`d. To fix: add an explicit arity check to `decode_terminal` that emits `MissingChildren { expected: n, got: actual }` when the children loop exhausts the buffer.
