# v0.6 plan review (round 1)

**Status:** DONE_WITH_CONCERNS
**Commit:** N/A — review of the plan at `design/IMPLEMENTATION_PLAN_v0_6_strip_layer_3.md` (commit `fde66f4`)
**File(s):**
- `design/IMPLEMENTATION_PLAN_v0_6_strip_layer_3.md`
- `design/SPEC_v0_6_strip_layer_3.md`
- `design/agent-reports/v0-6-spec-review-1.md`
- `crates/md-codec/src/bytecode/encode.rs`
- `crates/md-codec/src/bytecode/decode.rs`
- `crates/md-codec/tests/error_coverage.rs`
- `crates/md-codec/tests/vectors_schema.rs`
- `crates/md-codec/tests/vectors/v0.2.json`
**Role:** reviewer (plan)

## Summary

Plan reviewed against spec, round-1 spec-review report, and live in-tree state. Verified spec coverage, TDD discipline, cross-phase ordering, SHA-pin deferral, sed-rename safety, the negative-vector audit, and the Tag::Bare ripple. 3 critical findings (CRIT-1/2/3), 5 important findings (IMP-4 through IMP-8), one spec-coverage concern (§6.3 byte-order test), and 10 nits/nice-to-haves for FOLLOWUPS. None are blocking — the plan can execute as-is — but pre-pinning the in-flight decisions is substantially more efficient.

## Critical findings

### CRIT-1: SHA-pin deferral leaves intermediate phases failing the SHA-lock test

Phase 8 explicitly defers SHA pinning to Phase 10 (Step 8.3.1 `git checkout --` discards regenerated files). But `tests/vectors_schema.rs` has TWO failing tests across Phases 5-9:

- `committed_v0_2_json_matches_regenerated_if_present` (line 207) — `committed.vectors == regenerated.vectors` fails because in-memory corpus diverges from on-disk JSON after Phase 5.
- `v0_2_sha256_lock_matches_committed_file` (line 252) — only fails after Phase 10 regen, but `V0_2_SHA256` constant is wrong relative to v0.6 content.

Phase 5's Step 5.1.5 says "Expected: SHA-pin tests fail (corpus content changed). That's expected — Phase 8 re-baselines SHAs." But it conflates two distinct test failures and doesn't whitelist them in Phase 6/7 reviewer briefs.

**Risk:** Phase 6 review (BIP draft) and Phase 7 review (READMEs) will be dispatched on top of a red test suite — obscuring whether their changes broke anything.

**Fix:** explicitly add the failing-test whitelist to Phase 6 and Phase 7 reviewer briefs, and to Step 5.1.6/5.2.4 commit bodies so the controller's review-time reasoning is on record.

Confidence: 90.

### CRIT-2: Phase 4 sed scope omits design/ markdown forward-pointing references

Step 4.1.3's `find ... -type f -name "*.rs" -exec sed -i 's/TapLeafSubsetViolation/SubsetViolation/g' {} \;` correctly handles src/ + tests/ Rust files. **However**, design/ markdown files (`MD_SCOPE_DECISION_2026-04-28.md`, `FOLLOWUPS.md`, agent reports) contain `TapLeafSubsetViolation` references in spec/decision text. Past-tense/historical references should stay; forward-pointing references (e.g., "callers get TapLeafSubsetViolation") need rewrite.

CHANGELOG/MIGRATION are handled in Phase 9. BIP draft in Phase 6. design/ markdown is unhandled.

**Fix:** add a short Step 4.1.5 — "Scrub design/ markdown for forward-pointing references to `TapLeafSubsetViolation`. Past-tense references (e.g., 'v0.5 raised TapLeafSubsetViolation') stay; forward-pointing references update."

Confidence: 85.

### CRIT-3: Phase 5b negative-vector audit is judgment-heavy when it can be pre-pinned

Step 5.2.2 says "categorize each match. Flip to positive / Keep but rename / Delete." This is the spec's §6.2 rephrased. But the negative vectors in v0.2.json are knowable now. Six existing `expected_error_variant: TapLeafSubsetViolation` negative vectors:

