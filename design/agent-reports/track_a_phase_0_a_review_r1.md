# Track A Phase 0.A — Code Review (Round 1)

**Commit reviewed:** `d47423e` — `docs(audit-v0.32 phase 0.A): refresh md1-repo docs for v0.30/v0.31/v0.32`
**Reviewer:** feature-dev:code-reviewer
**Date:** 2026-05-11
**Verdict:** 0 Critical / 0 Important / 1 Low / 1 Nit. Both findings folded inline; no follow-up commit required at Phase 0.A close.

## Findings

### Low — `docs/json-schema-v1.md:103`

Field order in the schema doc didn't match actual serde output. Struct in `crates/md-cli/src/format/json.rs:322–326` declares `is_nums, key_index, tree`; doc had them transposed.

**Fix applied:** swapped to `{"is_nums": bool, "key_index": u8, "tree": ...}` and added a clarifying clause that field order matches struct declaration. Folded inline; no separate commit.

### Nit — `MIGRATION.md:13`

The phrase "the trailing `/*` wildcard is handled by `miniscript::Descriptor::at_derivation_index`" was attributed to `to_miniscript_descriptor`'s body, but the call actually happens in the caller (`Descriptor::derive_address` at `crates/md-codec/src/derive.rs:122`).

**Fix applied:** clarified to "the trailing `/*` wildcard is resolved by the caller (`Descriptor::derive_address`) via …". Also added that `to_miniscript_descriptor` produces a `Wildcard::Unhardened` placeholder. Folded inline.

## Spot-checks that passed (sample)

- Feature flag declarations in `crates/md-codec/Cargo.toml` match the doc updates.
- `Error::UnsupportedDerivationShape` absent from `src/`; `Error::AddressDerivationFailed { detail: String }` present at `error.rs:365`.
- Public `to_miniscript_descriptor` exists at `src/to_miniscript.rs:54`.
- Test counts: 444 all-features / 395 no-default-features, consistent with the `#![cfg(feature = "derive")]` gate on `tests/address_derivation.rs`.
- All five v0.30→v0.31→v0.32 MIGRATION entries follow the same four-subsection voice (CLI users / Library consumers (md-codec) / Library consumers (md-cli) / Vector corpus) as older entries.

## Outcome

Both findings folded inline per Phase 0.A discipline (Lows/Nits inline when local). No FOLLOWUPs filed; no separate commit; the two edits are amended into the Phase 0.A close.
