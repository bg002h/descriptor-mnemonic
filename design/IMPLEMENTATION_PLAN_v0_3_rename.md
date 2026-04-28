# Implementation Plan: wdm → md Rename (v0.3.0)

**Source**: `design/RENAME_WORKFLOW.md` (Phases 2-11 procedure), `design/RENAME_v0_3_wdm_to_md.md` (decisions), `design/agent-reports/v0-2-3-rename-discovery.md` (touch-point inventory).

**Total estimated touch points**: 571 (101 MECHANICAL + 27 CONTEXTUAL + 76 WIRE + 26 HISTORICAL + 5 EXTERNAL; HISTORICAL items left untouched, so 545 actionable).

**Estimated phases**: 9 in-repo (Phases 2-10) + Phase 11 post-release (SLIP-0173 PR, deferred).

**SemVer target**: v0.3.0. **Wire-format break**: yes (HRP changes; HRP-expansion goes from 7 bytes to 5 bytes).

**Family-stable promise**: resets at v0.3.0 (`"md-codec 0.3"` is the new family token).

---

## Pre-execution checklist

- [ ] Worktree created: `git worktree add ../descriptor-mnemonic-rename rename/v0.3-wdm-to-md` (REQUIRED per `superpowers:using-git-worktrees` and the user's worktree-dispatch memory). Worktree branches cut from `origin/main`, so all design docs MUST be committed and pushed BEFORE worktree creation.
- [ ] Implementer subagent prompts include `RUSTUP_TOOLCHAIN=stable` prefix on EVERY cargo invocation (per project session memory — repo pins to stable).
- [ ] Implementer subagent prompts quote the discovery report's HISTORICAL list verbatim and forbid edits to those files (CHANGELOG.md historical lines, all 8 auto-memory files, BIP existing `wdm1...` example strings if any are tagged historical).
- [ ] After EVERY phase that touches code: 4 mandatory gates pass:
  1. `RUSTUP_TOOLCHAIN=stable cargo build --workspace --all-targets`
  2. `RUSTUP_TOOLCHAIN=stable cargo test -p md-codec` (or `wdm-codec` until Phase 3 completes)
  3. `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets -- -D warnings`
  4. `RUSTUP_TOOLCHAIN=stable cargo fmt --check`
- [ ] After Phase 6: also `RUSTUP_TOOLCHAIN=stable cargo run --quiet -p md-codec --bin gen_vectors -- --verify crates/md-codec/tests/vectors/v0.1.json` and `--verify crates/md-codec/tests/vectors/v0.2.json` — both PASS.
- [ ] Commits per phase: one focused commit per phase (or per sub-batch within a phase).

---

## Phase 2 — BIP / Spec rename

**Why first**: The BIP is the source of truth for HRP-expansion. Reviewers cross-check code against updated normative text, so the BIP must change first. The HRP-expansion byte sequence changes length (7 → 5), and that recomputation must be visible in the spec before any code references it.

**Entry condition**: All design docs (RENAME_WORKFLOW.md, RENAME_v0_3_wdm_to_md.md, discovery report, this plan) committed and pushed to `origin/main`. Worktree `../descriptor-mnemonic-rename` created on branch `rename/v0.3-wdm-to-md` cut from `origin/main`. Working tree clean.

**Edits** (extracted from discovery Category 7 and decision-log "HRP-expansion derivation"):

MECHANICAL group (do as a batch first):
- `git mv bip/bip-wallet-descriptor-mnemonic.mediawiki bip/bip-mnemonic-descriptor.mediawiki`
- Inside the renamed file:
  - Line 4: `Title: Wallet Descriptor Mnemonic` → `Title: Mnemonic Descriptor`
  - Line 16: `'''Wallet Descriptor Mnemonic''' (WDM)` → `'''Mnemonic Descriptor''' (MD)`
  - All other prose mentions of `Wallet Descriptor Mnemonic` → `Mnemonic Descriptor`
  - All other prose mentions of `WDM` → `MD`
  - All `wdm1...` example strings in BIP prose → `md1...` (note: actual generated examples will be regenerated in Phase 6; these prose illustrations are MECHANICAL textual replacements only)
  - All `wdm-codec` crate-name mentions → `md-codec`

CONTEXTUAL group:
- BIP §"Abstract" — rewrite the opening sentence to read naturally with "Mnemonic Descriptor" as the noun phrase (not just substring replace; the word order may change).
- Any "Naming rationale" section (if present) — rewrite to reflect the new name.

WIRE group (do last in this phase):
- BIP §"Checksum" item 1: replace literal `[3, 3, 3, 0, 23, 4, 13]` (length 7) with `[3, 3, 0, 13, 4]` (length 5). Update prose to read: "For the lowercase HRP `md` this yields the five 5-bit values `[3, 3, 0, 13, 4]` (length = 2*len(HRP)+1 = 5)." Show the derivation: `ord('m')=109; 109>>5=3, 109&31=13`. `ord('d')=100; 100>>5=3, 100&31=4`. With separator zero between high-half and low-half.
- BIP §"Test vectors" SHA reference: leave the OLD v0.2.json SHA in place but add an inline TODO comment `<!-- TODO Phase 6: update to new v0.2.json SHA after regen -->`. Will be replaced in Phase 6.

**Gates**: BIP file renders cleanly as mediawiki (no broken syntax). No cargo gates needed (no Rust code touched yet); skip cargo gates this phase.

**Exit condition**: BIP file renamed, retitled, all WDM→MD references rewritten, HRP-expansion bytes corrected to length-5 form, SHA reference TODO inserted. Working tree shows the rename + edits as a single logical change.

**Estimated effort**: 1-2 hours. ~25-30 individual edits. Single commit: `spec(bip): rename Wallet Descriptor Mnemonic → Mnemonic Descriptor; recompute HRP-expansion for md`.

---

## Phase 3 — Cargo + lib/bin renames

**Why before identifier sweep**: Crate name and directory must move BEFORE `use wdm_codec::` becomes `use md_codec::`, because `cargo build` resolves `use` paths against the lib crate name. Doing identifiers first would leave the build red.

**Entry condition**: Phase 2 committed and pushed. Working tree clean.

**Edits** (from discovery Category 4 and Category 9):

Step 1 — root workspace `Cargo.toml`:
- Line 3: `members = ["crates/wdm-codec"]` → `members = ["crates/md-codec"]`
- Line 17 comment: `# Temporary patch redirect: wdm-codec pins miniscript...` → `# Temporary patch redirect: md-codec pins miniscript...`
- Line 23 comment: `# fix lands and we bump the SHA pin in wdm-codec/Cargo.toml...` → `# fix lands and we bump the SHA pin in md-codec/Cargo.toml...`

Step 2 — crate `Cargo.toml` (edit BEFORE the directory move so paths resolve):
- Line 2: `name = "wdm-codec"` → `name = "md-codec"`
- Line 8 (description, EXTERNAL — visible on crates.io): `description = "Reference implementation of the Wallet Descriptor Mnemonic (WDM) format..."` → `description = "Reference implementation of the Mnemonic Descriptor (MD) format..."` (preserve any trailing text about scope)
- Line 15: `name = "wdm_codec"` → `name = "md_codec"` (lib name, snake_case)
- Line 18: `name = "wdm"` → `name = "md"` (binary name)
- Update `[[bin]] path` if it references `src/bin/wdm/main.rs` → `src/bin/md/main.rs`

Step 3 — directory moves:
- `git mv crates/wdm-codec crates/md-codec`
- `git mv crates/md-codec/src/bin/wdm crates/md-codec/src/bin/md`

Step 4 — refresh `Cargo.lock`:
- `RUSTUP_TOOLCHAIN=stable cargo update --workspace` (avoid bare `cargo update` which can churn unrelated deps)
- Inspect diff: should touch only the local crate's package entry; if other deps churn, revert and target more narrowly.

**Gates**: `cargo metadata` succeeds (verifies the manifest parses). `cargo build` will be RED until Phase 4 lands — that's expected. Defer the 4 mandatory gates to end of Phase 4.

**Exit condition**: Crate dir is `crates/md-codec/`, bin dir is `crates/md-codec/src/bin/md/`, manifest fields updated, `Cargo.lock` refreshed.

**Estimated effort**: 30 minutes. Single commit: `cargo: rename wdm-codec → md-codec; lib wdm_codec → md_codec; bin wdm → md`.

---

## Phase 4 — Identifier mass-rename

**Entry condition**: Phase 3 committed. Crate and directories renamed; `cargo build` may be red due to stale `use` paths.

**Edits** (from discovery Category 1: 47 MECHANICAL + 4 CONTEXTUAL; plus Category 2: 8 doc-comment MECHANICAL + 3 CONTEXTUAL):

PRE-STEP — temporarily disable SHA-lock tests so Phase 4 gates can pass without Phase 6 regen:
- In `crates/md-codec/tests/vectors_schema.rs` (or wherever `V0_1_SHA256` and `V0_2_SHA256` constants lock vectors): mark the lock-assertion tests with `#[ignore]` and a `// TODO Phase 6: re-enable after vector regen` comment. Do NOT change the constants themselves.
- Also grep for any test that asserts on the literal `"wdm-codec "` string (likely related to GENERATOR_FAMILY) and `#[ignore]` those too — they'd break in Phase 5 when the constant changes.

Sub-batch 4a — `use` path imports (most surface; do first):
- Replace `use wdm_codec::` → `use md_codec::` across ALL files. Use Edit `replace_all=true` per file.
- Specific file list from discovery: `tests/conformance.rs` (lines 17, 449, 492, 606, 643, 715, 755, 830, plus 18 more `use wdm_codec` statements in test files).
- Run `cargo build` after this sub-batch.

Sub-batch 4b — type renames:
- `WdmBackup` → `MdBackup` (across `policy.rs`, `lib.rs`, `encode.rs`, `decode_report.rs`, `bin/md/json.rs` doc-comment)
  - `crates/md-codec/src/policy.rs:608` (comment), `:639` (struct def), `:660` (impl), `:671` (doc comment)
  - `crates/md-codec/src/lib.rs:163` (re-export), `:40`, `:58`, `:68`, `:76`, `:122` (doc comments)
  - `crates/md-codec/src/encode.rs:1`, `:13`, `:29` (doc comments)
  - `crates/md-codec/src/decode_report.rs:129`, `:130`, `:131` (doc comments)
  - `crates/md-codec/src/bin/md/json.rs:5` (doc comment — note the path now reflects Phase 3 bin dir rename)
- `WdmKey` → `MdKey`
  - `crates/md-codec/src/bytecode/key.rs:1`, `:6`, `:20`
  - `crates/md-codec/src/bytecode/mod.rs:13`
- Run `cargo build` after this sub-batch.

Sub-batch 4c — constant renames (24 references total per discovery):
- `WDM_REGULAR_CONST` → `MD_REGULAR_CONST` (at definition `crates/md-codec/src/encoding.rs:183` and all references)
- `WDM_LONG_CONST` → `MD_LONG_CONST` (at definition `crates/md-codec/src/encoding.rs:226` and all references)
- All 24 references span `encoding.rs`, `encoding/bch_decode.rs` (note `:496` doc-comment reference), and tests. Use `Edit` with `replace_all=true` per file — these are unambiguous tokens.
- Run `cargo build` after this sub-batch.

Sub-batch 4d — test function name renames (22 functions total):
- `crates/md-codec/tests/cli.rs`: 19 functions `fn wdm_*()` → `fn md_*()` at lines 44, 57, 84, 127, 189, 246, 275, 297, 308, 319, 332, 347, 358, 374, 385, 401, 422, 442. (Use `replace_all=true` after a careful regex check that no body text contains `wdm_` — discovery suggests test bodies use `wdm` only in CLI assertion strings, handled in Phase 5.)
- `crates/md-codec/src/policy.rs:1228`: `fn wdm_backup_wallet_id_round_trips_via_words()` → `fn md_backup_wallet_id_round_trips_via_words()`
- `crates/md-codec/src/policy.rs:1434`: `fn wdm_backup_struct_construction()` → `fn md_backup_struct_construction()`
- `crates/md-codec/src/encoding.rs:854`: `fn wdm_target_constants_match_nums_derivation()` → `fn md_target_constants_match_nums_derivation()`
- Run `cargo build` and `cargo test` after this sub-batch.

CONTEXTUAL group (do last in Phase 4 — implementer reads surrounding code):
- 4 CONTEXTUAL items in Category 1 — implementer must inspect each occurrence's surrounding code to confirm the rename is semantically correct (not just textually). Likely candidates: doctest assertions, test setup with the binary name embedded, ASCII-art diagrams in doc comments (`crates/md-codec/src/lib.rs:68`).
- 3 CONTEXTUAL items in Category 2 — module-level doc comments where the prose talks about the format informally; rewrite for clarity, not just substring replace.

**Gates** (after the full phase, all sub-batches complete):
- `RUSTUP_TOOLCHAIN=stable cargo build --workspace --all-targets` PASS
- `RUSTUP_TOOLCHAIN=stable cargo test -p md-codec` PASS (with SHA-lock tests `#[ignore]`d)
- `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets -- -D warnings` PASS
- `RUSTUP_TOOLCHAIN=stable cargo fmt --check` PASS

**Exit condition**: All `wdm`-derived identifiers renamed to `md` form. SHA-lock tests temporarily ignored. Test suite green except for those ignored locks.

**Estimated effort**: 2-3 hours. Commit per sub-batch (4a, 4b, 4c, 4d, CONTEXTUAL). Final commit message: `code: rename WDM→MD identifiers (types, constants, test fns)`.

---

## Phase 5 — String literal sweep

**Entry condition**: Phase 4 committed; build + clippy + fmt green; tests green except SHA-lock ignores.

**Edits** (from discovery Category 3: 15 MECHANICAL + 8 CONTEXTUAL + 2 WIRE + 3 EXTERNAL; Category 10: 3 MECHANICAL + 2 CONTEXTUAL + 1 EXTERNAL):

MECHANICAL group:
- `crates/md-codec/src/policy.rs:141`: `/// use wdm_codec::WalletPolicy;` → `/// use md_codec::WalletPolicy;` (doctest)
- `crates/md-codec/src/policy.rs:144`: `/// # Ok::<(), wdm_codec::Error>(())` → `/// # Ok::<(), md_codec::Error>(())`
- `crates/md-codec/README.md:30`: `use wdm_codec::{...}` → `use md_codec::{...}`
- `crates/md-codec/README.md:45`: `# Ok::<(), wdm_codec::Error>(())` → `# Ok::<(), md_codec::Error>(())`
- All other doctest fragments referencing `wdm_codec::` across the crate (grep `wdm_codec` after Phase 4 — should be in doc comments only).

CONTEXTUAL group (8 items — implementer reads context):
- `crates/md-codec/tests/cli.rs:46`: assert string containing `wdm` binary name → `md` (the test asserts CLI output contains the binary name; verify scope after rename).
- Other test assertion strings checking CLI `--help` output, error messages embedding the format/binary name. Implementer must grep `wdm` and `WDM` across test files and read surrounding 5-10 lines to determine each replacement.
- Format-string args in `format!()`, `println!()`, `eprintln!()` that embed the format name in user-visible output.

WIRE group (2 items — handle here, NOT in Phase 6):
- `crates/md-codec/src/vectors.rs:578-583`: `pub const GENERATOR_FAMILY: &str = concat!("wdm-codec ", env!("CARGO_PKG_VERSION_MAJOR"), ".", env!("CARGO_PKG_VERSION_MINOR"));` — change literal `"wdm-codec "` to `"md-codec "`.
  - Note: at v0.3.0 the value resolves to `"md-codec 0.3"`. This is the family-stable token for the v0.3.x series.
  - The string itself is a code edit (Phase 5 territory). The CONSEQUENCE — that vectors regen with this new string baked in — is Phase 6 territory.
- Verify by grepping `"wdm"` in code paths that flow into the polymod input. Likely candidate: a test fixture string that's `wdm1...` literal embedded in a Rust source file (NOT in JSON vectors). If found, this becomes invalid after Phase 6 regen and must be replaced in lockstep.

EXTERNAL group (3 items):
- `crates/md-codec/README.md:51`: `[rustdoc-crate]: https://docs.rs/wdm-codec` → `https://docs.rs/md-codec` (docs.rs badge URL; the page won't exist until v0.3.0 publishes, but the link target is correct)
- `crates/md-codec/Cargo.toml` description field — handled in Phase 3 already.
- CLI `--help` text strings that embed the binary name "wdm" — search `crates/md-codec/src/bin/md/` for any clap `#[arg(help = "...")]` or `#[command(about = "...")]` containing `wdm` or `WDM`.

**Gates**: All 4 mandatory gates PASS. Test suite green (still with SHA-lock ignores).

**Exit condition**: All string literals updated. `GENERATOR_FAMILY` constant changed. Tests green. SHA-lock tests still `#[ignore]`d (re-enabled in Phase 6).

**Estimated effort**: 1-2 hours. Single commit: `code: sweep wdm/WDM string literals → md/MD; update GENERATOR_FAMILY constant`.

---

## Phase 6 — Test vector regeneration (WIRE STEP)

**Why this is its own phase**: This is THE wire-format-breaking step. After it, every old test vector is invalid and every new vector has a different SHA. The polymod input changes because (a) the HRP-expansion bytes change (`[3,3,3,0,23,4,13]` → `[3,3,0,13,4]`), and (b) the GENERATOR_FAMILY string baked into vector metadata changes (`"wdm-codec 0.X"` → `"md-codec 0.3"`).

**Entry condition**: Phases 2-5 committed. Build + clippy + fmt green. Test suite green except SHA-lock tests `#[ignore]`d.

Steps in EXACT order:

1. Regenerate v0.1 vectors:
   ```
   RUSTUP_TOOLCHAIN=stable cargo run --quiet -p md-codec --bin gen_vectors -- --output crates/md-codec/tests/vectors/v0.1.json --schema 1
   ```

2. Regenerate v0.2 vectors:
   ```
   RUSTUP_TOOLCHAIN=stable cargo run --quiet -p md-codec --bin gen_vectors -- --output crates/md-codec/tests/vectors/v0.2.json --schema 2
   ```

3. Capture both new SHAs:
   ```
   sha256sum crates/md-codec/tests/vectors/v0.1.json crates/md-codec/tests/vectors/v0.2.json
   ```
   Record output verbatim — these go into the constants AND into the decision log.

4. Update SHA-lock constants in `crates/md-codec/tests/vectors_schema.rs` (or wherever `V0_1_SHA256` / `V0_2_SHA256` are defined): replace old hex values with new ones from step 3.

5. Re-enable any tests marked `#[ignore]` in Phase 4. Remove the `// TODO Phase 6:` comments.

6. Run vector verifier on both files:
   ```
   RUSTUP_TOOLCHAIN=stable cargo run --quiet -p md-codec --bin gen_vectors -- --verify crates/md-codec/tests/vectors/v0.1.json
   RUSTUP_TOOLCHAIN=stable cargo run --quiet -p md-codec --bin gen_vectors -- --verify crates/md-codec/tests/vectors/v0.2.json
   ```
   Both MUST report PASS.

7. Run full test suite:
   ```
   RUSTUP_TOOLCHAIN=stable cargo test -p md-codec
   ```
   All green.

8. Update BIP §"Test vectors" SHA reference (deferred from Phase 2): replace TODO comment + old SHA with new v0.2.json SHA from step 3. Edit `bip/bip-mnemonic-descriptor.mediawiki`.

9. Update `design/RENAME_v0_3_wdm_to_md.md` — the "Open items at decision-log freeze" section:
   - Replace `<TODO: new v0.1.json SHA after Phase 6 regen>` with the new v0.1 SHA.
   - Replace `<TODO: new v0.2.json SHA after Phase 6 regen>` with the new v0.2 SHA.
   - Replace `<TODO: discovery agent's "surprises" callout — anything not in this decision log that the rename will break>` with a one-line summary citing the discovery report's Surprises section.
   - Replace `<TODO: final touch-point count from discovery vs. plan estimate>` with the actual count (571 from discovery; document any deviation).

**Gates**: All 4 mandatory gates PASS. Both `gen_vectors --verify` calls PASS. The 72 occurrences of `wdm1...` strings in vector JSONs are now `md1...`.

**Exit condition**: Both vector files at NEW SHAs. Vector lock constants updated. SHA-lock tests re-enabled and green. Full test suite green. BIP SHA reference current. Decision log updated with actual SHAs.

**Estimated effort**: 30-45 minutes (mostly run-and-verify). Two commits recommended:
- `vectors: regenerate v0.1 + v0.2 for HRP md; update SHA pins`
- `spec(bip): update v0.2.json SHA reference; design: record final SHAs in decision log`

---

## Phase 7 — CI / release infra

**Entry condition**: Phase 6 committed. All gates green.

**Edits** (from discovery Category 5):

Discovery reports ZERO hits in `.github/workflows/*.yml` for `wdm` / `WDM`. CI is name-agnostic (uses `cargo test --workspace`). Verify this is still true:

1. `grep -rni "wdm" .github/` — expect no matches.
2. `grep -rni "wdm-codec" .github/` — expect no matches.
3. Check `.gitattributes` — look for `crates/wdm-codec/tests/vectors/*.json text eol=lf` line; rename to `crates/md-codec/tests/vectors/*.json text eol=lf` if present. (Discovery did not flag this; verify.)
4. Check for any `Justfile`, `Makefile`, or release scripts at repo root: `ls Justfile justfile Makefile makefile 2>/dev/null` — if any reference `wdm-codec` or `wdm` binary name, update them.
5. Verify GitHub Actions tag-trigger patterns (if present): any workflow triggered by `wdm-codec-v*` tag pattern needs to ALSO accept `md-codec-v*` (or be replaced). Discovery says zero hits, so likely no tag-triggered workflow exists; verify.

**Gates**: All 4 mandatory gates PASS. CI YAML still parses (`gh workflow view` if any change made).

**Exit condition**: CI config audited. If any change made, committed. If discovery confirmed (zero changes), this phase is a NO-OP commit-wise — record verification result in commit message of next phase.

**Estimated effort**: 15-30 minutes (mostly verification). Likely no commit, or one trivial commit: `ci: align release-tag patterns to md-codec-v* (if needed)`.

---

## Phase 8 — Documentation sweep (CHANGELOG / MIGRATION / READMEs)

**Entry condition**: Phase 7 audited. All gates green.

**Edits** (from discovery Category 8: 12 MECHANICAL + 8 CONTEXTUAL + 18 HISTORICAL):

HISTORICAL — DO NOT TOUCH:
- `CHANGELOG.md` lines describing v0.2.x and v0.1.x releases — these describe historical releases and MUST stay verbatim. The implementer prompt MUST list these line ranges and forbid edits.

MECHANICAL group — top-level README.md:
- `README.md:1`: `# Wallet Descriptor Mnemonic (WDM)` → `# Mnemonic Descriptor (MD)`
- `README.md:29`: tree-art line `└── bip-wallet-descriptor-mnemonic.mediawiki   ← the formal BIP draft` → `└── bip-mnemonic-descriptor.mediawiki   ← the formal BIP draft`
- `README.md:42`: `bip/bip-wallet-descriptor-mnemonic.mediawiki` → `bip/bip-mnemonic-descriptor.mediawiki`
- `README.md:46, :75, :77, :82, :99`: replace `crates/wdm-codec` → `crates/md-codec`

MECHANICAL group — crate README at `crates/md-codec/README.md`:
- Line 1: `# wdm-codec` → `# md-codec`
- Line 3: `Reference implementation of the **Wallet Descriptor Mnemonic (WDM)** format` → `Reference implementation of the **Mnemonic Descriptor (MD)** format`
- Line 11: `[BIP draft](../../bip/bip-wallet-descriptor-mnemonic.mediawiki)` → `[BIP draft](../../bip/bip-mnemonic-descriptor.mediawiki)`
- Line 23: `wdm-codec = "0.1"` → `md-codec = "0.3"` (note version bump)
- Line 60: `wdm-codec = { version...` → `md-codec = { version = "0.3"...}`
- Lines 80, 86, 101, 107: replace all `wdm-codec` and `wdm` binary references with `md-codec` / `md`. Specifically:
  - Line 80: `cargo run -p wdm-codec --bin wdm -- encode...` → `cargo run -p md-codec --bin md -- encode...`
  - Line 86: `cargo install --path crates/wdm-codec` → `cargo install --path crates/md-codec`
  - Line 101: `cargo run -p wdm-codec --bin gen_vectors -- --output crates/wdm-codec/tests/vectors/v0.1.json` → `cargo run -p md-codec --bin gen_vectors -- --output crates/md-codec/tests/vectors/v0.1.json`
  - Line 107: similar `--verify` invocation update
- Line 51 (EXTERNAL, already done in Phase 5): verify `https://docs.rs/md-codec`.

MECHANICAL group — `bip/README.md`:
- Line 6: `bip-wallet-descriptor-mnemonic.mediawiki` → `bip-mnemonic-descriptor.mediawiki`
- Lines 19, 27, 29, 31: replace `../crates/wdm-codec/...` → `../crates/md-codec/...`

CONTEXTUAL group — top-level README.md:
- Add a "Renamed from `wdm-codec`" admonition near the top, e.g.:
  > **Note:** This crate was renamed from `wdm-codec` (HRP `wdm`) to `md-codec` (HRP `md`) in v0.3.0. See [MIGRATION.md](./MIGRATION.md#v02x--v030) for upgrade guidance and [CHANGELOG.md](./CHANGELOG.md) for the v0.3.0 entry.
- Verify the crate-status / project-status text near the top still reads correctly.

CONTEXTUAL group — `MIGRATION.md`:
- Line 3: `Migration steps for upgrading between major releases of \`wdm-codec\`.` → `Migration steps for upgrading between major releases of \`md-codec\` (formerly \`wdm-codec\`).`
- Lines 11, 170-182: update all crate-name references and vector path references to `md-codec`.
- ADD a new section at the top of the version-by-version list titled `## v0.2.x → v0.3.0` containing the 6-point scope from decision log §"MIGRATION.md scope":
  1. Wire format: HRP `wdm` → `md`. Re-encode from descriptor source.
  2. Crate name: `wdm-codec = "..."` → `md-codec = "0.3"`.
  3. Library import: `use wdm_codec::...` → `use md_codec::...`. Type renames: `WdmBackup` → `MdBackup`, `WdmKey` → `MdKey`. Constant renames: `WDM_REGULAR_CONST` → `MD_REGULAR_CONST`, `WDM_LONG_CONST` → `MD_LONG_CONST`.
  4. CLI: `wdm encode ...` → `md encode ...`. Subcommand surface unchanged.
  5. Test vector SHAs: both `v0.1.json` and `v0.2.json` SHA pins changed (cite new SHAs from Phase 6).
  6. Repository URL: unchanged.

NEW CHANGELOG entry — `CHANGELOG.md`:
- Add a new top-of-file entry for v0.3.0 ABOVE the existing v0.2.x entries. Template:
  ```
  ## [0.3.0] - 2026-04-?? (target)

  ### Renamed (BREAKING — wire format)

  - **HRP**: `wdm` → `md`. Strings starting with `wdm1...` are no longer valid v0.3.0 inputs. HRP-expansion bytes change from 7 to 5 (`[3,3,3,0,23,4,13]` → `[3,3,0,13,4]`).
  - **Crate**: `wdm-codec` → `md-codec`. Update `Cargo.toml` dependency.
  - **Library**: `wdm_codec` → `md_codec`. Update `use` statements.
  - **Binary**: `wdm` → `md`. Update CLI invocations.
  - **Format name**: "Wallet Descriptor Mnemonic" (WDM) → "Mnemonic Descriptor" (MD).
  - **Type renames**: `WdmBackup` → `MdBackup`; `WdmKey` → `MdKey`.
  - **Constant renames**: `WDM_REGULAR_CONST` → `MD_REGULAR_CONST`; `WDM_LONG_CONST` → `MD_LONG_CONST`.

  ### Test vectors (regenerated)

  - `crates/md-codec/tests/vectors/v0.1.json` — new SHA-256: `<from Phase 6>`
  - `crates/md-codec/tests/vectors/v0.2.json` — new SHA-256: `<from Phase 6>`
  - Family-stable promise resets: `"md-codec 0.3"` is the new family token for v0.3.x patches.

  ### Migration

  See [MIGRATION.md §v0.2.x → v0.3.0](./MIGRATION.md#v02x--v030).

  ### Notes

  - Past releases `wdm-codec-v0.2.0` through `v0.2.3` remain published with deprecation banners on their GitHub Release notes; tags untouched.
  - Repository URL unchanged: `https://github.com/bg002h/descriptor-mnemonic`.
  - Test count: <fill from Phase 6 cargo test output>.
  ```

- `design/FOLLOWUPS.md`: close any open items the rename addresses. Add a new entry: `slip-0173-register-md-hrp` (per decision log Pre-flight Gate 1 defensive follow-up). Add anything discovered during execution that didn't make this rename.

**Gates**: All 4 mandatory gates PASS. BIP file still renders cleanly (mediawiki preview if available). README links resolve.

**Exit condition**: All tier-2 docs (READMEs, MIGRATION, CHANGELOG, FOLLOWUPS) updated. Historical CHANGELOG entries verified untouched. New v0.3.0 CHANGELOG entry drafted. New MIGRATION section drafted.

**Estimated effort**: 1-2 hours. Multiple commits recommended:
- `docs: rename wdm-codec → md-codec across READMEs`
- `docs: add MIGRATION v0.2.x → v0.3.0 section`
- `docs: add CHANGELOG v0.3.0 entry`
- `design: close rename followups; add slip-0173 followup`

---

## Phase 9 — Memory updates

**Entry condition**: Phase 8 committed. All gates green.

**Edits** (from discovery Category 11: 8 HISTORICAL files):

Files at `/home/bcg/.claude/projects/-scratch-code-shibboleth/memory/`:
- `MEMORY.md` — index hooks; ADD a section/link for the new memory file (do NOT rewrite existing entries that reference wdm-codec — those describe historical state).
- `project_no_bash_shell_impl.md`, `feedback_agent_review.md`, `feedback_subagent_workflow.md`, `project_followups_tracking.md`, `feedback_worktree_dispatch.md`, `project_shibboleth_wallet.md`, `project_apoelstra_pr_check.md` — these are HISTORICAL records. The discovery report tags them HISTORICAL precisely because they capture state-at-time-of-writing. The implementer should READ each one and decide per-file:
  - If the reference is purely historical ("at the time, the crate was called wdm-codec"): leave as-is.
  - If the reference is operational ("to test, run cargo test -p wdm-codec"): update the operational fragment to `md-codec` while preserving the historical context as a parenthetical (e.g., `cargo test -p md-codec` (was `wdm-codec` pre-v0.3.0)).

ADD a new memory file: `/home/bcg/.claude/projects/-scratch-code-shibboleth/memory/project_renamed_wdm_to_md_v0_3.md` capturing:
- The rename event (date, branch, decision log path).
- The SemVer cut (v0.2.3 → v0.3.0).
- The family-stable SHA reset (`"wdm-codec 0.2"` → `"md-codec 0.3"`).
- Cross-link to `design/RENAME_v0_3_wdm_to_md.md` and `design/IMPLEMENTATION_PLAN_v0_3_rename.md`.
- Reminder for future sessions: vX.0.0 is the rename boundary; pre-v0.3.0 vectors are NOT family-stable across the rename.

Update `MEMORY.md` index to reference the new memory file.

**Gates**: No code gates (memory files are outside the Cargo workspace). Verify the new memory file is syntactically valid markdown.

**Exit condition**: New `project_renamed_wdm_to_md_v0_3.md` exists. `MEMORY.md` index updated. Historical-but-operational fragments in existing memory files updated (preserve historical truth, fix operational guidance).

**Estimated effort**: 30 minutes. No git commits (memory files are outside the repo).

---

## Phase 10 — Past-release deprecation

**Entry condition**: Phase 9 committed. v0.3.0 release work complete in-repo. (Note: this phase touches GitHub Releases, NOT the repo. It can run before or after the v0.3.0 release publication — but per workflow, do it before so consumers see deprecation banners by the time v0.3.0 announce goes out.)

For each tag in `[wdm-codec-v0.2.0, wdm-codec-v0.2.1, wdm-codec-v0.2.2, wdm-codec-v0.2.3]`:

1. Capture current notes:
   ```
   gh release view <tag> --json body --jq .body > /tmp/<tag>-body.md
   ```

2. Prepend the deprecation banner from decision log Pre-flight Gate 3 to the captured body. Banner verbatim:
   ```
   > ⚠️ **DEPRECATED — superseded by `md-codec-v0.3.0`.** This release uses HRP `wdm` and crate name `wdm-codec`, both of which were renamed in v0.3.0 to `md` and `md-codec` respectively. The format is now called "Mnemonic Descriptor" (was "Wallet Descriptor Mnemonic"). **Wire format incompatibility:** strings produced by this release start with `wdm1...` and will not validate against v0.3.0 decoders, which expect `md1...` strings. Pin to this tag only for historical compatibility; new work should target [`md-codec-v0.3.0`](https://github.com/bg002h/descriptor-mnemonic/releases/tag/md-codec-v0.3.0) or later. Repository URL is unchanged.
   ```

3. Apply the new notes:
   ```
   gh release edit <tag> --notes-file /tmp/<tag>-body-with-banner.md
   ```

4. Verify:
   ```
   gh release view <tag>
   ```
   Confirm banner appears at top of notes body.

**HARD CONSTRAINTS** (from workflow):
- Do NOT delete any release.
- Do NOT touch tags.
- Do NOT unlist.
- Only edit the notes body (prepend banner to existing body — do not replace).

**Gates**: For each of the 4 tags, `gh release view <tag>` shows the banner at the top with the original notes preserved beneath.

**Exit condition**: All 4 historical GitHub Releases carry the deprecation banner. Tags and assets unchanged.

**Estimated effort**: 30 minutes. No git commits (touches GitHub UI only).

---

## Final review (after Phase 10, before merge to main)

Dispatch a `feature-dev:code-reviewer` agent over the cumulative diff:
```
git diff main...HEAD  # on the rename/v0.3-wdm-to-md worktree branch
```

Reviewer checklist:
- Any `wdm` token still present except in HISTORICAL contexts (CHANGELOG.md historical lines, deprecation banners, "renamed from" admonitions)? Flag each.
- All 4 historical GitHub Releases have deprecation banners (verify with `gh release view`).
- Doc-comment cross-references resolve (`[`MdBackup`]`, `[`MdKey`]` link targets exist).
- MIGRATION.md `v0.2.x → v0.3.0` section is complete (all 6 scope points).
- CHANGELOG.md v0.3.0 entry follows the format of prior entries.
- BIP file renders cleanly; HRP-expansion bytes are length 5; SHA reference is the new v0.2.json SHA.
- Test vector files contain ZERO `wdm1` strings (only `md1`).
- `Cargo.lock` diff is limited to the local crate package rename (no unrelated dependency churn).

If reviewer surfaces issues: fix in place on the worktree branch, re-run gates, re-review. Then merge to `main`.

---

## Release sequence (after merge to main)

1. On `main`, verify `crates/md-codec/Cargo.toml` is at `version = "0.3.0"`. If not, bump now.
2. `RUSTUP_TOOLCHAIN=stable cargo update --workspace` to refresh Cargo.lock if needed.
3. Final commit: `release(v0.3.0): rename wdm-codec → md-codec; HRP wdm → md`.
4. Annotated tag: `git tag -a md-codec-v0.3.0 -m "md-codec v0.3.0 — rename from wdm-codec"`.
5. Push: `git push origin main && git push origin md-codec-v0.3.0`.
6. Watch CI: 3-OS green (Linux + macOS + Windows per existing workflow matrix).
7. Draft GitHub Release `md-codec-v0.3.0` with `--prerelease --latest` flags. Body should reference CHANGELOG v0.3.0 entry, MIGRATION v0.2.x→v0.3.0 section, and link the deprecation banners on the v0.2.x releases.
8. Post-release: dispatch Phase 11 (SLIP-0173 PR) per workflow.

---

## Risk register

| Risk | Mitigation |
|---|---|
| Worktree branch cuts from `origin/main` (per session memory) — design docs not yet pushed would be invisible to the worktree. | STATUS: handled if all design commits (this plan, decision log, discovery report, workflow doc) are pushed BEFORE worktree creation. Verify with `git status && git log origin/main..HEAD` showing nothing. |
| `gen_vectors --verify` is invoked in tests; if Phase 6 ordering is off, test gates fail mid-Phase 4. | MITIGATION: Phase 4 PRE-STEP marks `V0_*_SHA256` lock tests as `#[ignore]`. Phase 6 step 5 re-enables them. |
| Subagent might MISS a HISTORICAL item and try to "helpfully" fix it (e.g., rewrite a v0.2.0 CHANGELOG entry to say `md-codec`). | MITIGATION: each phase's implementer prompt MUST quote the discovery report's HISTORICAL list verbatim and forbid edits to those line ranges. Reviewer subagent re-checks. |
| `Cargo.lock` churn surfaces unrelated dependency updates if `cargo update` runs without `--workspace` scoping. | MITIGATION: use `cargo update --workspace` only. Inspect diff before commit; if unrelated deps appear, revert and use `cargo update --package <local>` per crate. |
| BIP HRP-expansion update has an arithmetic error (`md` is 2 chars, expansion length is `2*2+1=5`, not 4 or 6). | MITIGATION: derivation is shown in decision log §"HRP-expansion derivation"; reviewer must spot-check the bytes against `ord('m')=109, ord('d')=100`. |
| `GENERATOR_FAMILY` constant change in Phase 5 silently invalidates vectors before Phase 6 regen — tests would have already gone red. | MITIGATION: SHA-lock `#[ignore]`s also cover the family-string assertion if there is one. Implementer must grep for assertions on `"wdm-codec "` literal in test files and ignore those too. |
| `git mv` history loss if implementer uses `mv` + `git add -A` instead. | MITIGATION: workflow explicitly says `git mv`. Reviewer verifies with `git log --follow` on a moved file. |
| `cargo install --path crates/md-codec` after the rename installs a binary called `md`, which shadows GNU `md` if installed (md is in some BSDs but not standard GNU). | LOW RISK: project already accepted `md` in pre-flight Gate 1 (no collision in cryptocurrency HRP namespace; binary-name collision is a separate concern but accepted by user). No action. |

---

## Success criteria

- [ ] All 545 actionable touch points addressed (571 total − 26 HISTORICAL).
- [ ] 26 historical touch points untouched and verified untouched (CHANGELOG historical lines + 8 auto-memory files' historical fragments + any HISTORICAL BIP examples).
- [ ] Full test suite green on `RUSTUP_TOOLCHAIN=stable cargo test -p md-codec` (target test count: ~565, ±a few from test-fn renames not changing count).
- [ ] 3-OS CI green (Linux/macOS/Windows).
- [ ] `gen_vectors --verify` PASS for both `crates/md-codec/tests/vectors/v0.1.json` and `v0.2.json`.
- [ ] BIP renders cleanly; HRP-expansion bytes show length-5 form.
- [ ] All 4 historical GitHub Releases (`wdm-codec-v0.2.0` through `v0.2.3`) have deprecation banners.
- [ ] New v0.3.0 GitHub Release published as `--prerelease --latest`.
- [ ] Decision log `design/RENAME_v0_3_wdm_to_md.md` "Open items at decision-log freeze" section filled in with actual SHAs.
- [ ] FOLLOWUPS.md has SLIP-0173 PR follow-up logged for Phase 11.

---

**End of plan.**