| Vector | Pre-decision |
|---|---|
| `n_tap_leaf_subset` (sha256 in tap leaf) | **DELETE** — sha256 is admitted in v0.6; redundant with new positive `tr_sha256_htlc_md_v0_6` |
| `n_taptree_inner_wpkh` | **KEEP, change `expected_error_variant`** — `wpkh` is a top-level descriptor tag; structurally invalid as tap-leaf inner regardless of strip. Error variant changes to whatever Phase 3 picks for the structural catch-all (see IMP-7) |
| `n_taptree_inner_sh` | **KEEP, change `expected_error_variant`** — same |
| `n_taptree_inner_wsh` | **KEEP, change `expected_error_variant`** — same |
| `n_taptree_inner_tr` | **KEEP, change `expected_error_variant`** — same |
| `n_taptree_inner_pkh` | **KEEP, change `expected_error_variant`** — same. Note: distinct from policy-level `pkh()` (which desugars to `c:pk_h(...)`); this vector tests the descriptor wrapper byte (Tag::Pkh = 0x02) showing up where a tap-leaf inner is expected |

Plus two more from the Tag::Bare ripple (per IMP-8): `n_sh_bare` and `n_top_bare` — KEEP-with-input-rebase (semantic intent preserved; input bytes shift because Tag layout shifted).

The `input_strings` byte values shift across all eight negative vectors because Tag bytes shift in v0.6.

**Fix:** rewrite Step 5.2.2 to PRE-PIN this audit table inline. Add an explicit instruction that provenance bodies must reference the new bytes after Phase 8/10 regen.

Confidence: 90.

## Important findings

### IMP-4: Phase 4 must rename conformance.rs test name to satisfy error_coverage gate

The error_coverage gate at `tests/error_coverage.rs` derives expected test names from variant names via snake_case. After renaming `Error::TapLeafSubsetViolation` → `Error::SubsetViolation`, the conformance.rs test currently named `rejects_tap_leaf_subset_violation` must rename to `rejects_subset_violation` or the gate fails at Step 4.3.2.

**Fix:** add Step 4.2.3 — "Rename the conformance.rs test from `rejects_tap_leaf_subset_violation` to `rejects_subset_violation`. Verify by `grep -n 'rejects_tap_leaf_subset' crates/md-codec/tests/conformance.rs`."

Confidence: 90.

### IMP-5: Phase 2 reviewer brief missing four explicit checks

Phase 2's reviewer brief (Step 2.7.1) is too thin. Missing checks:

- Option (a) decision verification: tap-illegal `Multi`/`SortedMulti` arms have appropriate `// tap-illegal but exhaustive ...` comments per the spec.
- New arms emit Tag bytes that match the v0.6 layout (cross-check against tag.rs commit). E.g., new Hash256 arm emits `Tag::Hash256.as_byte() == 0x21`.
- Hash terminal byte order matches spec §6.3 (encoder uses `as_byte_array()` directly — internal byte order). Confirm new arms preserve this; not swapped to display-order.
- `validate_tap_leaf_subset` body is unchanged (only its rustdoc updates). Tempting to mistake "retain pub" as license to refactor.

**Fix:** expand Phase 2 reviewer brief to include those four explicit checks.

Confidence: 80.

### IMP-6: Phase 4 and Phase 7 should each have a phase review

Phase 4 is "mechanical rename" but is a public-API breaking change spanning ~20 sites. Risk: rustdoc link sed missed (e.g., `Error :: TapLeafSubsetViolation` with whitespace) compiles fine but breaks rustdoc CI in Phase 10's Step 10.4.2.

Phase 7 (READMEs + CLI) ships user-visible "recovery responsibility" wording per spec §8. The wording is judgment-heavy. Letting it ship without review misses an opportunity.

**Fix:** roll Phase 4 review into Phase 5's reviewer brief (since they're proximate); roll Phase 7 review into Phase 6's reviewer brief (since both are doc work and Phase 6 reviewer is already loaded with layered-responsibility framing context).

Confidence: 80.

### IMP-7: Phase 3 catch-all error kind decision should be pre-pinned

Step 3.2.2 leaves the catch-all error kind as a Phase 3 review concern. The deferred decision is a real type-system change rippling into:
- Error::InvalidBytecode Display
- error_coverage.rs mirror
- Negative vector `n_taptree_inner_*` family (per CRIT-3)
- conformance.rs test names

**Fix:** pre-decide. Add a new variant `BytecodeErrorKind::TagInvalidContext { tag: u8, context: &'static str }`. Phase 3 introduces it; Phase 5b uses it for the audited `n_taptree_inner_*` family.

