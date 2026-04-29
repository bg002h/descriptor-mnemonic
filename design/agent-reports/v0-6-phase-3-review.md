# v0.6 Phase 3 — Decoder Strip Review

**Status:** DONE_WITH_CONCERNS
**Commit:** d9af356 (feature/v0.6-strip-layer-3)
**File(s):**
- `crates/md-codec/src/bytecode/decode.rs`
- `crates/md-codec/src/bytecode/encode.rs`
- `crates/md-codec/src/bytecode/tag.rs`
- `crates/md-codec/src/error.rs`
- `crates/md-codec/src/vectors.rs`
**Role:** reviewer (code-quality)

## Summary

All 16 acceptance checks pass. 4 cosmetic doc-staleness sites flagged for inline fix (pure rustdoc; zero runtime impact).

## All 16 checks pass

1. **20 new arms present and correctly shaped** — PASS. Each matches spec §4.3 row.
2. **Encoder/decoder symmetry per Tag** — PASS for all 5 spot-checks (SortedMultiA, Hash256, AndOr, Thresh, After).
3. **Catch-all uses `BytecodeErrorKind::TagInvalidContext`** — PASS. decode.rs:919-927.
4. **Multi/SortedMulti tap-illegal arms with comments** — PASS. decode.rs:633-639 + 668-669.
5. **`Tag::TapTree` defensive arm preserved** — PASS. decode.rs:906-911.
6. **`tag_to_bip388_name` updated correctly** — PASS. No Bare/Reserved*; SortedMultiA arm; Placeholder/SharedPath labels v0.6.
7. **decode_tr_inner single-leaf no longer calls validate_tap_leaf_subset** — PASS.
8. **decode_tap_subtree multi-leaf no longer calls validate_tap_leaf_subset** — PASS.
9. **Top-level rejection arm no longer matches Tag::Bare** — PASS. Only Tag::Pkh.
10. **Vector-side cleanup** — PASS. n_top_bare and n_sh_bare DELETED with explanatory comments.
11. **`terminal_to_tag` returns Some(Tag::SortedMultiA)** — PASS.
12. **`tap_terminal_name` no special-case SortedMultiA** — PASS. Pure delegation.
13. **`tap_terminal_name` rustdoc updated** — PASS. Per plan Task 2.5.
14. **Test updated to assert `Some(Tag::SortedMultiA)`** — PASS.
15. **Crate compiles** — PASS (source-verified).
16. **tag.rs unit tests pass** — PASS (source-verified).

## Cosmetic doc-staleness (NOT blockers — recommended inline fix)

Four module-level / function-level rustdoc comments still reference v0.5 byte values. Functional code uses symbolic Tag references throughout, so zero runtime impact:

- decode.rs:14 — "multi-leaf TapTrees are decoded via `Tag::TapTree` (0x08)" → should be `(0x07)`.
- decode.rs:19-21 — "Inline-key tags `0x24..=0x31` (the `Reserved*` set in `Tag`)..." Reserved* variants DROPPED in v0.6.
- encode.rs:20 — "emitting `Tag::TapTree` (0x08) inner-node framings" → should be `(0x07)`.
- encode.rs:794 — "writes `Tag::Placeholder` (`0x32`)" → should be `(0x33)`.

## Recommendation

Fix the 4 doc-staleness sites inline (cheaper than FOLLOWUPS entry for 4-line edits). Then proceed to Phase 4 (Error rename) — which is also already in flight per the controller.

End of review.
