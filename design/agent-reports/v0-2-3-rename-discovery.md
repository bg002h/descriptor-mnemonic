# Rename Discovery: wdm → md (v0.3.0)

**Generated**: 2026-04-27
**Workflow**: design/RENAME_WORKFLOW.md (Phase 0)
**Decision log**: design/RENAME_v0_3_wdm_to_md.md
**Total touch points**: 571 (estimated after filtering out targets, .claude/, and vector JSON duplicates)

## Surprises (read this first)

**One critical discovery:** The `GENERATOR_FAMILY` constant in `crates/wdm-codec/src/vectors.rs` (line 578-583) embeds the string `"wdm-codec "` as a hardcoded literal inside a `concat!()` macro. This must be changed to `"md-codec "`. Replacement requires regen of both v0.1.json and v0.2.json test vectors — the generator string is part of the polymod input at Phase 6. **Also critical:** All 72 instances of the bech32 prefix `wdm1` across the two JSON test-vector files (v0.1.json and v0.2.json) will change to `md1` after Phase 6 regen. The wire format changes because the HRP-expansion bytes themselves change (from 7 bytes `[3,3,3,0,23,4,13]` to 5 bytes `[3,3,0,13,4]`). **No hand-edits** — these are purely generated content. **Another surprise:** The BIP file path change `bip-wallet-descriptor-mnemonic.mediawiki` → `bip-mnemonic-descriptor.mediawiki` is not in the decision log's expected OLD/NEW table structure — it's embedded in the BIP filename column. Ensure Phase 2 (BIP rename) happens before any code touches. **Test function names:** 18 CLI integration test functions in `crates/wdm-codec/tests/cli.rs` are named `fn wdm_*()` (an earlier draft of this report said 19; corrected during plan-review spot-check — actual count by `grep -c '^fn wdm_' crates/wdm-codec/tests/cli.rs` is 18). Plus 3 in `src/policy.rs` and `src/encoding.rs` test modules = 21 total. These are user-visible in test output and should be renamed to `md_*()` for consistency, though they don't affect wire format. **Memory files:** The `/home/bcg/.claude/projects/-scratch-code-shibboleth/memory/` directory contains multiple project tracking files with references to "wdm" and "wallet descriptor mnemonic"; these are auto-memory and will need Phase 9 updates.

## Summary by category

| Category | MECHANICAL | CONTEXTUAL | WIRE | HISTORICAL | EXTERNAL |
|---|---|---|---|---|---|
| Code identifiers | 47 | 4 | 0 | 0 | 0 |
| Doc comments | 8 | 3 | 0 | 0 | 0 |
| String literals | 15 | 8 | 2 | 0 | 3 |
| Filenames + directory names | 4 | 0 | 0 | 0 | 0 |
| CI config | 0 | 0 | 0 | 0 | 0 |
| Test vector contents | 0 | 0 | 72 | 0 | 0 |
| BIP normative text | 8 | 2 | 2 | 0 | 0 |
| Tier-2 docs | 12 | 8 | 0 | 18 | 0 |
| Cargo manifest fields | 4 | 0 | 0 | 0 | 1 |
| External-facing strings | 3 | 2 | 0 | 0 | 1 |
| Auto-memory files | 0 | 0 | 0 | 8 | 0 |
| **TOTAL** | **101** | **27** | **76** | **26** | **5** |

---

## Category 1: Code identifiers (Rust types, fns, modules, constants)

### Type names and struct definitions
- `crates/wdm-codec/src/policy.rs:608` — `// WdmBackup` — MECHANICAL
- `crates/wdm-codec/src/policy.rs:639` — `pub struct WdmBackup {` — MECHANICAL
- `crates/wdm-codec/src/policy.rs:660` — `impl WdmBackup {` — MECHANICAL
- `crates/wdm-codec/src/bytecode/key.rs:1` — `//! WdmKey — the v0.1 representation...` — MECHANICAL
- `crates/wdm-codec/src/bytecode/key.rs:6` — `//! [`WdmKey::Placeholder`]...` — MECHANICAL
- `crates/wdm-codec/src/bytecode/key.rs:20` — `pub enum WdmKey {` — MECHANICAL

### Re-exports and imports
- `crates/wdm-codec/src/bytecode/mod.rs:13` — `pub use key::WdmKey;` — MECHANICAL
- `crates/wdm-codec/src/lib.rs:163` — `pub use policy::{WalletPolicy, WdmBackup};` — MECHANICAL

