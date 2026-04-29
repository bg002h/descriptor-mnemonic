# v0.5 multi-leaf TapTree — final cumulative reviewer (Opus 4.7)

**Status:** DONE
**Subject:** 15-commit branch `feature/v0.5-multi-leaf-taptree` (origin/main..HEAD)
**Tip:** `a77c914 test(v0.5 phase 8): CLI integration test for multi-leaf TapTree`
**Source spec:** `design/SPEC_v0_5_multi_leaf_taptree.md` (commit `7ef7cec` on main)
**Plan:** `design/IMPLEMENTATION_PLAN_v0_5_multi_leaf_taptree.md` (commit `c118d41` on main)
**Reviewer:** Opus 4.7, cumulative read of all 15 commits as a unit
**Verdict:** **READY-WITH-MINOR-FIXES**

The work is technically complete and correct. Spec §1–§5 are end-to-end traceable to landed code/tests/docs. §6 migration content (CHANGELOG, MIGRATION) is correctly Phase 10's track; nothing in the §6 required-now slice is missing. Wire-format-additive promise is upheld at the byte level — single-leaf and KeyOnly `tr` paths are preserved verbatim (no detour through `decode_tap_subtree`/`encode_tap_subtree`), and all 8 v0.4.x tap-context tests + all 51 conformance tests round-trip green. Hostile-input hardening (H1/H2/H4) executes the spec §1 decision-(B) gate exactly: peek before consume; gate is `depth > 128`; recursion bomb (10 K bytes) rejects cleanly with `PolicyScopeViolation` well before stack overflow.

Six minor doc/housekeeping items below need controller-level inline fixes before tag. None are correctness blockers; all are scoped to single-line edits or git-add operations.

## Verification — gates green

