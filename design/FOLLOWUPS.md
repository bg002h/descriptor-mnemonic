# Follow-up tracker

Single source of truth for items that surfaced during a review or implementation pass but were not fixed in the same commit. Replaces the previous practice of scattering follow-ups across decision docs, commit messages, inline TODOs, and conversation history.

## How to use this file

**Format for each entry:**

```markdown
### `<short-id>` — <one-line title>

- **Surfaced:** Phase X.Y review of commit <SHA>, or "inline TODO at <file>:<line>"
- **Where:** `<file>:<line>` or "design — Cargo.toml `[patch]` block"
- **What:** 1–3 sentences describing the gap or improvement opportunity
- **Why deferred:** the reason it didn't ship in the original commit
- **Status:** `open` | `resolved <COMMIT>` | `wont-fix — <one-line reason>`
- **Tier:** `v0.1-blocker` | `v0.1-nice-to-have` | `v0.2` | `v1+` | `external`
```

The `<short-id>` is a stable handle (e.g., `5d-from-impl`, `5e-checksum-correction-fallback`, `p10-miniscript-dep-audit`). Reference this id from commit messages when you close the item: `closes FOLLOWUPS.md 5d-from-impl`.

## Conventions for adding items

**During a review subagent run:** the reviewer should append to this file (with a small entry per minor item) and reference it in their report. Reviewers in parallel batches must not write to this file simultaneously — the controller appends afterwards from the consolidated reports.

**During an implementer subagent run:** if the implementer notices a side concern they explicitly chose not to fix in their commit, they append an entry here in the same commit. This keeps the deferral visible.

**During controller (main-thread) work:** when wrapping a task, the controller verifies all minor items from that task's reviews are either resolved or recorded here.

**Persisting agent reports to disk (durable audit trail):** in addition to FOLLOWUPS.md, every implementer or reviewer subagent that produces a commit MUST also save its full final report (the verbatim text the agent returns to the controller) to `design/agent-reports/<filename>.md` per the file-naming convention in `design/agent-reports/README.md`. This protects against the controller losing minor items between conversation sessions: the raw report is durable on disk, and the post-batch FOLLOWUPS.md aggregation can re-read agent reports if the controller's working memory missed something. For parallel-batch dispatches, each agent saves to a distinct filename (no merge conflicts since filenames embed the bucket id).

**When closing an item:** change `Status:` to `resolved <COMMIT>` (where `<COMMIT>` is the short SHA of the fix). Do not delete the entry — closure history is informative for future reviewers. After 6+ months of resolved entries, a separate cleanup pass can archive them to `FOLLOWUPS_ARCHIVE.md`.

## Tiers (definitions)

- **`v0.1-blocker`**: must fix before tagging `wdm-codec-v0.1.0` (Phase 10). Failing to fix = ship blocked.
- **`v0.1-nice-to-have`**: should fix before v0.1 if time permits, but won't block release. Document the deferral in v0.1's CHANGELOG/README if shipped.
- **`v0.2`**: explicitly deferred to v0.2 by a phase decision or spec note. Tracked here for visibility; no v0.1 fix expected.
- **`v1+`**: deferred indefinitely. May be revisited only as part of a major version revision.
- **`external`**: depends on work outside this repo (e.g., upstream PR merging).

---

## Open items

### `p2-taproot-tr-taptree` — taproot `Tr` / `TapTree` operator support

- **Surfaced:** Phase 2 (D-2, D-4, plan task 2.11 marked deferred)
- **Where:** `crates/wdm-codec/src/bytecode/{encode,decode}.rs` — Tr/TapTree match arms currently reject with `Error::PolicyScopeViolation`
- **What:** v0.1 rejects `Descriptor::Tr` at the top level; v0.2 should support taproot single-leaf (per BIP §"Taproot tree (forward-defined)") with the per-leaf miniscript subset constraints required by deployed signers (Coldcard subset: `pk`/`pk_h`/`multi_a`/`or_d`/`and_v`/`older`).
- **Why deferred:** explicitly out of scope for v0.1.
- **Status:** open
- **Tier:** v0.2

### `p2-inline-key-tags` — Reserved tags 0x24–0x31 (descriptor-codec inline-key forms)

- **Surfaced:** Phase 2 D-2 (`design/PHASE_2_DECISIONS.md`)
- **Where:** `crates/wdm-codec/src/bytecode/{tag,encode,decode}.rs`
- **What:** Tags `0x24..=0x31` are reserved by descriptor-codec for inline-key forms (raw xpubs, key origins, wildcards). v0.1 rejects them per BIP-388 wallet-policy framing. v1+ may expose them for foreign-xpub support if/when WDM extends beyond pure BIP-388.
- **Why deferred:** v0.1 spec scope.
- **Status:** open
- **Tier:** v1+

