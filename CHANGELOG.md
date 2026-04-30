# Changelog

All notable changes to `md-codec` are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project follows [SemVer](https://semver.org/spec/v2.0.0.html) with the pre-1.0 convention that the second component (`0.X`) is the breaking-change axis.

## [0.10.0] — 2026-04-29

Closes the headline mk1-surfaced FOLLOWUPS item
[`md-per-at-N-path-tag-allocation`](design/FOLLOWUPS.md). v0.10 admits
**per-`@N` divergent origin paths** in BIP 388 wallet policies via a new
`Tag::OriginPaths = 0x36` block, gated by header bit 3 (reclaimed from
the v0.x ≤ 0.9 reserved range). v0.x ≤ 0.9 SharedPath-only encodings
remain byte-identical; new OriginPaths-using encodings need v0.10+
decoders. Test-vector corpora regenerate under family token
`"md-codec 0.10"`.

### Why a wire-format break?

v0.x ≤ 0.9 silently flattened policies with divergent per-`@N` origin
paths to a single shared path, losing information. The result was that
`decode(encode(p))` could differ from `p` for any policy where cosigners
derived xpubs from different paths — a real-world case for any multisig
with cosigners using distinct BIP 48 accounts. v0.10 fixes this with a
new `Tag::OriginPaths` block. Existing shared-path encodings remain
byte-identical (header `0x00` and `0x04` valid; bit 3 stays `0` for the
shared-path case); divergent-path policies now round-trip correctly and
produce correct (different) `PolicyId`s. The cost of the break is
bounded — header bit 3 reclaim narrows the reserved-bits mask from
`0x0B` to `0x03`, and the only API surfaces that change are
`BytecodeHeader::new_v0(bool)` → `new_v0(bool, bool)` and `encode_path`
gaining a `Result` return — and the value is high (correct encoding of
divergent-path policies, closing a silent path-divergence drop).

### Added

- `Tag::OriginPaths = 0x36` block for per-`@N` divergent path
  declarations. Dense encoding (count-prefixed; one path-decl per `@N`
  in placeholder-index order; no deduplication). See BIP draft
  §"Per-`@N` path declaration".
- Header bit 3 (`0x08`) reclaimed as the OriginPaths flag (parallel to
  bit 2 = Fingerprints). `RESERVED_MASK` narrows from `0x0B` to `0x03`.
- `MAX_PATH_COMPONENTS = 10` cap enforced uniformly on `Tag::SharedPath`
  and `Tag::OriginPaths` explicit-form path declarations. Defense-in-
  depth; aligns with mk1 SPEC §3.5; no real-world BIP path family
  exceeds 6 components.
- `WalletPolicy::decoded_origin_paths: Option<Vec<DerivationPath>>`
  field for round-trip stability when `from_bytecode` decoded a
  `Tag::OriginPaths` block.
- `EncodeOptions::origin_paths: Option<Vec<DerivationPath>>` Tier 0
  override for deterministic test-vector generation.
- `EncodeOptions::with_origin_paths(...)` builder method.
- `PolicyId::fingerprint() -> [u8; 4]` short-identifier API. Top 32
  bits as a 4-byte array, parallel to BIP 32 master-key fingerprints.
  Renders as 8 lowercase hex characters; offered as a minimal-cost
  display alternative to the 12-word PolicyId phrase.
- `md encode --policy-id-fingerprint` CLI flag. Additive: prints the
  freshly-computed PolicyId in its 4-byte / 8-hex-char short form
  (`0x{:08x}`, via `PolicyId::fingerprint()`) on a second line after
  the existing 12-word phrase. Use cases: CLI scripts, log lines, and
  minimal-cost engraving anchors for users who don't want the full
  phrase.
- BIP draft §"Per-`@N` path declaration", §"PolicyId types"
  (Type 0/Type 1 typology), §"Authority precedence with MK"
  (cross-reference to mk1 BIP).
- BIP draft path-component-cap statement under §"Explicit path
  encoding".
- BIP draft §"Policy identifier (Tier 3)" engraving language softened
  from "optionally engraved" to MAY-engrave with explicit fingerprint
  alternative.

### Changed

- `BytecodeHeader::new_v0(bool)` → `new_v0(bool, bool)` — gains
  `origin_paths: bool` argument. **Public-API break.** See
  `MIGRATION.md` for sed snippet.
- `encode_path(&DerivationPath) -> Vec<u8>` →
  `encode_path(&DerivationPath) -> Result<Vec<u8>, Error>` — surfaces
  `Error::PathComponentCountExceeded` when the path exceeds
  `MAX_PATH_COMPONENTS = 10`. **Public-API break.** Symmetric change
  on `encode_declaration`.
- `md encode --fingerprint <@INDEX=HEX>` →
  `md encode --master-key-fingerprint <@INDEX=HEX>`. **CLI break.**
  The renamed flag still embeds BIP 32 master-key fingerprints into
  the bytecode's fingerprints block; the more explicit name
  disambiguates from the new `--policy-id-fingerprint` output flag.
  No deprecation alias — pre-v1.0 break freedom.
- BIP draft tag table: `0x36` no longer in the reserved range; reserved
  range narrows to `0x37`–`0xFF`.
- BIP draft bytecode header table: bit 3 documented as the
  v0.10+ OriginPaths flag.

### New error variants

- `BytecodeErrorKind::OriginPathsCountTooLarge { count, max: 32 }` —
  bytecode-layer structural error: count byte is zero or exceeds the
  BIP 388 placeholder cap of 32.
- `Error::OriginPathsCountMismatch { expected, got }` — policy-layer
  semantic error: bytecode count doesn't match the tree's actual
  placeholder count after parse.
- `Error::PathComponentCountExceeded { got, max: 10 }` — applies to
  both `Tag::SharedPath` and `Tag::OriginPaths` when an explicit-form
  path-decl declares more than 10 components.

(`Error` is `#[non_exhaustive]`; adding variants is API-additive, not
breaking.)

### Wire format

- v0.10+ valid header bytes: `0x00`, `0x04`, `0x08`, `0x0C`. (v0.x ≤
  0.9 was `0x00`, `0x04`.)
- v0.x ≤ 0.9 SharedPath-only encodings remain byte-identical (header
  bit 3 stays `0`; family token roll alone doesn't churn the bytes for
  these vectors).
- v0.10 OriginPaths-using encodings are NEW; v0.x ≤ 0.9 decoders
  reject them via `Error::ReservedBitsSet` (intended forward-compat).
- Encoder rule: emit `Tag::SharedPath` if all `@N` paths agree, emit
  `Tag::OriginPaths` otherwise. Pure function of policy state;
  round-trip stable.
- Test-vector corpora (`v0.1.json` + `v0.2.json`) regenerated;
  family-token rolls `"md-codec 0.9"` → `"md-codec 0.10"`. New positive
  vector `o1_sortedmulti_2of3_divergent_paths` (and optional `o2`/`o3`)
  exercising `Tag::OriginPaths`. New negative vectors covering each
  new error variant.

### FOLLOWUPS closed

- `md-per-at-N-path-tag-allocation` — the headline mk1-surfaced item;
  closed by allocating `Tag::OriginPaths = 0x36` and the per-`@N`
  encoder/decoder pipeline.
- `cli-policy-id-fingerprint-flag` — closed in-cycle by adding the
  `md encode --policy-id-fingerprint` flag and renaming the existing
  `md encode --fingerprint` to `--master-key-fingerprint` (the naming
  conflict the deferral cited).

### FOLLOWUPS deferred

- `v010-p3-tier-2-kiv-walk-deferred` — the Tier 2 KIV walk in the
  encoder per-`@N`-path precedence chain is currently stubbed; the
  Tier 0 (`opts.origin_paths` override) and Tier 1
  (`decoded_origin_paths` round-trip) cover all current use cases. v0.11
  follow-up.

### MSRV

Unchanged: 1.85.

## [0.9.1] — 2026-04-29

Patch-level housekeeping. Three small pre-existing items closed; no
functional change, no public-API change, no wire-format change. Test-
vector corpus byte-identical to v0.9.0 (validates the `MAJOR.MINOR`-only
family-generator design — patch bumps don't churn vector SHAs).

### Added

- `rust-toolchain.toml` pinning the compiler to `1.85.0` (matches CI's
  `dtolnay/rust-toolchain@1.85.0` action), with `rustfmt`/`clippy`/`rust-docs`
  components and `minimal` profile.
- `.cargo/config.toml` setting `[profile.release]` `codegen-units = 1` and
  `strip = "symbols"` for deterministic release-binary output. No effect
  on dev builds.

These together close phase 1 of the `reproducible-builds` FOLLOWUPS entry
("cheap pins"). Phase 2 (hermetic Nix/Docker build environment + repro-CI)
remains a v1.0 milestone.

### Fixed

- Pre-existing rustdoc references in `crates/md-codec/src/bytecode/path.rs`
  said `Tag::SharedPath` was `0x33`; actual byte value (per
  `bytecode/tag.rs:122`) is `0x34`. The v0.5→v0.6 renumber moved
  `Placeholder → 0x33` and `SharedPath → 0x33 → 0x34` but missed the
  rustdoc sweep. 8 sites updated. No functional change — encoders and
  decoders use `Tag::SharedPath.as_byte()`, which has always been correct.
- Pre-existing rustdoc broken intra-doc-link warning on
  `crates/md-codec/src/policy_compiler.rs:19`
  (`[\`Concrete::compile_tr(unspendable_key)\`]` — the `(unspendable_key)`
  parameter notation isn't valid intra-doc syntax). Dropped the bracket
  link form; kept the code-formatting backticks.

`cargo doc --workspace --all-features --no-deps` now emits zero warnings.

### FOLLOWUPS closed

- `reproducible-builds` (phase 1 only; phase 2 stays open as v1.0 milestone)
- `tag-sharedpath-rustdoc-stale-0x33`
- `policy-compiler-rustdoc-broken-link`

## [0.9.0] — 2026-04-29

Closes three mk1-surfaced FOLLOWUPS items:
[`chunk-set-id-rename`](design/FOLLOWUPS.md),
[`md-path-dictionary-0x16-gap`](design/FOLLOWUPS.md), and
[`path-dictionary-mirror-stewardship`](design/FOLLOWUPS.md). Wire format
unchanged for the rename portion; wire-additive (`0x16`) for the new
testnet BIP 48 P2SH-P2WSH path indicator. Test-vector corpora regenerate
because `GENERATOR_FAMILY` rolls `"md-codec 0.8"` → `"md-codec 0.9"` and
because `expected_error_variant` strings rename in lockstep with code.

### Why a rename, *again*?

v0.8.0 (2026-04-28) renamed `WalletId → PolicyId` to align with BIP 388's
policy-template framing (Tier-3 = "the policy"). That rename mechanically
renamed the chunk-header 20-bit field `ChunkWalletId → ChunkPolicyId` and
two error variants (`WalletIdMismatch → PolicyIdMismatch`,
`ReservedWalletIdBitsSet → ReservedPolicyIdBitsSet`) along with it. On
review for the sibling Mnemonic Key (mk1) BIP submission, this turned out
to be miscategorized: those names belong to the chunk-header sub-domain
and identify a *chunk-set assembly* — not a Policy ID, not a Wallet
Instance ID. v0.9.0 corrects the chunk-header sub-domain to
`ChunkSetId`/`ChunkSetIdMismatch`/`ReservedChunkSetIdBitsSet`. v0.8's
`PolicyId` and `WalletInstanceId` are stable and unchanged. We expect
this to be the last identifier rename in this family.

### Changed (chunk-header field renames)

- `ChunkPolicyId` → `ChunkSetId`
- `PolicyIdSeed` → `ChunkSetIdSeed`
- `EncodeOptions::policy_id_seed` → `EncodeOptions::chunk_set_id_seed`
- `EncodeOptions::with_policy_id_seed(seed)` →
  `EncodeOptions::with_chunk_set_id_seed(seed)` (if/when the builder is
  used; check call sites)
- `Error::PolicyIdMismatch { expected, got }` →
  `Error::ChunkSetIdMismatch { expected, got }`
- `Error::ReservedPolicyIdBitsSet` → `Error::ReservedChunkSetIdBitsSet`
- `ChunkHeader::Chunked.policy_id` field → `chunk_set_id`
- `Chunk.policy_id` field → `chunk_set_id`
- `Verifications.policy_id_consistent` field → `chunk_set_id_consistent`
- BIP §"Wallet identifier" → §"Chunk-set identifier" with a v0.8→v0.9
  naming-note explaining the correction
- Test helpers: `test_wallet_id`, `expected_wallet_id`,
  `chunked_round_trip_max_wallet_id`, `wid_a`/`wid_b` →
  `test_chunk_set_id`, `expected_chunk_set_id`,
  `chunked_round_trip_max_chunk_set_id`, `csid_a`/`csid_b`

`PolicyId`, `PolicyIdWords`, `WalletInstanceId`,
`compute_policy_id_for_policy`, `compute_wallet_instance_id`,
`MdBackup::policy_id()` are intentionally unchanged.

### Added

- Path-dictionary indicator `0x16 = m/48'/1'/0'/1'` (BIP 48 testnet
  P2SH-P2WSH; mirror of mainnet `0x06`). Wire-additive — existing
  decoders rejected `0x16` as an unknown indicator.
- Corpus vector `t1_sh_wsh_testnet_0x16` exercising the new dictionary
  entry via `EncodeOptions::with_shared_path("m/48'/1'/0'/1'")`.
- `design/RELEASE_PROCESS.md` documenting the md1↔mk1 path-dictionary
  lockstep release invariant and standard 16-step release checklist.
- BIP §"Path dictionary" trailing "Cross-format inheritance" paragraph.

### Wire format

- Unchanged for the rename portion (chunk-set identifier is the same
  20-bit field, just spelled differently in code, prose, and JSON).
- Additive for `0x16` (forward-only — old encodings remain valid;
  encodings that select the testnet `m/48'/1'/0'/1'` path now serialize
  as a single `0x16` byte rather than the explicit-path fallback).
- Test-vector corpus JSON files (`v0.1.json` and `v0.2.json`)
  regenerated to absorb the renamed `expected_error_variant` strings,
  the family-token roll to `"md-codec 0.9"`, and (for v0.2) the new T1
  vector. Both SHA pins update.

### FOLLOWUPS closed

- `chunk-set-id-rename` (mk1-surfaced — hard precondition for mk1's BIP
  submission)
- `md-path-dictionary-0x16-gap` (mk1-surfaced)
- `path-dictionary-mirror-stewardship` (mk1-surfaced)

`md-per-at-N-path-tag-allocation` (the fourth mk1-surfaced item) remains
open and is deferred to v1+ pending per-cosigner-path scheduling.

## [0.8.0] — 2026-04-29

Closes [`wallet-id-is-really-template-id`](design/FOLLOWUPS.md). Renames
the 16-byte template-only hash from `WalletId` to `PolicyId` (and all
related identifiers) to reflect what the value actually identifies, and
introduces a new derived `WalletInstanceId` for per-wallet
disambiguation in tools that need it. Wire format byte-identical to
v0.7.x; vector files regenerate because `GENERATOR_FAMILY` rolls
`"md-codec 0.7"` → `"md-codec 0.8"` and JSON schema field names change
in lockstep with the code rename.

### Changed (renames)

The 16-byte hash `SHA-256(canonical_bytecode)[0..16]` covers the BIP
388 wallet-policy ''template'' only — no concrete cosigner xpubs — so
two distinct wallets that share an identical policy template (same
multisig shape and shared path, different cosigner sets) collide on
this value. The "wallet ID" name treated a one-to-many relationship as
one-to-one. Renamed to "Policy ID" to make the template-level scope
explicit.

Identifier renames:

- `WalletId` → `PolicyId`
- `WalletIdSeed` → `PolicyIdSeed`
- `WalletIdWords` → `PolicyIdWords`
- `ChunkWalletId` → `ChunkPolicyId`
- `compute_wallet_id` → `compute_policy_id`
- `compute_wallet_id_for_policy` → `compute_policy_id_for_policy`
- Module `wallet_id` → module `policy_id`
- `Error::WalletIdMismatch` → `Error::PolicyIdMismatch`
- `Error::ReservedWalletIdBitsSet` → `Error::ReservedPolicyIdBitsSet`
- All `wallet_id*` field names → `policy_id*` (e.g.,
  `Verifications::wallet_id_consistent` → `policy_id_consistent`,
  `EncodeOptions::wallet_id_seed` → `policy_id_seed`)
- All "Wallet ID" / "wallet ID" prose strings (CLI output, error
  messages, rustdoc) → "Policy ID" / "policy ID"
- BIP draft section `===Wallet identifier (Tier 3)===` → `===Policy identifier (Tier 3)===`

### Added

- `WalletInstanceId` — new 16-byte derived identifier defined as
  `SHA-256(canonical_bytecode || canonical_xpub_serialization)[0..16]`,
  where `canonical_xpub_serialization` is the concatenation of each
  `@N`-resolved xpub's full 78-byte BIP 32 serialization in
  placeholder-index order. Distinguishes wallets that share a policy
  template but differ in cosigner xpub sets.
- `pub fn compute_wallet_instance_id(canonical_bytecode: &[u8], xpubs: &[Xpub]) -> WalletInstanceId`
  helper. Recovery tools, descriptor-backup verification flows, and
  cross-implementation consistency checks all hash through this
  function.
- BIP draft `===Wallet Instance ID===` section defining the
  computation. Wallet Instance IDs are not carried by any physical card
  — they are recovery-time derivations.
- Three unit tests on `compute_wallet_instance_id`: differs-when-xpubs-
  differ, xpub-order-sensitive, deterministic.

### Notes

- Wire format byte-identical to v0.7.x. Existing MD chunks decode
  unchanged.
- `GENERATOR_FAMILY` rolls `"md-codec 0.7"` → `"md-codec 0.8"`.
- `tests/vectors/v0.1.json` and `tests/vectors/v0.2.json` regenerate;
  v0.2.json `V0_2_SHA256` pin updates:
  - 0.7.3: `4f8afba0cb379e58b9b03cb9397c37a11b4a038a96698664798ab84985dbb8b9`
  - 0.8.0: `b3f4138937a8f129d218c45d8732776c5d9942be72861a8f8a3eed1ddafcae7d`
- `md-signer-compat` not bumped (no API surface changes; it didn't
  reference any of the renamed types).
- MSRV unchanged: 1.85.

### Closes FOLLOWUPS

- `wallet-id-is-really-template-id`

## [0.7.3] — 2026-04-29

Patch release. Three v0.7.x cleanup items closed in one pass.
Wire format byte-identical to v0.7.x; the v0.2.json SHA pin churns
because one negative-vector description string changed from
"(0x08)" → "(0x07)" — the byte itself was already correct since v0.6.

### Changed (visibility tightenings — pre-1.0 zero-users window)

- `bytecode::encode::HISTORICAL_COLDCARD_TAP_OPERATORS`: `pub const`
  → `pub(crate) const`. The only consumer is the same-module
  `validate_tap_leaf_subset` back-compat shim. md-signer-compat
  defines its own `COLDCARD_TAP.allowed_operators` (the canonical
  "current" Coldcard subset); no cross-crate consumer references the
  historical constant. Closes
  `v07-historical-coldcard-const-visibility`.

- `bytecode::decode::decode_tap_miniscript`: `pub(crate)` → `pub(super)`.
- `bytecode::decode::decode_tap_terminal`: `pub(crate)` → `pub(super)`.

  Both functions are exposed solely for the sibling
  `bytecode::hand_ast_coverage` test sub-module. `pub(super)`
  constrains visibility to the `bytecode::` parent (which is
  sufficient) instead of the whole crate. Closes
  `v07-phase2-decode-helpers-pub-super-tightening`.

### Changed (vector description)

- `n_taptree_at_top_level` description string: "Tag::TapTree (0x08)"
  → "Tag::TapTree (0x07)". The byte itself was correct in code (v0.6
  already shifted TapTree to 0x07); only the description and an
  inline comment lagged the byte-shift rework. Vector files
  regenerated; v0.2.json SHA pin updated:

  - 0.7.2: `014006eaf870d4a853e49850f483fe7f884450033fddb443ef5be88aebf99628`
  - 0.7.3: `4f8afba0cb379e58b9b03cb9397c37a11b4a038a96698664798ab84985dbb8b9`

  Closes `v07-n_taptree_at_top_level-description-stale-v05-byte`.

### Notes

- `GENERATOR_FAMILY` stays `"md-codec 0.7"` (token tracks MAJOR.MINOR
  only; patch bumps don't roll it).
- `md-signer-compat` unchanged at `0.1.1`.
- Library API is technically narrower (`pub` → `pub(crate)` is a
  contraction). Pre-1.0 zero-users window makes this safe; the
  reasonable strict-SemVer reading would push to v0.8, but the
  contraction is cosmetic in practice (no consumers reference the
  affected items via the public path).

## [0.7.2] — 2026-04-29

Patch release. Renames the Tap-context fallback parameter on
`policy_to_bytecode` and the corresponding CLI flag on `md from-policy`
to mirror the upstream `Concrete::compile_tr` naming. Surfaced by v0.7.1
real-world smoke testing — the prior `internal_key` name implied
"force this internal key" but the parameter is actually a fallback
hint that upstream uses only when no key can be extracted from the
policy itself.

### Changed (CLI-flag rename)

- `md from-policy --internal-key <KEY>` → `md from-policy --unspendable-key <KEY>`.
  The `cli-compiler` feature shipped 1 day ago in v0.7.0; impact is
  expected to be zero in practice.

### Changed (library — non-ABI-breaking in Rust; doc-only at the call site)

- `policy_compiler::policy_to_bytecode` parameter `internal_key:
  Option<DescriptorPublicKey>` renamed to `unspendable_key:
  Option<DescriptorPublicKey>`. Rust positional arguments don't carry
  the parameter name through the call site, so existing callers compile
  unchanged. Rustdoc rewritten to explicitly describe the precedence
  rule: `compile_tr` first calls `extract_key(unspendable_key)`, which
  prefers a key extracted from the policy; the fallback parameter is
  used only when no extraction is possible.

### Changed (rustdoc)

- `policy_compiler` module-level docs gain a "Tap-context internal key
  — `unspendable_key` semantics" section explaining the upstream
  precedence rule and noting the v0.7.2 rename rationale.
- `Error::PolicyScopeViolation` rustdoc unchanged from v0.7.1; same
  variant is re-used by `policy_to_bytecode` for compiler-output shapes
  MD does not encode.

### Notes

- Wire format byte-identical to v0.7.x. Vector files unchanged.
- `md-signer-compat` not bumped (no changes).
- Closes `v07-from-policy-internal-key-semantic-clarification`.

## [0.7.1] — 2026-04-29

Patch release. 12 housekeeping items closing v0.7.x defensive-cleanup
FOLLOWUPS plus the deferred signer-validate CLI. Wire format and
public library API unchanged from v0.7.0.

### Added

- **NEW binary `md-signer-compat`** in the `md-signer-compat` crate
  (gated behind that crate's default-on `cli` feature). Subcommands:
  - `validate --signer <coldcard|ledger> --bytecode-hex <HEX>`
  - `validate --signer <coldcard|ledger> --string <md1...>...`
  - `list-signers`

  Closes `v07-cli-validate-signer-subset`. The CLI ships in
  md-signer-compat rather than as a `md validate --signer` subcommand
  on the main `md` binary because md-signer-compat already depends on
  md-codec; adding the reverse dep would cycle.
- **NEW pub fn `bytecode::cursor::Cursor::remaining()`** (test-only,
  `#[cfg(test)]`) — slice of unconsumed input. Used by the
  decoder-arm tests' trailing-sentinel pattern.
- **NEW `test-helpers` cargo feature** on md-codec (default-off):
  exposes `pub mod test_helpers` with `dummy_key_a/b/c()` for
  downstream crates' integration tests.
- **NEW unit test `walker_reports_deepest_violation_first`** —
  regression-pin for the v0.7.0 depth-first leaf-first walker
  semantic refinement (`thresh(1, sha256(H))` with empty allowlist
  rejects with `operator == "sha256"`, not `"thresh"`).

### Changed

- All six `decoder_arm_*` tests in `hand_ast_coverage.rs` now
  assert exact byte consumption via a trailing `0xFF` sentinel
  pattern instead of `cur.is_empty()` — catches under-consumption
  bugs the prior pattern could not.
- `Error::PolicyScopeViolation` rustdoc updated: removes the v0.1-only
  framing, mentions the v0.7+ `policy_to_bytecode` use site.
- `Error::PolicyScopeViolation` Display message: `"policy violates
  v0.1 scope"` → `"policy violates MD encoding scope"`.
- `decode_descriptor` `Tag::TapTree` arm diagnostic now formats the
  byte at runtime (`Tag::TapTree.as_byte()`) so future Tag-byte rolls
  don't desync the diagnostic from the enum.
- `decode_rejects_sh_bare` test renamed to
  `decode_rejects_sh_with_disallowed_inner_tag` and updated to
  exercise `Tag::TapTree` (the v0.6 occupant of byte 0x07) — the
  prior test name and inline byte comment dated to v0.5's `Tag::Bare`.
- `LEDGER_TAP` rustdoc relabelled the variant list as
  "representative subset" and notes the operator union remains
  complete (the cited 7 variants are no longer the full 16, but the
  operator set covers all admitted shapes).
- `validate_tap_tree` rustdoc fixed: `TapTree::leaves()` yields a
  `TapTreeIterItem` struct, not the previously-claimed
  `(depth, leaf_ms)` tuple.
- `or_c_unwrapped_tap_leaf_byte_form` docstring tightened to
  "encoder wire-byte pin only" — the prior text promised a
  decoder-rejection assertion the test body did not perform.
- `md from-policy --context` error message now enumerates all four
  accepted forms (`segwitv0`, `wsh`, `tap`, `tr`).
- Sweep of stale `// 0x32` / `// 0x19` / etc. byte-value comments in
  `tests/{conformance,fingerprints}.rs` (the symbolic refs were
  correct; only the trailing comments dated to v0.5 byte values).
- md-codec internal: `dummy_key_a/b` fixtures consolidated into
  `test_helpers` (one source-of-truth across `hand_ast_coverage` and
  the new shared module).

### Spec

- `design/SPEC_v0_6_strip_layer_3.md` §2.2.1 — new alphabetical
  `Tag → byte` audit-convenience index. Authoritative grouping
  remains §2.2.

### Closes FOLLOWUPS (12)

- `v07-cli-validate-signer-subset`
- `v06-spec-tag-byte-display-table`
- `v07-decode-rejects-sh-bare-rename`
- `v07-stale-byte-annotation-comments`
- `v07-phase2-or-c-unwrapped-test-docstring-drift`
- `v07-ledger-rustdoc-variant-enumeration-incomplete`
- `v07-phase5-policyscopeviolation-rustdoc`
- `v07-phase5-cli-context-error-msg`
- `v07-walker-deepest-violation-pin-test`
- `v07-phase2-decoder-arm-cursor-sentinel-pattern`
- `v07-md-signer-compat-shared-test-key-helpers`
- `v07-taptree-diagnostic-runtime-byte`

### Notes

- Wire format byte-identical to v0.6.x and v0.7.0. Vector files
  unchanged (`GENERATOR_FAMILY` stays `"md-codec 0.7"` since it
  embeds only MAJOR.MINOR).
- `md-signer-compat` crate also bumps from `0.1.0` → `0.1.1`
  (independent versioning).

## [0.7.0] — 2026-04-29

First post-strip-Layer-3 release. Bundles four tracks: test rebaseline,
defensive corpus growth, `md-signer-compat` workspace crate, and policy
compiler wrapper. **Wire format byte-identical to v0.6.x.** Purely additive
public API changes — no breaking changes since v0.6.0.

### Added

- **NEW workspace crate `md-signer-compat`** providing opt-in
  caller-driven validation of MD-encoded BIP 388 wallet policies
  against named hardware-signer subsets:
  - `pub const COLDCARD_TAP: SignerSubset` — Coldcard `firmware/edge`
    `docs/taproot.md` allowed-descriptors set, verified 2026-04-28.
  - `pub const LEDGER_TAP: SignerSubset` — LedgerHQ/vanadium
    `apps/bitcoin/common/src/bip388/cleartext.rs`, verified 2026-04-28.
  - `pub fn validate(subset, ms, leaf_index)` — single-leaf delegation
    to md-codec's `validate_tap_leaf_subset_with_allowlist`.
  - `pub fn validate_tap_tree(subset, tap_tree)` — multi-leaf walker
    threading enumerated DFS-pre-order `leaf_index` through each call.
- **NEW pub function `bytecode::encode::validate_tap_leaf_subset_with_allowlist`**
  (in md-codec) — caller-supplied operator allowlist; the existing
  `validate_tap_leaf_subset` becomes a back-compat shim around the
  new function with `HISTORICAL_COLDCARD_TAP_OPERATORS`.
- **NEW cargo feature `compiler` (default-off)** on md-codec: pulls in
  rust-miniscript's `compiler`. Exposes:
  - `pub enum ScriptContext { Segwitv0, Tap }`.
  - `pub fn policy_to_bytecode(policy, options, script_context, internal_key)`
    — wraps `Concrete::compile`/`compile_tr`, projects to BIP 388 wallet
    policy, and emits MD bytecode. Tap-context internal key is
    caller-supplied (`Option<DescriptorPublicKey>`); `None` defers to
    rust-miniscript's NUMS-unspendable internal-key default.
- **NEW cargo feature `cli-compiler`** (= `cli` + `compiler`): adds
  `md from-policy <expr> --context <tap|segwitv0> [--internal-key <KEY>]`
  CLI subcommand.
- **12 hand-AST defensive tests** in
  `crates/md-codec/src/bytecode/hand_ast_coverage.rs` (`#[cfg(test)]`):
  - `or_c`, `d:`, `j:`, `n:` typing-awkward operators (encoder wire
    bytes pinned + `t:or_c` round-trip).
  - Hash byte-order pin (encode + decode round-trip with asymmetric
    inputs `[0x00..0x1F]` / `[0x80..0x93]`).
  - 6 per-arm decoder tests (multi_a, andor, thresh, after,
    sortedmulti_a, hash256).

### Changed

- **Test rebaseline.** All ~38 tests that pinned v0.5 byte literals
  rebaselined to v0.6 codes using symbolic `Tag::Foo.as_byte()` refs
  where helpful (closes `v06-test-byte-literal-rebaseline`).
- **`validate_tap_leaf_subset` walk order** is now depth-first
  leaf-first: the walker recurses into every Terminal variant's
  children before checking the parent, so the deepest violation is
  reported (more actionable diagnostic). Behaviour-preserving for the
  back-compat shim's allowlist (no test outcome changes).
- **CHANGELOG / MIGRATION discipline:** `[Unreleased]` entries
  consolidated at release time per Plan §9 Q6.

### Notes

- Wire format byte-identical to v0.6.x. Existing chunks decode
  unchanged.
- `GENERATOR_FAMILY` rolls `"md-codec 0.6"` → `"md-codec 0.7"`.
- `tests/vectors/v0.1.json` and `tests/vectors/v0.2.json` regenerate
  with the new family token; `V0_2_SHA256` pin updates once.
- MSRV unchanged: 1.85.

### Closes FOLLOWUPS

- `v06-test-byte-literal-rebaseline`
- `v06-corpus-or-c-coverage`
- `v06-corpus-d-wrapper-coverage`
- `v06-corpus-j-n-wrapper-coverage`
- `v06-corpus-byte-order-defensive-test`
- `v06-plan-targeted-decoder-arm-tests`
- `md-signer-compat-checker-separate-library`
- `md-policy-compiler-feature`
- `v07-tap-leaf-iterator-with-index-coverage`
- `v07-phase2-asymmetric-byte-order-test-inputs`
- `v07-coldcard-multi-a-citation-gap`
- `v07-tap-tree-leaves-docstring-iterator-shape`

### Deferred to v0.7.x

- `v07-cli-validate-signer-subset` — `md validate --signer <name> <bytecode>` CLI track (Plan §9 Q5).
- `v07-decode-rejects-sh-bare-rename`
- `v07-stale-byte-annotation-comments`
- `v07-taptree-diagnostic-runtime-byte`
- `v07-n_taptree_at_top_level-description-stale-v05-byte`
- `v07-historical-coldcard-const-visibility`
- `v07-walker-deepest-violation-pin-test`
- `v07-phase2-decoder-arm-cursor-sentinel-pattern`
- `v07-phase2-or-c-unwrapped-test-docstring-drift`
- `v07-phase2-decode-helpers-pub-super-tightening`
- `v07-ledger-rustdoc-variant-enumeration-incomplete`
- `v07-md-signer-compat-shared-test-key-helpers`

## [0.6.0] — 2026-04-28

The v0.6 release strips MD's signer-compatibility curation layer. MD's scope
is now encoding-only: it serializes any BIP 388 wallet policy losslessly,
without enforcing a hardware-signer-specific operator subset. Whether a given
policy is signable on a given signer becomes a layered concern handled by
tools above MD (wallet software) and below MD (signer firmware).

See [`design/MD_SCOPE_DECISION_2026-04-28.md`](./design/MD_SCOPE_DECISION_2026-04-28.md)
for the full rationale, and [`MIGRATION.md`](./MIGRATION.md#v05x--v060) for
upgrade steps. The BIP draft §"Taproot tree" subset clause is rewritten from
MUST to MAY-informational; a new §"Signer compatibility (informational)"
section frames the responsibility chain.

### Changed (breaking)

- **Tag enum reorganized** (wire-format-breaking): every tap-leaf-bearing
  bytecode regenerates. `Tag::TapTree` 0x08 → 0x07 (adjacent to `Tr=0x06`);
  multisig family contiguous at 0x08-0x0B (Multi 0x08, SortedMulti 0x09,
  MultiA 0x0A, NEW SortedMultiA 0x0B); wrappers and logical operators shift
  by 2 positions; `Placeholder` 0x32 → 0x33 (byte 0x32 left intentionally
  unallocated to surface v0.5→v0.6 transcoder mistakes); `SharedPath`
  0x33 → 0x34. Constants (False/True), top-level descriptors (Pkh/Sh/Wpkh/
  Wsh/Tr), keys (PkK/PkH/RawPkH), timelocks (After/Older), hashes
  (Sha256/Hash256/Ripemd160/Hash160), and `Fingerprints` byte-identical
  from v0.5.

- **`Tag::Bare` variant DROPPED**: the v0.5 byte 0x07 is reused for
  `TapTree`. `Descriptor::Bare` continues to be rejected by the encoder
  via `PolicyScopeViolation` (unchanged behaviour); only the unused Tag
  variant is gone.

- **`Reserved*` variant range DROPPED** (14 variants at 0x24-0x31):
  descriptor-codec inline-key vendoring is incompatible with MD's BIP-388
  wallet-policy scope. `Tag::from_byte` returns `None` for these bytes in
  v0.6.

- **`Error::TapLeafSubsetViolation` renamed to `Error::SubsetViolation`**:
  variant name presumed Tap-context, but the explicit-call validator
  infrastructure could plausibly extend to Segwitv0 subsets. Field shape
  unchanged (`{ operator: String, leaf_index: Option<usize> }`).

- **Encoder/decoder default validator stripped**: `validate_tap_leaf_subset`
  no longer called by default. Callers depending on this rejection for
  safety must invoke `validate_tap_leaf_subset` explicitly. The function
  is retained as `pub fn` in `bytecode::encode` for opt-in use.

- **`DecodedString.data` field removed** (already shipped in `d79125d`):
  replaced by `pub fn data(&self) -> &[u8]` accessor backed by the
  existing `data_with_checksum: Vec<u8>` field. Migration: `decoded.data`
  → `decoded.data()`.

### Added

- **`Tag::SortedMultiA` (0x0B)** for taproot sorted-multisig. Coldcard's
  `firmware/edge` `docs/taproot.md` and Ledger's `vanadium`
  `apps/bitcoin/common/src/bip388/cleartext.rs` both document this shape;
  rust-miniscript's `VALID_TEMPLATES` test corpus uses it.

- **`BytecodeErrorKind::TagInvalidContext { tag, context }`** structural
  diagnostic used by the decoder catch-all when a Tag is valid in some
  context but not in the expected position (e.g., a top-level descriptor
  tag appearing where a tap-leaf inner is expected). Replaces the v0.5
  default-path `TapLeafSubsetViolation` rejection.

- **17 new positive corpus fixtures** for newly-admitted shapes:
  `sortedmulti_a`, `thresh`, `or_b`, `or_i`, `andor`, all four hash
  terminals, `after`, `a:` and `d:` wrappers, plus Coldcard- and
  Ledger-documented compound shapes (timelocked multisig, recovery
  paths). See `design/SPEC_v0_6_strip_layer_3.md` §6.1 for the full
  list.

### Removed

- 14 `Reserved*` Tag variants (descriptor-codec inline-key forms).
- `Tag::Bare` variant.
- 2 negative corpus vectors (`n_top_bare`, `n_sh_bare`) made redundant
  by the Tag::Bare drop (their semantic intent is covered by
  `n_taptree_at_top_level` and `n_sh_inner_script`).
- 1 negative corpus vector (`n_tap_leaf_subset`) made obsolete by the
  default-validator strip; the round-trip is now covered by the new
  positive vector `tr_sha256_htlc_md_v0_6`.

### Wire format

- v0.5.x-encoded MD strings are NOT decodable under v0.6 (different Tag
  bytes for almost every tap-leaf operator). v0.6 is a clean break;
  pre-1.0 + no users yet means no deprecation cycle.
- Family-stable SHA reset at v0.5.x → v0.6.0 boundary.
  `GENERATOR_FAMILY` rolls `"md-codec 0.5"` → `"md-codec 0.6"`.

### Notes

- MSRV: 1.85 (unchanged)
- Workspace `[patch]` block unchanged (apoelstra/rust-miniscript#1 still open)

### Closes FOLLOWUPS

- `md-scope-strip-layer-3-signer-curation` (master)
- `md-strip-validator-default-and-corpus`
- `md-strip-spec-and-docs`
- `md-tag-space-rework`
- `decoded-string-data-memory-microopt` (already closed in d79125d)
- `v0-6-release-prep-revised` (this release)

### NEW FOLLOWUPS (v0.6+ defensive testing)

- `md-signer-compat-checker-separate-library` (aspirational v0.6+)
- `md-policy-compiler-feature` (future v0.7+)
- `v06-corpus-or-c-coverage` (V-typing constraint workaround needed)
- `v06-corpus-j-n-wrapper-coverage` (typing-awkward wrapper fixtures)
- `v06-corpus-byte-order-defensive-test` (hand-pinned hash byte-order test)

---

## [Unreleased]

(empty — v0.6.0 just shipped; new entries go here)

## [0.5.0] — 2026-04-28

The v0.5 release admits multi-leaf `tr(KEY, TREE)` descriptors per BIP 388
§"Taproot tree". `Tag::TapTree (0x08)` transitions from reserved/rejected to
fully active. Wire format is additive: v0.4.x-shaped inputs (`tr(KEY)` and
single-leaf `tr(KEY, leaf)`) decode byte-identical under v0.5.

See [`MIGRATION.md`](./MIGRATION.md#v04x--v050) for upgrade steps.

### Added
- `tr(KEY, TREE)` multi-leaf TapTree admittance per BIP 388 §"Taproot tree"
- `Tag::TapTree (0x08)` now active (was reserved/rejected since v0.2 Phase D)
- BIP 341 control-block depth-128 enforcement during decode (peek-before-recurse)
- `DecodeReport.tap_leaves: Vec<TapLeafReport>` field (NEW field on existing struct — non-breaking via `#[non_exhaustive]`)
- `TapLeafReport` public struct (`leaf_index`, `miniscript`, `depth`)

### Changed
- `Error::TapLeafSubsetViolation` extended with `leaf_index: Option<usize>` field; variant now `#[non_exhaustive]` so destructure patterns must use `..` (additive — non-breaking for wildcard `match` arms; breaking for field-exhaustive destructures, but no known external consumers)
- `validate_tap_leaf_subset(ms)` → `validate_tap_leaf_subset(ms, leaf_index: Option<usize>)` — public API additive but technically breaking (no known external callers)
- Top-level dispatcher message for `0x08`-at-top-level updated to "TapTree (0x08) is not a valid top-level descriptor; it appears only inside `tr(KEY, TREE)`..."
- `v0.1.json` SHA `6d5dd831d05ab0f02707af117cdd2df5f41cf08457c354c871eba8af719030aa` (was `bb2bcc78835d519c7f7595994c6113ef62c379cee99e4d62288772834d4f1c26` at v0.4.1; only the family generator string changed from `"md-codec 0.4"` → `"md-codec 0.5"` — vector content is byte-identical aside from that one field)
- `v0.2.json` SHA `4206cce1f1977347e795d4cc4033dca7780dbb39f5654560af60fbae2ea9c230` (was `caddad36ecc3893e3aae87a6bb57ff1928ed9d8b8710d05a78a6501dbd1e5770` at v0.4.1; Phase 6 added multi-leaf fixtures and Phase 11 rolled the family generator token from `"md-codec 0.4"` → `"md-codec 0.5"`)
- Family-stable promise resets at v0.5.0: `"md-codec 0.5"` is the new family token. v0.5.x patches will produce byte-identical SHAs.

### Removed
- v0.4 single-leaf-with-non-zero-depth `PolicyScopeViolation` rejection (subsumed by multi-leaf path; theoretical-only, no producer emits this shape)

### Wire format
- v0.4.x-shaped inputs (KeyOnly `tr(KEY)` and single-leaf `tr(KEY, leaf)`) byte-identical
- New: multi-leaf trees emit `[Tr=0x06][Placeholder][key_index][TapTree=0x08][LEFT][RIGHT]` recursive framing

### Notes
- MSRV: 1.85 (unchanged)
- Test count: 634 passing + 0 ignored (was 609 at v0.4.1; +25 net)
- Workspace `[patch]` block unchanged (apoelstra/rust-miniscript#1 still open)

### Closes FOLLOWUPS
- `v0-5-multi-leaf-taptree` — this release.

### Files NEW FOLLOWUPS
- `v0-5-t7-chunking-boundary-misnomer` (v0.5-nice-to-have: rename or tune T7 fixture)
- `v0-5-multi_a-curly-parser-quirk` (deferred: `multi_a` in curly-brace contexts)

---

## [0.4.1] — 2026-04-27

Patch release. Three FOLLOWUPS items closed.

### Spec
- BIP §"Status" line aligned with ref-impl-aware string ("Pre-Draft, AI + reference implementation, awaiting human review"). Closes `p10-bip-header-status-string`.
- BIP §"Why a new HRP?" disclaimer reconciled with collision-vet claim (HRP "subject to formal SLIP-0173 registration" rather than the prior ambiguous "subject to change"). Closes `bip-preliminary-hrp-disclaimer-tension`.

### Test code
- `bch_known_vector_regular` and `bch_known_vector_long` in `crates/md-codec/src/encoding.rs` repinned with hardcoded expected-checksum byte arrays computed via independent Python BIP 93 `ms32_polymod` reference (per `/tmp/compute_bch_md_pins.py` script). Round-trip assertions preserved as defense in depth. Closes `bch-known-vector-repin-with-md-hrp` (v0.3-nice-to-have, deferred from v0.3.0 release).

### Notes
- MSRV: 1.85 (unchanged)
- Test count: 609 passing + 0 ignored (unchanged from v0.4.0; no new tests, just stronger assertions in 2 existing tests)
- Wire format unchanged from v0.4.0; v0.4.x backups round-trip across patches
- v0.2.json SHA `caddad36ecc3893e3aae87a6bb57ff1928ed9d8b8710d05a78a6501dbd1e5770` UNCHANGED — first v0.4.x patch; family-stable promise validated
- v0.1.json SHA `bb2bcc78835d519c7f7595994c6113ef62c379cee99e4d62288772834d4f1c26` UNCHANGED
- Workspace `[patch]` block unchanged (apoelstra/rust-miniscript#1 still open)

### Closes FOLLOWUPS
- `p10-bip-header-status-string`
- `bip-preliminary-hrp-disclaimer-tension`
- `bch-known-vector-repin-with-md-hrp`

## [0.4.0] — 2026-04-27

The v0.4 release adds the three remaining post-segwit BIP 388 surface
descriptor types (`wpkh`, `sh(wpkh)`, `sh(wsh(...))`) per design at
`design/SPEC_v0_4_bip388_modern_segwit_surface.md`. MD remains narrower
than BIP 388 by design — see BIP §FAQ "Why is MD narrower than BIP 388?"
for the rejected-by-design types.

### Added — top-level descriptor types
- `wpkh(@0/**)` — BIP 84 native-segwit single-sig
- `sh(wpkh(@0/**))` — BIP 49 nested-segwit single-sig
- `sh(wsh(SCRIPT))` — BIP 48/1' nested-segwit multisig

### Wire format
- ADDITIVE expansion. v0.3.x-produced strings continue to validate identically.
- v0.4.0-produced strings using new types are rejected by v0.3.x decoders
  with `PolicyScopeViolation`.
- Restriction matrix on `sh(...)` admits only `sh(wpkh)` and `sh(wsh)`;
  legacy `sh(multi/sortedmulti)` permanently EXCLUDED (see BIP §FAQ).
- HRP `md`, header bits, tag space ALL unchanged from v0.3.

### Test vectors
- `crates/md-codec/tests/vectors/v0.1.json` regenerated; new SHA-256: `bb2bcc78835d519c7f7595994c6113ef62c379cee99e4d62288772834d4f1c26`
  (changed only because family token bumps; no fixture content changes)
- `crates/md-codec/tests/vectors/v0.2.json` regenerated with v0.4 fixtures + new family token; new SHA-256: `caddad36ecc3893e3aae87a6bb57ff1928ed9d8b8710d05a78a6501dbd1e5770`
- Family-stable promise resets at v0.4.0: `"md-codec 0.4"` is the new family token. v0.4.x patches will produce byte-identical SHAs.

### CLI
- `md encode <policy>` now accepts `wpkh`, `sh(wpkh)`, `sh(wsh)` policies.
- `--path bip48-nested` (NEW) maps to indicator `0x06`.

### Notes
- MSRV: 1.85 (unchanged)
- Test count: 609 passing + 0 ignored (was 565 at v0.3.0 baseline)
- Repository URL: unchanged
- Workspace `[patch]` block: unchanged (still waiting on apoelstra/rust-miniscript#1)

### Closes FOLLOWUPS
- `v0-4-bip-388-surface-completion` — this release.

### Files NEW FOLLOWUPS
- `v0-5-multi-leaf-taptree` (deferred BIP 388 surface item)
- `legacy-pkh-permanent-exclusion` (wont-fix)
- `legacy-sh-multi-permanent-exclusion` (wont-fix)
- `legacy-sh-sortedmulti-permanent-exclusion` (wont-fix)

## [0.3.0] — 2026-04-27

The v0.3 release renames the project from "Wallet Descriptor Mnemonic" (WDM) to "Mnemonic Descriptor" (MD). The shorter name better matches Bitcoin spec naming conventions (compare BIP 93's `ms` HRP for codex32). This is a wire-format-breaking change because the HRP enters the polymod via HRP-expansion.

See [`MIGRATION.md`](./MIGRATION.md#v02x--v030) for upgrade steps.

### Breaking — wire format

- **HRP**: `wdm` → `md`. Strings starting with `wdm1...` are no longer valid v0.3.0 inputs. HRP-expansion bytes change from `[3, 3, 3, 0, 23, 4, 13]` (length 7) to `[3, 3, 0, 13, 4]` (length 5).
- **Test vectors regenerated**:
  - `crates/md-codec/tests/vectors/v0.1.json` — new SHA-256: `aac3677fd84f06915c7bb5148a25ed80c399daa4f9bf56c8052ed84f83c9b71b`
  - `crates/md-codec/tests/vectors/v0.2.json` — new SHA-256: `18804929d54f94fe4b83a135f3e53d3a26b6ae3565729970ce02ef38f74e9909`
  - Family-stable promise resets at v0.3.0: `"md-codec 0.3"` is the new family token. Future v0.3.x patches will produce byte-identical SHAs (per the design from v0.2.1).

### Breaking — crate identifiers

- **Crate package**: `wdm-codec` → `md-codec`. Update `Cargo.toml` dependency.
- **Library**: `wdm_codec` → `md_codec`. Update `use` statements.
- **CLI binary**: `wdm` → `md`. Update CLI invocations.
- **Format name**: "Wallet Descriptor Mnemonic" (WDM) → "Mnemonic Descriptor" (MD).
- **Type renames**: `WdmBackup` → `MdBackup`; `WdmKey` → `MdKey`.
- **Constant renames**: `WDM_REGULAR_CONST` → `MD_REGULAR_CONST`; `WDM_LONG_CONST` → `MD_LONG_CONST`.

### BIP rename

- BIP filename: `bip/bip-wallet-descriptor-mnemonic.mediawiki` → `bip/bip-mnemonic-descriptor.mediawiki`.
- BIP title: "Wallet Descriptor Mnemonic" → "Mnemonic Descriptor".
- §"Payload" gains an explicit normative MUST clause for malformed-payload-padding rejection (carried from v0.2.3).
- §"Checksum" HRP-expansion bytes recomputed for HRP `md`.

### Notes

- **MSRV: 1.85** (unchanged)
- **Test count**: 565 passing (unchanged from v0.2.3 baseline; identifier renames preserved test count)
- **Repository URL**: unchanged at `https://github.com/bg002h/descriptor-mnemonic`
- **Past releases** `wdm-codec-v0.2.0` through `v0.2.3` remain published with deprecation banners on their GitHub Release notes (see Phase 10 of the rename); tags untouched

### HRP collision vet

Pre-flight vet against SLIP-0173 + Lightning + Liquid + codex32 + Nostr + Cosmos + general web search confirmed `md` is unregistered and unused as a bech32 HRP. Defensive SLIP-0173 PR planned post-release (`slip-0173-register-md-hrp` follow-up).

### Workspace `[patch]` block

Still ships unchanged (waiting on `apoelstra/rust-miniscript#1`); same downstream UX as v0.2.x.

## [0.2.3] — 2026-04-27

Audit-of-audit closure. Patches the two findings caught during the v0.2.2 retrospective on whether the v0.2.1 audit itself generated items that should have been filed in `design/FOLLOWUPS.md`. Wire format unchanged from v0.2.0/v0.2.1/v0.2.2; v0.2.x backups round-trip across all four patch releases. **No `MIGRATION.md` changes required.**

### Spec

- **BIP §"Payload" gains an explicit normative MUST clause** for the malformed-payload-padding rejection. v0.2.2 fixed the decoder panic and pinned the structured-error path in `tests/conformance.rs`, but the BIP only said "padding enabled on the encode side; reversed on decode" — a phrasing that admitted the v0.2.1 panic interpretation. The new paragraph names the rejection (`Error::InvalidBytecode { offset: 0, kind: BytecodeErrorKind::MalformedPayloadPadding }` in the reference impl) and requires cross-implementations to surface a semantically equivalent rejection that is distinguishable from generic checksum failure and from generic bytecode-parse failure. This is what a second-implementer needs to find by skim-reading the spec rather than by reading the reference impl's source.

### Changed

- **4 panic-style test sites in `crates/wdm-codec/src/bytecode/decode.rs` brought into style-consistency** with the rest of the file's `assert!(matches!(...))` pattern. The previous `match { Ok => round_trip; Err(SpecificKind) => {} Err(other) => panic!(...) }` shape collapsed each Err arm pair into a single `Err(e) => assert!(matches!(e, ...))`, preserving the inline rationale comments. Test behavior unchanged. Sites: `decode.rs:992/1186/1202/1234` (now consolidated).

### Notes

- **MSRV: 1.85** (unchanged)
- **`v0.2.json` SHA `b403073b…` UNCHANGED** — second consecutive v0.2.x patch with no SHA migration. The family-stable generator design from v0.2.1 continues to deliver byte-identical regen across patches.
- **Test count**: 565 passing (unchanged from v0.2.2; the 4 decode.rs sweeps preserved test semantics)
- **Workspace `[patch]` block** still ships unchanged (waiting on `apoelstra/rust-miniscript#1`)

### Audit-of-audit closure

After v0.2.2 shipped, the user asked whether the v0.2.1 full code audit had itself generated items that should have been added to FOLLOWUPS but were silently acknowledged. Two slipped items were caught:
- `bip-payload-padding-must-clause` (v0.2-nice-to-have): BIP needed an explicit MUST clause to match the structured rejection added in v0.2.2 — closed by the §"Payload" paragraph above.
- `audit-decode-rs-panic-style-consistency` (v0.3-nit, pulled forward): 4 verbose panic-match sites in `decode.rs` tests — closed by the `assert!(matches!(...))` consolidation above.

The audit-of-audit pattern (residual-nits sweep after audit closure) has now caught two real cases where audits generated dropped items, validating it as part of the post-audit workflow.

## [0.2.2] — 2026-04-28

Security + audit-followup patch. Closes the one BLOCKER from the v0.2.1 full code audit (`design/agent-reports/v0-2-1-full-code-audit.md`) plus the audit's IMPORTANT and NIT findings. Wire format unchanged from v0.2.0/v0.2.1; v0.2.x backups remain valid v0.2.2 inputs and vice versa. **No `MIGRATION.md` changes required.**

### Security

- **Decoder no longer panics on hostile input.** A crafted Long-code WDM string (93 5-bit symbols ending with a non-zero low bit + a legitimate Long-code BCH checksum) passed Stage 2 of decode and panicked at Stage 3 via `expect()` in `decode.rs:135-136`. The `expect`'s justification ("structurally impossible") only held for encoder-produced strings; the decoder accepts any 5-bit sequence that satisfies the BCH polymod, including non-byte-aligned hostile inputs. v0.2.2 returns a structured `Error::InvalidBytecode { kind: BytecodeErrorKind::MalformedPayloadPadding }` instead. Affected entry points: `wdm_codec::decode()`, `wdm decode`, `wdm verify`. Reproducer + structured-error path are pinned by `tests/conformance.rs::rejects_malformed_payload_padding`.
- The corresponding 4 `expect("five-bit decode")` sites in `crates/wdm-codec/src/encode.rs` (lines 266/340/375/464) are inside `#[cfg(test)]` and consume only encoder-produced strings, so they were never user-reachable. Their messages are updated to clarify the encoder-produced-input invariant for future readers.

### Added

- **`BytecodeErrorKind::MalformedPayloadPadding`** variant. Additive on `#[non_exhaustive]`; surfaces from the decoder as `Error::InvalidBytecode { offset: 0, kind: BytecodeErrorKind::MalformedPayloadPadding }`. Display: `"malformed payload padding: 5-bit data does not byte-align"`.

### Changed

- `chunk_code_to_bch_code` private helper at `encode.rs:17-22` removed; call site uses the existing `From<ChunkCode> for BchCode` impl directly. Pure cleanup; no behavioral change. (NIT from audit.)

### Notes

- **MSRV: 1.85** (unchanged)
- **`v0.2.json` SHA `b403073b…` UNCHANGED** — the family-stable generator field shipped in v0.2.1 means the regen at v0.2.2 produces the byte-identical v0.2.json file. **First v0.2.x patch with no SHA migration**, validating the v0.2.1 design fix.
- **Test count**: 565 passing (was 564 at v0.2.1; +1 `rejects_malformed_payload_padding` conformance test)
- **Workspace `[patch]` block** still ships unchanged (waiting on `apoelstra/rust-miniscript#1`); same downstream UX as v0.2.x predecessors

### Audit closure

The v0.2.1 full code audit (commit `3ac3bf6`, agent report at `design/agent-reports/v0-2-1-full-code-audit.md`) found 1 BLOCKER + 1 IMPORTANT + 2 NITs + a substantial POSITIVE. v0.2.2 closes all 4 findings:
- BLOCKER (decode.rs:135 panic): fixed via new `MalformedPayloadPadding` variant + structured `?` propagation
- IMPORTANT (4 false-invariant sites in encode.rs tests): comments updated to clarify encoder-produced-input invariant
- NIT (vestigial `chunk_code_to_bch_code` helper): removed
- NIT (pre-`expect` block-comment): updated to acknowledge the malicious-input case + reference the structured error

Audit's verdict was `READY-WITH-CAVEATS`; with v0.2.2 the codebase is `READY-FOR-V0.3-AND-SHELL-IMPL`.

## [0.2.1] — 2026-04-28

Patch release. Two post-release ergonomics items from `design/FOLLOWUPS.md`. Wire format identical to v0.2.0; `MIGRATION.md` from v0.2.0 still applies for v0.1.x → v0.2.x upgrades.

### Added

- **`EncodeOptions::with_chunking_mode(ChunkingMode)`** builder method. Closes `p4-with-chunking-mode-builder`. The existing `with_force_chunking(bool)` shim is preserved; new code should prefer the typed enum form, which becomes the only way to select a future 3rd `ChunkingMode` variant (e.g., a `MaxChunkBytes(u8)` variant per BIP §"Chunking" line 438) without ambiguity.
- **`wdm encode --fingerprint @INDEX=HEX`** CLI flag (repeatable). Closes `phase-e-cli-fingerprint-flag`. Library API for fingerprints (Phase E in v0.2.0) is now exposed at the CLI. The flag accepts `@0=deadbeef` (canonical) or `0=deadbeef` (no `@`) or `@1=0xcafebabe` (with `0x` prefix). All `@i` indices must cover `0..N-1` with no gaps; the encoder validates `N == placeholder_count(policy)` per BIP §"Fingerprints block" MUST clause.
- **CLI privacy warning** when `--fingerprint` is used: stderr message reminds the user that fingerprints leak which seeds match which `@i` placeholders. Per BIP §"Fingerprints block" Privacy paragraph (recovery tools MUST warn before encoding).
- **3 new CLI integration tests** covering `--fingerprint` happy path, index-gap rejection, and short-hex rejection.

### Changed

- **`v0.2.json` regenerated** with a family-stable `generator` field (`"wdm-codec 0.2"`, was `"wdm-codec 0.2.0"` at v0.2.0). New SHA: `b403073b8a925bdda37adb92daa8521d527476aa7937450bd27fcbe0efdfd072` (was `3c208300…` at v0.2.0). **The new SHA is stable across the entire v0.2.x patch line** — future v0.2.2 / v0.2.3 etc. will produce the same SHA on regen. Patch-version traceability is preserved in `gen_vectors --output`'s stderr log. Wire format unchanged. The v0.2.0 SHA `3c208300…` remains correct for the v0.2.0 tag; if your conformance suite pins it, expect a one-time SHA migration at v0.2.1 then no churn afterward. Closes the design defect filed during v0.2.1 prep as `vectors-generator-string-patch-version-churn`.

- **`gen_vectors --output`** now logs the full crate version to stderr (`family generator = "wdm-codec 0.2"; full crate version = "0.2.1"`) so contributors can identify which exact build produced a regen without touching the on-disk SHA.

### Notes

- **MSRV: 1.85** (unchanged from v0.1.x)
- **Wire format unchanged** from v0.2.0; v0.2.0 backups remain valid v0.2.1 inputs and vice versa
- **Workspace `[patch]` block** still ships unchanged (waiting on `apoelstra/rust-miniscript#1`); same downstream UX as v0.2.0
- **Test count**: 564 passing on main (was 561 at v0.2.0; +3 new CLI tests)

## [0.2.0] — 2026-04-28

The v0.2 release expands the WDM codec from v0.1's BIP 388 wsh-only baseline to ship taproot single-leaf, the BIP 93 BCH 4-error correction promise, and the BIP §"Fingerprints block" privacy-controlled feature. Test vectors are bumped to schema 2 with byte-for-byte exact negative fixtures generated programmatically.

See [`MIGRATION.md`](./MIGRATION.md) for v0.1.x → v0.2.0 migration steps.

### Breaking

- **`WalletPolicy::to_bytecode` signature change** (Phase B): `to_bytecode(&self)` → `to_bytecode(&self, opts: &EncodeOptions)`. Migration: callers needing no override pass `&EncodeOptions::default()`. See `MIGRATION.md` §1.
- **`EncodeOptions` lost `Copy`** (Phase B side-effect): `DerivationPath` (the new `shared_path` field's type) is not `Copy`, so `EncodeOptions` lost its derived `Copy` impl. Still derives `Clone + Default + PartialEq + Eq`. Callers assuming `Copy` need explicit `.clone()`. See `MIGRATION.md` §1.
- **`WalletPolicy` `PartialEq` semantics** (Phase A): `WalletPolicy` gained a `decoded_shared_path: Option<DerivationPath>` field, so two logically-equivalent policies — one from `parse()` (`None`) and one from `from_bytecode()` (`Some(...)`) — now compare unequal. Recommended: compare via `.to_canonical_string()` for construction-path-agnostic equality. See `MIGRATION.md` §2.
- **Header bit 2 `PolicyScopeViolation` removed** (Phase E): v0.1 rejected bytecode with header bit 2 = 1 with `Error::PolicyScopeViolation("v0.1 does not support the fingerprints block")`. v0.2 implements the fingerprints block; the rejection no longer fires. Callers that intercepted that error to "detect fingerprints support" should instead inspect `WdmBackup.fingerprints` / `DecodeResult.fingerprints` directly. See `MIGRATION.md` §3.
- **`force_chunking: bool` → `chunking_mode: ChunkingMode`** (Phase A): `pub fn chunking_decision(usize, bool)` is now `(usize, ChunkingMode)`; `EncodeOptions.force_chunking: bool` field renamed to `chunking_mode: ChunkingMode`. The `with_force_chunking(self, force: bool)` builder method is preserved as a `bool → enum` shim for source compatibility with v0.1.1 callers.
- **Test vector schema bumped 1 → 2** (Phase F): `v0.1.json` is locked at SHA `1957b542ed0388b51f01a7b467c8e802942dc6d6507abffaefaf777c90f3cd2c` (the v0.1.0 contract); v0.2.0 ships an additional `v0.2.json` at SHA `3c208300f57f1d42447f052499bab4bdce726081ecee139e8689f6dedb5f81cb`. Schema 2 is additive over schema 1; readers MAY ignore unknown fields.

### Added

- **Taproot Tr single-leaf support** (Phase D): `tr(K)` and `tr(K, leaf_ms)` now encode and decode end-to-end. Per-leaf miniscript subset enforced at both encode AND decode time per BIP §"Taproot tree" MUST clause. Allowed leaf operators: `pk`, `pk_h`, `multi_a`, `or_d`, `and_v`, `older`. Wrapper terminals `c:` and `v:` allowed (BIP 388 emits them implicitly). Multi-leaf `Tag::TapTree` (`0x08`) reserved for v1+ and rejected with `PolicyScopeViolation("multi-leaf TapTree reserved for v1+")`.
- **Fingerprints block** (Phase E): `EncodeOptions::fingerprints: Option<Vec<bitcoin::bip32::Fingerprint>>` (additive on `#[non_exhaustive]`) + `with_fingerprints()` builder. `DecodeResult.fingerprints: Option<Vec<Fingerprint>>` exposes the parsed block. Encoder default `None` → header byte `0x00` (preserves v0.1 wire output for callers who don't opt in). New `Tag::Fingerprints = 0x35` enum variant.
- **BCH 4-error correction** (Phase C): replaces v0.1's brute-force 1-error baseline with proper Berlekamp-Massey + Forney syndrome-based decoding over `GF(1024) = GF(32)[ζ]/(ζ²-ζ-1)` per BIP 93. Reaches the BCH code's full 4-error capacity. Public `bch_correct_regular`/`bch_correct_long` signatures unchanged; only behavioral difference is that 2/3/4-error inputs that previously returned `Err(BchUncorrectable)` now succeed.
- **`EncodeOptions::shared_path: Option<DerivationPath>`** (Phase B): top-tier override for the bytecode shared-path declaration. Wired to the CLI `--path` flag (which v0.1.1 parsed but did not apply). 4-tier precedence: `EncodeOptions::shared_path > WalletPolicy.decoded_shared_path > WalletPolicy.shared_path() > BIP 84 mainnet fallback`.
- **`WalletPolicy.decoded_shared_path: Option<DerivationPath>`** (Phase A, internal field): populated by `from_bytecode` so first-pass `encode → decode → encode` is byte-stable for template-only policies.
- **`Correction.corrected` is the real character even for checksum-region positions** (Phase B): replaces the v0.1 `'q'` placeholder. New `DecodedString::corrected_char_at(usize) -> char` accessor backed by a new `data_with_checksum: Vec<u8>` field.
- **`EncodeOptions::with_shared_path()`, `with_fingerprints()`, `with_force_chunking(bool)` builder methods** (Phase A/B/E): fluent builders for the three opt-in `EncodeOptions` knobs.
- **`From<&BchCode> for BchCodeJson` + 6 other JSON wrapper types** (Phase B Bucket C): the CLI's `--json` output is now backed by `#[derive(Serialize)]` wrappers (`bin/wdm/json.rs`) instead of hand-built `serde_json::json!{}` literals. JSON output is byte-identical to v0.1.1.
- **`pub fn decode_declaration_from_bytes(&[u8]) -> Result<(DerivationPath, usize), Error>`** (post-v0.1.1 followup batch): slice-consuming alt to the cursor-based internal decoder.
- **`Cursor::is_empty()` + `peek_byte()` helpers** (Phase D): for the optional-leaf delimiter detection in the Tr decoder.
- **`Error::TapLeafSubsetViolation { operator: String }`** (Phase D): new variant for tap-leaf-subset violations.
- **`Error::FingerprintsCountMismatch { expected, got }`** (Phase E): new variant for fingerprints-block count validation failures.
- **12 CLI integration tests via `assert_cmd`** (post-v0.1.1 followup batch): `crates/wdm-codec/tests/cli.rs` covers `encode`, `decode`, `verify`, `inspect`, `bytecode` happy and error paths.
- **Taproot test corpus** (Phase F): `tr_keypath`, `tr_pk`, `tr_multia_2of3` positive vectors + `n_tap_leaf_subset`, `n_taptree_multi_leaf` negative vectors in v0.2.json.
- **Fingerprints test corpus** (Phase F): `multi_2of2_with_fingerprints` positive vector + `n_fingerprints_count_mismatch`, `n_fingerprints_missing_tag` negative vectors in v0.2.json.

### Changed

- **`gen_vectors` CLI gains `--schema <1|2>`** (Phase F): defaults to 2 for `--output`; `--verify` infers schema from the file's `schema_version` field.
- **`Tr` rejection removed from encode/decode pipelines** (Phase D): v0.1's `Descriptor::Tr(_) => Err(PolicyScopeViolation(...))` arms in `bytecode/{encode,decode}.rs` are gone.
- **`BytecodeErrorKind::MissingChildren { expected, got }`** is now actually emitted (post-v0.1.1 followup batch): the variant existed since Phase 0.5 scaffolding but no code path produced it. v0.2 adds an explicit arity check at variable-arity decoder branches.
- **Schema-2 `Vector` gains `expected_fingerprints_hex` and `encode_options_fingerprints` optional fields** (Phase F): for the fingerprints positive vector. Both use `serde(default, skip_serializing_if = "Option::is_none")` so schema-1 readers parse v0.2.json cleanly.
- **Schema-2 `NegativeVector` gains `provenance` optional field** (Phase F): one-sentence note on how each negative fixture was generated.

### Fixed

- **Cross-platform CI green on Linux + Windows + macOS** (post-v0.1.1 followup): cleaned 4 latent bugs that emerged once CI ran past the workspace clone step (workflow branch-clone fix, 3-OS test matrix, clippy 1.85.0 `precedence` lint in `polymod_step`, clippy 1.85.0 `format_collect` lint in `vectors.rs` and `bin/wdm.rs`). Recurring `format_collect` was also caught in Phase E (`tests/fingerprints.rs`).
- **Cross-platform SHA stability for committed test vectors** (Phase F): `.gitattributes` rule forces LF line endings on `crates/wdm-codec/tests/vectors/*.json` so SHA-256 lock tests pass on Windows.
- **First-pass `encode → decode → encode` byte stability** (Phase A): `WalletPolicy.decoded_shared_path` field eliminates the v0.1 dummy-key-origin-path drift.

### Notes

- **MSRV: 1.85** (unchanged from v0.1.x). Phase C's BCH BM/Forney decoder is pure arithmetic; no toolchain bump required.
- **Wire format unchanged for the v0.1 corpus**: `gen_vectors --verify v0.1.json` produces byte-identical output. v0.1.0 backups remain valid v0.2.0 inputs.
- **Workspace `[patch]` block**: v0.2.0 ships with the workspace `[patch."https://github.com/apoelstra/rust-miniscript"]` block redirecting to `../rust-miniscript-fork`. Same approach as v0.1.0 + v0.1.1. The fork carries the hash-terminal translator patch (PR submitted upstream as `apoelstra/rust-miniscript#1`). Downstream consumers of `wdm-codec` need to either use a git-dep with the same `[patch]` redirect OR wait for upstream merge. Tracked as `external-pr-1-hash-terminals` in `design/FOLLOWUPS.md`. When upstream merges, `wdm-codec-v0.2.1` will drop the `[patch]` block and bump the `rev =` pin.
- **BIP draft updated**: §"Taproot tree" no longer "forward-defined" (Phase D); §"Error-correction guarantees" gained a SHOULD-clause naming Berlekamp-Massey + Forney as the canonical BCH decoder algorithm (Phase C); §"Fingerprints block" gained a normative Privacy paragraph + concrete byte-layout example (Phase E); §"Test Vectors" restructured for dual-file documentation (Phase F).
- **Test count**: 561 passing on main (was 445 at v0.1.0; +116 across v0.1.1 + v0.2 work).
- **Coverage**: not re-measured for v0.2.0; v0.1.0 baseline was 95% library line. Re-measurement deferred; track via post-release task if relevant.
- **FOLLOWUPS state at tag time**: see `design/FOLLOWUPS.md`. v0.2.0 closes 9 substantive v0.2 items + 4 polish items.

## [0.1.1] — 2026-04-27

Patch release. 17 tests + bug fixes + cross-platform CI work after v0.1.0. See git history `wdm-codec-v0.1.0..wdm-codec-v0.1.1`.

## [0.1.0] — 2026-04-27

Initial release. BIP 388 wsh-only wallet-policy backup format reference implementation. 445 tests, 95% library line coverage, 10 positive + 30 negative test vectors locked in `v0.1.json`. See `design/IMPLEMENTATION_PLAN_v0.1.md` and `design/agent-reports/phase-10-task-controller-closure.md` for the v0.1.0 phase-by-phase summary.

[0.5.0]: https://github.com/bg002h/descriptor-mnemonic/releases/tag/md-codec-v0.5.0
[0.4.1]: https://github.com/bg002h/descriptor-mnemonic/releases/tag/md-codec-v0.4.1
[0.4.0]: https://github.com/bg002h/descriptor-mnemonic/releases/tag/md-codec-v0.4.0
[0.3.0]: https://github.com/bg002h/descriptor-mnemonic/releases/tag/md-codec-v0.3.0
[0.2.3]: https://github.com/bg002h/descriptor-mnemonic/releases/tag/wdm-codec-v0.2.3
[0.2.2]: https://github.com/bg002h/descriptor-mnemonic/releases/tag/wdm-codec-v0.2.2
[0.2.1]: https://github.com/bg002h/descriptor-mnemonic/releases/tag/wdm-codec-v0.2.1
[0.2.0]: https://github.com/bg002h/descriptor-mnemonic/releases/tag/wdm-codec-v0.2.0
[0.1.1]: https://github.com/bg002h/descriptor-mnemonic/releases/tag/wdm-codec-v0.1.1
[0.1.0]: https://github.com/bg002h/descriptor-mnemonic/releases/tag/wdm-codec-v0.1.0