- `cargo test --workspace --no-fail-fast`: **634 passed; 0 failed; 0 ignored** (matches the controller-asserted state; +25 over the 609 baseline; spec target ≥638 was speculative — Phase 6 implementer documented why the actual count landed at 633 + Phase 8's CLI test = 634).
- `cargo run --bin gen_vectors -- --verify <v0.1.json>`: **PASS** (10 positive, 30 negative).
- `cargo run --bin gen_vectors -- --verify <v0.2.json>`: **PASS** (27 positive, 51 negative).
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`: PASS (asserted by controller; not re-runnable here because the worktree's pinned nightly toolchain lacks `rustfmt`/`clippy` and this reviewer is read-only).

## Spec coverage — §1–§6 traceability

| Spec § | Required at v0.5.0 | Landed in | Status |
|---|---|---|---|
| §1 scope (a) pure admission | `validate_tap_leaf_subset` unchanged in semantics | `encode.rs:497` (sig change only — added `leaf_index` plumbing per §4) | OK |
| §1 depth ceiling 128 | `depth > 128` gate at `decode_tap_subtree` | `decode.rs:776` exact match to spec §3 snippet | OK |
| §1 hostile-input (B) peek-before-recurse | `peek_byte()` before `read_byte()` + gate | `decode.rs:768-781` matches spec §3 verbatim | OK |
| §2 wire format `[Tr][Placeholder][k][TapTree=0x08][LEFT][RIGHT]` | Encoder `encode_tap_subtree` + decoder `decode_tap_subtree` | `encode.rs:546-567`, `decode.rs:762-810` | OK |
| §2 single-leaf and KeyOnly byte-identical | Single-leaf path preserved verbatim, no detour | `encode.rs:150-154` (carve-out `leaves.len()==1 && leaves[0].0==0`); `decode.rs:281-297` (peek first, dispatch v0.4 path inline if not `0x08`) | OK |
| §3 decoder helper signature + body | `decode_tap_subtree(cur, keys, depth, leaf_counter)` | `decode.rs:762-810` | OK |
| §3 leaf-index propagation | DFS pre-order via `leaf_counter`; threaded into `validate_tap_leaf_subset` | `decode.rs:288, 295, 798` | OK |
| §4 encoder helper signature + body | `encode_tap_subtree(leaves, cursor, target_depth, out, ph_map)` | `encode.rs:546-567` | OK |
| §4 multi-leaf detection `len()==1 && depth==0` | Single-leaf carve-out tight | `encode.rs:150` | OK |
| §4 `Error::TapLeafSubsetViolation { operator, leaf_index }` | Variant marked `#[non_exhaustive]`; field added | `error.rs:325-333` | OK |
| §4 `DecodeReport.tap_leaves: Vec<TapLeafReport>` (NEW field) | Added under `#[non_exhaustive]` | `decode_report.rs:154` | OK |
| §4 `TapLeafReport` (NEW public struct) | `leaf_index, miniscript, depth` exactly | `decode_report.rs:103-113` | OK |
| §4 `tap_leaves` populated for all `tr` decodes | DFS pre-order `enumerate()` walk | `decode.rs:226-241` (`build_tap_leaves`) | OK |
| §4 lib.rs re-export `TapLeafReport` | Added | `lib.rs:156-158` | OK |
| §4 BIP doc inventory (admit list, §"Taproot tree", tag table, FAQ, fixtures) | All sections updated | `bip-mnemonic-descriptor.mediawiki` lines 84-87, 391-392, 540-548, 834-859, 915-923 | OK (one minor wording miss — see M3) |
| §5 T1-T7 positive | 7 fixtures landed | `vectors.rs:226-278` (TAPROOT_FIXTURES) | OK |
| §5 N1-N9 negative | 9 fixtures landed | `vectors.rs:1592-1793` | OK |
| §5 H1-H5 hostile inline | 5 tests | `tests/v0_5_taptree_hostile.rs` | OK |
| §5 RT1-RT4, LI1-LI3, PR1-PR2 | 9 tests | `tests/v0_5_taptree_roundtrip.rs` | OK |
| §5 generator-token bump 0.4→0.5 + SHA roll | DEFERRED to Phase 11 (Cargo.toml is still `0.4.1`; family token currently emits `"md-codec 0.4"`) | `vectors.rs:705-710` (uses `CARGO_PKG_VERSION_MAJOR.MINOR`) | EXPECTED — Phase 11 task 11.1 |
| §6 wire-format-additive | Verified — single-leaf + KeyOnly byte-identical, all v0.4.x positives still round-trip | `tests/taproot.rs` 8/8, `tests/conformance.rs` 51/51 | OK |
| §6 CHANGELOG + MIGRATION | DEFERRED to Phase 10 | — | EXPECTED |
| §6 release sequencing (10 of 11 phases done) | Phases 2-8 done, Phase 9 in progress (this review), Phase 10/11 pending | per task list | OK |

## Wire-format-additive promise — verification

Re-read both code paths end-to-end:

- **Decoder** `decode_tr_inner` (`decode.rs:269-304`): peeks the byte after the placeholder; if it equals `Tag::TapTree.as_byte()` the v0.5 multi-leaf helper is invoked; **otherwise the v0.4.x single-leaf path is followed inline** with `decode_tap_miniscript(cur, keys, Some(0))` and `validate_tap_leaf_subset(&leaf, Some(0))`. The `Some(0)` is the only difference from v0.4.x and is plumbed through `validate_tap_leaf_subset`'s now-required `leaf_index` parameter. Decoded bytes are identical at every position; only the diagnostic-side index is gained. KeyOnly path (`cur.is_empty()`) returns `None` exactly as before.
- **Encoder** `Descriptor::Tr` arm (`encode.rs:132-175`): `len() == 1 && leaves[0].0 == 0` carve-out invokes `validate_tap_leaf_subset(leaf_ms, Some(0))` then `leaf_ms.encode_template(...)` — byte-identical to v0.4.x. KeyOnly omits the leaf entirely (no `0x08` written). Multi-leaf is the only path that emits `Tag::TapTree.as_byte()`.

All 8 of `tests/taproot.rs` (KeyOnly, single-leaf pk, single-leaf multi_a, nested-subset round-trip, etc.) and the 12-fixture v0.4 conformance suite (`tests/conformance.rs` S1-S4, M1-M3, Cs) pass without modification. Single-leaf v0.4 fixture `tr_pk` (RENAMED at v0.5 to `tr_single_leaf_pk_md_v0_5`) carries bytecode-unchanged annotation in `vectors.rs:236`.

## Hostile-input hardening — verification

| Fixture | Construction | Decoded by | Result |
|---|---|---|---|
| H1 (legal 128) | 128 framings + 1 bottom leaf + 128 right leaves | `decode_template` | Decodes; max miniscript-depth = 128 (BIP 341 boundary). Gate `depth > 128` does NOT fire because at recursion-depth 128 the next byte read is a leaf, not a `0x08`. |
| H2 (illegal 129) | 129 framings + leaves | `decode_template` | `PolicyScopeViolation("TapTree depth exceeds BIP 341 consensus maximum (128)")`. Gate fires at recursion-depth 129. |
| H3 (truncated) | `[Tr][Placeholder][0][TapTree]` then EOF | `decode_bytecode` | `InvalidBytecode { kind: UnexpectedEnd or Truncated }` |
| H4 (recursion bomb) | 10 000 `[TapTree]` bytes, no leaves | `decode_template` | `PolicyScopeViolation` at depth 129 — verified to reject WITHOUT stack overflow |
| H5 (unknown at depth) | `[Tr][Placeholder][0][TapTree][TapTree][0xff]` | `decode_template` | `InvalidBytecode { kind: UnknownTag }` |

The decoder's peek-before-recurse pattern at `decode.rs:768-781` is the spec-exact (B) defense. `cur.read_byte()` to commit-consume `0x08` happens AFTER `Tag::from_byte` matches `Some(TapTree)` BUT BEFORE the depth check (per spec §3 line 147 — intentional, so `cur.offset()` reports the next-frame byte for diagnostic tooling, matching the v0.4 Sh restriction matrix precedent).

## FOLLOWUPS audit

Open items relevant to v0.5 ship:

| ID | Tier | Verdict |
|---|---|---|
| `v0-5-spec-plan-encode-tap-subtree-entry-depth-bug` | v0.5-must-close-before-ship | Filed correctly. Doc-only fix on `main` (spec line 220 + plan line 1325 say `target_depth=1`; working code at `encode.rs:166` correctly uses `0`). Plan Task 11.5 / 11.6 is the catch-up window; the controller should fold the literal `1` → `0` patch as part of Phase 10/11. **Not blocking Phase 9 verdict.** |
| `v0-5-stale-v0-4-message-strings-sweep` | v0.5-nice-to-have | RESOLVED in commit `bca2804` (Phase 4); moved to Resolved items in `766c580`. Confirmed: zero hits for `"v0\.4 does not support"\|"reserved for v1\+"` in `encode.rs` and `decode.rs`. |
| `v0-5-t7-chunking-boundary-misnomer` | v0.5-nice-to-have | Open. T7's 6-leaf right-spine encodes well below the chunking boundary; fixture name suggests boundary coverage that isn't actually exercised. Pragmatic ship-now path is rename to `tr_multi_leaf_right_spine_md_v0_5`; fully-correct path is to retune the tree. **Not blocking — fixture still adds asymmetric multi-leaf coverage.** |
| `rust-miniscript-multi-a-in-curly-braces-parser-quirk` | v1+ external | Filed correctly as upstream issue. Workaround (use `@N` template form) is in place; not blocking md-codec v0.5. |
| `v0-5-multi-leaf-taptree` | v0.5+ (this release) | Open — expected; closes on Phase 11 commit per plan Task 11.2. |

## Per-phase implementer reports

All 7 reports are present at the expected locations:

```
design/agent-reports/v0-5-multi-leaf-phase-2-implementer.md   (untracked)
design/agent-reports/v0-5-multi-leaf-phase-3-implementer.md   (committed in 9548286)
design/agent-reports/v0-5-multi-leaf-phase-4-implementer.md   (committed in c450a9e)
design/agent-reports/v0-5-multi-leaf-phase-5-implementer.md   (untracked)
design/agent-reports/v0-5-multi-leaf-phase-6-implementer.md   (untracked)
design/agent-reports/v0-5-multi-leaf-phase-7-implementer.md   (untracked)
design/agent-reports/v0-5-multi-leaf-phase-8-implementer.md   (untracked)
```

5 of 7 are present in the worktree but not yet committed. **This is a Phase 11 housekeeping item — they need to be `git add`ed and committed before the release tag** so the audit trail in `design/agent-reports/` is complete in `main`'s history. See M1 below.

## Out-of-scope additions

None found. Every code/test/doc change is traceable to a spec §1–§6 requirement or to a FOLLOWUPS-tracked item that was resolved during execution. The two FOLLOWUPS-derived inline edits (stale-strings sweep folded into Phase 4 commit `bca2804`; the doc-bug FOLLOWUPS entry filed in `04481fa`) are documented in their respective implementer reports.

## Cross-cutting issues found

These emerged only from reading the 15-commit diff as a single unit. None are correctness-blocking; all are minor.

### M1 — 5 implementer reports untracked

**Where:** worktree `design/agent-reports/`. Phase 2, 5, 6, 7, 8 reports are present but `git status` shows them as untracked. Phase 3 and Phase 4 reports were committed (`9548286`, `c450a9e`) but the cadence wasn't carried forward.

**Fix:** at Phase 11 (or just before tag), `git add design/agent-reports/v0-5-multi-leaf-phase-{2,5,6,7,8}-implementer.md && git commit -m "chore(v0.5): persist Phase {2,5,6,7,8} implementer reports"`. Per memory `feedback_subagent_workflow` and the plan's "Audit trail" expectations at lines 447-451, all per-phase implementer reports must land in `main`'s history.

### M2 — Spec/plan `target_depth=1` literal disagrees with working code (`target_depth=0`)

**Where:** `design/SPEC_v0_5_multi_leaf_taptree.md` line 220; `design/IMPLEMENTATION_PLAN_v0_5_multi_leaf_taptree.md` line 1325 (per FOLLOWUPS entry). Both files live on `main`, not on the feature branch.

**Why this matters:** A future reader cross-referencing the spec to the implementation will hit a contradiction at the entry call. The working code `encode.rs:166` uses `0` (correct); spec/plan say `1` (incorrect; would short-circuit emission for symmetric depth-1 trees and trip the `debug_assert_eq!(cursor, leaves.len())` post-condition).

**Fix:** per FOLLOWUPS tier `v0.5-must-close-before-ship`, controller should patch the literal `1` → `0` in spec line 220 and plan line 1325 (both on `main`) as part of the Phase 10/11 release PR.

**Concrete edit suggestion (spec §4, line 220):**

```diff
-                encode_tap_subtree(&leaves, &mut cursor, 1, out, placeholder_map)?;
+                encode_tap_subtree(&leaves, &mut cursor, 0, out, placeholder_map)?;
```

Plan line 1325 takes the same one-character change.

### M3 — BIP doc misnames N9 as "depth-overflow"

**Where:** `bip/bip-mnemonic-descriptor.mediawiki` line 923.

**Issue:** Line 923 reads "...`n_taptree_unknown_tag_inner` (N8), and depth-overflow (N9) — decoder rejects each with...". The actual N9 fixture in the corpus is `n_taptree_at_top_level` (top-level dispatcher rejection of `0x08`), per spec §5 table at line 325 ("N9 | `n_taptree_at_top_level` | `[TapTree]` as top-level descriptor (no `Tr` outer)"). The implementation in `vectors.rs:1764-1793` matches the spec, not the BIP doc. There is no depth-overflow fixture in the JSON corpus — depth checks are exercised exclusively by the inline H2 / H4 hostile tests.

**Fix:** Phase 10 BIP doc patch (single-line edit in commit at Phase 10 or as part of release prep):

```diff
- ...<code>n_taptree_unknown_tag_inner</code> (N8), and depth-overflow (N9) — decoder rejects each with...
+ ...<code>n_taptree_unknown_tag_inner</code> (N8), and <code>n_taptree_at_top_level</code> (N9; <code>Tag::TapTree</code> as top-level descriptor) — decoder rejects each with...
```

This was not flagged by the Phase 7 implementer or Phase 7 reviewer (Phase 7 reviewer caught the dangling cross-reference fixed in `5c703b7` but missed this one). The misnomer is doc-only; the test corpus is correct.

### M4 — `decode.rs:63` doc says "v0.4 accepts" — stylistic

**Where:** `decode.rs:63-64` doc comment on `decode_descriptor`: "v0.4 accepts `Tag::Wsh`, `Tag::Tr`, `Tag::Wpkh`, and `Tag::Sh`."

**Issue:** This is technically still true (v0.5 accepts the same set; the change is INSIDE the `tr` body), so it's not factually wrong. But after a v0.5 release, version-prefixed claims read awkwardly when v0.5 is current. The `v0-5-stale-v0-4-message-strings-sweep` FOLLOWUPS only targeted strings that actively contradicted v0.5 behavior; this comment doesn't, so it correctly wasn't included. Listing here so the controller can choose to relax the prefix to "Top-level descriptors accepted: `Tag::Wsh`..." (one-line rustdoc edit) if desired. **Not blocking.**

Same applies to `decode.rs:74-75`, `decode.rs:136`, `decode.rs:143`, `decode.rs:948`, `decode.rs:2100`, `decode.rs:2117`, `decode.rs:2213` — all describe historical scope at the version when introduced and are factually correct.

### M5 — 634 vs ≥638 spec target

**Where:** spec §6 line 455 ("≥638 tests passing + 0 ignored").

**Issue:** Actual count is 634. Phase 6 implementer report (line 99) explained why the speculative 638 floor wasn't hit: spec assumed +1 round-trip pair per new positive fixture, but the harness consumes new fixtures inside existing per-fixture-iterating tests rather than expanding into separate `#[test]` functions. All 14 spec-mandated new inline tests landed; coverage matches the §5 enumeration.

**Fix:** Phase 10 CHANGELOG / MIGRATION should report the actual final count rather than the speculative 638, and Phase 11 task 11.1 step 3's expected message should be updated from "≥638 tests" to "≥634 tests" (or the actual count post-version-bump). **Not blocking — this is a target adjustment, not a coverage gap.**

### M6 — Phase 7 dangling cross-reference fix landed correctly

**Where:** `bip-mnemonic-descriptor.mediawiki:540` (Phase 7 polish commit `5c703b7`).

**Verdict:** Fix landed cleanly. Verified: line 540 now reads "...detailed below in this section." (no more dangling reference to nonexistent §"Tapscript miniscript subset"). Listing this in the report as a positive confirmation that the Phase 7 reviewer's catch is resolved.

## Summary of minor fixes for the controller

The controller should fold the following inline before Phase 10's CHANGELOG/MIGRATION pass or as part of the Phase 11 release commit:

1. **M1** — `git add design/agent-reports/v0-5-multi-leaf-phase-{2,5,6,7,8}-implementer.md` and commit, completing the audit trail per plan lines 447-451.
2. **M2** — Edit spec line 220 + plan line 1325 (both on `main`): `target_depth=1` → `target_depth=0`. Closes FOLLOWUPS `v0-5-spec-plan-encode-tap-subtree-entry-depth-bug` (must-close-before-ship).
3. **M3** — Edit BIP doc line 923: replace "depth-overflow (N9)" with "`n_taptree_at_top_level` (N9; `Tag::TapTree` as top-level descriptor)". Doc/code parity.
4. **M5** — Adjust spec §6 line 455 + plan line 2643's "≥638" to the actual landed count after Phase 11's version bump (634 today; will be re-counted after Cargo.toml bump + gen_vectors regen). Phase 10 CHANGELOG should report the actual number.
5. (Optional, **M4**) — Relax "v0.4" version-prefixed rustdoc comments in decode.rs that are now historical (lines 63, 74, 75, 136, 143, 948, 2100, 2117, 2213). Not required for ship; choose at controller's discretion.
6. (Optional) — `v0-5-t7-chunking-boundary-misnomer` rename pass before tag (per the FOLLOWUPS entry's "ship-now path"): rename `tr_multi_leaf_chunking_boundary_md_v0_5` to `tr_multi_leaf_right_spine_md_v0_5`. This is wire-affecting (vector SHA changes), so the rename must happen WITH the Phase 11 generator-token bump or be deferred to v0.5.x.

## Verdict

**READY-WITH-MINOR-FIXES.** The implementation is correct, the spec is fully traced through code/tests/docs, the wire-format-additive promise holds at the byte level, and the hostile-input hardening matches the (B) decision precisely. The 6 minor fixes are housekeeping (audit trail, doc/code parity, target adjustment, optional rename) — none alter behavior. After the controller folds M1–M3 (the must-do trio), the branch is tag-ready.

This branch is the cleanest cumulative agent-driven release in this codebase's history: 15 commits, zero failed phases, two implementer-flagged deviations both correctly captured in FOLLOWUPS (stale-strings sweep — resolved; entry-depth doc-bug — open with proper tier), all gates green, no out-of-scope additions.
