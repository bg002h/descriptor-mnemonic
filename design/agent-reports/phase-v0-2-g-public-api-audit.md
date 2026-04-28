# Phase v0.2 G — Public API audit (G-5)

**Status:** DONE

**Verdict:** AUDIT CLEAN

**Subject:** public-API delta `wdm-codec-v0.1.1` (`2123a38`) → `HEAD` (`c712861`)

**Worktree:** `.claude/worktrees/agent-ad3f13f5f4156c7ec` (commit `c712861`)

**Role:** read-only audit (no code change; no FOLLOWUPS edit)

## Tools and toolchain

| Tool | Version | Notes |
|---|---|---|
| `cargo-public-api` | 0.51.0 | requires `rustup`-managed nightly for `rustdoc-json` JSON output |
| `cargo-semver-checks` | 0.47.0 | uses cargo-managed `target/semver-checks/` baseline checkout |
| `rustup` | 1.29.0 | installed mid-audit (system Rust 1.94.1 is non-rustup); rustup-managed nightly required by both tools |
| `nightly rustc` | 1.97.0-nightly (52b6e2c20 2026-04-27) | `rustup toolchain install nightly --profile minimal --component rust-docs-json` |
| stable system rustc | 1.94.1 (e408947bf 2026-03-25) (Arch Linux 1:1.94.1-1.1) | builds the worktree itself |

The worktree's workspace `[patch."https://github.com/apoelstra/rust-miniscript"]` redirect uses path `../rust-miniscript-fork`, which from this worktree resolves to `.claude/worktrees/rust-miniscript-fork`. Symlinked once at audit start to `/scratch/code/shibboleth/rust-miniscript-fork` per Phase D / F precedent.

## `cargo public-api diff wdm-codec-v0.1.1..HEAD` — full output

Run from `crates/wdm-codec/`:

```
Removed items from the public API
=================================
-pub wdm_codec::options::EncodeOptions::force_chunking: bool
-impl core::marker::Copy for wdm_codec::options::EncodeOptions
-impl core::marker::Copy for wdm_codec::options::EncodeOptions
-pub fn wdm_codec::policy::WalletPolicy::to_bytecode(&self) -> core::result::Result<alloc::vec::Vec<u8>, wdm_codec::error::Error>
-pub fn wdm_codec::policy::WalletPolicy::to_bytecode(&self) -> core::result::Result<alloc::vec::Vec<u8>, wdm_codec::error::Error>
-pub wdm_codec::EncodeOptions::force_chunking: bool

Changed items in the public API
===============================
-pub fn wdm_codec::chunking::chunking_decision(bytecode_len: usize, force_chunked: bool) -> wdm_codec::error::Result<wdm_codec::chunking::ChunkingPlan>
+pub fn wdm_codec::chunking::chunking_decision(bytecode_len: usize, mode: wdm_codec::chunking::ChunkingMode) -> wdm_codec::error::Result<wdm_codec::chunking::ChunkingPlan>
-pub fn wdm_codec::chunking_decision(bytecode_len: usize, force_chunked: bool) -> wdm_codec::error::Result<wdm_codec::chunking::ChunkingPlan>
+pub fn wdm_codec::chunking_decision(bytecode_len: usize, mode: wdm_codec::chunking::ChunkingMode) -> wdm_codec::error::Result<wdm_codec::chunking::ChunkingPlan>

Added items to the public API
=============================
+pub fn wdm_codec::bytecode::encode::validate_tap_leaf_subset(ms: &miniscript::miniscript::private::Miniscript<miniscript::descriptor::key::DescriptorPublicKey, miniscript::miniscript::context::Tap>) -> core::result::Result<(), wdm_codec::error::Error>
+pub fn wdm_codec::bytecode::path::decode_declaration_from_bytes(bytes: &[u8]) -> core::result::Result<(bitcoin::bip32::DerivationPath, usize), wdm_codec::error::Error>
+pub wdm_codec::bytecode::tag::Tag::Fingerprints = 53
+pub wdm_codec::bytecode::Tag::Fingerprints = 53
+pub enum wdm_codec::chunking::ChunkingMode
+pub wdm_codec::chunking::ChunkingMode::Auto
+pub wdm_codec::chunking::ChunkingMode::ForceChunked
+impl core::clone::Clone for wdm_codec::chunking::ChunkingMode
+pub fn wdm_codec::chunking::ChunkingMode::clone(&self) -> wdm_codec::chunking::ChunkingMode
+impl core::cmp::Eq for wdm_codec::chunking::ChunkingMode
+impl core::cmp::PartialEq for wdm_codec::chunking::ChunkingMode
+pub fn wdm_codec::chunking::ChunkingMode::eq(&self, other: &wdm_codec::chunking::ChunkingMode) -> bool
+impl core::default::Default for wdm_codec::chunking::ChunkingMode
+pub fn wdm_codec::chunking::ChunkingMode::default() -> Self
+impl core::fmt::Debug for wdm_codec::chunking::ChunkingMode
+pub fn wdm_codec::chunking::ChunkingMode::fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::marker::Copy for wdm_codec::chunking::ChunkingMode
+impl core::marker::StructuralPartialEq for wdm_codec::chunking::ChunkingMode
+impl core::marker::Freeze for wdm_codec::chunking::ChunkingMode
+impl core::marker::Send for wdm_codec::chunking::ChunkingMode
+impl core::marker::Sync for wdm_codec::chunking::ChunkingMode
+impl core::marker::Unpin for wdm_codec::chunking::ChunkingMode
+impl core::marker::UnsafeUnpin for wdm_codec::chunking::ChunkingMode
+impl core::panic::unwind_safe::RefUnwindSafe for wdm_codec::chunking::ChunkingMode
+impl core::panic::unwind_safe::UnwindSafe for wdm_codec::chunking::ChunkingMode
+impl<T, U> core::convert::Into<U> for wdm_codec::chunking::ChunkingMode where U: core::convert::From<T>
+pub fn wdm_codec::chunking::ChunkingMode::into(self) -> U
+impl<T, U> core::convert::TryFrom<U> for wdm_codec::chunking::ChunkingMode where U: core::convert::Into<T>
+pub type wdm_codec::chunking::ChunkingMode::Error = core::convert::Infallible
+pub fn wdm_codec::chunking::ChunkingMode::try_from(value: U) -> core::result::Result<T, <T as core::convert::TryFrom<U>>::Error>
+impl<T, U> core::convert::TryInto<U> for wdm_codec::chunking::ChunkingMode where U: core::convert::TryFrom<T>
+pub type wdm_codec::chunking::ChunkingMode::Error = <U as core::convert::TryFrom<T>>::Error
+pub fn wdm_codec::chunking::ChunkingMode::try_into(self) -> core::result::Result<U, <U as core::convert::TryFrom<T>>::Error>
+impl<T> alloc::borrow::ToOwned for wdm_codec::chunking::ChunkingMode where T: core::clone::Clone
+pub type wdm_codec::chunking::ChunkingMode::Owned = T
+pub fn wdm_codec::chunking::ChunkingMode::clone_into(&self, target: &mut T)
+pub fn wdm_codec::chunking::ChunkingMode::to_owned(&self) -> T
+impl<T> core::any::Any for wdm_codec::chunking::ChunkingMode where T: 'static + ?core::marker::Sized
+pub fn wdm_codec::chunking::ChunkingMode::type_id(&self) -> core::any::TypeId
+impl<T> core::borrow::Borrow<T> for wdm_codec::chunking::ChunkingMode where T: ?core::marker::Sized
+pub fn wdm_codec::chunking::ChunkingMode::borrow(&self) -> &T
+impl<T> core::borrow::BorrowMut<T> for wdm_codec::chunking::ChunkingMode where T: ?core::marker::Sized
+pub fn wdm_codec::chunking::ChunkingMode::borrow_mut(&mut self) -> &mut T
+impl<T> core::clone::CloneToUninit for wdm_codec::chunking::ChunkingMode where T: core::clone::Clone
+pub unsafe fn wdm_codec::chunking::ChunkingMode::clone_to_uninit(&self, dest: *mut u8)
+impl<T> core::convert::From<T> for wdm_codec::chunking::ChunkingMode
+pub fn wdm_codec::chunking::ChunkingMode::from(t: T) -> T
+pub wdm_codec::decode_report::DecodeResult::fingerprints: core::option::Option<alloc::vec::Vec<bitcoin::bip32::Fingerprint>>
+pub wdm_codec::encoding::DecodedString::data_with_checksum: alloc::vec::Vec<u8>
+impl wdm_codec::encoding::DecodedString
+pub fn wdm_codec::encoding::DecodedString::corrected_char_at(&self, char_position: usize) -> char
+pub wdm_codec::error::Error::FingerprintsCountMismatch
+pub wdm_codec::error::Error::FingerprintsCountMismatch::expected: usize
+pub wdm_codec::error::Error::FingerprintsCountMismatch::got: usize
+pub wdm_codec::error::Error::TapLeafSubsetViolation
+pub wdm_codec::error::Error::TapLeafSubsetViolation::operator: alloc::string::String
+pub wdm_codec::options::EncodeOptions::chunking_mode: wdm_codec::chunking::ChunkingMode
+pub wdm_codec::options::EncodeOptions::fingerprints: core::option::Option<alloc::vec::Vec<bitcoin::bip32::Fingerprint>>
+pub wdm_codec::options::EncodeOptions::shared_path: core::option::Option<bitcoin::bip32::DerivationPath>
+pub fn wdm_codec::options::EncodeOptions::with_fingerprints(self, fps: alloc::vec::Vec<bitcoin::bip32::Fingerprint>) -> Self
+pub fn wdm_codec::options::EncodeOptions::with_shared_path(self, path: bitcoin::bip32::DerivationPath) -> Self
+pub fn wdm_codec::policy::WalletPolicy::from_bytecode_with_fingerprints(bytes: &[u8]) -> core::result::Result<(Self, core::option::Option<alloc::vec::Vec<bitcoin::bip32::Fingerprint>>), wdm_codec::error::Error>
+pub fn wdm_codec::policy::WalletPolicy::to_bytecode(&self, opts: &wdm_codec::options::EncodeOptions) -> core::result::Result<alloc::vec::Vec<u8>, wdm_codec::error::Error>
+pub wdm_codec::policy::WdmBackup::fingerprints: core::option::Option<alloc::vec::Vec<bitcoin::bip32::Fingerprint>>
+pub wdm_codec::vectors::NegativeVector::provenance: core::option::Option<alloc::string::String>
+pub wdm_codec::vectors::Vector::encode_options_fingerprints: core::option::Option<alloc::vec::Vec<[u8; 4]>>
+pub wdm_codec::vectors::Vector::expected_fingerprints_hex: core::option::Option<alloc::vec::Vec<alloc::string::String>>
+pub fn wdm_codec::vectors::build_test_vectors_v1() -> wdm_codec::vectors::TestVectorFile
+pub fn wdm_codec::vectors::build_test_vectors_v2() -> wdm_codec::vectors::TestVectorFile
+pub enum wdm_codec::ChunkingMode
+pub wdm_codec::ChunkingMode::Auto
+pub wdm_codec::ChunkingMode::ForceChunked
+pub wdm_codec::Error::FingerprintsCountMismatch
+pub wdm_codec::Error::FingerprintsCountMismatch::expected: usize
+pub wdm_codec::Error::FingerprintsCountMismatch::got: usize
+pub wdm_codec::Error::TapLeafSubsetViolation
+pub wdm_codec::Error::TapLeafSubsetViolation::operator: alloc::string::String
+pub wdm_codec::DecodeResult::fingerprints: core::option::Option<alloc::vec::Vec<bitcoin::bip32::Fingerprint>>
+pub wdm_codec::DecodedString::data_with_checksum: alloc::vec::Vec<u8>
+pub wdm_codec::EncodeOptions::chunking_mode: wdm_codec::chunking::ChunkingMode
+pub wdm_codec::EncodeOptions::fingerprints: core::option::Option<alloc::vec::Vec<bitcoin::bip32::Fingerprint>>
+pub wdm_codec::EncodeOptions::shared_path: core::option::Option<bitcoin::bip32::DerivationPath>
+pub wdm_codec::NegativeVector::provenance: core::option::Option<alloc::string::String>
+pub wdm_codec::Vector::encode_options_fingerprints: core::option::Option<alloc::vec::Vec<[u8; 4]>>
+pub wdm_codec::Vector::expected_fingerprints_hex: core::option::Option<alloc::vec::Vec<alloc::string::String>>
+pub wdm_codec::WdmBackup::fingerprints: core::option::Option<alloc::vec::Vec<bitcoin::bip32::Fingerprint>>
```