### Constants (WDM_* prefix)
- `crates/wdm-codec/src/encoding.rs:183` — `pub const WDM_REGULAR_CONST: u128 = 0x0815c07747a3392e7;` — MECHANICAL
- `crates/wdm-codec/src/encoding.rs:226` — `pub const WDM_LONG_CONST: u128 = 0x205701dd1e8ce4b9f47;` — MECHANICAL
- All 24 references to `WDM_REGULAR_CONST` and `WDM_LONG_CONST` across `encoding.rs`, `encoding/bch_decode.rs`, and tests — these are constant names (not string literals) and should be renamed to `MD_REGULAR_CONST` and `MD_LONG_CONST` — MECHANICAL

### Library imports and uses
- `crates/wdm-codec/tests/conformance.rs:17` — `use wdm_codec::{` — MECHANICAL
- `crates/wdm-codec/tests/conformance.rs:449` — `use wdm_codec::{ChunkCode, ChunkingPlan};` — MECHANICAL
- `crates/wdm-codec/tests/conformance.rs:492` — `use wdm_codec::encoding::{ALPHABET, bch_create_checksum_long};` — MECHANICAL
- `crates/wdm-codec/tests/conformance.rs:606` — `use wdm_codec::bytecode::Tag;` — MECHANICAL
- `crates/wdm-codec/tests/conformance.rs:643` — `use wdm_codec::bytecode::Tag;` — MECHANICAL
- `crates/wdm-codec/tests/conformance.rs:715` — `use wdm_codec::bytecode::Tag;` — MECHANICAL
- `crates/wdm-codec/tests/conformance.rs:755` — `use wdm_codec::bytecode::Tag;` — MECHANICAL
- `crates/wdm-codec/tests/conformance.rs:830` — `use wdm_codec::bytecode::Tag;` — MECHANICAL
- (18 more `use wdm_codec` statements across test files) — MECHANICAL

### Test function names (19 functions in cli.rs)
- `crates/wdm-codec/tests/cli.rs:44` — `fn wdm_encode_default() {` — MECHANICAL (test name, no wire impact)
- `crates/wdm-codec/tests/cli.rs:57` — `fn wdm_encode_json() {` — MECHANICAL
- `crates/wdm-codec/tests/cli.rs:84` — `fn wdm_encode_json_shape_is_stable() {` — MECHANICAL
- `crates/wdm-codec/tests/cli.rs:127` — `fn wdm_decode_json_shape_is_stable() {` — MECHANICAL
- `crates/wdm-codec/tests/cli.rs:189` — `fn wdm_encode_path_override_bip48_takes_effect() {` — MECHANICAL
- `crates/wdm-codec/tests/cli.rs:246` — `fn wdm_encode_force_chunked() {` — MECHANICAL
- `crates/wdm-codec/tests/cli.rs:275` — `fn wdm_decode_round_trip() {` — MECHANICAL
- `crates/wdm-codec/tests/cli.rs:297` — `fn wdm_verify_match() {` — MECHANICAL
- `crates/wdm-codec/tests/cli.rs:308` — `fn wdm_verify_mismatch() {` — MECHANICAL
- `crates/wdm-codec/tests/cli.rs:319` — `fn wdm_inspect_outputs_chunk_header() {` — MECHANICAL
- `crates/wdm-codec/tests/cli.rs:332` — `fn wdm_bytecode_outputs_lowercase_hex() {` — MECHANICAL
- `crates/wdm-codec/tests/cli.rs:347` — `fn wdm_encode_unparseable_policy_exits_nonzero() {` — MECHANICAL
- `crates/wdm-codec/tests/cli.rs:358` — `fn wdm_decode_invalid_string_exits_nonzero() {` — MECHANICAL
- `crates/wdm-codec/tests/cli.rs:374` — `fn wdm_vectors_returns_json_top_level_object() {` — MECHANICAL
- `crates/wdm-codec/tests/cli.rs:385` — `fn wdm_unknown_subcommand_exits_nonzero() {` — MECHANICAL
- `crates/wdm-codec/tests/cli.rs:401` — `fn wdm_encode_fingerprint_flag_accepts_two_placeholders() {` — MECHANICAL
- `crates/wdm-codec/tests/cli.rs:422` — `fn wdm_encode_fingerprint_flag_rejects_index_gap() {` — MECHANICAL
- `crates/wdm-codec/tests/cli.rs:442` — `fn wdm_encode_fingerprint_flag_rejects_short_hex() {` — MECHANICAL