### `p1-bch-4-error-correction` — proper Berlekamp-Massey/Forney decoder for full 4-error correction

- **Surfaced:** inline `// TODO(v0.2)` at `crates/wdm-codec/src/encoding.rs:379` (since Phase 1)
- **Where:** `crates/wdm-codec/src/encoding.rs` `bch_correct_*` functions (~line 379)
- **What:** v0.1 ships brute-force 1-error correction. BIP-93 supports up to 4-error correction; we'd need a proper syndrome-based decoder (Berlekamp-Massey + Forney) to reach the full ECC capacity.
- **Why deferred:** documented v0.2 scope per the implementation plan's risk register.
- **Status:** open
- **Tier:** v0.2

### `external-pr-1-hash-terminals` — apoelstra/rust-miniscript PR #1

- **Surfaced:** Phase 5-B; submitted 2026-04-27
- **Where:** https://github.com/apoelstra/rust-miniscript/pull/1
- **What:** PR fixing `WalletPolicyTranslator` to support hash terminals (sha256/hash256/ripemd160/hash160). Until merged, our workspace `[patch]` redirects to a local clone of the patched fork.
- **Why deferred:** waiting for upstream maintainer review.
- **Status:** open
- **Tier:** external

### `p2-fingerprints-block` — v0.2 fingerprints block support

- **Surfaced:** Phase 5-B; documented at `crates/wdm-codec/src/policy.rs:316-317` and `:668`
- **Where:** `crates/wdm-codec/src/bytecode/{header,decode}.rs`, `crates/wdm-codec/src/policy.rs::from_bytecode`
- **What:** The bytecode header has a fingerprints flag (bit 2) and a reserved tag `Tag::Fingerprints = 0x35`, but v0.1 rejects any input with the flag set via `Error::PolicyScopeViolation`. v0.2 should implement the full fingerprints-block format (per BIP §"Fingerprints block"): a count byte followed by `count` 4-byte master-key fingerprints in placeholder index order, allowing recovery tools to verify which seed corresponds to which `@i` placeholder before deriving.
- **Why deferred:** v0.1 spec scope. The recovery flow can match seeds to placeholders by trial derivation; the fingerprints block is an ergonomics + privacy improvement.
- **Status:** open
- **Tier:** v0.2

### `8-negative-fixture-dynamic-generation` — generate negative vectors dynamically by exercising actual error paths

- **Surfaced:** v0.2 carry-forward from `8-negative-fixture-placeholder-strings` closure
- **Where:** `crates/wdm-codec/src/vectors.rs` `NEGATIVE_FIXTURES` array (replace static const with a runtime `build_negative_vectors()`)
- **What:** v0.1 ships representative-placeholder `input_strings` with honest provenance docs. v0.2 should (if cross-implementation interop demands it) replace the placeholders with byte-for-byte exact strings produced by encoding a valid policy then mutating it precisely until the named `expected_error_variant` is returned. This is per-variant fixture work (~30 variants).
- **Why deferred:** v0.1's schema lock-in purpose is met by representative fixtures + honest docs. Real conformance implementations can generate their own byte-for-byte fixtures locally using the same API surfaces.
- **Status:** open
- **Tier:** v0.2

### `7-serialize-derives` — manual JSON construction vs `#[derive(Serialize)]` on library types

- **Surfaced:** Phase 7 implementation
- **Where:** `crates/wdm-codec/src/bin/wdm.rs` (all JSON output paths)
- **What:** JSON output is hand-built via `serde_json::json!{}` rather than `#[derive(Serialize)]` on `WdmBackup`, `DecodeResult`, etc. This was option (b) per the Phase 7 spec, because library types contain a non-Serialize miniscript `WalletPolicy` inner field. If future versions add serde derives to those types (e.g., behind a `serde` feature flag), the JSON handlers in `bin/wdm.rs` should be updated to use the derived impls.
- **Why deferred:** design decision to avoid forcing serde derives on library types in v0.1.
- **Status:** open
- **Tier:** v0.2

### `p10-bip-header-status-string` — align BIP draft header with the ref-impl-aware status

- **Surfaced:** Phase 10 Task 10.7 closure
- **Where:** `bip/bip-wallet-descriptor-mnemonic.mediawiki:8`
- **What:** The BIP draft preamble's `Status:` field still reads `Pre-Draft, AI only, not yet human reviewed`. The root README and project memory now use `Pre-Draft, AI + reference implementation, awaiting human review`. The BIP draft is its own artifact and could legitimately stay on the older string (the spec text itself hasn't been ref-impl-validated by a human), but for consistency the next BIP touch should consider aligning.
- **Why deferred:** stylistic; not a contract issue. The BIP draft predates the impl; the spec's status is independent.
- **Status:** open
- **Tier:** v0.1-nice-to-have