(Note: `cargo public-api` outputs each item twice — once at its canonical module path `wdm_codec::<module>::<item>` and once at the top-level re-export path `wdm_codec::<item>`. The transcript above is condensed to one line per logical API item; the verbatim output emits both. The audit cross-reference treats each pair as a single change.)

## `cargo semver-checks check-release --baseline-rev wdm-codec-v0.1.1` — full output

Run from `crates/wdm-codec/`:

```
     Cloning wdm-codec-v0.1.1
    Building wdm-codec v0.1.1 (current)
       Built [   3.861s] (current)
     Parsing wdm-codec v0.1.1 (current)
      Parsed [   0.006s] (current)
    Building wdm-codec v0.1.1 (baseline)
       Built [   4.420s] (baseline)
     Parsing wdm-codec v0.1.1 (baseline)
      Parsed [   0.005s] (baseline)
    Checking wdm-codec v0.1.1 -> v0.1.1 (no change; assume minor)
     Checked [   0.008s] 196 checks: 193 pass, 3 fail, 0 warn, 56 skip

--- failure derive_trait_impl_removed: built-in derived trait no longer implemented ---

Description:
A public type has stopped deriving one or more traits. This can break downstream code that depends on those types implementing those traits.
        ref: https://doc.rust-lang.org/reference/attributes/derive.html#derive
       impl: https://github.com/obi1kenobi/cargo-semver-checks/tree/v0.47.0/src/lints/derive_trait_impl_removed.ron

Failed in:
  type EncodeOptions no longer derives Copy, in /…/crates/wdm-codec/src/options.rs:28

--- failure method_parameter_count_changed: pub method parameter count changed ---

Description:
A publicly-visible method now takes a different number of parameters, not counting the receiver (self) parameter.
        ref: https://doc.rust-lang.org/cargo/reference/semver.html#fn-change-arity
       impl: https://github.com/obi1kenobi/cargo-semver-checks/tree/v0.47.0/src/lints/method_parameter_count_changed.ron

Failed in:
  wdm_codec::policy::WalletPolicy::to_bytecode takes 0 parameters in <baseline>:288, but now takes 1 parameters in <current>:327

--- failure struct_pub_field_missing: pub struct's pub field removed or renamed ---

Description:
A publicly-visible struct has at least one public field that is no longer available under its prior name. It may have been renamed or removed entirely.
        ref: https://doc.rust-lang.org/cargo/reference/semver.html#item-remove
       impl: https://github.com/obi1kenobi/cargo-semver-checks/tree/v0.47.0/src/lints/struct_pub_field_missing.ron

Failed in:
  field force_chunking of struct EncodeOptions, previously in file <baseline>:30

     Summary semver requires new major version: 3 major and 0 minor checks failed
    Finished [   9.341s] wdm-codec
```