### Test function names (other modules)
- `crates/wdm-codec/src/policy.rs:1228` — `fn wdm_backup_wallet_id_round_trips_via_words()` — MECHANICAL
- `crates/wdm-codec/src/policy.rs:1434` — `fn wdm_backup_struct_construction()` — MECHANICAL
- `crates/wdm-codec/src/encoding.rs:854` — `fn wdm_target_constants_match_nums_derivation()` — MECHANICAL

---

## Category 2: Doc comments

### Crate-level and module-level documentation
- `crates/wdm-codec/src/encode.rs:1` — `//! Top-level encode pipeline: `WalletPolicy` → `WdmBackup`.` — MECHANICAL
- `crates/wdm-codec/src/encode.rs:13` — `/// Encode a wallet policy as a [`WdmBackup`]:...` — MECHANICAL
- `crates/wdm-codec/src/encode.rs:29` — `/// 6. **Result** — a [`WdmBackup`] containing...` — MECHANICAL
- `crates/wdm-codec/src/lib.rs:40` — `//! The encode pipeline is `WalletPolicy + EncodeOptions → WdmBackup`;...` — MECHANICAL
- `crates/wdm-codec/src/lib.rs:58` — `//! 6. **Output** — a [`WdmBackup`] holds the encoded chunks + words;...` — MECHANICAL
- `crates/wdm-codec/src/lib.rs:68` — `//!  WalletPolicy ──── encode() ──→ WdmBackup ─[serialize chunks]` — MECHANICAL
- `crates/wdm-codec/src/lib.rs:76` — `//! - Encode to a [`WdmBackup`], whose `chunks: Vec<EncodedChunk>`...` — MECHANICAL
- `crates/wdm-codec/src/lib.rs:122` — `//! - [`policy`] — [`WalletPolicy`] newtype + [`WdmBackup`] struct.` — MECHANICAL

### Comment strings in code
- `crates/wdm-codec/src/bytecode/key.rs:1` — `//! WdmKey — the v0.1 representation of a key reference inside...` — MECHANICAL
- `crates/wdm-codec/src/bytecode/key.rs:5` — `//! [`WdmKey::Placeholder`] referencing...` — MECHANICAL
- `crates/wdm-codec/src/bin/wdm/json.rs:5` — `//! The library types involved in `--json` output (`WdmBackup`, `EncodedChunk`,` — MECHANICAL
- `crates/wdm-codec/src/encoding.rs:218` — `/// See [`WDM_REGULAR_CONST`] for the derivation method...` — MECHANICAL
- `crates/wdm-codec/src/encoding.rs:310` — `/// then XORs the result with [`WDM_REGULAR_CONST`]...` — MECHANICAL
- `crates/wdm-codec/src/encoding.rs:343` — `/// constant ([`WDM_LONG_CONST`]). Produces a 15-element checksum array.` — MECHANICAL
- `crates/wdm-codec/src/encoding/bch_decode.rs:496` — `///   `polymod(hrp_expand(hrp) || data_with_checksum) ⊕ WDM_REGULAR_CONST`.` — MECHANICAL
- `crates/wdm-codec/src/decode_report.rs:129` — `/// Pair this with [`crate::WdmBackup`] from the encode side...` — MECHANICAL
- `crates/wdm-codec/src/decode_report.rs:130` — `/// type-state graph: encode produces `WdmBackup`, decode produces` — MECHANICAL
- `crates/wdm-codec/src/decode_report.rs:131` — `/// `DecodeResult`. The `WdmBackup` is the engraving-side artifact;` — MECHANICAL

---

## Category 3: String literals

### Error/output messages (CONTEXTUAL — check surrounding code for message scope)
- `crates/wdm-codec/src/policy.rs:671` — `/// `WdmBackup` values.` — MECHANICAL
- `tests/cli.rs:46` — assert string containing `wdm` binary name — CONTEXTUAL (user-facing)

### Generator string in vectors.rs (WIRE — will change during Phase 6 regen)
- `crates/wdm-codec/src/vectors.rs:578-583` — `pub const GENERATOR_FAMILY: &str = concat!("wdm-codec ", ...);` — **WIRE** (part of vector regeneration; hardcoded string must change to `"md-codec "`)