### `p4-chunking-mode-stale-test-names` — sweep `force_chunked_*` test names + comments to new terminology

- **Surfaced:** Phase A bucket A reviewer (Opus 4.7) on commit `fbbe6ec`
- **Where:** `crates/wdm-codec/src/chunking.rs:1072,1164,1178` (test fn names + inline comments) and `crates/wdm-codec/src/decode.rs:231` (`force_chunking_opts` helper); `crates/wdm-codec/src/options.rs:34-36` (field rustdoc could cross-reference `ChunkingMode` directly); `crates/wdm-codec/src/policy.rs:461` (doc-link line ~120 chars vs surrounding ~80; reflow optional).
- **What:** Cosmetic test-name + comment + rustdoc sweep to align terminology with the new `ChunkingMode { Auto, ForceChunked }` enum. All sites are functionally correct; only the names/comments are stale.
- **Why deferred:** test-only churn; bundle into a single sweep before v0.2.0 release rather than spreading across phases.
- **Status:** open
- **Tier:** v0.2-nice-to-have

### `p4-with-chunking-mode-builder` — additive `EncodeOptions::with_chunking_mode(ChunkingMode)` builder

- **Surfaced:** Phase A bucket A dispatch (deferred per controller); reaffirmed by reviewer
- **Where:** `crates/wdm-codec/src/options.rs::EncodeOptions`
- **What:** Today the only chunking-mode builder is `with_force_chunking(self, force: bool)`, kept as a `bool → enum` shim for v0.1.1 source-compat. When a third `ChunkingMode` variant is introduced (e.g., Phase D's `MaxChunkBytes(u8)` per BIP §"Chunking" line 438, if it lands), add `with_chunking_mode(ChunkingMode)` so callers can select the new variant explicitly.
- **Why deferred:** purely additive; no existing or imminent caller needs it.
- **Status:** open
- **Tier:** v0.2-nice-to-have

### `wallet-policy-eq-migration-note` — document `WalletPolicy` `PartialEq` semantics around `decoded_shared_path` in MIGRATION.md

- **Surfaced:** Phase A bucket B reviewer (Opus 4.7) on commit `86ca5df`
- **Where:** `MIGRATION.md` (to be created at Phase G); `crates/wdm-codec/src/policy.rs` field rustdoc (already added inline by controller fixup commit)
- **What:** With the new `decoded_shared_path: Option<DerivationPath>` field and derived `PartialEq, Eq`, two logically-equivalent template-only policies — one from `parse()` (`None`) and one from `from_bytecode()` (`Some(...)`) — compare unequal. Field rustdoc now says so. MIGRATION.md needs a corresponding "Phase A breaking changes" bullet pointing at this so v0.1.x consumers upgrading to v0.2 understand the new equality semantics + the recommended `.to_canonical_string()` workaround for construction-path-agnostic equality.
- **Why deferred:** MIGRATION.md is a Phase G deliverable; this entry is a tracker so the doc isn't missed at release prep.
- **Status:** open
- **Tier:** v0.2-nice-to-have

### `phase-b-encode-signature-and-copy-migration-note` — document Phase B breaking changes in MIGRATION.md

- **Surfaced:** Phase B bucket B reviewer (Opus 4.7) on commit `0993dc0`
- **Where:** `MIGRATION.md` (to be created at Phase G)
- **What:** Phase B introduces two breaking changes to `EncodeOptions` / `WalletPolicy::to_bytecode` that require migration guidance:
  - **Signature change**: `WalletPolicy::to_bytecode(&self)` → `WalletPolicy::to_bytecode(&self, opts: &EncodeOptions)`. Migration: callers needing no override should pass `&EncodeOptions::default()`.
  - **`Copy` removed from `EncodeOptions`**: `DerivationPath` (the new `shared_path` field's type) is not `Copy`, so `EncodeOptions` lost its derived `Copy` impl. It still derives `Clone + Default + PartialEq + Eq`. Migration: any callers that assumed `EncodeOptions: Copy` (e.g., taking `EncodeOptions` by value into a closure) need explicit `.clone()` calls.
- **Why deferred:** MIGRATION.md is a Phase G deliverable; this entry is a tracker so neither item is missed.
- **Status:** open
- **Tier:** v0.2-nice-to-have

### `decoded-string-data-memory-microopt` — drop `DecodedString.data`, replace with accessor backed by `data_with_checksum`

- **Surfaced:** Phase B bucket A reviewer (Opus 4.7) on commit `5f13812`
- **Where:** `crates/wdm-codec/src/encoding.rs::DecodedString`
- **What:** With `data_with_checksum: Vec<u8>` added in Phase B (so `corrected_char_at` works for checksum-region positions), `data` and `data_with_checksum` redundantly store the same symbol array (data + a 13/15-char suffix). Memory cost is ~26 bytes for Regular / ~30 for Long per `DecodedString`, plus `Vec` overhead — negligible at v0.1 scale. An obvious micro-opt: drop the `data: Vec<u8>` field; replace with a `pub fn data(&self) -> &[u8]` accessor that returns `&self.data_with_checksum[..self.data_with_checksum.len() - checksum_len]`.
- **Why deferred:** breaking API change (the `data` field is currently `pub`); v0.3 breaking-window candidate. Negligible at v0.1/v0.2 scale; not worth the breakage in v0.2.
- **Status:** open
- **Tier:** v0.3

---

## Resolved items

(Closure log. Items move here from "Open items" when their `Status:` changes to `resolved <COMMIT>`. Useful for spec/audit reasons; not deleted to preserve provenance.)

### `5a-from-inner-visibility` — `WalletPolicy::from_inner` should be `pub(crate)` not `pub`

- **Surfaced:** Phase 5-A code review of `56124c3`
- **Status:** resolved `2ec1d41` — function was removed entirely; no in-crate caller existed.
- **Tier:** v0.1-nice-to-have (closed)

### `5b-hash-byte-overcount` — `count_placeholder_indices` byte-scan over-counts on hash bytes ≡ 0x32

- **Surfaced:** Phase 5-B code review of `f0d9346`
- **Status:** resolved `48809b7` — Option A adopted; `count_placeholder_indices` deleted; `decode_template` now receives 32 dummy keys and `from_descriptor` re-derives `key_info` from actual descriptor structure.
- **Tier:** v0.1-blocker (closed)

### `5b-dummy-table-too-small` — DUMMY_KEYS table 8 entries; corpus C5 needs 11

- **Surfaced:** Phase 5-B code review of `f0d9346`
- **Status:** resolved `48809b7` — table grown to 32 entries (BIP 388 max placeholder count).
- **Tier:** v0.1-blocker (closed)

### `5c-walletid-words-display` — `WdmBackup::wallet_id()` hand-rolled space-join

- **Surfaced:** Phase 5-C code review of `62ae611`
- **Status:** resolved `8e00766` — uses `WalletIdWords::Display::to_string()`; also fixed an adjacent pre-existing `clippy::needless_borrows_for_generic_args` warning in `bip39::Mnemonic::parse` call.
- **Tier:** v0.1-nice-to-have (closed)

### `5d-chunk-index-from-header` — `EncodedChunk.chunk_index`/`total_chunks` should read from header, not loop counter

- **Surfaced:** Phase 5-D code review of `308b2e1`
- **Status:** resolved `9529ee8` — fields now destructured from `chunk.header`; loop is plain `for chunk` (no enumerate).
- **Tier:** v0.1-nice-to-have (closed)

### `5d-loop-invariant-bch-code` — BCH code lookup hoisted out of Stage 5 loop

- **Surfaced:** Phase 5-D code review of `308b2e1`
- **Status:** resolved `9529ee8` — match on `plan` to determine `bch_code` now happens once before the loop.
- **Tier:** v0.1-nice-to-have (closed)

### `5e-tests-13-14-merge` — `decode_report_outcome_clean` and `verifications_all_true_on_happy_path` were one combined test

- **Surfaced:** Phase 5-E code review of `7b7400b`
- **Status:** resolved `111f176` — split into two `#[test]` functions sharing a `happy_path_decode()` helper.
- **Tier:** v0.1-nice-to-have (closed)

### `5e-corrupted-hash-test-name` — `decode_rejects_corrupted_cross_chunk_hash` didn't exercise public API

- **Surfaced:** Phase 5-E code review of `7b7400b`
- **Status:** resolved `111f176` — test deleted; equivalent coverage already exists in `chunking.rs::tests::reassemble_cross_chunk_hash_mismatch_with_corrupted_hash_byte` (Phase 4-E followup) and `reassemble_cross_chunk_hash_mismatch`.
- **Tier:** v0.1-nice-to-have (closed)

### `5f-rustdoc-broken-links` — 5 rustdoc errors blocking the new `cargo doc` CI job

- **Surfaced:** Phase 5-F implementer's DONE_WITH_CONCERNS report on `571104b`
- **Status:** resolved across `111f176` (decode.rs:28 fix) + `4c73338` (4 fixes in key.rs/encode.rs/wallet_id.rs/encoding.rs); `RUSTDOCFLAGS="-D warnings" cargo doc` now finishes cleanly.
- **Tier:** v0.1-blocker (closed; doc CI green)

### `5b-from-exact-bytes-removed` — `Chunk::from_exact_bytes` and `Error::TrailingChunkBytes` were unreachable dead code

- **Surfaced:** Phase 4-E review of `f0d9346` (the Opus reviewer noticed the helper was structurally identical to `from_bytes` because chunk fragments have no length-bound)
- **Status:** resolved `e7a7a16` (Phase 4-E followup); rationale captured in `design/PHASE_7_DECISIONS.md` CF-1 (Phase 7 codex32 layer is the chunk byte-boundary source of truth).
- **Tier:** v0.1-nice-to-have (closed)

### `5a-test-7-tautology` — `shared_path_returns_none_for_template_only_policy` used `matches!(.., None | Some(_))`

- **Surfaced:** Phase 5-A code review of `56124c3`
- **Status:** resolved `22beba8` (Phase 5-B); test now uses `assert!(p.shared_path().is_none())` since the 5-B implementation correctly returns `None` for template-only policies.
- **Tier:** v0.1-nice-to-have (closed)

### `5a-key-count-cast` — `(m + 1) as usize` cast in `key_count`

- **Surfaced:** Phase 5-A code review of `56124c3`
- **Status:** resolved `2ec1d41` (Phase 5-A followup); `key_count` now uses `usize` throughout its scan, eliminating the cast entirely.
- **Tier:** v0.1-nice-to-have (closed)

### `5a-key-count-numeric-type` — `key_count` should use `usize` end-to-end (was `u32`-then-cast)

- **Surfaced:** Phase 5-A code review of `56124c3`
- **Status:** resolved `2ec1d41` (Phase 5-A followup); `Option<u32>` → `Option<usize>`, `parse::<u32>()` → `parse::<usize>()`.
- **Tier:** v0.1-nice-to-have (closed)

### `5a-key-count-rustdoc` — rustdoc clarification that `inner.to_string()` writes only the template

- **Surfaced:** Phase 5-A code review of `56124c3`
- **Status:** resolved `2ec1d41` (Phase 5-A followup); rustdoc explicitly notes BIP 388 template form (`@N`-only) and that origin xpubs appear only in full-descriptor display.
- **Tier:** v0.1-nice-to-have (closed)

### `5d-from-impl` — add `From<ChunkCode> for BchCode` impl

- **Surfaced:** Phase 5-D code review of `308b2e1`
- **Status:** resolved `430dbfc` (post-v0.1 followup batch 1, bucket A); `From<ChunkCode> for BchCode` impl added in `chunking.rs`; private `chunk_code_to_bch_code` helper in `encode.rs` removed and call sites switched to `BchCode::from(plan.code)`.
- **Tier:** v0.1-nice-to-have (closed)

### `5d-decision-cross-reference` — note force_long_code post-processor in chunking_decision rustdoc

- **Surfaced:** Phase 5-D code review of `308b2e1`
- **Status:** resolved `430dbfc` (post-v0.1 followup batch 1, bucket A); `chunking_decision` rustdoc now cross-references `EncodeOptions.force_long_code` and the `encode.rs` post-processor.
- **Tier:** v0.1-nice-to-have (closed)

### `6c-encode-options-builder` — `EncodeOptions` `#[non_exhaustive]` blocks struct-update syntax from external tests

- **Surfaced:** Phase 6 bucket C; Task 6.18 (`natural_long_code_boundary`)
- **Status:** resolved `a74e21b` (post-v0.1 followup batch 1, bucket B); fluent builder added — `EncodeOptions::default().with_force_chunking(true).with_force_long_code(true).with_seed(seed)` now works from external integration tests despite `#[non_exhaustive]`.
- **Tier:** v0.1-nice-to-have (closed)

### `5e-skip-silent` — tests with size-conditional assertions skip silently

- **Surfaced:** Phase 5-E code review of `7b7400b`
- **Status:** resolved `fa83737` (post-v0.1 followup batch 1, bucket C); tests at `decode.rs:270` and `decode.rs:530` now use `with_force_chunking(true)` so the chunked path is exercised deterministically regardless of bytecode length.
- **Tier:** v0.1-nice-to-have (closed)

### `5e-dead-branch` — `decode_rejects_chunks_with_duplicate_indices` has unreachable fallback

- **Surfaced:** Phase 5-E code review of `7b7400b`
- **Status:** resolved `fa83737` (post-v0.1 followup batch 1, bucket C); the unreachable `if backup.chunks.len() < 2` branch removed; test now goes straight to the multi-chunk assertion path on the 9-key multisig.
- **Tier:** v0.1-nice-to-have (closed)

### `5e-correction-position-doc` — rustdoc cross-reference for `Correction.char_position` coordinate system

- **Surfaced:** Phase 5-E code review of `7b7400b`
- **Status:** resolved `fa83737` (post-v0.1 followup batch 1, bucket C); `decode` rustdoc now cross-references the `Correction.char_position` coordinate system documented at `chunking.rs::Correction`.
- **Tier:** v0.1-nice-to-have (closed)

### `5e-five-bit-truncated-mapping` — `five_bit_to_bytes` failure error-variant choice

- **Surfaced:** Phase 5-E code review of `7b7400b`
- **Status:** resolved `fa83737` (post-v0.1 followup batch 1, bucket C); branch now `unreachable!()` with a justification comment that successful BCH validation guarantees a multiple-of-8 data part.
- **Tier:** v0.1-nice-to-have (closed)

### `6e-missing-children-unreachable` — `BytecodeErrorKind::MissingChildren` defined but never emitted

- **Surfaced:** Phase 6 bucket E; Task 6.21 — `rejects_invalid_bytecode_missing_children` was `#[ignore]`d
- **Status:** resolved `1ccc1d4` (post-v0.1 followup batch 1, bucket D); explicit arity check added in variable-arity decoder branches now emits `MissingChildren { expected, got }`; conformance test un-`#[ignore]`d (test count: 1 ignored → 0 ignored).
- **Tier:** v0.1-nice-to-have (closed)

### `7-cli-integration-tests` — CLI integration tests via `assert_cmd`

- **Surfaced:** Phase 7 implementation (Task 7 prompt, §Tests)
- **Status:** resolved `1ccc1d4` (post-v0.1 followup batch 1, bucket E); `tests/cli.rs` added with 12 `assert_cmd` tests (8 happy-path + 4 error-path) covering `encode`, `decode`, `verify`, `inspect`, `bytecode`; `assert_cmd = "2"` and `predicates = "3"` added as dev-deps. Closed early (was tier'd v0.2; accelerated to post-v0.1 nice-to-have).
- **Tier:** v0.2 (closed; accelerated)

### `p10-miniscript-dep-audit` — release-readiness audit of the miniscript git pin

- **Surfaced:** Phase 5 D-1 (`design/PHASE_5_DECISIONS.md`); Phase 7 carry-forward CF-1 documents adjacent context
- **Status:** resolved at tag `wdm-codec-v0.1.0` (`fef8dcb`) via option (b): git-dep pin documented in `crates/wdm-codec/Cargo.toml`, the workspace `[patch]` rationale captured in the root `Cargo.toml`, the BIP draft's reference-implementation section names the apoelstra fork dep, and the root README status notes the dep. Tag annotation message also contains the dep rationale. Forward work (flipping the `[patch]` block off when upstream PR merges) is tracked separately as `external-pr-1-hash-terminals`.
- **Tier:** v0.1-blocker (closed)

### `p4-chunking-rs-split` — split `chunking.rs` into a `chunking/` directory

- **Surfaced:** Phase 4-A and 4-D code reviews; Phase 4-E code review
- **Status:** wont-fix — every reviewer through Phase 7 confirmed the section-banner organization is navigable; no Phase 6/7/8/9/10 consumer found it unwieldy. Splitting now is pure churn (touches every test in the file, breaks any external pin to module path) for no reader-experience win. Revisit only if a future caller is genuinely impeded.
- **Tier:** v0.1-nice-to-have (closed)

### `6a-coldcard-corpus-shape` — Coldcard corpus entry uses representative shape (same as C2)

- **Surfaced:** Phase 6 bucket A; Task 6.11
- **Status:** wont-fix — v0.1 corpus is operator-shape based by design; the Coldcard entry is an existence-proof that real-world export shapes round-trip, not a coverage gap. Revisit if a future signer's BIP 388 export is structurally distinct from existing corpus shapes.
- **Tier:** v0.1-nice-to-have (closed)

### `6d-rand-gen-keyword` — `rng.r#gen()` raw-identifier workaround for Rust 2024 reserved keyword

- **Surfaced:** Phase 6 bucket D; Task 6.20 (`many_substitutions_always_rejected`)
- **Status:** resolved `ff7d1ea` — `rand` dev-dep bumped 0.8 → 0.9; all `r#gen()` and `gen_range` callsites switched to `random()` and `random_range()`.
- **Tier:** v0.1-nice-to-have (closed)

### `8-negative-fixture-placeholder-strings` — negative vector `input_strings` are placeholder-grade, not confirmed-correct WDM strings

- **Surfaced:** Phase 8 implementation (Task 8.3); implementer's own follow-up
- **Status:** resolved `c46f2c0` via option (b) — `vectors.rs` `NEGATIVE_FIXTURES` rustdoc rewritten to honestly document fixture provenance: `expected_error_variant` is the authoritative contract; `input_strings` are representative placeholders demonstrating the error class; n12, n29, n30 explicitly flagged as targeting lower-level APIs (`reassemble_chunks`, `policy.parse`, `chunking_decision`). The original misleading "all placeholder inputs are confirmed to trigger the correct variant" claim was deleted. Dynamic generation (option a) deferred as `8-negative-fixture-dynamic-generation` (open, v0.2).
- **Tier:** v0.1-nice-to-have (closed)

### `p10-cross-platform-ci-sanity` — confirm GitHub Actions green on Windows + macOS

- **Surfaced:** Phase 10 Task 10.2; deferred at controller closure
- **Status:** resolved `651c402` (post-push verification at run [25022150945](https://github.com/bg002h/descriptor-mnemonic/actions/runs/25022150945)) — full pipeline now green across `cargo test (ubuntu/windows/macos)` + `cargo clippy` + `cargo fmt` + `cargo doc`. Required four code/CI fixes that previous local-only validation never caught: `f4c8d3c` (workflow `git clone --depth` couldn't reach the SHA on a non-default branch), `06557a3` (matrix-ize the test job), `b12b814` (clippy 1.85.0 `precedence` lint in `polymod_step`), and `651c402` + `c46f2c0` (clippy 1.85.0 `format_collect` lint in `vectors.rs` and `bin/wdm.rs`). Lesson: pin a CI-equivalent toolchain locally if you need pre-push lint parity.
- **Tier:** v0.1-nice-to-have (closed)

### `p3-decode-declaration-from-bytes` — `decode_declaration_from_bytes(&[u8]) -> Result<(DerivationPath, usize), Error>` ergonomic alt

- **Surfaced:** Phase 3.5' code review of `bdeb639`
- **Status:** resolved (post-v0.1.1 v0.2 batch 1) — new `pub fn decode_declaration_from_bytes(&[u8]) -> Result<(DerivationPath, usize), Error>` added to `crates/wdm-codec/src/bytecode/path.rs`. Constructs an internal Cursor, calls the existing `pub(crate)` cursor-based decoder, returns `(path, cur.offset())`. Four new tests cover dictionary path round-trip, explicit path round-trip, trailing-bytes-not-consumed semantics, and error propagation. Purely additive; no existing API changed.
- **Tier:** v0.2 (closed)

### `p2-decoded-template-hybrid` — hybrid `DecodedTemplate` decoder shape

- **Surfaced:** Phase 2 D-5 (`design/PHASE_2_DECISIONS.md`)
- **Status:** wont-fix — Phase 2 D-5 chose option (A) (`decode_template` returns `Descriptor<DescriptorPublicKey>` directly via key substitution); through v0.1.1 no caller has surfaced needing lazy key substitution. The 2-arg `decode_template(bytes, &keys)` API is the natural inverse of `encode_template(d, &map)`. Revisit only if a real recovery-flow consumer needs to inspect the template before binding keys.
- **Tier:** v0.2 (closed)

### `4a-from-bytes-shape` — reconsider `Chunk::from_bytes` shape (slice+usize vs `&mut Cursor`)

- **Surfaced:** Phase 4-A code review of `aefdf3f` (deferred to "after 4-E"); 4-E used the slice+usize shape unchanged
- **Status:** wont-fix — through v0.1.1 no caller has surfaced needing the shape switched. Phase 7 CLI consumed `Chunk::from_bytes` via the slice+usize shape without friction; no Phase 5–10 consumer needed the Cursor shape. Both shapes do equivalent work; consolidating now is style-only churn. Revisit only if a non-test consumer surfaces a concrete need.
- **Tier:** v0.2 (closed)

### `p4-chunking-mode-enum` — `force_chunked: bool` → `ChunkingMode { Auto, ForceChunked }`

- **Surfaced:** Phase 4-D code review of `1fe9505`
- **Status:** resolved `fbbe6ec` (v0.2 Phase A bucket A) — pub enum `ChunkingMode { Auto, ForceChunked }` added to `chunking.rs`; `pub fn chunking_decision(usize, ChunkingMode)`; `EncodeOptions.force_chunking: bool` → `chunking_mode: ChunkingMode`. `with_force_chunking(self, bool)` builder preserved as a `bool → enum` shim for v0.1.1 source-compat. Wire format unchanged; vectors verify byte-identical. 2 new tests cover the bool↔enum shim and `Default = Auto`. Reviewer `APPROVE_WITH_FOLLOWUPS`; the `matches!` → exhaustive `match` nit applied inline by controller; 3 minor follow-ups filed (`p4-chunking-mode-stale-test-names`, `p4-with-chunking-mode-builder`).
- **Tier:** v0.2 (closed; breaking — see commit `fbbe6ec` body for full migration note)

### `6a-bytecode-roundtrip-path-mismatch` — encode→decode→encode is not byte-stable for template-only policies

- **Surfaced:** Phase 6 bucket A (corpus.rs idempotency test); Task 6.12 had to be reframed
- **Status:** resolved `86ca5df` (v0.2 Phase A bucket B) — `WalletPolicy` newtype gains `decoded_shared_path: Option<DerivationPath>` field; `from_bytecode` populates it from `decode_declaration`'s return value; `to_bytecode` consults it under the Phase A precedence rule (`decoded_shared_path > shared_path() > BIP 84 fallback`). Public signatures of `from_bytecode` / `to_bytecode` unchanged. `tests/corpus.rs` idempotency test tightened to assert FIRST-pass raw-byte equality (was second-pass-onward only). New inline test in `policy.rs` proves the round-trip for `m/48'/0'/0'/2'` (distinguishes from both BIP 84 fallback and dummy-key origin). Wire format unchanged; vectors verify byte-identical. Reviewer `APPROVE_WITH_FOLLOWUPS`; the field rustdoc note about `PartialEq` semantics applied inline by controller; MIGRATION.md follow-up filed (`wallet-policy-eq-migration-note`).
- **Tier:** v0.2 (closed; behavioral — see commit `86ca5df` body for full migration note)

### `5e-checksum-correction-fallback` — `Correction.corrected = 'q'` for checksum-region corrections

- **Surfaced:** Phase 5-E code review of `7b7400b`; `// TODO(post-v0.1)` added inline at `decode.rs:119` in `111f176`
- **Status:** resolved `5f13812` (v0.2 Phase B bucket A) — `DecodedString` extended with `pub fn corrected_char_at(char_position: usize) -> char` backed by a new `pub data_with_checksum: Vec<u8>` field (`#[non_exhaustive]` so additive). `decode.rs` Correction translator now uses `corrected_char_at(pos)` instead of the `'q'` placeholder; the `// TODO(post-v0.1)` comment is removed. Two new tests cover both checksum-region and data-region correction reporting. Wire format unchanged; vectors verify byte-identical. Reviewer `APPROVE_WITH_FOLLOWUPS`; rustdoc disambiguation on `corrected_char_at` Panics section applied inline by controller; v0.3 memory micro-opt filed (`decoded-string-data-memory-microopt`).
- **Tier:** v0.2 (closed)

### `7-encode-path-override` — `--path` override does not yet affect bytecode encoder

- **Surfaced:** Phase 7 implementation
- **Status:** resolved `0993dc0` (v0.2 Phase B bucket B) — `EncodeOptions::shared_path: Option<DerivationPath>` field added (additive on `#[non_exhaustive]`) along with a `with_shared_path(path)` builder method. `WalletPolicy::to_bytecode(&self)` signature changed to `to_bytecode(&self, opts: &EncodeOptions)` (breaking) so the encoder can consult the override. The 4-tier shared-path precedence is now: `EncodeOptions::shared_path > WalletPolicy.decoded_shared_path > WalletPolicy.shared_path() > BIP 84 mainnet fallback`. CLI `cmd_encode` no longer prints "warning: --path is parsed but not applied" — it actually applies the override. 22 `to_bytecode` call sites updated (1 pipeline, 1 wrapper, 1 wallet-id helper, 1 vector builder, 1 CLI handler, 16 tests). 5 new tests including a CLI integration test. Side-effect: `EncodeOptions` lost its derived `Copy` impl because `DerivationPath` isn't `Copy`. Wire format unchanged for default-path case; vectors verify. Reviewer `APPROVE_WITH_FOLLOWUPS`; the override-wins test strengthening (assert bytes != baseline) applied inline by controller; MIGRATION.md follow-up filed (`phase-b-encode-signature-and-copy-migration-note`).
- **Tier:** v0.2 (closed; breaking — see commit `0993dc0` body for full migration note)

---

## Convention notes for future agents

If you are an implementer or reviewer subagent dispatched on a task and you identify **minor items** (Important or Minor severity per the standard review rubric) that you are NOT fixing in your own commit, append an entry to this file in the same commit. Use a `<short-id>` like `<phase>-<keyword>` (e.g., `6c-corpus-fixture-helper`, `8a-vectors-schema-comment`).

If you are running in a **parallel batch** with sibling agents, do NOT write to this file directly — return your follow-up items in your final report and the controller will append them. Two parallel agents writing here cause merge conflicts.

If you are **closing** an item, edit its entry from `Status: open` → `Status: resolved <COMMIT>` and move the entry to the "Resolved items" section. Don't delete entries.
