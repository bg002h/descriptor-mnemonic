# v0.5 Phase 5 — Implementer Report

**Status**: DONE
**Date**: 2026-04-28
**Commit**: `3097c99`
**Branch**: `feature/v0.5-multi-leaf-taptree`
**Worktree**: `/scratch/code/shibboleth/descriptor-mnemonic-v0.5`

## Summary of work done

### Task 5.1 — Add `build_tap_leaves` helper and wire at report-construction site

**Files modified**:
- `crates/md-codec/src/decode.rs`

**API adaptation required**: The plan's test pseudocode called `md_codec::policy::WalletPolicy::from_descriptor_str`, `md_codec::encode_policy`, and `md_codec::decode_string` — none of which exist in the public API. The new tests were rewritten to use the actual API (`policy_str.parse::<WalletPolicy>()`, `encode(&policy, &EncodeOptions::default())`, `md_codec::decode(&strings, &DecodeOptions::new())`), matching the pattern of the existing `multi_leaf_two_leaf_symmetric_round_trips` test.

**Implementation approach**: The plan's `build_tap_leaves(desc: &Descriptor<DescriptorPublicKey>)` helper was added to `decode.rs` after the `decode()` function. At the `DecodeReport` construction site, the `Descriptor` is obtained via `policy.inner().clone().into_descriptor()`. This works because `WalletPolicy::from_bytecode_with_fingerprints` constructs the inner `InnerWalletPolicy` via `from_descriptor(&descriptor)` (using 32 dummy keys), which stores those keys as `key_info`. Calling `into_descriptor()` on the clone translates the template back to a full descriptor using those same dummy keys. The tap_tree structure is faithfully preserved regardless of which concrete keys are used.

**Imports added** to `decode.rs`:
- `miniscript::Descriptor`
- `miniscript::descriptor::DescriptorPublicKey`
- `crate::TapLeafReport`

**`build_tap_leaves` behavior**:
- Non-`tr` descriptors (`wsh`, `wpkh`, etc.): returns `vec![]`
- KeyOnly `tr(KEY)` with `tap_tree = None`: returns `vec![]`
- Single-leaf `tr(KEY, leaf)`: returns 1 entry with `leaf_index=0, depth=0`
- Multi-leaf `tr(KEY, TREE)`: returns N entries in DFS pre-order with sequential `leaf_index` values and depths matching the tree shape

**`into_descriptor()` fallibility**: wrapped with `.map(...).unwrap_or_default()` so any future edge-case failure silently degrades to empty `tap_leaves` rather than panicking.

### Task 5.2 — Restore `multi_leaf_two_leaf_symmetric_round_trips` tap_leaves assertions

The deferred placeholder in `v0_5_type_wiring.rs`:
```rust
// TODO Phase 5 Task 5.2: ...
let _ = decoded.report.tap_leaves;
```

Replaced with the full assertions from the plan:
```rust
assert_eq!(decoded.report.tap_leaves.len(), 2);
assert_eq!(decoded.report.tap_leaves[0].leaf_index, 0);
assert_eq!(decoded.report.tap_leaves[0].depth, 1);
assert_eq!(decoded.report.tap_leaves[1].leaf_index, 1);
assert_eq!(decoded.report.tap_leaves[1].depth, 1);
```

All pass (symmetric 2-leaf tree at depth 1 for both leaves).

### Task 5.3 — Commit Phase 5

Committed as `3097c99` with message matching the plan template.

## Gate results

| Gate | Result |
|------|--------|
| `cargo test --workspace --no-fail-fast` | 619 passed, 0 failed, 0 ignored |
| `cargo fmt --check` | clean (no output) |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean (no output) |

**Test count delta**: 617 (Phase 4) → 619 (Phase 5). +2 new tests (`keyonly_tr_produces_empty_tap_leaves`, `single_leaf_tr_produces_one_tap_leaf_at_depth_zero`).

## Concerns / deviations

**No blocking concerns.** One minor adaptation:

- **Plan test pseudocode used non-existent APIs**: `from_descriptor_str`, `encode_policy`, and `decode_string` (the last exists but returns `DecodedString`, not `DecodeResult`). Adapted to use the real public API. The adaptation does not change the semantic coverage of the tests — they exercise the same encode→decode→tap_leaves round-trip path.

## FOLLOWUPS.md actions

None required. No new deferred items introduced.
