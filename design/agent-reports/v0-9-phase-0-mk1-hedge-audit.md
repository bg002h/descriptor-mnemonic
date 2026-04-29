# v0.9.0 Phase 0 — mk1 hedge audit

**Date:** 2026-04-29
**Phase:** 0 (audit-only; no draft PR opened — see decision below)
**Sibling repo:** `/scratch/code/shibboleth/mnemonic-key`

## Decision: skip the formal draft PR

The plan's original Phase 0 called for opening a coordinated draft PR in mk1 with `chunk_set_id` terminology so reviewers see the full cross-repo shape. F-OQ1 reconnaissance revealed:

1. mk1's BIP, DECISIONS, IMPLEMENTATION_PLAN, FOLLOWUPS already use `chunk_set_id` from day 1. Their BIP §"Naming and identifiers" carries an explicit forward-reference: "(md-codec v0.8.x calls this field 'wallet identifier' — a misleading name slated for rename to `chunk_set_id` across both repos.)"
2. mk1 has zero md-codec dependency — no Rust import, no pattern-match on `Error::PolicyIdMismatch`. Their `mk-codec` crate forks md-codec's BCH plumbing per D-13 rather than depending on it.
3. mk1's reference implementation is mid-flight (v0.1 in progress per `design/IMPLEMENTATION_PLAN_mk_v0_1.md`). Opening a formal draft PR there would conflict with parallel implementation work.

The cross-repo coordination is therefore: **wait for md1 v0.9.0 to ship; then update mk1's hedge prose in a follow-on commit pinning the resolved md-codec tag.** This satisfies F5's spirit (reviewers see the cross-repo shape) without the bureaucratic overhead of a draft-PR ceremony — mk1's BIP is already public on GitHub and the rename is already documented as planned cross-repo work.

## Hedge prose to clean up post md1 v0.9.0 release

The following sites in `/scratch/code/shibboleth/mnemonic-key` carry forward-reference hedges that resolve once md-codec v0.9.0 ships. Phase 4 step 10 of the v0.9.0 plan mass-updates these.

### `bip/bip-mnemonic-key.mediawiki`

- **Line 40** — drop the parenthetical: "(md-codec v0.8.x calls this field 'wallet identifier' — a misleading name slated for rename to `chunk_set_id` across both repos.)" — replace with link to md-codec v0.9.0 release notes for historical context, or delete entirely.
- **Lines 50** — rewrite the "in flight" paragraph: `"A separate naming-only rename — md-codec's chunked-string-header field 'wallet identifier' → \`chunk_set_id\` — is in flight per a coordinated cross-repo follow-up..."` → past-tense reference: `"The chunked-string-header field rename to \`chunk_set_id\` shipped in md-codec v0.9.0 (link)."`

### `design/DECISIONS.md`

- **Lines 188-196** — D-15 sequencing-requirement prose. Update "the rename in md1 (likely as md-codec v0.9.0, docs-and-symbols-only) is a sequencing prerequisite for mk1's BIP submission" → "the rename shipped in md-codec v0.9.0 (link). mk1's BIP-submission gate is now cleared."

### `design/FOLLOWUPS.md`

- **Companion `chunk-set-id-rename` entry (line 44+)** — move from "Open items" to "Resolved items" with `Status: resolved by md-codec-v0.9.0 (commit ...)`.
- **Companion `bip-cross-reference-completeness` entry (line ~85)** — drop the conditional on the `chunk-set-id-rename` entry; the precondition is met.
- **`md-path-dictionary-0x16-gap` companion** — mark resolved.
- **`path-dictionary-mirror-stewardship` companion** — mark resolved.

### `docs/superpowers/specs/2026-04-29-mk1-open-questions-closure-design.md`

- **§3 item (1) (line 360-362)** — sequencing pin paragraph. Update "the rename MUST land in md-codec... before mk1's BIP draft is submitted" to past tense, citing md-codec v0.9.0.
- **§3 item (3) (line 378)** — same treatment for the "Any post-rename of 'wallet identifier' → `chunk_set_id` in md1 must land before mk1's draft is finalized" clause.
- **§"(d) Naming alignment" (line 131)** — reword to past tense.

### `design/IMPLEMENTATION_PLAN_mk_v0_1.md`

- **Line 106** — "Use `chunk_set_id` naming (not 'wallet identifier')" — drop the parenthetical (md1 no longer uses 'wallet identifier').
- **Line 210** — D-15 verbatim quote includes "md1 currently calls the same field 'wallet identifier'..." — strike or rewrite to past tense.
- **Line 270** — "Mirror md1's wording with `chunk_set_id` substituted for 'wallet identifier'." — drop substitution clause.

## Summary

- 6 files, ~10-12 hedge sites to clean up.
- All edits are prose-only (no code, no test, no wire-format). Suitable for a single follow-up commit in mk1 pinned to md-codec-v0.9.0's tag.
- mk1's `chunk-set-id-rename`, `md-path-dictionary-0x16-gap`, and `path-dictionary-mirror-stewardship` companion FOLLOWUPS entries all close in lockstep.
- Optional: the `Error::ChunkSetIdMismatch` / `Error::ReservedChunkSetIdBitsSet` renames in md1 v0.9.0 are not referenced by mk1 docs at all, so no cross-repo cleanup obligation.