### Note on the "v0.1.1 -> v0.1.1" label

`cargo-semver-checks` displays both labels as `v0.1.1` because the current Cargo.toml version is `0.2.0-dev` and the tool normalizes the pre-release suffix to the closest released version for label purposes. The baseline-rev tag (`wdm-codec-v0.1.1`) and the current worktree HEAD (`c712861`) ARE in fact compared correctly (the file paths in the failure messages confirm this — "previously" references `target/semver-checks/git-wdm_codec_v0_1_1/57761b9.../crates/wdm-codec/src/options.rs:30`, and "now" references `<worktree>/crates/wdm-codec/src/options.rs:28`). The `(no change; assume minor)` parenthetical is the labelling artefact, not the comparison verdict — the 3 failures are real diffs detected by the tool. After Phase G's planned `0.2.0-dev` → `0.2.0` bump (G-7 step 5), the label will display as `v0.1.1 -> v0.2.0` correctly.

## Cross-reference: every breaking change is documented

The 3 `cargo semver-checks` failures + the corresponding `cargo public-api` removed/changed entries are mapped to the 3 MIGRATION-tracker FOLLOWUPS entries:

| Reported change | Severity | Documented in | Notes |
|---|---|---|---|
| `EncodeOptions::force_chunking` field removed | Breaking (`struct_pub_field_missing`) | (none-direct) — covered by Phase A bucket A `p4-chunking-mode-enum` resolution | Field renamed to `chunking_mode: ChunkingMode`. Plan G-2/G-3 lists the rename under "Breaking" in CHANGELOG and the resolved `p4-chunking-mode-enum` FOLLOWUPS entry (commit `fbbe6ec`) names the rename + the `with_force_chunking(bool)` shim that preserves source-compat for the v0.1.1 builder usage. Tracker MIGRATION entries do not call this out separately because the pre-Phase-A controller decision was that the `bool→enum` shim plus the documented field rename via the `p4-chunking-mode-enum` resolved entry are sufficient migration guidance; CHANGELOG.md (G-2, in-flight) is the primary surface. ✅ tracked |
| `EncodeOptions` no longer derives `Copy` | Breaking (`derive_trait_impl_removed`) | `phase-b-encode-signature-and-copy-migration-note` | Side-effect of adding `shared_path: Option<DerivationPath>` (DerivationPath is not Copy). FOLLOWUPS entry explicitly enumerates this as one of the two Phase B breaking changes; G-3 will codify in MIGRATION.md. ✅ tracked |
| `WalletPolicy::to_bytecode` arity change `0` → `1` | Breaking (`method_parameter_count_changed`) | `phase-b-encode-signature-and-copy-migration-note` | Same FOLLOWUPS entry — covers both the signature change (`(&self)` → `(&self, opts: &EncodeOptions)`) and the `Copy` removal. ✅ tracked |
| `chunking_decision(bytecode_len, force_chunked: bool)` parameter type changed to `mode: ChunkingMode` | Breaking (changed signature) — surfaced by `cargo public-api`, NOT by `cargo semver-checks` (the param-count is unchanged at 2; type-only changes are not in the lint set) | Resolved FOLLOWUPS `p4-chunking-mode-enum` (commit `fbbe6ec`) | Same Phase A change as the `force_chunking` field rename. CHANGELOG.md (G-2, in-flight) lists in "Breaking". MIGRATION.md (G-3) does not need a new tracker entry for this because callers were already passing a `bool` literal that the tool maps cleanly to `ChunkingMode::ForceChunked` / `ChunkingMode::Auto`. ✅ tracked via `p4-chunking-mode-enum` resolution |
| `Tag::Fingerprints = 0x35` added to `#[non_exhaustive]` enum | Behavioral break (header bit 2 = 1 no longer rejected) | `phase-e-fingerprints-behavioral-break-migration-note` | Resolved FOLLOWUPS `p2-fingerprints-block` body explicitly tags this as "v0.1 `PolicyScopeViolation` rejection at `policy.rs:416` REMOVED". Migration tracker entry covers the v0.1-pattern-match break. ✅ tracked |
| `Error::FingerprintsCountMismatch` variant added to `#[non_exhaustive]` enum | Additive on `#[non_exhaustive]` (not a SemVer break under the non_exhaustive attribute) | Resolved `p2-fingerprints-block` (Phase E commit `6559c17`) | New variant explicitly registered in `error_coverage.rs` exhaustiveness mirror; counted as additive. ✅ |
| `Error::TapLeafSubsetViolation` variant added to `#[non_exhaustive]` enum | Additive on `#[non_exhaustive]` | Resolved `p2-taproot-tr-taptree` (Phase D commit `6f6eae9`) | Same as above: new variant on `#[non_exhaustive] Error`. ✅ |
| `DecodedString::data_with_checksum` field added | Additive on `#[non_exhaustive]` | Resolved `5e-checksum-correction-fallback` (Phase B commit `5f13812`) | New public field + accessor `corrected_char_at`. Per Phase B bucket A report, intentionally pub field for advanced consumers. ✅ |
| `DecodedString::corrected_char_at` method added | Additive | Resolved `5e-checksum-correction-fallback` (Phase B commit `5f13812`) | ✅ |
| `EncodeOptions::shared_path` field added | Additive on `#[non_exhaustive]` | Resolved `7-encode-path-override` (Phase B commit `0993dc0`) | ✅ |
| `EncodeOptions::with_shared_path` builder added | Additive | Resolved `7-encode-path-override` (Phase B commit `0993dc0`) | ✅ |
| `EncodeOptions::fingerprints` field added | Additive on `#[non_exhaustive]` | Resolved `p2-fingerprints-block` (Phase E commit `6559c17`) | ✅ |
| `EncodeOptions::with_fingerprints` builder added | Additive | Resolved `p2-fingerprints-block` (Phase E commit `6559c17`) | ✅ |
| `EncodeOptions::chunking_mode` field added | Additive (paired with the `force_chunking` removal) | Resolved `p4-chunking-mode-enum` (Phase A commit `fbbe6ec`) | ✅ |
| `WalletPolicy::from_bytecode_with_fingerprints` method added | Additive | Resolved `p2-fingerprints-block` (Phase E commit `6559c17`) | ✅ |
| `WdmBackup::fingerprints` field added | Additive on `#[non_exhaustive]` | Resolved `p2-fingerprints-block` (Phase E commit `6559c17`) | ✅ |
| `DecodeResult::fingerprints` field added | Additive on `#[non_exhaustive]` | Resolved `p2-fingerprints-block` (Phase E commit `6559c17`) | ✅ |
| `chunking::ChunkingMode` enum (full impl set) added | Additive | Resolved `p4-chunking-mode-enum` (Phase A commit `fbbe6ec`) | ✅ |
| `bytecode::path::decode_declaration_from_bytes` function added | Additive | Resolved `p3-decode-declaration-from-bytes` (post-v0.1.1 v0.2 batch 1) | ✅ |
| `bytecode::encode::validate_tap_leaf_subset` function added | Additive (incidental pub exposure of the Phase D taproot subset validator) | Resolved `p2-taproot-tr-taptree` (Phase D commit `6f6eae9`) | Phase D agent report enumerates the function at `crates/wdm-codec/src/bytecode/encode.rs:480-503`. The function is `pub fn` in a `pub mod` chain (`bytecode::encode::validate_tap_leaf_subset`), making it part of the public API surface. The Phase D dispatch did not earmark it as a public-API addition explicitly — but the surrounding `pub mod bytecode` modules in `lib.rs:139` are already public for v0.1, so this addition is consistent with the existing convention of exposing the encode/decode helpers. ✅ tracked under Phase D's resolved `p2-taproot-tr-taptree` entry |
| `vectors::Vector::expected_fingerprints_hex` field added | Additive on `#[non_exhaustive]` | Resolved `8-negative-fixture-dynamic-generation` (Phase F commit `5348b12`) | ✅ |
| `vectors::Vector::encode_options_fingerprints` field added | Additive on `#[non_exhaustive]` | Resolved `8-negative-fixture-dynamic-generation` (Phase F commit `5348b12`) | ✅ |
| `vectors::NegativeVector::provenance` field added | Additive on `#[non_exhaustive]` | Resolved `8-negative-fixture-dynamic-generation` (Phase F commit `5348b12`) | ✅ |
| `vectors::build_test_vectors_v1` / `build_test_vectors_v2` functions added | Additive (dual-builder dispatch for schema 1 / schema 2) | Resolved `8-negative-fixture-dynamic-generation` (Phase F commit `5348b12`) | ✅ |

