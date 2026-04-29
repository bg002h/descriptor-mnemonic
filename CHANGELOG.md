# Changelog

All notable changes to `md-codec` are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project follows [SemVer](https://semver.org/spec/v2.0.0.html) with the pre-1.0 convention that the second component (`0.X`) is the breaking-change axis.

## [Unreleased] (next breaking release, planned 0.6.0)

### Changed (breaking)
- `DecodedString.data` field removed; replaced by `pub fn data(&self) -> &[u8]` accessor backed by the existing `data_with_checksum: Vec<u8>` field. Eliminates the redundant per-`DecodedString` allocation that previously duplicated the data-symbol prefix. Migration: `decoded.data` → `decoded.data()` (yields `&[u8]` instead of an owned `Vec<u8>`). For consumers that need an owned copy, use `decoded.data().to_vec()`. Do **not** substitute `decoded.data_with_checksum` — it includes the trailing BCH checksum symbols and is NOT a drop-in replacement for the old `data` in payload-processing contexts.

### Migration
See [`MIGRATION.md`](./MIGRATION.md#v05x--v060) for upgrade steps.

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