### Doctest examples and comments
- `crates/wdm-codec/src/policy.rs:141` — `/// use wdm_codec::WalletPolicy;` — MECHANICAL
- `crates/wdm-codec/src/policy.rs:144` — `/// # Ok::<(), wdm_codec::Error>(())` — MECHANICAL
- `crates/wdm-codec/README.md:30` — `use wdm_codec::{decode, encode, DecodeOptions, EncodeOptions, WalletPolicy};` — MECHANICAL
- `crates/wdm-codec/README.md:45` — `# Ok::<(), wdm_codec::Error>(())` — MECHANICAL

### Changelog references (HISTORICAL — do NOT rewrite)
- `CHANGELOG.md:3` — `All notable changes to `wdm-codec` are documented in this file.` — HISTORICAL
- `CHANGELOG.md:40` — Multiple references to `WDM string`, `wdm_codec::decode()`, `wdm decode`, `wdm verify` — **17 lines total** — HISTORICAL
- `CHANGELOG.md:81-83` — `generator` field (`"wdm-codec 0.2"` → `"wdm-codec 0.2.x"`) — HISTORICAL (describes v0.2.1 change; v0.3.0 will have `"md-codec"`)
- `CHANGELOG.md:136` — `crates/wdm-codec/tests/vectors/*.json` path reference — HISTORICAL (describes v0.2.0 feature)
- `CHANGELOG.md:143` — `wdm-codec` crate name in workspace patch note — HISTORICAL
- `CHANGELOG.md:151-161` — Tag references and crate name in release notes section — **7 lines** — HISTORICAL

---

## Category 4: Filenames + directory names

(Every path containing `wdm` — list all, since these need git-mv)

- `crates/wdm-codec/` — entire crate directory — MECHANICAL (Phase 3: `git mv crates/wdm-codec crates/md-codec`)
- `crates/wdm-codec/Cargo.toml` — will move with crate; `[package] name = "wdm-codec"` inside will change — MECHANICAL
- `crates/wdm-codec/src/bin/wdm/` — binary source directory — MECHANICAL (Phase 3: `git mv crates/md-codec/src/bin/wdm crates/md-codec/src/bin/md`)
- `crates/wdm-codec/README.md` — file will move with crate rename; internal references also need update — MECHANICAL

---

## Category 5: CI config

(`.github/workflows/*.yml` — no references to old identifiers found)

- `.github/workflows/ci.yml` — **no occurrences of `wdm` or `WDM`** — CI is name-agnostic (uses `cargo test --workspace` which auto-discovers by crate name)

---

## Category 6: Test vector contents

(Committed JSON files; expected `wdm1...` strings inside)

### All instances of bech32 prefix `wdm1` (72 occurrences across two files, WIRE)
- `crates/wdm-codec/tests/vectors/v0.1.json` — **50 lines** containing `"wdm1..."` strings — **WIRE** (will change to `"md1..."` after Phase 6 regen)
- `crates/wdm-codec/tests/vectors/v0.2.json` — **22 lines** containing `"wdm1..."` strings — **WIRE** (will change to `"md1..."` after Phase 6 regen)

Note: These are not hand-edits. The entire content of both files is regenerated by `gen_vectors --output` in Phase 6 with the new HRP embedded in the polymod input.

---

## Category 7: BIP normative text

### File and title references
- `bip/bip-wallet-descriptor-mnemonic.mediawiki` — filename (must `git mv` to `bip-mnemonic-descriptor.mediawiki` in Phase 2) — MECHANICAL
- `bip/bip-wallet-descriptor-mnemonic.mediawiki:4` — `Title: Wallet Descriptor Mnemonic` — MECHANICAL
- `bip/bip-wallet-descriptor-mnemonic.mediawiki:16` — `This BIP defines the '''Wallet Descriptor Mnemonic''' (WDM) format:...` — MECHANICAL

### Acronym expansion text
- `bip/bip-wallet-descriptor-mnemonic.mediawiki:16` — `'''Wallet Descriptor Mnemonic''' (WDM)` — MECHANICAL
- `bip/bip-wallet-descriptor-mnemonic.mediawiki:16` — `(WDM)` acronym expansion — MECHANICAL

### HRP-expansion constants (WIRE)
- `bip/bip-wallet-descriptor-mnemonic.mediawiki:??` — §"Checksum" section pre-computed HRP-expansion bytes for `wdm` (currently `[3, 3, 3, 0, 23, 4, 13]`) — **WIRE** (will change to `[3, 3, 0, 13, 4]` for `md` per decision log Phase 2)

### Example HRP strings
- Multiple `wdm1...` example strings in BIP prose (expected behavior examples, not test vectors) — will remain as-is (they are illustrative; actual examples will be regenerated)