### Phase A `WalletPolicy` PartialEq behavioural break

The third MIGRATION tracker entry (`wallet-policy-eq-migration-note`, surfaced by Phase A bucket B) covers the `WalletPolicy` `PartialEq` semantics change introduced by the new private `decoded_shared_path` field. Neither `cargo public-api` nor `cargo semver-checks` surfaces this as a finding because the field is private (so the public API delta doesn't include it) and the derived `PartialEq` impl wasn't removed (so semver-checks doesn't flag a derive-removal). The break is purely behavioural — two logically-equivalent `WalletPolicy` values from different construction paths now compare unequal — and is by design captured in the tracker entry rather than detectable by these tools.

This is a known blind spot of static API auditors (per `cargo-semver-checks`'s own scope: it lints API shape, not behavioural semantics) and is correctly tracked via the migration tracker FOLLOWUPS plus the inline rustdoc on `decoded_shared_path` (added via Phase A bucket B controller fixup, per resolved `6a-bytecode-roundtrip-path-mismatch`).

### Schema bump v1 → v2 (Phase F)

The schema-2 file `tests/vectors/v0.2.json` is a NEW artifact alongside the byte-frozen `v0.1.json`; v0.1.json verifies byte-identical post-v0.2. The `cargo public-api` diff surfaces this as additive `Vector` / `NegativeVector` field additions (`expected_fingerprints_hex`, `encode_options_fingerprints`, `provenance`) — all `Option<...>` types with `serde(default, skip_serializing_if = "Option::is_none")` so schema-1 readers parse v0.2.json cleanly. Phase G plan G-2 lists the schema bump under "Breaking" in CHANGELOG.md (in-flight); resolved FOLLOWUPS `8-negative-fixture-dynamic-generation` documents the additive-field design decision. ✅ tracked

## Gaps

(none)

Every breaking change reported by `cargo semver-checks` is covered by one of the three MIGRATION-tracker FOLLOWUPS entries (`wallet-policy-eq-migration-note`, `phase-b-encode-signature-and-copy-migration-note`, `phase-e-fingerprints-behavioral-break-migration-note`) OR by a Phase A/F resolved FOLLOWUPS entry that the controller's CHANGELOG.md (G-2) and MIGRATION.md (G-3) work-in-flight will incorporate per the G-2/G-3 decision text. Every additive entry reported by `cargo public-api` is covered by a Phase A/B/C/D/E/F resolved FOLLOWUPS entry that names the addition in its scope.

The `chunking_decision` signature change (`force_chunked: bool` → `mode: ChunkingMode`) and the `EncodeOptions::force_chunking` field rename are surfaced by `cargo public-api` but appear under the Phase A `p4-chunking-mode-enum` resolved entry rather than under one of the three MIGRATION-tracker entries. The Phase G plan G-3 names exactly three breaking changes for MIGRATION.md (the three tracker entries); the Phase A `chunking_mode` rename is intentionally absorbed into the CHANGELOG.md (G-2) "Breaking" section rather than a fourth migration narrative, because the `with_force_chunking(bool)` shim provides source-compat for the most common builder pattern usage. **This is a controller decision documented in PHASE_v0_2_G_DECISIONS.md G-2/G-3, not a gap surfaced by the audit tools** — flagged here for the controller's awareness but the absence of a fourth MIGRATION.md narrative is intentional per G-3's explicit "three breaking changes" scope.

## Top-line verdict

**AUDIT CLEAN** — 0 gaps.

- 3 `cargo semver-checks` failures: all 3 documented (2 in `phase-b-encode-signature-and-copy-migration-note`; 1 via the `p4-chunking-mode-enum` resolved entry + CHANGELOG.md "Breaking" section per G-2 plan).
- 1 `cargo public-api` Changed entry (`chunking_decision`): documented via `p4-chunking-mode-enum`.
- 25 `cargo public-api` Added entries: all covered by Phase A/B/C/D/E/F resolved FOLLOWUPS.
- 1 behavioural break (`WalletPolicy` `PartialEq`): not detectable by these tools; documented in `wallet-policy-eq-migration-note` per design.
- 1 behavioural break (header bit 2 = 1 no longer rejected for fingerprints): not directly surfaced by static audit (the wire-format header byte is not part of the Rust API), but covered by `phase-e-fingerprints-behavioral-break-migration-note`.

The controller's in-flight CHANGELOG.md (G-2) and MIGRATION.md (G-3) work — combined with the three migration-tracker FOLLOWUPS already filed — correctly enumerates every breaking change surfaced by the audit. No additional follow-up entries needed.

## Manual-fallback note

If `cargo public-api` or `cargo semver-checks` had failed to install or run, the manual fallback was to run `git diff wdm-codec-v0.1.1..HEAD -- 'crates/wdm-codec/src/**/*.rs'` and produce the cross-reference table from the diff. Both tools succeeded, so the manual fallback was not exercised.

## Quality gates (audit-side)

| Gate | Result |
|---|---|
| Both tools installed | ✅ `cargo-public-api 0.51.0`, `cargo-semver-checks 0.47.0` |
| `rustup` toolchain available (required by both) | ✅ installed mid-audit (system Rust is non-rustup); `nightly-x86_64-unknown-linux-gnu` set as default with `rust-docs-json` component |
| `cargo public-api diff wdm-codec-v0.1.1..HEAD` | ✅ ran successfully; full output captured |
| `cargo semver-checks check-release --baseline-rev wdm-codec-v0.1.1` | ✅ ran successfully; 3 expected failures, all documented |
| Cross-reference table complete | ✅ every reported change mapped to a tracker entry or resolved FOLLOWUP |
| Audit-only (no code changes) | ✅ only this report file added |
