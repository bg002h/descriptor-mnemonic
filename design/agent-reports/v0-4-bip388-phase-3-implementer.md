# Phase 3 — Default Path-Tier Selector: Implementer Report

**Agent:** Claude Sonnet 4.6  
**Date:** 2026-04-27  
**Status:** DONE — all gates green, all 7 tests pass  

---

## Summary

Added `default_path_for_v0_4_types` helper to `crates/md-codec/src/policy.rs` and wired it into the existing `or_else()` fall-through chain as Tier 3 (between Tier 1 origin-extracted and the Tier 4 BIP 84 fallback).

## Chain shape — compatible

`policy.rs:390-397` uses nested `unwrap_or_else()` calls (not an `if let` chain or `IndicatorOrPath` enum). The new tier was inserted cleanly as an additional `.unwrap_or_else()` layer. No structural incompatibility; no DONE_WITH_CONCERNS needed.

## Architecture note resolved

The spec's `§3` doc showed `default_indicator_for_v0_4_types` returning `Option<u8>`, but the implementation instructions correctly specified `Option<DerivationPath>` to match the chain shape. Implementation follows the instructions.

## Tests (7 total)

- `wpkh_default_tier_is_bip84` — PASSES (BIP 84 indicator 0x03)
- `sh_wpkh_default_tier_is_bip49` — was FAILING, now PASSES (0x02)
- `sh_wsh_default_tier_is_bip48_nested` — was FAILING, now PASSES (0x06)
- `wpkh_path_override_wins` — PASSES (Tier 0 wins)
- `sh_wpkh_path_override_wins` — PASSES
- `sh_wsh_path_override_wins` — PASSES
- `wsh_no_origin_default_unchanged_from_v0_3` — PASSES (regression pin)

Note: `wpkh_default_tier_is_bip84` passed before implementation because wpkh BIP 84 = existing BIP 84 fallback. The real new work was `sh_wpkh` and `sh_wsh`.

## Gates

| Gate | Result |
|---|---|
| `cargo build --workspace --all-targets` | PASS |
| `cargo test --workspace` | PASS (all prior tests still pass) |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo fmt --all -- --check` | PASS (use reorder needed) |

## Commit

`b700a88` — `feat(policy): scoped default-tier selector for v0.4 top-level types`  
Pushed to `origin/feature/v0.4-bip388-modern-surface`.

## Concerns

None. Implementation is clean and scoped correctly.