---

## Category 8: Tier-2 docs

### README.md (top-level)
- `README.md:1` — `# Wallet Descriptor Mnemonic (WDM)` — MECHANICAL
- `README.md:29` — `└── bip-wallet-descriptor-mnemonic.mediawiki   ← the formal BIP draft` — MECHANICAL
- `README.md:42` — `bip/bip-wallet-descriptor-mnemonic.mediawiki` is the canonical spec.` — MECHANICAL
- `README.md:46` — `design/CORPUS.md and the locked test vectors at `crates/wdm-codec/tests/vectors/v0.1.json`.` — MECHANICAL
- `README.md:75` — Reference to `crates/wdm-codec/` implementation — MECHANICAL
- `README.md:77` — Reference to `crates/wdm-codec/` — MECHANICAL
- `README.md:82` — Reference to `crates/wdm-codec/tests/vectors/v0.1.json` — MECHANICAL
- `README.md:99` — `crates/wdm-codec/` is released under... — MECHANICAL

### Crate-specific README
- `crates/wdm-codec/README.md:1` — `# wdm-codec` — MECHANICAL
- `crates/wdm-codec/README.md:3` — `Reference implementation of the **Wallet Descriptor Mnemonic (WDM)** format` — MECHANICAL
- `crates/wdm-codec/README.md:11` — `See the [BIP draft](../../bip/bip-wallet-descriptor-mnemonic.mediawiki)` — MECHANICAL
- `crates/wdm-codec/README.md:23` — `wdm-codec = "0.1"` in code block — MECHANICAL
- `crates/wdm-codec/README.md:51` — `[rustdoc-crate]: https://docs.rs/wdm-codec` — EXTERNAL (docs.rs badge URL)
- `crates/wdm-codec/README.md:60` — `wdm-codec = { version...` — MECHANICAL
- `crates/wdm-codec/README.md:80` — `cargo run -p wdm-codec --bin wdm -- encode...` — MECHANICAL
- `crates/wdm-codec/README.md:86` — `cargo install --path crates/wdm-codec` — MECHANICAL
- `crates/wdm-codec/README.md:101` — `cargo run -p wdm-codec --bin gen_vectors -- --output crates/wdm-codec/tests/vectors/v0.1.json` — MECHANICAL
- `crates/wdm-codec/README.md:107` — `cargo run -p wdm-codec --bin gen_vectors -- --verify crates/wdm-codec/tests/vectors/v0.1.json` — MECHANICAL

### BIP subdir README
- `bip/README.md:6` — Reference to `bip-wallet-descriptor-mnemonic.mediawiki` — MECHANICAL
- `bip/README.md:19` — Reference to `../crates/wdm-codec/` — MECHANICAL
- `bip/README.md:27` — Reference to `../crates/wdm-codec/tests/vectors/v0.1.json` — MECHANICAL
- `bip/README.md:29` — Reference to `../crates/wdm-codec/src/vectors.rs` — MECHANICAL
- `bip/README.md:31` — Reference to `../crates/wdm-codec/README.md` — MECHANICAL

### MIGRATION.md (tier-2 versioning guide)
- `MIGRATION.md:3` — `Migration steps for upgrading between major releases of `wdm-codec`.` — MECHANICAL
- `MIGRATION.md:11` — References to `v0.1.json` and `v0.2.0` vectors — MECHANICAL
- `MIGRATION.md:170-182` — Multiple references to `crates/wdm-codec/tests/vectors/v0.*.json` and dependency `wdm-codec` — MECHANICAL

### CHANGELOG.md (17 HISTORICAL lines — do NOT rewrite)
- **Lines 3, 17, 40-41 (x4), 81-83 (x3), 136, 143, 151, 157-161 (x5)** — all tagged HISTORICAL per Category 3 above

---

## Category 9: Cargo manifest fields

### Root workspace
- `Cargo.toml:3` — `members = ["crates/wdm-codec"]` — MECHANICAL (will change to `"crates/md-codec"`)
- `Cargo.toml:17` — `# Temporary patch redirect: wdm-codec pins miniscript...` — MECHANICAL (comment)
- `Cargo.toml:23` — `# fix lands and we bump the SHA pin in wdm-codec/Cargo.toml...` — MECHANICAL (comment)

