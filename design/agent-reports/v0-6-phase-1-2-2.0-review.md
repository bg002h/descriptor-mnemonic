# v0.6 Phases 1, 2.0, 2 — Code review

**Status:** DONE_WITH_CONCERNS
**Commit:** `479d5f6` (Phase 1), `b12308f` (Phase 2.0), `18fc354` (Phase 2)
**File(s):**
- `crates/md-codec/src/bytecode/tag.rs`
- `crates/md-codec/src/error.rs`
- `crates/md-codec/src/bytecode/encode.rs`
- `crates/md-codec/tests/error_coverage.rs`
**Role:** reviewer (code-quality)

## What was reviewed

Verified the 17 acceptance checks across Phase 1 (Tag enum rework), Phase 2.0 (BytecodeErrorKind::TagInvalidContext), and Phase 2 (encoder strip + exhaustive Tap Terminal match). Cross-checked every Tag variant byte value against spec §2.2 by hand; counted Tap Terminal arms; verified `validate_tap_leaf_subset` body untouched; verified hash terminal byte-order encoding; verified the catch-all uses `TapLeafSubsetViolation` (Phase 4 will rename).

## Phase 1 (tag.rs) — all checks pass

- **Check 1 (layout byte-for-byte):** PASS. All 39 variants cross-checked against spec §2.2. Multisig contiguous at 0x08-0x0B, wrappers at 0x0C-0x12, logical at 0x13-0x1A, framing at 0x33-0x35.
- **Check 2 (`from_byte` exhaustiveness):** PASS. Allocated 0x00-0x23 + 0x33-0x35 each return `Some`; unallocated 0x24-0x32 + 0x36-0xFF each return `None`.
- **Check 3 (test coverage):** PASS (over-delivers — 14 tests vs 13 prompted; over-delivery is fine).
- **Check 4 (dropped variants absent):** PASS.
- **Check 5 (rustdoc accuracy):** PASS.

## Phase 2.0 (error.rs) — all checks pass

- **Check 6 (variant placement):** PASS. After `UnexpectedTag` is acceptable structural-rejection grouping.
- **Check 7 (Display impl):** PASS. `tag {tag:#04x} is invalid in context {context}` format matches spec.
- **Check 8 (no mirror update needed):** PASS. `INVALID_BYTECODE_PREFIX` wildcard covers all sub-variants.

## Phase 2 (encode.rs) — passes with two cosmetic concerns

- **Check 9 (default validator calls removed):** PASS.
- **Check 10 (Tap Terminal exhaustive 30 arms):** PASS. All 30 Terminal variants present.
- **Check 11 (Multi/SortedMulti tap-illegal comments):** PASS.
- **Check 12 (SortedMultiA encoding shape):** PASS. Functionally symmetric byte-wise with MultiA.
- **Check 13 (hash terminal byte order):** PASS. All four use `as_byte_array()` directly.
- **Check 14 (`validate_tap_leaf_subset` body unchanged):** PASS.
- **Check 15 (catch-all wildcard):** PASS. Uses pre-rename `TapLeafSubsetViolation` as expected.
- **Check 16 (compile status):** PASS within scope. Encoder file itself clean of dropped Tag references.

## Concerns

### C1 (cosmetic, non-blocking): `terminal_to_tag` stale re: SortedMultiA

**Location:** `encode.rs:728-730` (rustdoc), `encode.rs:782-783` (function body), `encode.rs:2059-2066` (test).

With `Tag::SortedMultiA = 0x0B` allocated in Phase 1, the encoder's `Terminal::SortedMultiA` arm correctly emits the new tag. However, the helper `terminal_to_tag` still returns `None` for `Terminal::SortedMultiA`, and its rustdoc still claims "the wire format does not encode `sortedmulti_a` — it would require a future Tag allocation". The fall-through to manual match in `tap_terminal_name` produces the right diagnostic, so behavior is correct, but the test `tap_terminal_name_delegates_to_tag_to_bip388_name` locks `terminal_to_tag(&sma).is_none()` which is now the wrong invariant.

**Suggested fix:** update `terminal_to_tag` to map `Terminal::SortedMultiA(_) => Tag::SortedMultiA`, drop the SortedMultiA arm in `tap_terminal_name`, drop the `is_none()` assertion in the test, revise the rustdoc.

### C2 (cosmetic, non-blocking): `tap_terminal_name` rustdoc not updated per plan Task 2.5

**Location:** `encode.rs:723-742`.

Plan Task 2.5 prescribed updating `tap_terminal_name` rustdoc to clarify v0.6 narrowing ("no longer the universal naming hook for tap-context errors — only used by the explicit-call validator path"). Current rustdoc retains v0.5 framing.

## No Phase 3 blockers

Phase 3 (decoder strip) can proceed. C1 and C2 are encoder helpers; Phase 3 is independent.

## Status: DONE_WITH_CONCERNS

All 17 verification checks pass. Two cosmetic concerns flag stale rustdoc/test invariant drift around `Terminal::SortedMultiA`.