Confidence: 85.

### IMP-8: Tag::Bare ripple — n_sh_bare / n_top_bare negative vectors need input rebase

Tag::Bare references in tree:
- `decode.rs:71` (top-level rejection arm): drop the `| Some(Tag::Bare)` term
- `decode.rs:822` (`tag_to_bip388_name`): plan's Step 3.4.2 handles
- BIP Tag table at ~line 390: Phase 6 handles
- `n_sh_bare`, `n_top_bare` in v0.2.json: provenance text is fine (refers to `Descriptor::Bare` rejection conceptually, unchanged), BUT the input bytecode references Tag::Bare = 0x07 which in v0.6 is Tag::TapTree. Negative vector input_strings shift; expected_error_variant stays PolicyScopeViolation.

**Fix:** add `n_sh_bare` and `n_top_bare` to the Step 5.2.2 audit table as KEEP-with-input-rebase.

Confidence: 90.

## Spec coverage check

| Spec § | Plan phase | Coverage |
|---|---|---|
| §1 | (informational) | OK |
| §2 | Phase 1 | OK |
| §3 | Phase 2 | OK |
| §4 | Phase 3 | OK |
| §5 | Phase 4 | OK |
| §6.1 | Phase 5 Task 5.1 | OK |
| §6.2 | Phase 5 Task 5.2 | Partial (CRIT-3) |
| §6.3 (hash byte order) | Phase 5 corpus locks via round-trip | **CONCERN** — corpus alone doesn't catch round-trip-stable-but-format-changed case (encoder + decoder both swap to display-order). Recommend adding hand-coded byte-pin assertion in `tests/taproot.rs` for Hash256: take a known SHA256d hash, encode, assert the bytecode bytes equal internal-order bytes. Likewise for Sha256/Ripemd160/Hash160. Defensive against accidental "fix" to display order. |
| §7 | Phase 6 | OK |
| §8 | Phase 7 | OK |
| §9 | Phase 9 | OK |
| §10 | Phase 10 | OK |
| §11 | (validation across phases) | Partial — criterion #10 (error_coverage) is implicit; #11 (per-Terminal round-trips) is implicit via Phase 5 corpus + Phase 10 final test pass |
| §12 | (informational) | OK |

Confidence on §6.3: 75.

## TDD discipline audit

- **Phase 1**: Genuine TDD (Task 1.1 writes failing tests first).
- **Phase 2**: regen-driven, not TDD. Justifiable because existing test suite covers the encoder.
- **Phase 3**: regen-driven, not TDD. New 20 arms uncovered until Phase 5 corpus expansion regenerates fixtures.
- **Phase 4**: mechanical; no test-first appropriate.
- **Phase 5**: corpus IS the test artifact.

**Verdict:** the plan's "TDD" framing in the architecture summary is loose. Recommend either softening the wording OR adding per-arm decoder unit tests in Phase 3 (~30 min effort, catches bugs round-trip alone cannot — e.g., decoder arm consuming wrong byte count that happens to match a symmetrically-wrong encoder arm).

Confidence: 80.

## Cross-phase ordering check

Phase 4 lands AFTER Phases 2/3. Intermediate state DOES compile because the Error variant still exists. Phase 4 then renames everywhere. Verified by `grep -n "TapLeafSubsetViolation" crates/md-codec/src/`. **Clean.**

Confidence: 85.

## Concerns / deviations summary

The plan is solid; spec→plan mapping is faithful and round-1 reviewer's findings are folded. Concerns are about (a) the SHA-pin-deferral red-test window (CRIT-1), (b) sed scope (CRIT-2), (c) judgment-heavy Phase 5b (CRIT-3), (d) missing Phase 4/7 reviews (IMP-6), (e) catch-all error kind decision (IMP-7), (f) Tag::Bare negative-vector ripple (IMP-8), (g) byte-order test (§6.3).

None blocking. Status `DONE_WITH_CONCERNS`.

## Nits and nice-to-haves (collect for FOLLOWUPS)

Most overlap with inline fixes above. The truly-deferred items:

- **`v06-plan-targeted-decoder-arm-tests`**: add 5-7 targeted Phase 3 unit tests for individual decoder arms to catch round-trip-stable-but-format-changed bugs the corpus alone cannot. ~30 min effort.

End of review.
