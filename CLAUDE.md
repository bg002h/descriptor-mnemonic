# CLAUDE.md — md1 (`descriptor-mnemonic`) repo notes for Claude Code sessions

This file is auto-loaded by Claude Code when starting a session in this repository.

## Cross-repo coordination with `bg002h/mnemonic-key` (mk1)

mk1 is a sibling format (HRP `mk`, codex32-derived xpub backup) developed in `/scratch/code/shibboleth/mnemonic-key`. The two formats share the BCH plumbing (BIP 93 polynomials with HRP-mixing + per-format target residues) and are designed to engrave alongside each other for foreign-xpub multisig recovery.

**Cross-repo follow-up convention:** when mk1 work surfaces an action item that affects md1 (rename, missing dictionary entry, wire-format extension, process invariant), the item is mirrored here:

- A primary entry lives in mk1's `design/FOLLOWUPS.md` at tier `cross-repo`.
- A companion entry is mirrored into this repo's `design/FOLLOWUPS.md` so md-codec sessions discover the action item natively from this repo's tracker.
- Both entries cite each other (`Companion:` line in each).
- When the md1-side action ships, both entries are updated in lockstep: the md1 entry is marked `resolved <COMMIT>`; the mk1 entry's `Status:` notes the resolving md1 commit.

**Currently open mk1-surfaced items affecting md1** (see `design/FOLLOWUPS.md` for full entries):

- (none — `md-per-at-N-path-tag-allocation` resolved in v0.10.0)

**Recently resolved:**

- **md-codec v0.10.0:** `md-per-at-N-path-tag-allocation` (Tag::OriginPaths = 0x36; header bit 3 reclaimed; per-`@N` divergent-path encoding shipped). mk1's companion `md-per-N-path-tag-allocation` closes in lockstep.
- **md-codec v0.9.0:** `chunk-set-id-rename`, `md-path-dictionary-0x16-gap`, `path-dictionary-mirror-stewardship`. mk1's BIP-submission gate was cleared in v0.9.0.

## Other repo-specific notes

- The reference implementation is in `crates/md-codec/`. Sibling crates: `crates/md-signer-compat/`.
- Implementation plans live in `design/IMPLEMENTATION_PLAN_v0_X_*.md`; per-phase opus reviews persist to `design/agent-reports/`.
- Per-phase TDD discipline: tests written before impl; the `superpowers:executing-plans` skill is the canonical sub-skill for plan execution in this repo.
