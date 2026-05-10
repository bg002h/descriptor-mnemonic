# v0.18 Phase 6 — Docs + version bumps + BIP draft (per-phase report)

**Date:** 2026-05-09
**Branch:** `feat/v0.18-full-tap-and-nums-engraving`

## Scope

In-repo documentation and version-bump work. Cross-repo manual-mirror PR (`mnemonic-toolkit/docs/manual/src/40-cli-reference/42-md.md`) deferred to Phase 8 release coordination per CLAUDE.md `manual-cli-surface-mirror` invariant — opens BEFORE the v0.18 tag is pushed.

## Artifacts

### Version bumps

- `crates/md-codec/Cargo.toml`: `0.17.0` → `0.18.0` (semver-minor for breaking change in pre-1.0 convention).
- `crates/md-cli/Cargo.toml`: `0.2.0` → `0.3.0` (semver-minor for breaking change).
- `crates/md-cli/Cargo.toml` md-codec path-dep version: `0.17.0` → `0.18.0`.

### CHANGELOG.md (workspace)

Two new entries at the top:

- `md-codec [0.18.0] — 2026-05-09 [BREAKING]` — wire-format break (NUMS sentinel rule, Tag::TrUnspendable removed, key_index_width formula change). Documents what's new, what didn't change, and migration cost-of-zero (v0.17 shipped same day, no engraved phrases known in wild).
- `md-cli [0.3.0] — 2026-05-09 [BREAKING]` — companion bump for codec break + 4 net-new feature surfaces (Items A, B, F, G, J): `--path` fix, `--unspendable-key` rejection, full miniscript walker coverage, NUMS sentinel emission, render_node n-threading, round-trip integration tests.

### MIGRATION.md (workspace)

New `md-codec v0.17.0 + md-cli v0.2.0 → md-codec v0.18.0 + md-cli v0.3.0 [BREAKING]` entry at top. Covers:

- v0.17→v0.18 phrase non-decodability (`Error::UnknownExtensionTag(0x05)`).
- CLI users: `--unspendable-key` narrowing, `--path` working, --help example phrases changed.
- Library consumers (md-codec): `Tag::TrUnspendable` / `Body::TrUnspendable` removed, `Descriptor::key_index_width` formula change, validate/canonicalize bounds loosening.
- Library consumers (md-cli): `render_node` signature change (now requires `n: u8`), `JsonBody::TrUnspendable` removed, 17 new walker arms.
- Vector corpus regenerated.

### BIP draft (`bip/bip-mnemonic-descriptor.mediawiki`)

- Removed the v0.17 `<code>0x37</code>` row for `tr_unspendable()` from the Tree-operators tag table.
- Replaced the v0.17 `tr_unspendable() shape` subsection with a new `tr() NUMS sentinel rule (v0.18)` subsection. Documents the new key_index width formula (`⌈log₂(n+1)⌉`), sentinel value semantics, canonicalization invariant, use cases, and v0.17→v0.18 history (with the freed `0x05` ext-sub-code).
- Reserved-tag block updated: `Tag::TrUnspendable` text removed.

### `crates/md-cli/README.md`

Not updated this phase. The compile examples added in v0.17 Phase 6 still describe the externally-observable behavior correctly (the `tr(<NUMS-hex>, ...)` rendering is unchanged from a CLI-user perspective; only the internal wire encoding moved from a separate tag to a sentinel). README may benefit from a v0.18-specific note about the new walker coverage; defer to a separate doc PR if needed.

## Verification

- `cargo test --workspace --all-features` → 420 pass with bumped versions (no test changes).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean.
- Cargo.lock auto-updated for the version bumps.

## Per-phase code-reviewer round

Skipped (docs + version bumps only; reviewable artifacts already pinned by Phases 1-5 reviews).

## Out of scope (deferred to Phase 8)

- **mnemonic-toolkit companion PR** for `docs/manual/src/40-cli-reference/42-md.md` (CLAUDE.md `manual-cli-surface-mirror` invariant). Opens BEFORE the v0.18 tag is pushed; coordinated with Phase 8 release tagging.

## Exit gate (Phase 6 in-repo portion)

- ✅ Versions bumped (md-codec 0.17.0→0.18.0, md-cli 0.2.0→0.3.0, dep version updated).
- ✅ CHANGELOG entries pinned.
- ✅ MIGRATION.md entry added.
- ✅ BIP draft updated (drop tr_unspendable row + add sentinel-rule subsection).
- ✅ Workspace tests + clippy green.

Phase 6 in-repo portion closed; Phase 7 (whole-PR architect review) up next.
