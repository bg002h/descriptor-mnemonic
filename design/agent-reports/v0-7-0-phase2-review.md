# v0.7.0 Phase 2 review

**Status:** DONE_WITH_CONCERNS
**Reviewer:** Claude Opus 4.7 (1M context)
**Date:** 2026-04-28
**Commit reviewed:** `56d7857` on `feature/v0.7.0-development`
**Files reviewed:**
- `crates/md-codec/src/bytecode/hand_ast_coverage.rs` (NEW; 12 tests)
- `crates/md-codec/src/bytecode/mod.rs` (`#[cfg(test)] mod hand_ast_coverage;`)
- `crates/md-codec/src/bytecode/encode.rs` (`pub(crate)` exposure)
- `crates/md-codec/src/bytecode/decode.rs` (`pub(crate)` exposure of helpers)
- `design/SPEC_v0_7_0.md` §3.1–§3.3
- `design/IMPLEMENTATION_PLAN_v0_7_0.md` §2 (Phase 2)
- `design/agent-reports/v0-7-0-plan-review-1.md` (Concern 5 source)
**Role:** reviewer (Phase 2)

## Summary

**No Critical findings. 1 Important. 2 Nits.** Phase 2 is acceptable to ship after the Important finding (palindromic byte-order test inputs) is folded inline. The plan-deviation (in-source `#[cfg(test)]` module instead of `tests/hand_ast_coverage.rs`) is sound and well-justified. The 12 tests cover §3.1–§3.3 of the spec with genuine coverage. The `t_or_c_tap_leaf_round_trips` test does exercise the `Terminal::OrC` decode arm end-to-end. SortedMultiA distinct-variant decoding is verified against the pinned upstream fork. `pub(crate)` exposure is minimal.

## Important

### IMP-1. Palindromic byte-order test inputs defeat the decode-direction asymmetric-reversal check (Confidence: 95)

**Location:** `crates/md-codec/src/bytecode/hand_ast_coverage.rs` lines 264-265 and the four hash sub-cases at lines 270-327.

The test docstring explicitly cites Plan reviewer #1 Concern 5: a single-direction encode-only check would miss the "encode reverses, decode reverses back" bug class. The test reads back the decoded hash bytes and asserts `h.as_byte_array() == &known_32`.

**The chosen inputs `[0xAA; 32]` and `[0xBB; 20]` are byte-wise palindromic — every byte is identical, so reversing the array is a no-op.** A bug where both encoder and decoder reversed the bytes (the exact asymmetric-reversal bug class the test is supposed to catch) would still produce the assertion `h.as_byte_array() == &[0xAA; 32]` after the round-trip. The test, as written, is observationally indistinguishable from a single-direction check.

**Fix (folded inline by controller):** replace the constant fills with strictly increasing patterns:

```rust
let known_32: [u8; 32] = std::array::from_fn(|i| i as u8);  // [0x00, 0x01, ..., 0x1F]
let known_20: [u8; 20] = std::array::from_fn(|i| 0x80 + i as u8);  // [0x80, 0x81, ..., 0x93]
```

With those inputs, an erroneous byte reversal in either direction produces a distinguishable failure. **Note:** the controller folded this fix in the same commit that lands Phase 4 (md-signer-compat).

## Nits

### N-1. `decoder_arm_*` cursor-consumption assertion is weaker than its sibling tests (Confidence: 85)

All six `decoder_arm_*` tests assert `cur.is_empty()` after decode but the wire bytes contain no trailing sentinel. Stronger pattern: append a `0xFF` sentinel byte and assert exactly `[0xFF]` remains unconsumed. Requires adding a `pub(crate) fn remaining(&self) -> &[u8]` accessor to `Cursor` (~3 lines in `cursor.rs`).

### N-2. `or_c_unwrapped_tap_leaf_byte_form` does not verify the "decoder rejects" branch its docstring contemplates (Confidence: 85)

The docstring describes a two-branch policy ("If decoder accepts... If decoder rejects, this test asserts the rejection diagnostic") but the test only asserts encoder wire bytes — the decoder is never run on `out`. Either tighten the docstring to "encoder wire-form pin only" or extend the test to also run the decoder.

## Verification of the 6 areas requested

| Area | Verdict |
|---|---|
| 1. Test-intent fidelity vs spec §3.1–§3.3 | All 12 tests present; 11 fully cover, 1 (byte-order pin) covered partially per IMP-1 |
| 2. `t_or_c_tap_leaf_round_trips` end-to-end OrC decode | YES — verified the decode flow: AndV→OrC dispatch through `decode_tap_terminal`, `Terminal::OrC` constructed via `Miniscript::from_ast` |
| 3. Hash byte-order pin's decode-direction half | PARTIAL — see IMP-1; palindromic inputs undermine the stated goal |
| 4. `pub(crate)` exposure is minimal | MINIMAL. `decode_tap_terminal` and `decode_tap_miniscript` could tighten to `pub(super)` (defensive nit, not blocking). `EncodeTemplate` was already `pub(crate)` per the file's pattern. |
| 5. SortedMultiA distinct variant verified against pinned fork | YES — verified `apoelstra/rust-miniscript@f7f1689b` exports `Terminal::SortedMultiA` as distinct from `MultiA`. Compile-time pattern match guards against silent drift. |
| 6. Thresh wire form correctness | YES — `[k=2, n=3, c:pk_k(@0), s:c:pk_k(@1), s:c:pk_k(@2)]` correctly represents the encoder output; children typecheck (B-type at pos 0, W-type via `s:` swap at pos 1..N) |

## Plan-deviation evaluation

In-source `#[cfg(test)]` module instead of `tests/hand_ast_coverage.rs`: SOUND. Tests are runnable via `cargo test`; equivalent coverage; plan intent ("keep defensive tests separate from corpus-driven tests in `taproot.rs`") is satisfied; API surface stays tight (`pub(crate)` instead of full `pub trait`). Trade-off favorable.

## FOLLOWUPS to add

1. **`v07-phase2-asymmetric-byte-order-test-inputs`** (Tier: v0.7-blocker) — RESOLVED inline with Phase 4 commit (palindromic → asymmetric byte sequences).
2. **`v07-phase2-decoder-arm-cursor-sentinel-pattern`** (Tier: v0.7.x defensive cleanup) — add `Cursor::remaining()` accessor and trailing sentinel pattern to all six `decoder_arm_*` tests.
3. **`v07-phase2-or-c-unwrapped-test-docstring-drift`** (Tier: v0.7.x) — either tighten docstring or extend test to cover both decoder-behavior branches.
4. **`v07-phase2-decode-helpers-pub-super-tightening`** (Tier: v0.8 housekeeping) — `decode_tap_miniscript` / `decode_tap_terminal` can tighten to `pub(super)`.

## Verdict

Phase 2 met its acceptance criteria functionally (12 tests added, all passing). The plan-deviation is sound. **IMP-1 is a real coverage gap**; the controller folded the fix in the Phase 4 commit. Phase 2 closes clean once that fix lands.
