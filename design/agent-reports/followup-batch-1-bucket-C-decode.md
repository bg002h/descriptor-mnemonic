# Follow-up batch 1 — bucket C (decode.rs)

**Status:** DONE
**Commit:** fa83737
**File(s):** crates/wdm-codec/src/decode.rs
**Role:** implementer

## Items closed

- `5e-skip-silent`
- `5e-dead-branch`
- `5e-correction-position-doc`
- `5e-five-bit-truncated-mapping`

## Changes made

### `5e-skip-silent` — replace size-conditional test skips with `force_chunking: true`

Added `force_chunking_opts()` helper to the test module:

```rust
fn force_chunking_opts() -> EncodeOptions {
    EncodeOptions::default().with_force_chunking(true)
}
```

Replaced the two silently-skippable tests:

1. `decode_round_trip_chunked_two_chunks` (originally around line 264): removed the `if bytecode.len() <= 56 { return; }` guard and the 9-key multi-sig policy. Switched to a sha256-terminal policy (`wsh(and_v(v:pk(@0/**),sha256(111...)))`) with `force_chunking_opts()`. The sha256 terminal embeds 32 bytes into the bytecode, driving it above the 45-byte Regular single-chunk fragment capacity, so ≥2 chunks are produced deterministically.

2. `decode_report_chunked_clean_confirmed` (originally around line 526): removed the `if bytecode.len() <= 56 { return; }` skip and the `p.to_bytecode()` intermediate call. Now encodes the same 9-key multi-sig directly with `force_chunking_opts()`. Since this test only checks report fields (not chunk count), the sha256 change was not needed here — the Chunked encoding path is exercised regardless.

**Note on policy selection for `decode_round_trip_chunked_two_chunks`:** The task spec suggested using the 9-key or 12-key multi-sig policy. Both produce compact bytecodes (31 bytes) because `@0/**` keys encode as single-byte indices, not full xpubs. `force_chunking=true` with a 31-byte policy produces `ceil((31+4)/45) = 1` chunk — still Chunked header but count=1. The sha256 policy produces ~55 bytes of bytecode, giving `ceil(59/45) = 2` chunks.

### `5e-dead-branch` — remove unreachable fallback in `decode_rejects_chunks_with_duplicate_indices`

Removed the `if backup.chunks.len() < 2 { ... return; }` fallback block entirely. The test now uses `force_chunking_opts()` directly, encoding with a Chunked header that always allows duplicate-index detection. The original fallback re-encoded the same policy redundantly and was unreachable for the 9-key fixture with `encode_opts()` (which also happened to produce count=1).

### `5e-correction-position-doc` — add rustdoc cross-reference for `Correction.char_position`

Added a `# Note on \`char_position\` in corrections` section to the `pub fn decode` rustdoc, explaining that each `Correction.char_position` is a 0-indexed offset into the chunk's data part (after the `wdm1` HRP+separator) and cross-referencing `crate::Correction::char_position`.

### `5e-five-bit-truncated-mapping` — replace `Truncated` mapping with `expect()`

Replaced the `.ok_or(Error::InvalidBytecode { offset: 0, kind: BytecodeErrorKind::Truncated })?` with `.expect("...")`, using the `expect` form as recommended in the task spec. Added an inline comment explaining why this path is structurally unreachable after a successful BCH decode + checksum strip.

## Gates

- `cargo test -p wdm-codec` — all tests pass (no count change; refactors only; the pre-existing CLI compile error in `tests/cli.rs` also resolved as a side-effect of `cargo fmt` fixing a pre-existing import-order issue in `src/bytecode/decode.rs`)
- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `cargo fmt --check` — clean (also fixed pre-existing import-order issue in `src/bytecode/decode.rs`)
- `RUSTDOCFLAGS="-D warnings -D missing_docs" cargo doc` — clean
