# WDM v0.2 Implementation Plan

## Goal

Ship `wdm-codec-v0.2.0`: extends v0.1.1 with the substantive features the v0.1 spec already names but defers — BIP §"Fingerprints block", BIP §"Taproot tree" forward-defined, full BCH 4-error correction — plus dynamic negative test vectors and a small set of API ergonomics improvements.

Per cargo SemVer, the leftmost non-zero version component is the breaking-change axis. For `0.x.y` releases that means a `0.1 → 0.2` bump may include breaking changes; v0.2 deliberately does so.

## Scope

### In scope (9 v0.2 items from `design/FOLLOWUPS.md`)

| Short-id | Class | Spec impact | Phase |
|---|---|---|---|
| `p4-chunking-mode-enum` | refactor (breaking) | none | A |
| `6a-bytecode-roundtrip-path-mismatch` | refactor (breaking) | none | A |
| `5e-checksum-correction-fallback` | refactor (additive on `DecodedString`) | none | B |
| `7-encode-path-override` | impl + CLI wiring (breaking on `WalletPolicy::to_bytecode`) | none | B |
| `7-serialize-derives` | impl (output formatting) | none | B |
| `p1-bch-4-error-correction` | feature | minor — new BCH correction test vectors + algorithm SHOULD-clause in BIP | C |
| `p2-taproot-tr-taptree` | feature | substantial — BIP §"Taproot tree" | D |
| `p2-fingerprints-block` | feature (behavioral break: header bit 2 reject → accept) | substantial — BIP §"Fingerprints block" | E |
| `8-negative-fixture-dynamic-generation` | tooling | minor — bumps test-vector schema 1 → 2 | F |

### Out of scope (deferred to v1+ or external)

- `p2-inline-key-tags` — v1+
- `external-pr-1-hash-terminals` — external; should land during v0.2 window so we can drop the workspace `[patch]`
- `p10-bip-header-status-string` — v0.1 nice-to-have, kept open

## Phase ordering

Foundational refactors first (so later phases build on the cleaned-up API surface), then implementation-only items, then the big spec-changing features (each its own phase), then tooling, then release.

### Phase A — Internal API tightening (foundational; breaking)

Two API-shape changes bundled so downstream phases never see the old shapes. (`5e-checksum-correction-fallback` is additive on `DecodedString` and moved to Phase B.)

- **`p4-chunking-mode-enum`**: `pub fn chunking_decision(bytecode_len, bool)` → `(bytecode_len, ChunkingMode)`. Cascades to `EncodeOptions.force_chunking: bool` → `chunking_mode: ChunkingMode`. Builder method `with_force_chunking(bool)` preserved as a `bool → enum` shim for source-compat at call sites that already migrated to the v0.1.1 builder.
- **`6a-bytecode-roundtrip-path-mismatch`**: `WalletPolicy::from_bytecode` currently substitutes dummy keys whose origin path is `m/44'/0'/0'`, breaking first-pass byte-stability of `encode→decode→encode`. **Resolved design** (decided here, not at phase entry, because Phase B `7-encode-path-override` depends on it): the `WalletPolicy` newtype gains a `decoded_shared_path: Option<DerivationPath>` field, populated by `from_bytecode` and consulted by `to_bytecode`. The returned-wrapper alternative is rejected because it would force every existing `WalletPolicy` consumer through a wrapper unwrap.

  **Shared-path precedence** in `to_bytecode` (Phase B builds on this): `EncodeOptions::shared_path` > `WalletPolicy.decoded_shared_path` > `WalletPolicy.shared_path()` (real keys) > BIP 84 mainnet fallback (`m/84'/0'/0'`).

**Spec impact**: none.
**Cargo.toml**: bump to `0.2.0-dev` at phase entry (signals main is the v0.2 development line).
**Breaking surface**: `pub fn chunking_decision` signature; `EncodeOptions.force_chunking` field rename; `WalletPolicy` field addition (additive at struct level since the type is constructed via `parse()`/`from_bytecode`, not struct-literal init from external crates).

