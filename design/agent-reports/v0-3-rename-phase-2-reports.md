# Phase 2 (BIP rename) — agent reports

**Phase**: BIP/Spec rename — `wdm` → `md` rename project, v0.3.0
**Branch**: `rename/v0.3-wdm-to-md`
**Commit**: `30f880e` — `spec(bip): rename Wallet Descriptor Mnemonic → Mnemonic Descriptor; recompute HRP-expansion for md`
**Date**: 2026-04-27

---

## Implementer report (Sonnet)

**Status**: DONE_WITH_CONCERNS

**What was implemented**: Phase 2 BIP/Spec rename per `design/IMPLEMENTATION_PLAN_v0_3_rename.md` §"Phase 2". File renamed via `git mv`; all prose rewritten WDM→MD throughout; HRP-expansion bytes recomputed for `md` (length 5); `wdm-codec` → `md-codec` in code blocks and vector paths; HRP note updated from "three-character" to "two-character" and HRP+separator from 4 to 3 chars; "Why a new HRP?" rationale expanded for `md`; TODO Phase 6 marker inserted at the v0.2.json SHA.

**Verification results** (all greps run in worktree):

1. `grep -nE 'wdm|WDM' bip/bip-mnemonic-descriptor.mediawiki` — ONE match (line 744): the literal JSON `generator` field value `"wdm-codec 0.2"` quoted while describing the frozen v0.2.x vector files. Intentionally preserved: changing it would falsely describe those historical files.
2. `grep -niE 'wallet descriptor mnemonic'` — EMPTY (PASS)
3. New HRP bytes `[3, 3, 0, 13, 4]` — present at line 132 (PASS)
4. Old HRP bytes `[3, 3, 3, 0, 23, 4, 13]` — EMPTY (PASS)
5. `grep 'TODO Phase 6'` — one match at line 743 (PASS)

**Files changed**: `bip/{bip-wallet-descriptor-mnemonic.mediawiki → bip-mnemonic-descriptor.mediawiki}` — 1 file, 59 insertions, 57 deletions; rename at 83% similarity.

**Self-review findings**: The Length envelope section had "HRP + separator: 4 characters" in two places (single-string and chunked) — updated both to 3 (HRP `md` is 2 chars + 1 separator). The capacity math formulas (93 − header − checksum) are correctly unchanged because BCH polymod length limits apply to the data part only, not the total string including HRP.

**Concerns**: (1) The retained `"wdm-codec 0.2"` quote on line 744 is factually correct — describes the literal JSON content of the frozen v0.2.x files. Surrounding sentence clarifies the v0.3.0 change. Correct treatment per HISTORICAL tagging.

---

## Spec compliance review (Sonnet)

**Verdict**: ✅ SPEC COMPLIANT

**Verifications passed**: all 15 from the spec-reviewer checklist (rename via `git mv` confirmed via `git show 30f880e --stat` + `git log --follow`; HRP-expansion bytes correct with derivation; old bytes excised; only one intentional `wdm` retention at line 744; title heading is "Mnemonic Descriptor"; abstract reads naturally; TODO marker preserves OLD SHA + flags Phase 6; length-envelope arithmetic updated in two places to "3 characters"; only the BIP file touched; no historical sections rewritten; "Why a new HRP?" stays factual; derivation reproducible from `ord('m')`/`ord('d')`).

**Concerns flagged for controller**:
- The "preliminary HRP" disclaimer at line 93 reads oddly alongside the confident collision-vet prose at line 662. Not a Phase 2 defect — flag for finalization. → **Captured as FOLLOWUPS item `bip-preliminary-hrp-disclaimer-tension`.**
- The intentional `"wdm-codec 0.2"` retention at line 744 is correctly justified in the commit message.

---

## Code quality review (superpowers:code-reviewer, Sonnet)

**Assessment**: APPROVED

**Strengths**:
- Git history preserved (`git log --follow` shows 5 commits carried through the rename).
- Change is strictly substitutional, not additive — net growth of exactly 2 lines (TODO comment + explanatory sentence).
- HRP-expansion derivation independently reproducible by Python.
- Length-envelope arithmetic consistent across both updated sites.
- `wdm-codec 0.2` retention appropriately contextualized — line 744 explicitly states it's the historical/frozen generator field and names the successor token.
- All `§"..."` cross-references intact; no `[[#anchor]]` wiki-link syntax used anywhere, so no broken-anchor risk.
- No marketing language; rationale stays factual.

**Issues**:
- **Critical**: None.
- **Important**: None.
- **Minor (1)**: Line 93 disclaimer ("preliminary HRP, subject to change") in tension with line 662 collision-vet claim ("verified clean against six registries prior to adoption"). Not contradictory but reads awkwardly. → **FOLLOWUPS item `bip-preliminary-hrp-disclaimer-tension`** (same as spec-reviewer flagged).
- **Minor (2)**: Phase-6 TODO comment leaks internal phase nomenclature into the spec; auto-resolves when Phase 6 replaces it. Acceptable.

**Verdict**: APPROVED. Both minor notes are pre-existing tensions or intentional placeholders, not Phase 2 errors. No blocking issues.

---

## Phase 2 closure

✅ Both reviews passed. One minor item filed in FOLLOWUPS (`bip-preliminary-hrp-disclaimer-tension`). Phase 3 unblocked.
