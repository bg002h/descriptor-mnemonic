# CLAUDE.md — md1 (`descriptor-mnemonic`) repo notes for Claude Code sessions

This file is auto-loaded by Claude Code when starting a session in this repository.

## Cross-repo coordination with `bg002h/mnemonic-key` (mk1) and `bg002h/mnemonic-secret` (ms1)

mk1 is a sibling format (HRP `mk`, codex32-derived xpub backup) developed in `/scratch/code/shibboleth/mnemonic-key`. md1 and mk1 share the BCH plumbing (BIP 93 polynomials with HRP-mixing + per-format target residues — *forked*, not shared as a crate) and are designed to engrave alongside each other for foreign-xpub multisig recovery.

ms1 is a third sibling format (HRP `ms`, repo `bg002h/mnemonic-secret`) added 2026-05-03 for the secret-material slot (BIP-39 entropy / BIP-32 master seed / xpriv). Unlike md1↔mk1's forked BCH, ms1 adopts BIP-93 codex32 *directly* via Andrew Poelstra's `rust-codex32`. The three formats engrave together as a coherent backup bundle: md1 = template/policy, mk1 = xpubs, ms1 = secret. v0.1 of all three is single-string (threshold = 0); K-of-N share encoding is planned across the family in v0.2+, ms1 first because BIP-93 already specifies the math.

The previously-planned `mc-codex32` shared-crate extraction (originally gated on "both md+mk at v1.0 with cross-validated vectors") is **RETIRED as of 2026-05-03**: ms1 doesn't need it (uses `rust-codex32` directly), and md1↔mk1's HRP-mixed BCH isn't upstreamable to that crate. md1↔mk1 BCH stays forked indefinitely; the *pattern* will be documented in a future cross-repo `PATTERNS.md`. See `design/FOLLOWUPS.md` entry `mc-codex32-extraction-retired-2026-05-03` for the full record.

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
