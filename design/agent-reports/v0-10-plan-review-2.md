# v0.10 plan review (opus, pass 2)

**Date:** 2026-04-29
**Plan:** design/IMPLEMENTATION_PLAN_v0_10_per_at_N_paths.md (commit 998d462)
**Reviewer:** opus-4.7 (pass 2, narrow F1-F15 verification)

## Summary

**Verdict: clean — ready for Phase 1 kickoff.** All 11 of the pass-1 findings claimed as ✅ in the plan's status table land cleanly under spot-check. The two no-action statuses (F11 nice-to-have not addressed; F13 nice-to-have folded as guidance) are coherent. Cross-references between the new Step 3.6.5 tier-precedence tests and the helper definitions in earlier steps are consistent. No new blockers surfaced. Two minor-cosmetic observations are filed below as F16-F17, neither blocking.

The plan does carry visible "per F<N>" inline-comment markers that, taken together, slightly clutter the implementer flow (F17). This is borderline: the markers also serve as auditable provenance for the revisions, which is valuable for the per-phase opus-reviewer gates. Recommend leaving as-is.

## Pass-1 finding verifications

| F | Severity | Pass-2 verdict | Notes |
|---|---|---|---|
| F1 | blocker | ✅ Verified | Step 1.2 paragraph after the "Expected" call-out explicitly enumerates the 2 existing-test failures and states "**Action: delete**" for `reserved_bit_3_set` and "**Action: rewrite** as `all_reserved_bits_set_in_v0_10` testing `0x03`" for the other. Step 1.3 title now reads "Update `BytecodeHeader` struct + impl, plus migrate the two existing tests." Step 1.2 framing of "all 7 failures (5 new tests + 2 existing-test renames/deletions)" makes the pass-bar unambiguous. Spot-confirmed against codebase: lines 194 and 209 in `crates/md-codec/src/bytecode/header.rs` carry the named tests, and `RESERVED_MASK = 0x0B` lives at line 21. |
| F2 | blocker | ✅ Verified | Step 1.8 has the explicit substep titled "extend `ErrorVariantName` mirror enum" (the **Per F2 (blocker):** paragraph at lines ~351-364), with a code block showing the two new variants `OriginPathsCountMismatch` and `PathComponentCountExceeded` added to `tests/error_coverage.rs::ErrorVariantName`. The clarifying note about `BytecodeErrorKind` sub-variants NOT needing entries is also there, accurate, and helps the implementer avoid the wrong-variant trap. |
| F3 | blocker | ✅ Verified | Step 4.1 now opens with the verbatim doc comment proposed in F3 (lines 1041-1045): "Vectors o1 and o2 mirror SPEC §2 Example C ... If the spec example values change, both spec and corpus update in lockstep." The inline assertion test `o2_vector_origin_paths_block_matches_spec_example_b` is at lines 1105-1114 with the pinned hex substring `36030505fe046101 01c901`. |
| F4 | strong | ✅ Verified | Step 1.6 (lines 288-294) names all three tests with line numbers — `tag_v0_6_high_bytes_unallocated` (~294), `tag_rejects_unknown_bytes` (~317), `tag_round_trip_all_defined` (~305) — and the rename guidance to `tag_v0_10_*` is explicit. The `v0_6_allocated → v0_10_allocated` Vec rename is included. Spot-confirmed against codebase: actual line numbers (294, 305, 317) match. |
| F5 | strong | ✅ Verified | Step 2.1 includes the `decode_path_cap_check_fires_before_component_decode` test at lines 451-463 with the exact assertion-priority pin (`vec![0xFE, 0x0B]` with no following components, expecting `PathComponentCountExceeded` rather than `UnexpectedEnd`). The comment "if the cap check is moved after component decoding (a future refactor risk)" preserves the F5 rationale. |
| F6 | strong | ✅ Verified | Step 2.3 (lines 499-511) explicitly takes the API break: "Decision: take the break (option B in F6)." The MIGRATION.md instruction is present: "This API break MUST be added to MIGRATION.md (Phase 6 Step 6.9): `encode_path(&DerivationPath) -> Vec<u8>` becomes `encode_path(&DerivationPath) -> Result<Vec<u8>, Error>`." Step 6.9 (line 1391) lists "Hand-rename items (`BytecodeHeader::new_v0` signature)" — see F16 below for a minor nit there. |
| F7 | strong | ✅ Verified | Step 7.11 (lines 1543-1598) has the full multi-step coordination protocol with substeps a/b/c/d/e: (a) audit forward-reference text in mk1, (b) update mk1 companion FOLLOWUPS entry, (c) audit mk1 BIP for post-brainstorm edits affecting cross-reference, (d) open mk1 PR (with worktree-off-origin/main hedge fallback), (e) verify CLAUDE.md updated. Persistent hedge-audit report path explicit: `design/agent-reports/v0-10-phase-7-mk1-hedge-audit.md`. Mirrors v0.9.0 discipline. |
| F8 | strong | ✅ Verified | New Step 3.6.5 (lines 870-952) contains:<br>• `tier_0_origin_paths_override_wins_over_tier_1` (line 876)<br>• `tier_1_decoded_wins_over_tier_2_kiv_walk` (line 894, sketched with caveat that the WalletPolicy construction may need investigation)<br>• `tier_3_shared_fallback_for_template_only_policy` (line 904)<br>The "Tier 2 fires for full-descriptor parses" test from F8's recommendation is implicitly covered by the Tier-1-vs-Tier-2 collision test plus the open implementer question #1 (carried from pass-1). Acceptable. |
| F9 | strong | ✅ Verified | Step 3.6.5 contains:<br>• `double_round_trip_origin_paths_byte_identical` (line 918) — exercises `encode → decode → encode → decode → encode` with explicit assertions for both round-trips and a Tier-1-stability comment.<br>• `decoded_shared_path_and_decoded_origin_paths_mutually_exclusive_after_decode` (line 938) — exactly the mutual-exclusion test recommended in F9. |
| F10 | nice-to-have | folded as guidance | Status table marks "folded into Phase 6 implementer guidance — copy spec prose verbatim." The plan's prose at Steps 6.1-6.4 keeps the "per spec §6 prose" framing without inlining verbatim copy instructions; this is consistent with the implementer-guidance status. Acceptable. |
| F11 | nice-to-have | not addressed | Status table marks "not addressed inline; can fold into P5 by implementer if desired." Confirmed: Phase 5, Step 5.2 has only the `is_first_4_bytes` and `deterministic_from_policy` tests; no `policy_id_fingerprint_stable_across_round_trip` test. As stated in the table, the implementer may add this; the plan does not require it. Acceptable. |
| F12 | confirmation | ✅ Verified | Status table marks ✅; no plan-text change needed since this was a confirmation. Phase 7, Step 7.1 retains the audit step, as designed. |
| F13 | nice-to-have | folded as guidance | Status table marks "folded into Step 4.2 — implementer maps each negative vector to its conformance test." Step 4.2 (lines 1117-1129) lists the six negative vectors but does not contain the explicit mapping table from F13. This is acceptable as "implementer guidance" but slightly understated — see F18 below. |
| F14 | nice-to-have | ✅ Verified | Pre-Phase-0, Step 3 (line 78) reads: "Expect: ok=678 failed=0 (verified against main commit 2a9c969 on 2026-04-29; v0.9.1 baseline). Pin this number in Phase-end commit messages so post-v0.10 phase totals are diff-able." Concrete and dated. |
| F15 | nice-to-have | ✅ Verified | Pre-Phase-0, Step 4 (NEW, lines 87-108) bumps version to 0.10.0 BEFORE any vector regen, with a foundation-commit and the rationale captured in the commit message. Phase 7 Step 7.2 (line 1445) is now correctly labeled "Confirm version is 0.10.0" with a fallback-bump path documented in case the Pre-Phase-0 commit was missed. |

