# v0.5 Phase 4 — Implementer Report

**Status**: DONE_WITH_CONCERNS
**Date**: 2026-04-28
**Commits**: `bca2804` (Phase 4 + stale-strings sweep), `766c580` (FOLLOWUPS.md move-to-resolved)
**Branch**: `feature/v0.5-multi-leaf-taptree`
**Worktree**: `/scratch/code/shibboleth/descriptor-mnemonic-v0.5`

`DONE_WITH_CONCERNS` rather than `DONE` because the plan + spec contained a **bug in the encoder helper's entry-call argument** (`target_depth=1` instead of `target_depth=0`) that I had to fix to make the work correct. The bug is documented inline below and in the Phase 4 commit message; the fix is locally scoped to one literal in the call site. The plan/spec text should likely be patched in Phase 9 cumulative review or Phase 6 spec touch-up.

## Summary of work done

### Task 4.1 — Audit v0.4 single-leaf-non-zero-depth rejection
- One hit in `encode.rs:148` (the rejection itself).
- **Zero** test hits for the rejection text in `crates/md-codec/tests/`. Confirmed by `grep -rn 'single-leaf TapTree must have depth 0' crates/md-codec/tests/` returning empty.
- The rejection was removed in Task 4.3 (subsumed by the multi-leaf path).

### Task 4.2 — Add `encode_tap_subtree` helper
- Inserted after `validate_tap_leaf_terminal` at `encode.rs:520`.
- All required imports (`Arc`, `HashMap`, `Miniscript`, `Tap`, `Tag`, `DescriptorPublicKey`) were already in scope at the top of the file; no new `use` statements needed.
- Helper signature and body match the spec §4 verbatim.

### Task 4.3 — Replace the `Descriptor::Tr` arm
- Replaced lines 126-158 with the multi-leaf dispatch arm per plan.
- KeyOnly (`tr(KEY)`) and single-leaf (`tr(KEY, leaf)` with depth==0) paths preserved byte-identically. Verified by running all 6+ existing `tr(...)` round-trip tests in `crates/md-codec/tests/taproot.rs` and the pinned conformance fixtures — all pass after the rewrite.
- The v0.4 single-leaf-with-non-zero-depth `PolicyScopeViolation` rejection is now gone (subsumed by the multi-leaf path).
- `debug_assert_eq!(cursor, leaves.len())` post-condition catches off-by-N errors in the helper recursion.

### Task 4.4 — Un-ignore the multi-leaf round-trip test
- Removed the `#[ignore = "..."]` attribute from `multi_leaf_two_leaf_symmetric_round_trips` in `crates/md-codec/tests/v0_5_type_wiring.rs:121`.
- Per the plan, replaced the `tap_leaves` content assertions with a `// TODO Phase 5 Task 5.2: ...` comment + `let _ = decoded.report.tap_leaves;` placeholder. The round-trip itself (encode → decode without error) IS asserted; the report-vector contents are deferred to Phase 5.

### Task 4.5 — Commit
- Single Phase 4 commit at `bca2804`.
- Follow-up commit `766c580` moves the resolved FOLLOWUP entry from "Open items" to "Resolved items" with the canonical `Status: resolved bca2804` format.

## Deviations from plan

### Deviation 1 (substantive): entry call uses `target_depth=0`, not `target_depth=1`

**Plan text** (line 1325 of `IMPLEMENTATION_PLAN_v0_5_multi_leaf_taptree.md`) and **spec text** (line 220 of `SPEC_v0_5_multi_leaf_taptree.md`) both call:

```rust
encode_tap_subtree(&leaves, &mut cursor, 1, out, placeholder_map)?;
```

This is wrong. With a 2-leaf depth-1 tree (`leaves = [(1, ms_a), (1, ms_b)]`):
- Iteration 1: `cursor=0`, `leaf_depth=1`, `target_depth=1` ⇒ match the `==` arm, encode leaf 0, `cursor=1`, return.
- Function exits without consuming leaf 1.

The `debug_assert_eq!(cursor, leaves.len())` post-condition catches it: `assertion left == right failed: 1 vs 2`.

