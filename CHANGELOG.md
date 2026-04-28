# Changelog

All notable changes to `wdm-codec` are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project follows [SemVer](https://semver.org/spec/v2.0.0.html) with the pre-1.0 convention that the second component (`0.X`) is the breaking-change axis.

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

[0.2.1]: https://github.com/bg002h/descriptor-mnemonic/releases/tag/wdm-codec-v0.2.1
[0.2.0]: https://github.com/bg002h/descriptor-mnemonic/releases/tag/wdm-codec-v0.2.0
[0.1.1]: https://github.com/bg002h/descriptor-mnemonic/releases/tag/wdm-codec-v0.1.1
[0.1.0]: https://github.com/bg002h/descriptor-mnemonic/releases/tag/wdm-codec-v0.1.0