### Cross-cutting verification

- **Status-table accuracy.** All 15 entries in the table at lines 1604-1620 match the plan-body state spot-checked above.
- **No internal inconsistencies surfaced from the back-and-forth revisions.** The pass-1 references in the plan body (e.g., "per F15", "per F8 + F9", "per F2 (blocker)") all align with the status-table claims and with the pass-1 report text.
- **TDD step sequencing remains consistent.** Step N.1 (write failing) → N.2 (verify failure) → N.3 (impl) → N.4 (verify pass) holds across phases despite the inline finding-resolutions.
- **Forward-reference check (item 11).** The new Step 3.6.5 tier-precedence tests reference `decoded_origin_paths` (introduced in Step 3.1), `EncodeOptions::with_origin_paths` (Step 3.2), `placeholder_paths_in_index_order` (Step 3.3), and `to_bytecode` dispatch behavior (Step 3.4). All referents are defined upstream within Phase 3. No forward-reference defects.

## New findings (pass-2)

### F16: Step 6.9 MIGRATION snippet enumerates `BytecodeHeader::new_v0` but not `encode_path` API break in the bullet list itself
**Severity:** nice-to-have
**Location:** Phase 6, Step 6.9 (line 1391)
**Issue:** The Step 6.9 line reads: "Sections: What renamed/added/changed, Mechanical sed, Hand-rename items (`BytecodeHeader::new_v0` signature), Wire format, Test rewrite for `MAX_PATH_COMPONENTS`." The `encode_path` API break (per F6 fix) is not listed in the "Hand-rename items" bullet — it's only mentioned in Step 2.3's body text instructing the implementer to add it to MIGRATION. This is technically correct but easy to miss when implementing Phase 6.