### Phase B — Encoder/CLI completion + correction-position fix

- **`5e-checksum-correction-fallback`** (additive on `DecodedString`): extend `crate::encoding::DecodedString` to expose the corrected `data+checksum` slice so `Correction.corrected` reports the actual character, not the `ALPHABET[0] = 'q'` placeholder for checksum-position corrections. Pure addition; no existing API removed.
- **`7-encode-path-override`** (breaking on `WalletPolicy::to_bytecode`): add `EncodeOptions::shared_path: Option<DerivationPath>` (additive on `EncodeOptions` since it is `#[non_exhaustive]`); thread through `WalletPolicy::to_bytecode` so the bytecode encoder respects the override per the precedence rule established in Phase A. Replace the v0.1.1 CLI warning ("--path is parsed but not applied") with actual application.
- **`7-serialize-derives`**: replace hand-built `serde_json::json!{}` in `bin/wdm.rs` with `#[derive(Serialize)]` on output types (`WdmBackup`, `DecodeResult`, etc.). May require a thin local serde-able wrapper around miniscript's `WalletPolicy` since upstream's serde derive may still be missing.

**Spec impact**: none.
**Breaking surface**: `WalletPolicy::to_bytecode` signature change (gains shared-path consultation that may surface different output for callers relying on the v0.1 fallback behavior); the `Correction.corrected` value for checksum-position corrections changes from `'q'` placeholder to the real character (downstream consumers parsing the field on the assumption that `'q'` meant "checksum region" must be updated).

### Phase C — BCH 4-error correction (`p1-bch-4-error-correction`)

Replace brute-force 1-error correction in `encoding.rs::bch_correct_*` with proper syndrome-based decoding: Berlekamp-Massey for the error-locator polynomial, Forney for the error magnitudes. Reaches the BCH code's full 4-error capacity (which the BIP and the codex32 ancestor both promise).

**Spec impact**: minor. Add new positive correction test vectors covering 2/3/4-error inputs at representative position classes (data region, checksum region, mixed). Add a SHOULD-clause to the BIP's Reference Implementation section naming Berlekamp-Massey + Forney as the canonical decoder algorithm so cross-implementations report `Correction` values consistently.

**Risk**: non-trivial algorithm. Mitigation: write exhaustive tests covering 1/2/3/4-error inputs at every position class before claiming completion. Reference implementations exist in Sage/Python; port carefully.

### Phase D — Taproot Tr/TapTree (`p2-taproot-tr-taptree`)

Implement `Tr` / `TapTree` operator support in `bytecode/{encode,decode}.rs`. Single-leaf first per BIP §"Taproot tree (forward-defined)", with the per-leaf miniscript subset constraints required by deployed signers (Coldcard subset: `pk` / `pk_h` / `multi_a` / `or_d` / `and_v` / `older`).

**Spec impact**: substantial. BIP §"Taproot tree" forward-defined section gets its tag-table entries, encoding rules, and per-leaf subset definition pinned down. New positive corpus vectors (Tr-only and Tr-with-TapTree shapes) and negative vectors (out-of-subset miniscript inside a leaf) added to the test-vector lockfile.

**Open design questions** (resolve at phase entry):
- TapTree shape encoding: depth-first leaf list with Merkle-path bits? Compact tree-shape header? What does miniscript's own TapTree representation look like and can we mirror it?
- Per-leaf subset enforcement: at decode time, at encode time, or both?

### Phase E — Fingerprints block (`p2-fingerprints-block`)

Implement BIP §"Fingerprints block": header bit 2 plus reserved tag `Tag::Fingerprints = 0x35`. v0.1 rejects either with `PolicyScopeViolation`; v0.2 implements both — encoder accepts an optional fingerprints array, decoder parses and surfaces it via a new `Backup::fingerprints()` accessor.

