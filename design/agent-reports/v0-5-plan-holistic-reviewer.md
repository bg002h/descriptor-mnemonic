# v0.5 implementation plan holistic review — Opus 4.7

**Status:** DONE
**Subject:** `design/IMPLEMENTATION_PLAN_v0_5_multi_leaf_taptree.md` at commit `590605c`
**Source spec:** `design/SPEC_v0_5_multi_leaf_taptree.md` at commit `7ef7cec`
**Codebase ground truth:** `crates/md-codec` at v0.4.1
**Reviewer:** Opus 4.7 holistic peer review
**Verdict:** **READY-WITH-MINOR-FIXES** — spec coverage complete; code references verified against the actual file:line numbers; phase ordering correct. Two important findings (placeholder `todo!()` in H1/H2 hostile fixtures left to implementer; LI2 hostile-bytecode construction also `todo!()`) and a handful of minor cosmetic / cross-reference issues. No blocking critical findings.

## Executive summary

The plan is mechanically thorough. Code snippets in Phases 2–5 line up with the spec's §3 / §4 helpers and with the actual function signatures and lines in `encode.rs` and `decode.rs`. Phase ordering is sound: Phase 2 lands all type-wiring + decoder helper + multi-leaf decode routing, then Phase 4 lands the encoder; Phase 4 only depends on Phase 2 (Tag::TapTree dispatch and `validate_tap_leaf_subset` signature) which Phase 2 actually delivers.

Five concrete code references (`encode.rs:126-158`, `encode.rs:443`, `encode.rs:468-470`, `encode.rs:487`, `decode.rs:256-284`, `decode.rs:603-707`, `decode.rs:67-100`, `decode.rs:680-683`) match the codebase exactly. The §3 decoder helper and §4 encoder helper code blocks reproduce the spec's helpers byte-for-byte (with one beneficial addition: Plan Task 2.7 introduces `let tag_offset = cur.offset();` before `peek_byte` to thread an offset into the `None` arm — a small improvement over the spec).

The most material gap is the hostile-input fixtures (H1, H2, LI2) which leave `todo!()` for "construct hostile bytecode." Given that H1 (legal-128 boundary) and H2 (illegal-129 boundary) are the cornerstone tests of the v0.5 hardening posture, the plan SHOULD provide concrete construction code rather than handing the implementer a build-it-yourself note. See I1 below.

After folding the I1–I5 important findings, the plan is ready for execution.

## Critical findings

**None.** No factual errors, no missing prerequisites, no contradictions with the spec.

## Important findings (fix before execution)

### I1 — H1 / H2 / LI2 hostile-bytecode construction left as `todo!()`
**Plan lines:** 1700-1722 (H1 helper), 1724-1736 (H1 test), 1738-1743 (H2 test), 1880-1889 (LI2 test).
**Issue:** These tests are the load-bearing hardening fixtures for the depth-128 gate (the entire spec §1 decision-matrix entry "depth ceiling: BIP 341 consensus depth (128)" rides on H1+H2). The plan emits a `todo!()` macro inside `build_left_spine_taptree_bytecode` with the comment "Implementer: construct left-spine bytecode programmatically; use encoder round-trip on a test TapTree as reference." LI2 has the same shape — `todo!("Implementer: construct hostile bytecode + assert error contents")`.

This is materially different from a placeholder for fixture data that's mechanical to fill in (e.g. an SHA value in CHANGELOG line 2390). The construction-of-hostile-input is non-trivial — getting the recursion-depth math right is exactly the bug class the spec's depth-semantics paragraph (line 142) was written to avoid. Leaving it for an implementer to figure out is asymmetric with how careful the spec was about getting the gate semantics right.

