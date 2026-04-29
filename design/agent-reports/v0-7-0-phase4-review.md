# v0.7.0 Phase 4 review

**Status:** DONE_WITH_CONCERNS
**Reviewer:** Claude Opus 4.7 (1M context)
**Date:** 2026-04-29
**Commit reviewed:** `9239c79` on `feature/v0.7.0-development`
**Files reviewed:**
- `crates/md-signer-compat/Cargo.toml`
- `crates/md-signer-compat/src/lib.rs`
- `crates/md-signer-compat/src/coldcard.rs`
- `crates/md-signer-compat/src/ledger.rs`
- `crates/md-signer-compat/src/tests.rs`
- `Cargo.toml` (workspace)
- `crates/md-codec/src/bytecode/hand_ast_coverage.rs` (Phase 2 IMP-1 fix)
- Phase 2 reviewer's report at `design/agent-reports/v0-7-0-phase2-review.md`
**Role:** reviewer (Phase 4)

## Summary

**1 Important. 2 Nits. No Critical findings.** The Important is folded inline by the controller in a follow-up commit. md-signer-compat ships clean once that's resolved.

## Important

### IMP-1. COLDCARD_TAP allowlist over-permissive vs. cited source (Confidence: 90)

**Location:** `crates/md-signer-compat/src/coldcard.rs::COLDCARD_TAP.allowed_operators`.

The rustdoc cites `Coldcard/firmware` `docs/taproot.md` §"Allowed descriptors" and includes `multi_a` in the operator list. WebFetch of the cited file confirmed: it lists only `sortedmulti_a` (in 4 example shapes), never bare `multi_a`. The rustdoc enumerated allowed shapes also do not include any `multi_a`.

Either Coldcard admits `multi_a` per a different (uncited) source, or the allowlist is wider than the citation supports. **Fix folded inline by controller:** removed `multi_a` from `COLDCARD_TAP.allowed_operators`; rustdoc now explicitly notes "multi_a deliberately omitted" with rationale tying back to the cited source.

## Verification of the 6 areas requested

1. **Vendor citations.** Coldcard URL/path/shapes verified accurate except the `multi_a` discrepancy (IMP-1). Ledger `cleartext.rs` exists at the cited path; the rustdoc enumerates 7 of 16 actual variants (incomplete but operator-set sound — the omitted variants use already-listed operators).
2. **`SignerSubset` `'static` design.** Right call for v0.7. `pub const` ergonomics are the primary use case. The typo-guard's bypass via `validate_tap_leaf_subset_with_allowlist` is a clean asymmetry — the runtime path remains accessible at md-codec level for tests/dynamic callers. No Cow needed.
3. **`validate_tap_tree`.** Upstream `TapTree::leaves()` is documented DFS pre-order. `enumerate()` faithfully threads that order. The 3-leaf test `{leaf_0, {leaf_1, leaf_2}}` → leaf_1 at index 1 is correct. Docstring drift on the iterator-yield shape (claimed tuples; reality is `TapTreeIterItem` struct) — folded inline.
4. **Typo-guard quality.** `_ => panic!` arm produces an actionable diagnostic naming the offending string. Sufficient.
5. **Phase 2 IMP-1 fix.** Asymmetric `from_fn(|i| i as u8)` / `from_fn(|i| 0x80 + i)` inputs correctly defeat the symmetric-reversal bug class. Sound.
6. **Workspace patch.** `[patch]` block uses `path = "../rust-miniscript-fork"` resolved from workspace root — applies uniformly to all members. Correct.

## Nits

### N-1. `lib.rs:91` docstring drift on iterator-yield shape (Confidence: 90)

Originally said "yields `(depth, leaf_ms)` tuples" but the test calls `leaf.miniscript()` (struct accessor on `TapTreeIterItem`). **Folded inline by controller** — docstring updated to reference the actual API.

### N-2. `dummy_key_a`/`dummy_key_b` duplication (Confidence: 70)

Same key strings as `crates/md-codec/src/bytecode/hand_ast_coverage.rs`. Minor; not worth a shared test util at this scale. Tracked as defensive cleanup in FOLLOWUPS.

## FOLLOWUPS to add

1. **`v07-coldcard-multi-a-citation-gap`** — RESOLVED inline (multi_a removed from COLDCARD_TAP.allowed_operators).
2. **`v07-tap-tree-leaves-docstring-iterator-shape`** — RESOLVED inline (docstring updated).
3. **`v07-ledger-rustdoc-variant-enumeration-incomplete`** (Tier: v0.7.x) — `ledger.rs` rustdoc enumerates 7/16 vanadium variants; expand to full list or add "representative subset" framing.
4. **`v07-md-signer-compat-shared-test-key-helpers`** (Tier: v0.7.x) — `dummy_key_a`/`dummy_key_b` duplicated between md-codec hand-AST tests and md-signer-compat tests; consider a shared test-only helper module.

## Verdict

Phase 4 met its acceptance criteria functionally (6 tests added, all passing). The Important finding (IMP-1) is folded inline. md-signer-compat ships clean with the fix applied. Controller proceeds to Phase 5.
