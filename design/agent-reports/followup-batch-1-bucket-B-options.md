# Agent Report: followup-batch-1-bucket-B-options

**Status:** Complete
**Commit:** a74e21b
**File(s):** `crates/wdm-codec/src/options.rs`
**Role:** implementer

## Closed short-ids

- `6c-encode-options-builder`

## Summary

Added three fluent builder methods to `EncodeOptions` — `with_force_chunking`, `with_force_long_code`, and `with_seed` — plus two new unit tests covering the full chain and passthrough cases.

`WalletIdSeed` was already imported via `use crate::wallet_id::WalletIdSeed;` on line 3; no import changes were needed.

## Gate results

- `cargo test -p wdm-codec` — all tests pass, including both new tests (`encode_options_builder_chain`, `encode_options_builder_default_passthrough`)
- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `cargo fmt --check` — clean
- `RUSTDOCFLAGS="-D warnings -D missing_docs" cargo doc` — clean (all three builder methods carry `///` doc comments)
