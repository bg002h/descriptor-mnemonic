# Follow-up batch 1 — bucket A (chunking.rs)

**Status:** DONE
**Commit:** 430dbfcdd5f98737fb09719c00acf0927a7fa4f5
**File(s):** crates/wdm-codec/src/chunking.rs
**Role:** implementer

## Items closed

- `5d-from-impl`
- `5d-decision-cross-reference`

## Changes made

### `5d-from-impl` — `From<ChunkCode> for BchCode`

Added `impl From<ChunkCode> for crate::BchCode` immediately after the `ChunkCode` impl block (lines 228–245 in the updated file). Maps `ChunkCode::Regular` → `BchCode::Regular` and `ChunkCode::Long` → `BchCode::Long`. Import is via fully-qualified `crate::BchCode` (no extra `use` statement needed; `BchCode` is re-exported from `lib.rs`).

Added the required unit test `chunk_code_converts_to_bch_code` in the `#[cfg(test)] mod tests` block. The test asserts both directions with `BchCode::from(ChunkCode::…)`.

**Side-effect note for follow-up:** The helper `chunk_code_to_bch_code` in `crates/wdm-codec/src/encode.rs` (lines 13–22) is now redundant. Call sites can be migrated to `let bch_code: BchCode = code.into()` in a separate commit; `encode.rs` was not touched here per scope constraints.

### `5d-decision-cross-reference` — `chunking_decision` rustdoc note

Added a `## Notes` section to the `chunking_decision` function's rustdoc, placed between the existing `force_chunked` preference note and the `# Errors` section. The note reads:

> Note: when `EncodeOptions::force_long_code` is set, the top-level `encode()` function post-processes the returned plan to swap Regular → Long. See `crates/wdm-codec/src/encode.rs::encode` Stage 3.

## Gate results

| Gate | Result |
|------|--------|
| `cargo test -p wdm-codec` | All green; chunking suite 32 → 33 (+1 new test). No failures, 1 pre-existing ignored. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Clean |
| `cargo fmt --check` | Clean |
| `RUSTDOCFLAGS="-D warnings -D missing_docs" cargo doc ...` | Clean |

## Notes

The `decode.rs` and `options.rs` files have uncommitted modifications in the working tree from parallel bucket work; these were not staged or touched.