**Encoder default**: bit 2 = 0 (no fingerprints block emitted) unless `EncodeOptions::fingerprints` is `Some(_)`. Opt-in only.
**Decoder behavior**: accepts both `0x00` (no fingerprints) and `0x04` (fingerprints present) header values. The v0.1 `PolicyScopeViolation` rejection of `0x04` is removed.

**Spec impact**: substantial. BIP §"Fingerprints block" gets the count-byte + 4-byte-fingerprint format pinned down (already partially specified). Add a privacy clause: fingerprints leak which seeds match which `@i` placeholders, so they're optional and recovery tools should warn before encoding them.

**Behavioral break**: any v0.1 caller pattern-matching on `PolicyScopeViolation` for header bit 2 inputs will no longer see that error variant for valid fingerprints headers. The exhaustiveness gate at `tests/error_coverage.rs` will need its `rejects_*` test for that path retired or repurposed.

### Phase F — Test vectors (`8-negative-fixture-dynamic-generation`)

Replace the static `NEGATIVE_FIXTURES` placeholder strings with programmatically-generated byte-for-byte exact strings produced by encoding a valid policy then mutating it precisely until the named `expected_error_variant` is returned by `decode()`. Per-variant fixture work (~30 variants).

**Spec impact**: minor — bump test-vector schema to `2`; update BIP §"Test Vectors" with the new schema version + sha256 lock + count.

### Phase G — Release prep

Mirror v0.1 Phase 10:
- Full local CI green across all gates (test / clippy / fmt / doc / vectors --verify)
- 3-OS GitHub Actions green
- Coverage ≥ 90 % library line
- Public API audit against this plan, mechanically checked via `cargo public-api` (snapshot diff vs. v0.1.1 release) and `cargo semver-checks` (breaking-change report)
- **Workspace `[patch]` block REMOVED**: `crates/wdm-codec/Cargo.toml` `miniscript = { git = ..., rev = ... }` must point at a published miniscript release OR an upstream `apoelstra/rust-miniscript` SHA reachable from a public branch. Root `Cargo.toml` `[patch."https://github.com/apoelstra/rust-miniscript"]` block gone. Without this, `cargo install wdm-codec` from crates.io would fail to resolve. If `apoelstra/rust-miniscript#1` hasn't merged by Phase G entry, fallback options: (a) vendor the patch into a `bg002h/rust-miniscript-wdm-fork` published crate, (b) submit additional upstream PR(s), (c) hold the v0.2 release.
- `CHANGELOG.md` written covering all v0.1 → v0.2 changes with a "Breaking" section
- `MIGRATION.md` written documenting the Phase A breaking changes (`force_chunking: bool` → `chunking_mode: ChunkingMode`, the `WalletPolicy.decoded_shared_path` field addition, new `EncodeOptions::shared_path` precedence rule, `Correction.corrected` placeholder change)
- MSRV statement: rust-version unchanged at workspace `1.85` unless Phase C's BM/Forney code requires a bump (decided at Phase C entry; if bumped, documented in CHANGELOG)
- Bump Cargo.toml `0.2.0-dev` → `0.2.0`
- Annotated tag `wdm-codec-v0.2.0`
- Push commits + tag

## Quality gates (per phase + at v0.2.0)