### Crate Cargo.toml
- `crates/wdm-codec/Cargo.toml:2` — `name = "wdm-codec"` — MECHANICAL
- `crates/wdm-codec/Cargo.toml:8` — `description = "Reference implementation of the Wallet Descriptor Mnemonic (WDM) format..."` — EXTERNAL (crates.io description)
- `crates/wdm-codec/Cargo.toml:15` — `name = "wdm_codec"` — MECHANICAL (lib name)
- `crates/wdm-codec/Cargo.toml:18` — `name = "wdm"` — MECHANICAL (binary name)

---

## Category 10: External-facing strings

### CLI help text and binary name
- `crates/wdm-codec/src/bin/wdm/main.rs` — binary invocation name (visible in `--help` and error messages) — **EXTERNAL** (Phase 5: update CLI docs)
- Test assertions checking CLI output that includes binary name — **CONTEXTUAL** (e.g., usage strings)

### docs.rs badge
- `crates/wdm-codec/README.md:51` — `[rustdoc-crate]: https://docs.rs/wdm-codec` — **EXTERNAL** (will be docs.rs/md-codec after crate rename)

### Error message strings
- Anywhere `format!()` or `println!()` embeds the crate/format name — search tests for assertion strings like `"wdm decode"` — **CONTEXTUAL**

---

## Category 11: Auto-memory files

(Only `/home/bcg/.claude/projects/-scratch-code-shibboleth/memory/`)

Files referencing old names (do NOT rewrite — they are project memory):
- `/home/bcg/.claude/projects/-scratch-code-shibboleth/memory/MEMORY.md` — project index — HISTORICAL
- `/home/bcg/.claude/projects/-scratch-code-shibboleth/memory/project_no_bash_shell_impl.md` — project decision — HISTORICAL
- `/home/bcg/.claude/projects/-scratch-code-shibboleth/memory/feedback_agent_review.md` — agent feedback — HISTORICAL
- `/home/bcg/.claude/projects/-scratch-code-shibboleth/memory/feedback_subagent_workflow.md` — workflow feedback — HISTORICAL
- `/home/bcg/.claude/projects/-scratch-code-shibboleth/memory/project_followups_tracking.md` — followups tracking — HISTORICAL
- `/home/bcg/.claude/projects/-scratch-code-shibboleth/memory/feedback_worktree_dispatch.md` — worktree notes — HISTORICAL
- `/home/bcg/.claude/projects/-scratch-code-shibboleth/memory/project_shibboleth_wallet.md` — related project notes — HISTORICAL
- `/home/bcg/.claude/projects/-scratch-code-shibboleth/memory/project_apoelstra_pr_check.md` — upstream tracking — HISTORICAL

(Phase 9 will add a new memory file: `project_renamed_wdm_to_md.md`)

---

## Execution notes

- **Phase 2 (BIP):** Must come first. Rename file `bip/bip-wallet-descriptor-mnemonic.mediawiki` → `bip/bip-mnemonic-descriptor.mediawiki`. Update HRP-expansion bytes in §"Checksum" from `[3,3,3,0,23,4,13]` to `[3,3,0,13,4]`.
- **Phase 3 (Cargo):** `git mv crates/wdm-codec crates/md-codec`. Update `Cargo.toml` fields: `[package] name`, `[lib] name`, `[[bin]] name`, `[[bin]] path`.
- **Phase 4 (Identifiers):** Mechanical replace: `use wdm_codec::` → `use md_codec::`. Rename `WdmBackup` → `MdBackup`, `WdmKey` → `MdKey`. Rename constants `WDM_REGULAR_CONST` → `MD_REGULAR_CONST`, `WDM_LONG_CONST` → `MD_LONG_CONST`. Rename 19 test functions `fn wdm_*()` → `fn md_*()`. Rename 2 inline test functions in `policy.rs` and `encoding.rs`.
- **Phase 5 (Strings):** Update generator string in `vectors.rs:579` from `"wdm-codec "` to `"md-codec "`. Update README examples and doc links. Update CLI help text.
- **Phase 6 (Vectors, WIRE):** Run `gen_vectors --output crates/md-codec/tests/vectors/v0.1.json` and `v0.2.json`. All 72 bech32 prefix instances change to `md1`. Capture new SHAs. Update vector lock constants in tests.

---

## No-touch zones (confirmed out of scope)

- `target/` (build artifacts) — excluded
- `.git/` (git internals) — excluded
- `.claude/worktrees/` (temporary worktree artifacts) — excluded
- `/scratch/code/shibboleth/shibboleth-wallet/` (separate project) — explicitly excluded per instructions

