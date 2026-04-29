# v0.7.0 Phase 1 review

**Status:** DONE_WITH_CONCERNS
**Reviewer:** Claude Opus 4.7 (1M context)
**Date:** 2026-04-28
**Commits reviewed:** `35caa24`, `d7de42d`, `de63db3` on `feature/v0.7.0-development`
**Files reviewed:**
- `crates/md-codec/src/bytecode/decode.rs`
- `crates/md-codec/src/bytecode/encode.rs`
- `crates/md-codec/src/vectors.rs`
- `crates/md-codec/tests/taproot.rs`
- `crates/md-codec/tests/fingerprints.rs`
- `crates/md-codec/tests/conformance.rs`
- `crates/md-codec/tests/v0_5_taptree_roundtrip.rs`
- `crates/md-codec/tests/vectors_schema.rs`
- `design/agent-reports/v0-7-0-phase1-decode-encode-path-rebaseline.md`
- `design/agent-reports/v0-7-0-plan-review-1.md`
- `design/IMPLEMENTATION_PLAN_v0_7_0.md`
- `design/SPEC_v0_6_strip_layer_3.md`
**Role:** reviewer (Phase 1)

## Summary

**No Critical findings. 1 Important. 4 Nits.** Phase 1 is acceptable to ship. The Important finding is a coverage-attribution gap that needs a Phase 4 commitment (FOLLOWUPS entry). The Nits are stale inline byte-annotation comments and one mislabelled test that all pass mechanically but are confusing for future readers.

## Important

### IMP-1. `tap_leaf_subset_violation_carries_leaf_index` deletion: coverage commitment unclear (Confidence: 85)

Location: `crates/md-codec/tests/v0_5_taptree_roundtrip.rs:117-122` (in-place comment block); the deleted test was the LI2 multi-leaf leaf-index attribution check.

The agent report and the in-place comment claim "leaf-index attribution moves to md-signer-compat (v0.7+) which calls `validate_tap_leaf_subset` per leaf." That's a forward commitment to Phase 4, but Phase 4's plan (Plan §4.5 / Spec §4.6 test list) does not commit to a multi-leaf DFS-pre-order test. The five enumerated tests are:

1. `coldcard_admits_documented_pk_shape`
2. `coldcard_rejects_thresh_with_operator_name` — checks `leaf_index == Some(2)` for a single-leaf shape
3. `ledger_admits_relative_timelock_multisig_shape`
4. `ledger_rejects_sha256` — checks `leaf_index == Some(1)` for single-leaf
5. `allowlist_entries_are_recognized_by_naming_hook`

None exercise the multi-leaf DFS-pre-order assignment LI2 was guarding (LI2 fed a multi-leaf TapTree and checked the offending operator's `leaf_index` matched the leaf's ordinal in pre-order). The single-leaf cases use synthetic constants supplied by the caller, not derived from any DFS walk. The iteration primitive that would produce a real `leaf_index` from a tap tree (a `tap_leaves` walker + per-leaf `validate(...)` call) is not specified anywhere in Phase 4 — Plan §4.2.1's example doc-comment says "(Exact API for 'iterate tap leaves' refined during implementation.)"

So the coverage claim "moved to md-signer-compat" is half-true: the operator-name attribution moves, but the DFS pre-order leaf-index correctness has no committed home. If the iterator helper is never written, the project loses LI2's coverage silently.

**Fix:** Append a FOLLOWUPS entry `v07-tap-leaf-iterator-with-index-coverage` (tier: v0.7-blocker) requiring at least one multi-leaf test in `md-signer-compat/src/tests.rs` whose `leaf_index` is *derived from the iterator*, not supplied by the test body.

## Nits

### N-1. `decode_rejects_sh_bare` test name + inline comment have drifted (Confidence: 90)

Location: `crates/md-codec/src/bytecode/decode.rs:2413-2423`.

The test still passes (because `decode_sh_inner` returns `PolicyScopeViolation` for any non-`Wpkh`/non-`Wsh` inner tag, and the assertion is `msg.contains("sh(")` — generic). But in v0.6 byte 0x07 is `Tag::TapTree`, not `Tag::Bare`. The test currently exercises "Sh→TapTree is rejected" while claiming to test "Sh→Bare". This is the structural twin of `decode_rejects_top_bare_legacy` which was deleted; consistency would either delete this one too or rename it `decode_rejects_sh_taptree`.

### N-2. `taptree_at_top_level_produces_specific_diagnostic` adds `(0x07)` to a production diagnostic — sustainable but creates a drift liability (Confidence: 85)

Location: `crates/md-codec/src/bytecode/decode.rs:78-82`.

The Phase 1 implementer added `(0x07)` to the user-facing error message in `decode_descriptor`'s `Tag::TapTree` arm to satisfy a test that asserts the byte is named in the diagnostic. Test-driven and useful for non-Rust consumers debugging raw bytecode hex. **However**, hardcoding the byte in a string is a Tag-byte-rolling liability: if a future major release re-numbers TapTree, both production string AND test pin must update in lockstep. Cleaner pattern: `format!("TapTree (0x{:02X}) ...", Tag::TapTree.as_byte())`.