Each phase ends green on:
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo fmt --all --check`
- `RUSTDOCFLAGS="-D warnings -D missing_docs" cargo doc --workspace --no-deps --document-private-items`
- `cargo run --bin gen_vectors -- --verify <vectors-file>` (if vectors changed, regen + commit)
- CI green on all 3 OS

At the v0.2.0 release additionally:
- Coverage gate
- Public API audit against this plan
- BIP draft consistent with shipped implementation
- FOLLOWUPS.md state: zero v0.2-blocker open

## Dependencies + risks

### Sequencing dependencies

- Phase A is foundational; Phases B–F assume the new API shapes (and the Phase A `6a` precedence rule that Phase B's `EncodeOptions::shared_path` extends).
- Phase F (vectors) must come after Phases C (BCH 4-error — new positive correction vectors), D (taproot — new operator vectors), and E (fingerprints — new header vectors).
- Phase G must be last.
- Phases B / C / D / E are otherwise independent and can run in any order. D and E are the tallest because they're spec-substantial and D additionally has an external miniscript-fork dependency for taproot policy support; doing them last in the feature-block (just before F) gives more time for spec questions and upstream PR(s) to settle.

### External

- `apoelstra/rust-miniscript#1` (hash-terminals translator) ideally merges during the v0.2 window. When it does: drop the workspace `[patch]`, bump the `Cargo.toml` `rev =` pin to the merged SHA, remove the CI fork-clone step. Tracked as `external-pr-1-hash-terminals` in FOLLOWUPS.md.
- Phase D may surface a need for additional miniscript fork patches around taproot policy support. Discover at Phase D entry; submit upstream PR(s) early like we did for `#1`.

### Risks

- **Phase C (BCH 4-error)**: algorithmic correctness. Reference implementations exist in Sage/Python; port carefully and test exhaustively.
- **Phase D (Taproot)**: miniscript v13's `WalletPolicy` may not yet model `TapTree` the way the BIP specifies. May require upstream coordination + waiting for an upstream change.
- **Phase E (Fingerprints)**: straightforward implementation but needs careful spec text around privacy implications.
- **Spec drift (self-introduced)**: spec changes (Phases C / D / E) land inline with implementation — keep BIP draft commits in the same commit as the implementing code where possible, or in an immediately-following commit.
- **BIP review/community feedback timing**: spec-substantial Phases D and E land BIP edits that may attract bitcoin-dev list comments forcing rework. Mitigation: post a draft of the section to bitcoin-dev as soon as Phase D or E begins (not when it ends) so review feedback overlaps implementation rather than blocking it.
- **miniscript fork divergence during the v0.2 window**: if upstream `apoelstra/rust-miniscript` lands changes ahead of our PR #1, our local `bg002h/rust-miniscript` fork-clone may need rebasing. Mitigation: weekly `git fetch upstream && git rebase` on the fork branch; if conflicts, address before Phase G entry.

## Workflow

Same as v0.1:
- Subagent-driven development with two-stage review per task (spec then code-quality)
- File-disjoint parallel buckets where possible
- All deferred minor items into FOLLOWUPS.md (controller aggregates parallel-batch entries)
- Per-phase decision log: `design/PHASE_v0_2_<X>_DECISIONS.md` (consistent with v0.1's `PHASE_<N>_DECISIONS.md` naming; `<X>` is the phase letter A–G)
- Persisted agent reports under `design/agent-reports/phase-v0-2-<x>-*.md`

## Versioning + branch strategy

- main carries the v0.2 development line from Phase A entry forward (`Cargo.toml` `0.2.0-dev`).
- If a critical bug surfaces in a v0.1.x consumer, fix it on a `v0.1.x` maintenance branch cut from the `wdm-codec-v0.1.1` tag, ship as `wdm-codec-v0.1.2`, then optionally merge the fix forward to main.
- Tag at `wdm-codec-v0.2.0` when Phase G ships; subsequent v0.2 patches are `wdm-codec-v0.2.1`, etc.

## Forward look (post-v0.2)

Items already known to defer to v0.3 / v1+ (not in v0.2 scope):

- `p2-inline-key-tags` — descriptor-codec inline-key forms (foreign-xpub support beyond pure BIP-388)
- Any taproot extensions beyond single-leaf if Phase D exposes them (multi-leaf TapTree depth-N)
- Test-vector schema improvements that surface during Phase F (the `8-negative-fixture-dynamic-generation` *item* itself ships in v0.2; Phase F may surface schema-3 ideas that defer to v0.3)