**Implication:** Phase 6 implementer must derive the construction independently. Two ways this can go wrong:
1. The constructed left-spine produces leaves at miniscript-depth ≠ 128, so H1 either passes vacuously (depth too shallow → no exercise of the gate) or fails for the wrong reason (constructed depth > 128 → rejection that's not at the boundary).
2. H2's "129 framings" off-by-one is mis-counted; gate doesn't fire at the boundary the spec promised.

**Fix:** Plan should provide the actual construction code. Suggested helper, where N is the desired count of `[TapTree]` framings:

```rust
/// Build [Tr][Placeholder][0] followed by N TapTree framings on the LEFT
/// spine, with leaves at every right-child position and the bottom-left.
/// Result: a tree where the deepest leaf is at miniscript-depth N.
fn build_left_spine_taptree_bytecode(framings: usize) -> Vec<u8> {
    use md_codec::bytecode::tag::Tag;
    let mut out = vec![Tag::Tr.as_byte(), Tag::Placeholder.as_byte(), 0u8];
    // Each framing has [TapTree][LEFT_RECURSE][RIGHT_LEAF]. We emit
    // framings first (left spine), then for each framing emit one right-leaf,
    // then close with one bottom-left leaf.
    for _ in 0..framings {
        out.push(Tag::TapTree.as_byte());
    }
    // Bottom: one leaf at the deepest left position
    out.extend_from_slice(&[Tag::PkK.as_byte(), Tag::Placeholder.as_byte(), 1u8]);
    // Then N right-children, one per framing (in right-spine order on the way back up)
    for i in 0..framings {
        out.extend_from_slice(&[Tag::PkK.as_byte(), Tag::Placeholder.as_byte(), (i + 2) as u8]);
    }
    out
}
```

The implementer can then iterate against the encoder side to validate (encode a constructed-via-`TapTree::combine` tree at depth 128 and compare bytecodes; if they don't match, revise). Or pre-compute the depth math in test setup and assert intermediate steps.

If the plan deliberately wants to hand this off, it must AT LEAST narrow the `todo!()` to a clearly-scoped "fill in N keys + leaf positions per the recursion structure" rather than "construct left-spine bytecode programmatically." Current wording invites the implementer to re-derive the recursion structure from scratch.

**Recommendation:** Replace the three `todo!()` macros with real code. The hostile fixtures are too important to the spec's safety story to leave half-specified.

### I2 — Phase 6 SHA pinning + CHANGELOG SHA placeholders are circular
**Plan lines:** 1929-1947 (Phase 6 Task 6.5 captures + sets new SHAs), 2380, 2390 (CHANGELOG `<NEW_SHA_FROM_PHASE_6>` placeholders), 2530 (Phase 11 final-gate run).
**Issue:** Phase 6 regenerates `v0.1.json` + `v0.2.json` and updates `vectors_schema.rs` SHA pin. Phase 10 inserts CHANGELOG entries with `<NEW_SHA_FROM_PHASE_6>` placeholders that the implementer must substitute. But Phase 10 is dispatched AFTER Phase 6 completed, so by the time the implementer reads it, the actual SHA is already known and recorded somewhere in `vectors_schema.rs`. The placeholder is never explicitly resolved.

**Implication:** Risk of CHANGELOG shipping with the literal string `<NEW_SHA_FROM_PHASE_6>`.

**Fix:** Add an explicit step in Task 10.1 between Step 1 and Step 2 reading "extract the current SHA values from `crates/md-codec/tests/vectors_schema.rs` `V0_1_SHA256` and `V0_2_SHA256` constants and substitute into the CHANGELOG entry." Or reorganise Phase 6 to write the SHAs to a known artifact (e.g. `/tmp/v0.5.0-shas.txt`) that Phase 10 reads.

### I3 — Task 2.5 Step 5 re-export example is wrong about `Correction`
**Plan lines:** 580-584.
**Issue:** Plan suggests `pub use decode_report::{DecodeReport, DecodeOutcome, Correction, Verifications, Confidence, TapLeafReport};`. Verified at `crates/md-codec/src/decode_report.rs:8`: `Correction` is imported from `crate::chunking::Correction`, not defined in `decode_report`. The real lib.rs:156 currently re-exports only `{Confidence, DecodeOutcome, DecodeReport, DecodeResult, Verifications}` from `decode_report`. The plan's example is a fictional list.

**Implication:** Implementer copy-pastes the example, gets a "no `Correction` in `decode_report`" error, has to figure out the actual layout. Hedged by the trailing "Adjust to match the actual re-export style" note, but the literal example is misleading.

**Fix:** Replace lines 580-584 with the actually-true current export plus the addition: `pub use decode_report::{Confidence, DecodeOutcome, DecodeReport, DecodeResult, TapLeafReport, Verifications};`. Drop the `Correction` reference; it's already exported via `crate::chunking::Correction` (verified at lib.rs).

### I4 — Test count math: plan does not enumerate the path to ≥638
**Plan lines:** 128 (Pre-0.3 baseline 609), 1502 (Phase 5 expected ~615), 1589 (Phase 6 mention "≥638"), 1965 (Phase 6 gate "≥638"), 2530 (Phase 11 final gate "≥638").
**Issue:** Spec says 29 NEW + 1 RENAMED tests (RENAMED adds 0 to count). Plan has these new tests:
- Phase 2 type-wiring file: 4 (Tasks 2.1, 2.3, 2.5×2 = `tap_leaf_subset_violation_has_leaf_index_field`, `tap_leaf_subset_violation_accepts_none_leaf_index`, `validate_tap_leaf_subset_takes_leaf_index_arg`, `decode_report_has_tap_leaves_field`, `tap_leaf_report_struct_has_required_fields` — that's 5) plus `decode_tap_subtree_helper_exists` smoke test (Task 2.7) plus `multi_leaf_two_leaf_symmetric_round_trips` (Task 2.10). Total Phase 2 new: ~7.
- Phase 3: `taptree_at_top_level_produces_specific_diagnostic` — 1.
- Phase 5: `keyonly_tr_produces_empty_tap_leaves`, `single_leaf_tr_produces_one_tap_leaf_at_depth_zero` — 2.
- Phase 6: H1-H5 (5), RT1-RT4 (4), LI1-LI3 (3), PR1-PR2 (2) = 14. Plus N1-N9 fixture-driven (9). Plus T1, T3-T7 fixture-driven (6 — but RENAMED T2 is +0). Plus implicit `gen_vectors`-expanded variants for T1, T3-T7.
- Phase 8: `cli_encode_decode_multi_leaf_taptree` — 1.

Sum: 7 + 1 + 2 + 14 + 9 + 6 + 1 = 40 explicitly listed new tests. The spec's "29 NEW" is a tighter count (Phase 6 only). The plan's Phase 2/3/5/8 type-wiring + smoke tests are EXTRA and not in the spec's 29 — that's fine, but the gates' "≥638" is loose: the floor is more like ~640-645.

**Implication:** The "≥638 + 0 ignored" gate at Phase 11 line 2554 is a low ceiling; if some Phase 2 type-wiring test gets removed during execution, the gate still trips on the spec's 638 minimum. But the plan never crisply states "Phase 2 introduces N type-wiring tests that don't count toward the spec's 29."

**Fix:** Add a one-paragraph "test budget" near Phase 1 that breaks down the 29 spec-ed tests vs the ~11+ structural type-wiring tests added in Phases 2/3/5/8. Phase 11 gate becomes "≥640" or "≥645" (whichever the actual sum is) for tighter confidence. Alternative: keep the loose ≥638 and note the type-wiring tests are bonus coverage.

### I5 — Plan does not specify per-phase implementer audit-report path
**Plan lines:** 36 (table mentions `design/agent-reports/v0-5-multi-leaf-phase-N-implementer.md`), 2638-2639 (cross-cutting concerns reference memory `feedback_subagent_workflow`).
**Issue:** Memory `feedback_subagent_workflow` says "every Phase's implementer subagent must persist a final report; every deferred minor item gets a FOLLOWUPS.md entry." The plan's File-Structure table at line 36 names a path template, but the per-phase task lists do NOT have an explicit "Step N: Persist implementer report to design/agent-reports/v0-5-multi-leaf-phase-N-implementer.md" step. Phase 9 (Task 9.1) does explicitly tell the dispatched reviewer to persist to `v0-5-multi-leaf-final-reviewer.md`, but the implementer subagents have no equivalent instruction.

**Implication:** Implementer subagents may forget the audit-trail file (especially in Subagent-Driven mode). Per memory, this is "non-negotiable."

**Fix:** Add a final task to each phase: "Task N.X: Persist implementer report" with the path template. Or add a single cross-cutting reminder in each phase's frontmatter ("Per-phase audit trail required: `design/agent-reports/v0-5-multi-leaf-phase-N-implementer.md`"). Phase 11 cleanup should include verification that all 7-8 phase reports exist (Phase 2-8, 10).

## Minor findings (nice-to-have)

### M1 — Phase 1 commit-history check fragile to rebase
**Plan line:** 142.
The `git log --oneline | grep -E '7ef7cec|e6e8477|fcef2a7'` will fail if the spec commits get rebased into a single squash commit on `main`. Suggest replacing with a content-based check: `grep -q "Status: Approved" design/SPEC_v0_5_multi_leaf_taptree.md`.

### M2 — Phase 2 Task 2.10 test re-uses the test-file across phases
**Plan lines:** 900, 1407, 1479, 2693.
`crates/md-codec/tests/v0_5_type_wiring.rs` accumulates tests across Phases 2-5. Eight tests in one file by Phase 5 — fine for a structural pin file but consider splitting into `v0_5_decoder.rs` and `v0_5_encoder.rs` for readability. Not critical.

### M3 — `decode_template` and `Cursor::new` paths may not be public (Phase 3 Task 3.1)
**Plan lines:** 1038, 1044, 1059.
Plan says "the `decode_template` and `Cursor::new` paths may differ — adjust to match. If `decode_template` is private, route through the public `decode_string` API and feed it a malformed bytecode (chunk-encoded)." Verified: `Cursor::new` is `pub(crate)` (cursor.rs:23), so an integration test in `tests/` cannot call it. The plan's hedge is correct, but the test as written WILL NOT COMPILE in `tests/`. Implementer needs to route through the public API. Suggest the plan fold this in: "Use `md_codec::decode(<chunked-md-string>)` with a chunked encoding of the single byte 0x08 + appropriate fingerprint chrome."

### M4 — `decode_descriptor` is private; Phase 3 test cannot call it
**Plan line:** 1038.
Same issue as M3: `decode_descriptor` is a private fn at `decode.rs:61`. The plan's test would need a public wrapper or chunked-string round-trip. The hedge in the plan ("adjust to match") covers this but should be explicit.

### M5 — Worktree path `descriptor-mnemonic-v0.5` is correct sibling-depth
**Plan line:** 60.
Worktree at `/scratch/code/shibboleth/descriptor-mnemonic-v0.5` — verified sibling depth (2 levels: `shibboleth/descriptor-mnemonic-v0.5` mirrors `shibboleth/descriptor-mnemonic`). Workspace `[patch]` block uses relative path `../rust-miniscript-fork/`, which from sibling-depth resolves correctly. Per memory `feedback_worktree_dispatch`: this is the correct fix for the symlink workaround. Plan handles this right.

### M6 — Branch is cut from `origin/main`, not local HEAD — correct
**Plan line:** 61.
`git worktree add ... origin/main`. Per memory, this is the right call: it avoids picking up uncommitted local state. Plan handles this right. The Pre-0.1 Step 1 push step (line 50) is correct.

### M7 — Phase 4 Task 4.4 un-ignore step is fragile
**Plan lines:** 1326-1352.
`multi_leaf_two_leaf_symmetric_round_trips` is created in Phase 2 with `#[ignore]` and un-ignored in Phase 4. But Phase 4 Step 2 says "drop the `tap_leaves` assertions" and "re-add them in Phase 5 Task 5.4." Phase 5 Task 5.2 does re-add them (line 1488). Across-phase test mutation is fragile; better would be to delete the assertion in Phase 2 (write a placeholder), then add a NEW test in Phase 5. Cosmetic.

### M8 — Plan references "Task 5.4" which doesn't exist
**Plan line:** 1339.
"Re-add them in Phase 5 Task 5.4." Phase 5 only has Tasks 5.1, 5.2, 5.3. The actual re-add is Task 5.2.

### M9 — Single-leaf carve-out's `leaves.len() == 1 && leaves[0].0 == 0` matches spec
**Plan line:** 1297.
Mirrors spec line 212. Verified consistent.

### M10 — `encode_tap_subtree` helper code matches §4 spec verbatim
**Plan lines:** 1232-1253 vs spec lines 177-198.
Compared character-by-character. Function signature, condition `leaf_depth == target_depth` vs `leaf_depth > target_depth`, comment about unreachable `<` case — all match. Good.

### M11 — `decode_tap_subtree` helper code matches §3 spec
**Plan lines:** 743-782 vs spec lines 94-130.
Plan's version adds `let tag_offset = cur.offset();` (line 750) before the match — this is a small improvement: the `None` arm at plan line 778 uses `tag_offset` instead of spec line 127's `cur.offset()` (which would point past the byte after read_byte advances). The plan's offset is more diagnostically useful. **This is an improvement, not a bug.**

### M12 — Test counter "≥638" vs spec target "≥640" handwave
**Plan lines:** 128, 1965, 2530, 2554.
Spec says floor 638 / target 640. Plan uses 638 throughout. If `gen_vectors` expansion produces +2 extra round-trip pairs (T1 + T3 or T7 expansion), the count climbs. Plan should say "≥638 (floor) / ≥640 (target after gen_vectors expansion)."

### M13 — Phase 11 release-commit FOLLOWUPS-SHA loop
**Plan lines:** 2538, 2611-2616.
Task 11.2 sets FOLLOWUPS entry's `resolved <SHA>` to the version-bump commit's own SHA. Then Task 11.5 Step 3 re-updates the FOLLOWUPS SHA to the post-merge commit. This requires two commits to FOLLOWUPS.md — fine, but the plan should explicitly note "Task 11.2 records pre-merge SHA placeholder; Task 11.5 finalises with merge-commit SHA."

### M14 — Spec's `validate_tap_leaf_subset` map_err pattern complexity
**Plan lines:** 391-405.
Inner-helper `map_err` re-binding pattern is correct but verbose. An alternative would be to refactor `validate_tap_leaf_terminal` to take `leaf_index: Option<usize>` directly (analogous to `validate_tap_leaf_subset`'s new signature). Cosmetic; the map_err pattern keeps the inner helper's signature simple.

### M15 — Phase 6 Task 6.2's "input_strings: &[]" with bytecode-only fixtures
**Plan lines:** 1614-1675.
The negative-fixture struct is described as having `input_strings: &str` field. Plan sets all bytecode-only fixtures to `input_strings: &[]`. If the harness REQUIRES at least one input_string per fixture, the empty slice will fail validation. Implementer should verify the harness handles "bytecode-only" fixtures or add a pre-built MD-encoded chunked string.

### M16 — Spec line numbers in BIP edits not verified against current BIP HEAD
**Plan line:** 2052.
Plan inherits BIP line numbers (85-89, 534-540, 391, etc.) from the spec, which the spec reviewer flagged as "M1 — BIP line numbers slightly imprecise" but didn't fix. Phase 7 implementer should verify against current BIP HEAD before editing.

## Strengths

- **Code-reference accuracy.** Every file:line reference I cross-checked matches the codebase: `error.rs:319-324` (TapLeafSubsetViolation variant), `encode.rs:443` (Terminal-encoder catch-all), `encode.rs:468-470` (validate_tap_leaf_subset signature), `encode.rs:487` (validate_tap_leaf_terminal catch-all), `encode.rs:126-158` (Descriptor::Tr arm), `encode.rs:154` (validate_tap_leaf_subset call site), `decode.rs:256-284` (decode_tr_inner), `decode.rs:276` (validate_tap_leaf_subset call site), `decode.rs:581-602` (decode_tap_miniscript), `decode.rs:603-707` (decode_tap_terminal), `decode.rs:691` (decode_tap_terminal catch-all), `decode.rs:680-685` (Tag::TapTree reservation rejection), `decode.rs:67-100` (top-level dispatcher), `decode_report.rs:111-120` (DecodeReport struct).
- **Phase ordering.** Phase 2 → 4 → 5 dependency chain is sound: Phase 4's encoder uses `validate_tap_leaf_subset` with the new signature (delivered Phase 2 Task 2.3), and the multi-leaf encode test (Phase 4 Task 4.4) requires the decoder to produce a `Descriptor::Tr` it can compare against (delivered Phase 2 Task 2.8). Phase 5 only depends on Phase 2's `TapLeafReport` struct (Task 2.5).
- **TDD discipline mostly clean.** Most tasks have clear Write-fail / Run-fail / Implement / Run-pass / Commit cadence. Task 2.1 explicitly walks "compile-error → fix variant → re-run → cascade error → fix call sites" which is honest about the cascading typing.
- **Worktree dispatch.** Pre-Phase-0 covers all three pitfalls from `feedback_worktree_dispatch.md`: sibling depth (line 60), `origin/main` not local HEAD (line 61), spec commit pushed first (Pre-0.1 Step 1 line 49-50).
- **Spec coverage.** Every section of the spec maps to a phase/task in the plan:
  - §1 Scope → Phase 0 (no code)
  - §2 Wire format → Phase 2 Tasks 2.7-2.9 + Phase 4 Task 4.3
  - §3 Decoder → Phase 2 Tasks 2.6-2.10
  - §4 Encoder + types → Phase 2 Tasks 2.1-2.5 + Phase 4 Tasks 4.1-4.4
  - §4 BIP draft updates → Phase 7
  - §5 Test corpus → Phase 6 Tasks 6.1-6.7 + Phase 2/3/5/8 type-wiring
  - §6 Migration + release → Phase 9-11
- **Defense-in-depth preserved.** Plan Task 2.9 keeps the `decode.rs:680` rejection as defense-in-depth rather than removing it (per Option A, the safer choice). Sound.
- **Encoder relies on `TapTree::combine`'s upstream invariant.** Plan Task 4.3 line 1304-1305 documents the rationale; matches spec §4 line 229.
- **Handles `Correction`-not-in-decode_report-module hedge correctly.** Despite I3 (the literal example is wrong), the plan's "Adjust to match the actual re-export style" hedge prevents the implementer from blindly copy-pasting bad code. A small fix improves the example but the plan won't break implementer flow.

## Cross-section consistency

- **`decode_tap_subtree` function name:** consistent across Task 2.7 (definition), Task 2.8 (call from `decode_tr_inner`), Task 2.9 (defense-in-depth comment), and behavioral-test references. ✓
- **`encode_tap_subtree` function name:** consistent across Task 4.2 (definition), Task 4.3 (call from `Descriptor::Tr` arm). ✓
- **`tap_leaves` field:** consistent across §4 type definition (Task 2.5), §5 test expectations (Tasks 5.1, 5.2, 6.4 LI1-LI3), §6 CHANGELOG (Task 10.1 line 2375). ✓
- **`leaf_index: Option<usize>`:** consistent across error variant (Task 2.1), `validate_tap_leaf_subset` (Task 2.3), `decode_tap_terminal` / `decode_tap_miniscript` (Task 2.6). ✓
- **Depth gate `> 128` (not `>= 128`):** matches spec line 104 + plan Task 2.7 line 756. ✓ (Spec C4 fix correctly inherited.)

## Code-snippet equivalence checks

I compared the plan's reproduced code blocks against the spec word-for-word:

| Plan task | Spec section | Match? |
|---|---|---|
| Task 2.7 `decode_tap_subtree` (lines 743-782) | §3 (lines 94-130) | Match (plan adds beneficial `tag_offset` capture for None-arm error reporting) |
| Task 4.2 `encode_tap_subtree` (lines 1232-1253) | §4 (lines 177-198) | Match exactly |
| Task 4.3 `Descriptor::Tr` arm (lines 1278-1313) | §4 routing (lines 203-225) | Match (plan hardens with explicit `tap_tree present but contains no leaves` check inherited from v0.4) |
| Task 2.5 Step 3 `DecodeReport` struct (lines 530-548) | §4 (lines 247-262) | Match |
| Task 2.5 Step 4 `TapLeafReport` (lines 565-575) | §4 (lines 251-260) | Match |

**All match.** No drift.

## Verdict

**READY-WITH-MINOR-FIXES.** Fold I1-I5 before kickoff (most important: I1 hostile-fixture construction code). M1-M16 are cosmetic; fold as convenient or skip without harm. Critical findings: zero. Code references: verified. Phase ordering: sound. Worktree hygiene: correct. Audit trail: filename pattern stated but per-phase persistence step missing (I5).

The plan is ready for the user's writing-plans handoff and execution-mode selection (Subagent-Driven recommended per the plan's own line 2659).

## FOLLOWUPS appended

**No FOLLOWUPS appended.** All findings are within the immediate plan-revision cycle (I1-I5) or cosmetic (M1-M16). None survive past the cycle.