### N-3. Stale inline byte annotations in `conformance.rs` / `fingerprints.rs` (Confidence: 90)

Several integration tests use symbolic `Tag::Foo.as_byte()` (correct) but trail with stale `// 0x32` / `// 0x19` / `// 0x33` / `// 0x05` comments naming v0.5 byte values. Examples:

- `tests/fingerprints.rs:175` — `Tag::Placeholder.as_byte(), // 0x32` (v0.6 is 0x33)
- `tests/conformance.rs:765, 768, 771, 773, 1083, 650` — multiple byte-value annotations naming v0.5 bytes

Each test functions correctly because the actual code uses symbolic refs. Annotation cleanup is mechanical sed work.

### N-4. `n_taptree_at_top_level` description string still says "0x08" (Confidence: 90)

Location: `crates/md-codec/src/vectors.rs:1867-1895`.

Both the in-source comment and the public-facing `description` field of the negative vector say "0x08" — v0.5 byte. The description ships in `tests/vectors/v0.2.json` and is part of the v0.2 schema-2 SHA pin; updating requires regenerating the SHA. This is a Phase 6 release-prep concern.

## Verification of the 5 high-risk areas requested

| Area | Finding |
|---|---|
| 1. Test-intent preservation (subset rewrites) | PASS. `taproot_rejects_wrapper_alt_outside_subset` still uses a `thresh(2, c:pk_k, sc:pk_k, sc:pk_k)` shape with the same pre/post-condition. The shift from `to_bytecode` → `validate_tap_leaf_subset` is forced by the v0.6 Layer-3 strip and preserves the bug-class coverage. |
| 2. Deletion #1 (`decode_rejects_top_bare_legacy`) | PASS. `Tag::Bare` is gone; byte 0x07 is `Tag::TapTree`; `taptree_at_top_level_produces_specific_diagnostic` covers the equivalent rejection. Genuine redundancy. |
| 2b. Deletion #2 (`tap_leaf_subset_violation_carries_leaf_index`) | CONCERNED — see IMP-1. |
| 3. `(0x07)` diagnostic-string addition | PASS with N-2 nit. |
| 4. `n_top_bare`/`n_sh_bare` removal | PASS. `n_top_bare` re-pointed to `n_taptree_at_top_level`. `n_sh_bare` claimed as covered by `n_sh_inner_script` + `n_sh_key_slot`; verified. |
| 5. BIP-example hex `0434033502deadbeefcafebabe0508020233003301` | PASS. Walked byte-by-byte against SPEC v0.6 §2.3: `04`=hdr, `34 03`=SharedPath(0x34)+BIP84, `35 02`=Fingerprints+count, 8B fps, `05 08`=Wsh+Multi(0x08-was-0x19), `02 02`=k/n, `33 00`/`33 01`=Placeholder(0x33-was-0x32)+idx. Every tag at its v0.6 position. |

## FOLLOWUPS to add

The controller should append the following entries to `design/FOLLOWUPS.md`:

1. **`v07-tap-leaf-iterator-with-index-coverage`** (Tier: v0.7-blocker — must land in Phase 4) — Phase 4 must include at least one multi-leaf test where `leaf_index` in the resulting `Error::SubsetViolation` is *derived* from a tap-tree walker, not supplied as a constant. Replaces deleted MD-codec test `tap_leaf_subset_violation_carries_leaf_index`.
2. **`v07-decode-rejects-sh-bare-rename`** (Tier: v0.7.x defensive cleanup) — rename `decode_rejects_sh_bare` → `decode_rejects_sh_taptree` (or delete as redundant).
3. **`v07-stale-byte-annotation-comments`** (Tier: v0.7.x defensive cleanup) — sweep stale `// 0x32`, `// 0x19`, `// 0x33`, `// 0x05` byte-value annotations.
4. **`v07-taptree-diagnostic-runtime-byte`** (Tier: v0.7.x or later) — refactor `decode_descriptor`'s TapTree arm message to format the byte at runtime via `Tag::TapTree.as_byte()` rather than hardcoded `(0x07)`.
5. **`v07-n_taptree_at_top_level-description-stale-v05-byte`** (Tier: Phase 6 — fold into v0.7 vector regen) — `vectors.rs:1887` description field says "Tag::TapTree (0x08)"; update to "(0x07)" when v0.7 regenerates `v0.2.json`.

## Verdict

Phase 1 met its acceptance criterion (`cargo test --workspace` 0 failures). The 38 unit tests rebaselined, 1 source-code message updated with reasonable test-intent justification, 2 tests deleted with mostly-sound rationale (one with a coverage-attribution gap that needs Phase 4 commitment).

**Controller can proceed to Phase 2 in parallel.** The Important finding (IMP-1) blocks Phase 4 dispatch, not Phase 2.