**Correct entry**: `target_depth=0`. With that:
- `cursor=0`, `leaf_depth=1 > 0` ⇒ emit `0x08`, recurse twice with `target_depth=1`.
- Recursion 1: `cursor=0, td=1` ⇒ encode leaf 0, `cursor=1`.
- Recursion 2: `cursor=1, td=1` ⇒ encode leaf 1, `cursor=2`.
- Output: `[0x08][LEAF0][LEAF1]`. Matches spec §2 example at line 60-65 verbatim.

I traced the asymmetric `[1, 2, 2]` case as well; with `target_depth=0` it produces `[0x08][LEAF0][0x08][LEAF1][LEAF2]`, matching spec §2 example at line 67-74.

**Recommended action**: Phase 9 reviewer (or Phase 6 spec touch-up) should update the SPEC and PLAN literal `1` → `0` at the entry call sites. The helper body itself is correct as written.

The Phase 4 commit message documents the deviation explicitly so a future reader of git log sees it.

### Deviation 2 (scope expansion): folded BOTH the encode.rs AND decode.rs portions of the stale-strings sweep into Phase 4

The Phase 4 prompt said the encode.rs portion was in scope and the decode.rs portion was optional ("if convenient since Phase 4 reads decode.rs anyway, OR defer to Phase 9"). I folded in the decode.rs portion as well because:
1. The decode.rs module-doc and `decode_tr_inner` doc actively contradicted v0.5 shipped behavior — these are release-blockers per the FOLLOWUP entry's own tier annotation.
2. Touching them is one-line-each Edit operations, no risk surface.
3. Leaves the FOLLOWUP fully resolved in one commit instead of split.

Verified post-commit: `grep -rn 'v0\.4 does not support\|reserved for v1\+' encode.rs decode.rs` returns zero hits.

## FOLLOWUPS

- **Closed**: `v0-5-stale-v0-4-message-strings-sweep` — fully resolved. Entry moved from "Open items" to "Resolved items" with `Status: resolved bca2804`. Commit `766c580` records the move.

- **Filed**: none (no new deferred items surfaced during Phase 4).

## Self-review

- `cargo test --workspace --no-fail-fast` — **617 passed; 0 failed; 0 ignored** (was 616 + 1 ignored at HEAD `b843f29`; net +1 from un-ignoring the multi-leaf round-trip test that now passes end-to-end).
- `cargo fmt --check` — exit 0, no diff.
- `cargo clippy --workspace --all-targets -- -D warnings` — exit 0, no warnings.
- `cargo build --workspace --all-targets` — clean.

## Critical correctness checks (per prompt)

- [x] **Byte-identical preservation for single-leaf and KeyOnly**: confirmed. All existing taproot tests in `crates/md-codec/tests/taproot.rs` (6+ tests covering `tr(KEY)`, `tr(KEY, pk(@1))`, `tr(KEY, multi_a(...))`, etc.) pass without modification, and the conformance + corpus tests against pinned `v0.2.json` SHAs continue to match.
- [x] **Multi-leaf detection**: `leaves.len() == 1 && leaves[0].0 == 0` is the single-leaf carve-out. Anything else (depth != 0 or N > 1) routes through `encode_tap_subtree`.
- [x] **No defensive depth check in encoder**: confirmed; helper has no depth-128 check. Comment in the call site notes the reliance on `TapTree::combine`'s upstream invariant.

## Files modified

- `crates/md-codec/src/bytecode/encode.rs` — Tr arm rewrite + `encode_tap_subtree` helper + 3 stale-string updates + module-doc rewrite
- `crates/md-codec/src/bytecode/decode.rs` — 1 stale-string update (decode_sh_inner) + module-doc rewrite + decode_tr_inner doc rewrite
- `crates/md-codec/tests/v0_5_type_wiring.rs` — un-ignored `multi_leaf_two_leaf_symmetric_round_trips`, deferred `tap_leaves` assertions to Phase 5 with TODO
- `design/FOLLOWUPS.md` — moved `v0-5-stale-v0-4-message-strings-sweep` from Open to Resolved

## Final commits (Phase 4 boundary)

```
766c580 followup(v0.5): move stale-v0.4-strings sweep to resolved (bca2804)
bca2804 feat(v0.5 phase 4): encoder multi-leaf TapTree dispatch
b843f29 followup(v0.5): file stale v0.4 message-strings sweep entry  ← Phase 3 boundary
```

Phase 5 (`tap_leaves` population) is the next track.
