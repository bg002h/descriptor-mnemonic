# v0.17 Phase 6 — Docs, BIP draft, CHANGELOG (per-phase report; in-repo portion)

**Date:** 2026-05-09
**Branch:** `feat/v0.17-tap-multi-leaf-policy`

## Scope (in-repo portion of Phase 6)

Documentation and version-bump work in this repo. The cross-repo manual-mirror PR (`mnemonic-toolkit/docs/manual/src/40-cli-reference/42-md.md`) is deferred to a separate step pending user confirmation.

## Artifacts

### Version bumps

- `crates/md-codec/Cargo.toml`: `0.16.2` → `0.17.0` (semver-minor for additive `Tag::TrUnspendable`).
- `crates/md-cli/Cargo.toml`: `0.1.1` → `0.2.0` (semver-minor for new `--unspendable-key` flag and walker extensions).
- `crates/md-cli/Cargo.toml` md-codec path-dep version: `0.16.1` → `0.17.0` (publish-cycle prerequisite).

### CHANGELOG.md (workspace)

Two new entries at the top:

- `md-codec [0.17.0] — 2026-05-09` — wire-format additive (Tag::TrUnspendable). Documents canonicalization invariant + use cases. Notes byte-identical decode of pre-v0.17 payloads.
- `md-cli [0.2.0] — 2026-05-09` — user-facing feature (multi-key tap policies + new flag). Documents headline use cases (2-of-3 multisig, inheritance/timelock pattern), removed v0.15-era error path, and explicit out-of-scope items deferred to v0.17.1+.

### BIP draft (`bip/bip-mnemonic-descriptor.mediawiki`)

- New row in §"Tree operators" tag table at code `0x37`: `tr_unspendable()` (note: 8-bit code in the BIP table; in the v0.11 5-bit format the implementation uses extension prefix sub-code `0x05`).
- New §"`tr_unspendable()` shape (v0.17)" subsection — explains: NUMS H-point hex, no-key-on-the-wire body shape, canonicalization invariant ("MUST emit iff internal key is BIP-341 NUMS"), use cases (threshold multisig, and-conjunction, force-script-path), and the forbidden-tap-leaf rejection.
- Side concern: the BIP table uses 8-bit codes (v0.x byte-aligned format) while the implementation uses 5-bit (v0.11 bit-aligned). Tracked in Phase B SPEC R4 as a separate post-v0.17 BIP-text refresh. Not v0.17-blocking.

### MIGRATION.md (workspace)

- New §"v0.16.x → md-codec v0.17.0 + md-cli v0.2.0" entry at the top.
- CLI users section: removed v0.15-era error, new `--unspendable-key` flag and rejection rules.
- Library consumers (md-codec) section: two new public enum variants, canonicalization invariant note.
- Library consumers (md-cli) section: `compile_policy_to_template` signature change (now takes `Option<&str>`).

### crates/md-cli/README.md

- Subcommand row for `md compile` updated to show `--unspendable-key` flag.
- New §"Compile examples (v0.17)" with five cases:
  - `pk(@0)` → `tr(@0)` (single-key key-path).
  - `or(pk(@0),and(pk(@1),older(144)))` → inheritance pattern.
  - `thresh(2,pk(@0),pk(@1),pk(@2))` → 2-of-3 multisig with auto-NUMS (headline).
  - `and(pk(@0),pk(@1))` with explicit `--unspendable-key <NUMS>`.
  - `multi(2,@0,@1,@2)` segwitv0.

## Verification

- `cargo test --workspace --all-features` → all pass with bumped versions.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean.

## Per-phase code-reviewer round

Skipped (docs + version bumps only; reviewable artifacts already pinned by Phase 1-5 reviews).

## Out of scope (deferred to next step)

- **mnemonic-toolkit companion PR** for `docs/manual/src/40-cli-reference/42-md.md` (CLAUDE.md `manual-cli-surface-mirror` invariant). Cross-repo work; awaiting user confirmation before opening the toolkit-side PR. The architect's I5 finding requires the companion PR to be opened BEFORE the v0.17 tag is pushed (Phase 8); doing it now keeps lockstep clean.

## Exit gate (Phase 6 in-repo portion)

- ✅ Versions bumped.
- ✅ CHANGELOG entries pinned.
- ✅ BIP draft updated.
- ✅ MIGRATION.md updated.
- ✅ README updated.
- ✅ Workspace tests + clippy green.

Phase 6 in-repo portion closed; pausing for user confirmation before opening the toolkit-side manual-mirror PR.
