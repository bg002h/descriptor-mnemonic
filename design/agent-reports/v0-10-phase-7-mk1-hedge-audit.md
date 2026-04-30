# v0.10.0 Phase 7.11 — mk1 hedge audit

**Date:** 2026-04-29
**Phase:** 7.11 (sibling-repo cross-update for md-codec v0.10.0 ship)
**Sibling repo:** `/scratch/code/shibboleth/mnemonic-key`
**md-codec ship reference:** [release tag md-codec-v0.10.0](https://github.com/bg002h/descriptor-mnemonic/releases/tag/md-codec-v0.10.0); merge commit `172830a` on `bg002h/descriptor-mnemonic`.

## Outcome at a glance

- **Status:** completed; mk1 PR opened (not merged).
- **mk1 worktree:** `/scratch/code/shibboleth/mk-v010-cross-update` (branch `feature/md-codec-v0.10-shipped-cross-update` cut from `origin/main`).
- **mk1 commit:** `8fde50e` — `docs(post-md-v0.10.0): de-hedge md1 path-tag references` (single commit).
- **mk1 PR:** https://github.com/bg002h/mnemonic-key/pull/2 — open for human review.
- **Files touched in mk1:** 6 (BIP + DECISIONS + FOLLOWUPS + IMPLEMENTATION_PLAN + SPEC + closure-design); +12 / −10 lines.
- **BIP "Authority precedence" drift:** none — md1's §"Authority precedence with MK" and mk1's §"Authority precedence (MK ↔ MD path information)" agree on all four normative claims and md1 explicitly cross-refers to mk1 as the source.

## File-by-file audit + edit summary

### 1. `bip/bip-mnemonic-key.mediawiki`

- **Line 364 — BIP §"Authority precedence (MK ↔ MD path information)" lead sentence.** Hedge wording: `MD's wire format is being extended to optionally carry per-@N origin paths (a separate cross-repo follow-up; this BIP does not pin the MD-side wire format).`
- **Edit:** rewrote to past tense citing the `Tag::OriginPaths = 0x36` allocation and the v0.10.0 release link, plus a pointer to MD's BIP §"Per-`@N` path declaration".

### 2. `design/SPEC_mk_v0_1.md`

- **Line 326 — SPEC §5.1 trailing parenthetical.** Hedge wording: `(The md1-side wire-format question — which tag byte md1 allocates for per-@N paths — is tracked as md-per-N-path-tag-allocation in FOLLOWUPS.md and is an md-repo decision.)`
- **Edit:** rewrote to cite the shipped allocation (`Tag::OriginPaths = 0x36` + header bit 3 reclaim) and link the v0.10.0 release.
- **Line 375 — Q-4 closure summary table row.** Hedge wording: `md1 tag-byte allocation deferred to descriptor-mnemonic repo` (§5.1 + FOLLOWUPS).
- **Edit:** rewrote to `md1 tag-byte allocation shipped as Tag::OriginPaths = 0x36 in md-codec v0.10.0` and dropped the `+ FOLLOWUPS` reference suffix from the section column.

### 3. `design/DECISIONS.md`

- **Line 209 — Q-4 closure summary table row.** Same hedge wording as SPEC line 375.
- **Edit:** same past-tense rewrite as SPEC §5.1 + table; the parenthetical FOLLOWUPS reference removed (the entry is now resolved).

### 4. `design/FOLLOWUPS.md`

- **Lines 55–62 — `md-per-N-path-tag-allocation` entry.** Status was `mk1-side complete; awaiting md1`.
- **Edit:** flipped Status to `resolved by md-codec-v0.10.0` ([release link](https://github.com/bg002h/descriptor-mnemonic/releases/tag/md-codec-v0.10.0), merge commit `172830a`) following the closure pattern used by the analogous `chunk-set-id-rename` and `path-dictionary-mirror-stewardship` entries (both resolved by md-codec-v0.9.0). Resolution note documents the wire-format change (Tag::OriginPaths = 0x36 + header bit 3 reclaimed) and the cross-update scope (BIP §"Authority precedence" + SPEC §5.1 + DECISIONS Q-4 + closure-design §Q-4 + §3 item (2) all updated past-tense). Renamed the heading suffix from no-marker to `(resolved)` matching the sibling pattern. The body's `What:` and `Why deferred:` clauses left intact as historical context (matching the `chunk-set-id-rename` precedent which keeps its `Where:` and `Sequencing requirement:` lines intact post-resolution).

### 5. `design/IMPLEMENTATION_PLAN_mk_v0_1.md`

- **Line 1119 — Step 8.3.1 (Cross-repo coordination follow-up).** Open task: `After mk-codec v0.1.0 ships, file an issue / PR / message to the descriptor-mnemonic repo for the chunk-set-id-rename and md-per-N-path-tag-allocation follow-ups`.
- **Edit:** flipped checkbox to `[x]` with strikethrough on the original text and a "Resolved upstream" note pointing at md-codec v0.9.0 (chunk-set-id-rename) and md-codec v0.10.0 (md-per-N-path-tag-allocation as Tag::OriginPaths = 0x36).

### 6. `docs/superpowers/specs/2026-04-29-mk1-open-questions-closure-design.md`

- **§Q-4 lines 71–81 — `Downstream:` clause.** Hedge wording: `The actual md1 wire-format change (tag-byte allocation) is deferred to the descriptor-mnemonic repo.`
- **Edit:** rewrote to past tense citing the `Tag::OriginPaths = 0x36` allocation, the header bit 3 reclaim, and the v0.10.0 release link. Annotated the §3 cross-repo coordination back-pointer as "(now-resolved)".
- **§3 item (2) line 364.** Hedge wording: `belongs to descriptor-mnemonic's next phase`.
- **Edit:** rewrote the paragraph as a `(resolved)` heading suffix + an explicit Resolution paragraph citing md-codec v0.10.0 + merge commit 172830a + the `Tag::OriginPaths = 0x36` allocation. The `Fresh-eyes finding` paragraph at line 75 left intact as a historical thinking-snapshot from the closure-design pass (matches the precedent set in §3 item (1) for `chunk-set-id-rename`, where the `Resolution:` paragraph was added below the original closure prose rather than rewriting the whole section in place).

## Sites deliberately left unchanged

- `design/AUDIT_bip_cross_reference_completeness.md` lines 66, 68, 152, 175 — these are historical audit-fix records documenting the prior cleanup of `chunk_set_id` "in flight" wording. The audit report is a closed historical artifact.
- `design/agent-reports/v0-1-phase-1-review-7830edd.md`, `v0-1-phase-2-review-4728230.md`, `v0-1-plan-review-1.md` — phase-review reports are immutable historical artifacts. Their hedging language reflected the state at review time and stays as-is.
- `design/FOLLOWUPS.md` line 59 (`md-repo decision` inside the `What:` clause of the now-resolved entry) — kept inside the entry body as historical framing; the `Status:` line above it carries the resolution. This matches the resolved `chunk-set-id-rename` entry which also keeps its body text in the original form.
- `closure-design.md` line 75 (`md-repo concern` inside the `Fresh-eyes finding:` paragraph of §Q-4) — closure-design `Fresh-eyes finding:` paragraphs are historical thinking-snapshots from the 2026-04-29 design pass; resolution is documented in the `Downstream:` clause edit and the §3 item (2) edit. Matches the precedent set during the `chunk-set-id-rename` cross-update.
- `closure-design.md` lines 20, 41 — `Future format extensions` references the general design-pattern of `shibbolethnums<suffix>`-style domain strings, not md1's per-`@N` work. Out of scope.

## BIP authority-precedence drift check (Step 7.11.c)

mk1 BIP §"Authority precedence (MK ↔ MD path information)" (lines 362–369 on `feature/md-codec-v0.10-shipped-cross-update`) and md1 BIP §"Authority precedence with MK" (lines 438–440 on main, post-PR-#12 merge `172830a`) compared:

| Normative claim | mk1 BIP wording | md1 BIP wording | Match? |
|---|---|---|---|
| MK origin_path authority | "MK's `origin_path` is **authoritative**" | "MK's `origin_path` is **authoritative**" | yes |
| MD per-`@N` role | "the policy's *expected* path. (...descriptive — the slot's intended derivation shape.)" | "**descriptive** — the policy's expected path layout" | yes |
| Per-format decoders not cross-aware | "Per-format decoders are not required to be aware of cross-format context; the cross-format consistency check belongs to the orchestrator layer that sits above both decoders." | "Per-format decoders are not required to be aware of cross-format context; consistency-checking is the recovery orchestrator's responsibility" | yes |
| Mismatch → orchestrator MUST reject | "Mismatch MUST cause **the recovery orchestrator** to reject the assembly." | "mismatch MUST cause the orchestrator to reject the assembly." | yes |
| Precise-error UX requirement | "Implementations MUST surface a precise error identifying both the policy-side expected path and the key-side actual path so the user can diagnose which card's path information is wrong." | (delegated to mk1's BIP via cross-reference: "See MK's BIP §"Authority precedence (MK ↔ MD path information)" for the full normative semantics") | yes (md1 defers to mk1) |

md1's heading slug `Authority precedence with MK` differs from mk1's `Authority precedence (MK ↔ MD path information)` — md1's prose links to the latter form (`MK's BIP §"Authority precedence (MK ↔ MD path information)"`), and mk1's heading still emits exactly that anchor. The cross-reference resolves correctly.

**Result: no drift.** No controller action required.

## Self-review

- ✅ All hedging references swept (or deliberately left as historical artifacts per the precedent set during the v0.9.0 cross-update).
- ✅ mk1 FOLLOWUPS entry status flipped to `resolved by md-codec-v0.10.0`.
- ✅ BIP "Authority precedence" drift checked — none.
- ✅ This hedge-audit report persisted to md1 repo at `design/agent-reports/v0-10-phase-7-mk1-hedge-audit.md`.
- ✅ mk1 PR opened (https://github.com/bg002h/mnemonic-key/pull/2), not merged — left for human review.
- ✅ mk1 worktree at `/scratch/code/shibboleth/mk-v010-cross-update` left in place per Step 7.11.f instructions; `git worktree list` confirms registration.

## Process notes for future cross-updates

The pattern established in v0.9.0 and refined here:

1. Use a worktree off `origin/main` to avoid disturbing in-flight work in mk1's primary checkout.
2. Update *prose framing* (temporal/availability) without touching *normative semantics*.
3. Keep historical `What:` / `Why deferred:` / `Fresh-eyes finding:` clauses intact in resolved FOLLOWUPS entries and closure-design docs — only the `Status:` / `Downstream:` / `Resolution:` lines shift to past tense.
4. Add a `(resolved)` suffix to the entry heading matching the `chunk-set-id-rename` precedent.
5. Cross-reference the resolution note in the agent-reports/ artifact (this file) so future audits can trace the cleanup.
6. Open the mk1 PR but do not merge — the user owns merge timing.

The third FOLLOWUPS-entry pattern (`md-per-N-path-tag-allocation` joins `chunk-set-id-rename`, `md-path-dictionary-0x16-gap`, `path-dictionary-mirror-stewardship` as the cross-repo entries that resolved on the md1 side without an mk1 wire-format change) confirms that the closure-design's framing of "mk1 declares semantics; md1 owns wire format" worked as intended: zero mk1-side wire-format edits needed across all four resolutions.
