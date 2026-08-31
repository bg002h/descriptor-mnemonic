# REVIEW — SPEC_mdcli_mini.md, mechanical fold-check of the r3 fold

| field | value |
| --- | --- |
| artifact | `design/SPEC_mdcli_mini.md` |
| commit | `d72728a1` |
| date | 2026-08-31 |
| reviewer | independent agent, mechanical fold-verification only |
| repo | `/scratch/code/shibboleth/descriptor-mnemonic` |
| scope | `git diff 8b592a43..d72728a1 -- design/SPEC_mdcli_mini.md` against `design/agent-reports/REVIEW-mdcli-mini-spec-r3.md` findings I-1, M-1, N-1. No fresh audit, no re-derivation of settled rounds (r1/r2/r3 Parts 1-2 not re-checked). |

## Method

Read each r3 finding verbatim, read the corresponding hunk in the fold diff,
judged whether the edit addresses what the finding says, machine-checked the
two code citations the fold introduces, then read surrounding unedited spec
text (lines 40-230, 380-403 at `d72728a1`) for contradiction, and grepped the
whole file for stale pre-fold phrasing.

## Findings

| finding | verdict | evidence |
| --- | --- | --- |
| **I-1** (Important — R-N1d row-pinned on T-row only; card route composes the delta at exit 0, no named row can fail) | **FIXED** | New Vectors clause (`SPEC:209-213`): "R-N1d CARD-INPUT refusals — `descriptor` and `address` on a minted delta card refuse, mirroring R-N1a's card-input rows (r3 I-1: both measured at exit 0 today; the card branch at `build.rs:69-77` runs no reuse check, and `refuse_key_reuse_across_slots` has its one call site inside the `--template` branch)". Both verbs named, on a minted (card-input) delta card, as refusal rows — the exact gap the finding cites (R-N1a already had this pair of rows, R-N1d did not). `build.rs:69-77` cite verified: `decode_md1_string`/`reassemble` at lines 73/76, no reuse check present. |
| **M-1** (Minor — single-source exemption covers the codec floor but not the S row's `check_no_repeated_xpub`, whose only mention the fold deleted) | **FIXED** | New clause (`SPEC:180-186`): "The S row's shipped `check_no_repeated_xpub` (`seat/satisfy.rs:294`) is a third, card-set-side implementation of the Family-2 predicate; it stays as shipped (r3 M-1) — the single-source rule binds the mint/compose surface, and T-row/S-row PARITY (each side refuses the wallet the other refuses) is pinned behaviorally by the row set, not by code unification." Names the function, cites `seat/satisfy.rs:294` (machine-verified: `pub fn check_no_repeated_xpub` is exactly line 294), states it stays, and states parity is pinned by rows rather than by code unification — matches the finding's ask to restore the fact and scope convergence work off the S row. |
| **N-1** (Nit — Acceptance 5's "refused shapes" unqualified; the floor's shape is a refused shape the fold declared out of scope) | **FIXED** | Criterion 5 (`SPEC:399-402`) now reads "…complete at exit 0 on already-engraved cards carrying shapes THIS CYCLE newly refuses — row-pinned, per the C1 constraint. (r3 N-1: the pre-existing codec floor's shape is out of scope and reads per shipped behavior.)" — narrowed exactly as directed. |

## Contradiction check

- Grepped the full file for `refused shapes` (old Acceptance-5 wording): one
  hit remains, line 157, "newly-refused shapes" — a different, already-correct
  usage in the Placement-constraint section (unrelated to Acceptance 5). No
  stale full-file claim that Acceptance 5 covers all refused shapes.
- Grepped for `check_no_repeated_xpub`: one hit, the new M-1 clause itself —
  no orphaned/contradicting second mention.
- Grepped for R-N1d template-path-only phrasing (`template.path`, `T-row
  only`, `template only`, `only.*template`): no hit asserting R-N1d rows are
  template-path-only. The one `template` hit nearby (line 169, "in hand at
  `parse_template_ext` time on the template path") is the unrelated
  classifier-input discussion, not a claim about which rows exist.
- Read the unedited dispositions block (`SPEC:190-191`, unchanged since r1/r2
  fold): "REFUSE (mint/compose): `encode`; `descriptor` (both `--template`
  and card input); `address` (both inputs)." The new I-1 card-input row is
  consistent with this pre-existing disposition, not a new requirement
  invented by the fold — the fold closes the gap between this line and the
  Vectors section, it does not contradict it.
- Read the unedited codec-floor paragraph (`SPEC:152-164`, Placement
  constraint / C1) against the new M-1 clause: the floor
  (`validate_no_duplicate_key_slots`, inside `encode_payload`) and the S-row
  floor (`check_no_repeated_xpub`, seat-side) are two distinct, separately
  named exemptions from different rules (C1 vs. single-source) — no overlap,
  no double-counting.
- Read the unedited Family-2 definition (`SPEC:87-112`) against the new
  I-1/M-1 text: "Mint/compose refuses the delta" (line ~106, unchanged)
  states the general requirement the new CARD-INPUT row now makes testable
  on the card route; no conflict.

No contradictions found.

## Verdict

r3 findings: 3/3 FIXED; contradictions: 0