**Recommendation:** Append to the parenthetical: "Hand-rename items (`BytecodeHeader::new_v0` signature, `encode_path` return-type change to `Result`)." Five-second edit.

### F17: Inline "per F<N>" markers are slightly cluttering but serve as auditable provenance
**Severity:** nice-to-have, observation only
**Location:** throughout the plan
**Issue:** The plan now contains 14 inline "**Per F<N>**", "**Per F<N> (blocker):**", "(NEW per F8 + F9)", "(per F7 + RELEASE_PROCESS.md)" markers. For a casual implementer reading the plan top-to-bottom, these markers can feel like finding-tracker noise. However, they also serve as auditable provenance — when the per-phase opus reviewer compares pass-1 vs pass-2 vs ship, the markers make it trivially clear which sections were revised.

**Recommendation:** Leave as-is. The audit trail value > the prose-cleanliness cost. If the user prefers cleanup, do it post-Phase-1-merge as a `chore(plan): strip pass-1 finding markers` commit, not before kickoff.

### F18: Step 4.2 negative-vector enumeration is missing the explicit conformance-mapping table from F13
**Severity:** nice-to-have
**Location:** Phase 4, Step 4.2 (lines 1117-1128)
**Issue:** The pass-1 status table marks F13 "folded into Step 4.2 — implementer maps each negative vector to its conformance test." However, Step 4.2's body lists the six negative vectors as a flat bulleted list without the explicit `→` mapping that F13 recommended. The implementer is left to derive the mapping during P4. Given that F13 was filed as nice-to-have, this is acceptable but slightly understated relative to the status-table claim.

**Recommendation:** Either:
- (A) Tighten the status-table entry to "F13 deferred to P4 implementer; not folded inline," or
- (B) Add the F13 mapping table to Step 4.2 as a six-line code-block. Either is fine; (A) is honest about the current state.

## Greenlight

The plan is ready for Phase 1 implementation kickoff; the three observations above (F16 cosmetic MIGRATION-snippet bullet, F17 audit-trail marker decision, F18 status-table-vs-plan-body honesty nit) are all nice-to-have and can be addressed post-Phase-1 if at all.
