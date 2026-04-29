# v0.9.0 P3 review (opus)

**Date:** 2026-04-29
**Commit:** abbec54
**Reviewer:** opus-4.7

## Summary

**Verdict: clean** (one optional MediaWiki style nit + two minor scope-of-doc observations the implementer can leave or absorb at discretion). Phase 3 is prose-only; nothing is wire-format-bearing or test-bearing, and the new artifacts read as accurate, internally consistent, and well-aimed at closing `path-dictionary-mirror-stewardship`.

## Findings

### N1 (nit, optional). MediaWiki link form for the mk1 cross-reference

`bip/bip-mnemonic-descriptor.mediawiki` line 362 introduces:

```
([https://github.com/bg002h/mnemonic-key])
```

In MediaWiki, the bracket-only form `[URL]` renders as an opaque numbered footnote (e.g., `[1]`), not as inline anchor text. Every other external link in this BIP uses the labeled form `[URL label]`:

- Line 40: `[https://www.rfc-editor.org/rfc/rfc2119 RFC 2119]`
- Line 995: `[https://github.com/bg002h/descriptor-mnemonic github.com/bg002h/descriptor-mnemonic]`
- Lines 1017–1021: BIP cross-references all use labeled form

Suggested: `[https://github.com/bg002h/mnemonic-key bg002h/mnemonic-key]` — matches house style and gives readers a visible target. Strictly stylistic; the bare form is valid wikitext.

### N2 (scope observation). Inverse-direction lockstep is not spelled out

The user explicitly asked about this case. The plan's Phase 3 §"Path-dictionary lockstep with mk1" describes the md1 → mk1 direction (md1 dictionary edit triggers mk1 spec amendment). It does not symmetrically describe what happens when mk1 lands a path-dictionary-affecting change first — e.g., mk1 surfacing a new path indicator allocation that requires md1 to mirror it before either format ships.

In practice the convention is symmetric (the FOLLOWUPS companion-entry mechanism captures it on the tracker side, per CLAUDE.md "Cross-repo coordination"), but this is implicit. Two acceptable resolutions:

- Add one sentence to RELEASE_PROCESS.md §"Path-dictionary lockstep" stating the rule applies in both directions and pointing to the `design/FOLLOWUPS.md` companion-entry convention.
- Or: leave as-is on the grounds that md1 is the canonical dictionary owner (mk1 mirrors byte-for-byte by spec), so any allocation request originating from mk1 still has to land in md1 first to be authoritative — making the doc's md1-centric framing correct by construction.

The second framing is defensible, but the doc doesn't say so explicitly. Implementer's call; not blocking.

### N3 (scope observation). Opus-reviewer / plan-reviewer convention not enumerated

CLAUDE.md "Other repo-specific notes" mentions the per-phase opus reviews persisted to `design/agent-reports/` and the `superpowers:executing-plans` plan-execution skill. RELEASE_PROCESS.md does not enumerate "open per-phase review" or "plan-reviewer pass" as a release-step item.

This is correct scoping — those conventions are about *plan execution*, not *release mechanics*. The 16-step checklist is for the final release dance (versions, vectors, SHAs, FOLLOWUPS, tags). Worth confirming the implementer is intentionally keeping this separation; if so, no change needed.

The doc does reference `design/agent-reports/` once (step 16, "per-release hedge audits"), which is sufficient touch-down for cross-repo cleanup work.

## Confirmations

- **FOLLOWUPS alignment.** RELEASE_PROCESS.md §"Path-dictionary lockstep with mk1" accurately captures the entry at `design/FOLLOWUPS.md` lines 121–128: byte-for-byte mirror, lockstep release window, both PRs cross-linked, both land before either tag. Matches the companion entry's framing in mk1.
- **SHA pin practice consistent with `tests/vectors_schema.rs`.** Confirmed `V0_2_SHA256` constant at line 251 of `crates/md-codec/tests/vectors_schema.rs`; the doc's `sha256sum crates/md-codec/tests/vectors/v0.2.json` regen recipe matches what the test enforces. The doc correctly hedges that schema-1 has no internal pin today but external consumers may pin it (explaining why the byte-identical regen invariant is load-bearing).
- **Family generator string.** `GENERATOR_FAMILY` at `crates/md-codec/src/vectors.rs` line 817 confirms the `MAJOR.MINOR`-only encoding via `concat!("md-codec ", env!("CARGO_PKG_VERSION_MAJOR"), ".", env!("CARGO_PKG_VERSION_MINOR"))`. The doc's churn guidance (regenerate vectors on minor bumps; patch bumps don't churn) is exactly correct.
- **16-step release checklist vs. plan Phase 4 (11 steps).** The 16-step checklist is a strict superset of plan Phase 4 steps 1–11. Specifically:
  - Plan steps 1 (versions) → checklist 1.
  - Plan step 2 (CHANGELOG) → checklist 2.
  - Plan step 3 (MIGRATION) → checklist 3.
  - Plan step 4 (regen + SHA pin BOTH schemas) → checklist 4–6 (split into regen / SHA pin / corpus count assertion, the last being a one-time check from P2 0x16 addition).
  - Plan step 7 (build/test/clippy/doc) → checklist 7–10.
  - Plan step 5 (FOLLOWUPS) → checklist 11.
  - Plan step 6 (CLAUDE.md crosspointer) → checklist 12.
  - Plan step 8 (PR) → checklist 13.
  - Plan step 9 (tag) → checklist 14–15 (split into git tag / GitHub Release).
  - Plan steps 10 + 11 (sibling-repo FOLLOWUPS update + flip mk1 PR) → checklist 16.
  No release-dance step in plan Phase 4 is missing from the checklist.
- **BIP §"Path dictionary" stewardship paragraph (line 362).** Correctly placed (immediately after the dictionary table and the "reserved" callout, before the explicit-encoding subsection). Prose accurately states (a) byte-for-byte sharing, (b) any allocation/deletion/renumbering requires coordinated update, (c) same release window, (d) pointer to `design/RELEASE_PROCESS.md`. No factual issues.
- **No contradiction with CLAUDE.md "Cross-repo coordination."** CLAUDE.md describes the FOLLOWUPS companion-entry convention (tracker-level); RELEASE_PROCESS.md describes the release-window checklist (release-dance-level). They are complementary, not duplicative. The "CLAUDE.md crosspointer maintenance" sub-section in RELEASE_PROCESS.md correctly references CLAUDE.md as a separate artifact to update.
- **No contradiction with README.md.** README mentions mk1 only obliquely; no path-dictionary or release-process prose is duplicated.
- **Commit message accuracy.** `abbec54`'s commit message accurately enumerates what the diff contains (RELEASE_PROCESS.md sections + BIP cross-format paragraph + FOLLOWUPS closure note). Co-author trailer present.
- **Diff scope.** Only 2 files touched, +88 / −0 lines. No code, no tests, no Cargo.toml. Wire format unchanged. Lowest-risk commit in the v0.9 series, as planned.
- **FOLLOWUPS entry is not yet marked resolved.** The plan defers the `Status: resolved <COMMIT>` flip to Phase 4 step 5, which is correct — Phase 3 ships the *artifact* that satisfies the FOLLOWUPS requirement, but the resolution flip happens in the release commit alongside the other two FOLLOWUPS entries (`chunk-set-id-rename`, `md-path-dictionary-0x16-gap`). No action needed in P3.

## Bottom line

Ship as-is, or fold N1 (link form) into Phase 4 if the implementer wants the BIP house style preserved. N2 and N3 are observations for the implementer to consider; neither is blocking. Phase 3 cleanly closes its FOLLOWUPS scope.
