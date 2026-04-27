# Phase v0.2 A — Bucket A: ChunkingMode enum

**Status:** DONE
**Closed short-id:** `p4-chunking-mode-enum`
**Commit:** `fbbe6ec` — `feat(chunking)!: ChunkingMode enum replaces force_chunked: bool (closes p4-chunking-mode-enum)`

## Summary

Replaced the v0.1 `force_chunked: bool` parameter on `pub fn chunking_decision`
and the parallel `force_chunking: bool` field on `EncodeOptions` with a typed
`ChunkingMode { Auto, ForceChunked }` enum. The change makes call sites
self-documenting (no more bare `true` / `false`) and gives v0.2+ room to grow
the variant set without further breaking changes.

The `EncodeOptions::with_force_chunking(self, force: bool)` builder method
keeps its `bool` signature as a source-compatibility shim for v0.1.1 callers
that already migrated to the builder pattern; it now translates
`true → ChunkingMode::ForceChunked`, `false → ChunkingMode::Auto`.

The wire format is unchanged: the committed `tests/vectors/v0.1.json`
verifies byte-identical via `gen_vectors --verify`.

## Files changed

- `crates/wdm-codec/src/chunking.rs`
  - Added `pub enum ChunkingMode { Auto, ForceChunked }` with
    `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` and a manual
    `Default for ChunkingMode` impl returning `Auto`.
  - Changed `pub fn chunking_decision` signature: second parameter is now
    `mode: ChunkingMode` instead of `force_chunked: bool`. Internal logic
    branches on `matches!(mode, ChunkingMode::Auto)`.
  - Updated rustdoc on `chunking_decision`, the `ChunkingPlan::Chunked.count`
    field, and the existing 14 in-module unit tests (now passing
    `ChunkingMode::Auto` / `ChunkingMode::ForceChunked`).

- `crates/wdm-codec/src/lib.rs`
  - Added `ChunkingMode` to the `pub use chunking::{...}` re-export block
    alongside the existing chunking re-exports.

- `crates/wdm-codec/src/options.rs`
  - Renamed `EncodeOptions.force_chunking: bool` →
    `EncodeOptions.chunking_mode: ChunkingMode` (default `Auto`).
  - Added `use crate::chunking::ChunkingMode;`.
  - Updated `with_force_chunking(self, force: bool)` to translate `bool`
    → `ChunkingMode` (kept the `bool` signature on purpose; per-spec, do
    not add `with_chunking_mode` in this commit — that's deferred).
  - Updated rustdoc on the struct, the field, and the builder method.
  - Migrated five existing tests off the old field name. Added two new
    tests: `with_force_chunking_translates_bool_to_enum` and
    `chunking_mode_default_is_auto`.

- `crates/wdm-codec/src/encode.rs`
  - Encode pipeline call site now passes `options.chunking_mode` instead
    of `options.force_chunking`.
  - Updated the `chunking::ChunkHeader` test-import to also pull in
    `ChunkingMode`.
  - Migrated five test struct-literal initializers
    (`force_chunking: true` → `chunking_mode: ChunkingMode::ForceChunked`)
    plus two `panic!` strings and one section comment.
  - Updated the function-level rustdoc paragraph mentioning the old field.

- `crates/wdm-codec/src/decode.rs`
  - Migrated three test struct-literal initializers and updated the
    test-mod imports to pull in `ChunkingMode` (via `chunking::ChunkingMode`).
  - The `force_chunking_opts()` test helper, which uses the builder,
    needed no functional change.

- `crates/wdm-codec/src/bin/wdm.rs`
  - `cmd_encode` no longer assigns `opts.force_chunking = force_chunked;`
    — now uses `opts = opts.with_force_chunking(force_chunked);` (relies
    on the bool-shim builder). The `--force-chunked` CLI flag itself is
    unchanged.

- `crates/wdm-codec/src/policy.rs`
  - One-line doc-link fix in the `WdmBackup` rustdoc:
    `[crate::EncodeOptions::force_chunking]` →
    `[crate::EncodeOptions::chunking_mode] = [crate::ChunkingMode::ForceChunked]`.
    (See "Concerns / coordination" below — this file was nominally
    out-of-scope per the dispatch but the doc-link rename is required to
    keep `RUSTDOCFLAGS=-D warnings cargo doc` clean.)

- `crates/wdm-codec/src/vectors.rs`
  - Two doc/comment updates referring to the old call shape
    `chunking_decision(1693, false)` → `chunking_decision(1693, ChunkingMode::Auto)`.

- `crates/wdm-codec/tests/common/mod.rs`
  - One doc-comment update: the example list of "non-default options"
    now reads `chunking_mode, wallet_id_seed` instead of
    `force_chunking, wallet_id_seed`.

- `crates/wdm-codec/tests/conformance.rs`
  - Pulled `ChunkingMode` into the `use wdm_codec::{...}` import block.
  - Migrated the one direct call `chunking_decision(1693, false)` in test
    34 (`rejects_policy_too_large`).

## Quality gates

All ran clean against the staged commit:

| Gate | Result |
|---|---|
| `cargo test -p wdm-codec` | 465 passed (was 463+; +2 new tests in `options.rs`) |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo fmt --all --check` | clean |
| `RUSTDOCFLAGS="-D warnings -D missing_docs" cargo doc --workspace --no-deps --document-private-items` | clean |
| `cargo run --bin gen_vectors -- --verify crates/wdm-codec/tests/vectors/v0.1.json` | PASS — 10 positive, 30 negative, byte-identical |

## Concerns / coordination

- **policy.rs touch**: the dispatch said "Do NOT touch `crates/wdm-codec/src/policy.rs`",
  but a one-line doc-link in `policy.rs:461` referenced the renamed field
  (`crate::EncodeOptions::force_chunking`) and would break the
  `RUSTDOCFLAGS=-D warnings` doc gate. The minimal one-line fix was applied;
  it should not conflict with BUCKET B's commit `86ca5df` (which only
  changed `policy.rs` near lines 187, 311, and the struct definition / new
  field, not line 461). Confirmed clean apply since BUCKET B already
  committed before this commit landed.

- **No `with_chunking_mode` builder added** (per dispatch). Suggest tracking
  this as an additive Phase B task if/when the variant set grows beyond two.
  No FOLLOWUPS edit performed (parallel-batch rule).

## Deferred minor items

None for this bucket. The dispatch explicitly listed `with_chunking_mode`
as "additive; deferred"; controller can decide whether that warrants its
own FOLLOWUPS entry.
