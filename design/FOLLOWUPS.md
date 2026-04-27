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

**When closing an item:** change `Status:` to `resolved <COMMIT>` (where `<COMMIT>` is the short SHA of the fix). Do not delete the entry — closure history is informative for future reviewers. After 6+ months of resolved entries, a separate cleanup pass can archive them to `FOLLOWUPS_ARCHIVE.md`.

## Tiers (definitions)

- **`v0.1-blocker`**: must fix before tagging `wdm-codec-v0.1.0` (Phase 10). Failing to fix = ship blocked.
- **`v0.1-nice-to-have`**: should fix before v0.1 if time permits, but won't block release. Document the deferral in v0.1's CHANGELOG/README if shipped.
- **`v0.2`**: explicitly deferred to v0.2 by a phase decision or spec note. Tracked here for visibility; no v0.1 fix expected.
- **`v1+`**: deferred indefinitely. May be revisited only as part of a major version revision.
- **`external`**: depends on work outside this repo (e.g., upstream PR merging).

---

## Open items

### `5d-from-impl` — add `From<ChunkCode> for BchCode` impl

- **Surfaced:** Phase 5-D code review of `308b2e1`
- **Where:** `crates/wdm-codec/src/encode.rs` (currently has a private `chunk_code_to_bch_code` helper); the `From` impl should live in `crates/wdm-codec/src/chunking.rs` (next to `ChunkCode`'s definition).
- **What:** `ChunkCode` (in `chunking.rs`) and `BchCode` (in `encoding.rs`) are parallel two-variant enums (`Regular`/`Long`). Code currently does the bridge via a private function in `encode.rs`. A `From<ChunkCode> for BchCode` impl on `ChunkCode`'s home module would be more idiomatic and consumable from any future call site (e.g., `decode.rs`'s eventual structural use).
- **Why deferred:** the parallel 5-E task was running on `decode.rs`; we kept buckets file-disjoint to avoid edit conflicts, so chunking.rs was untouchable during the 5-D nit-fix pass.
- **Status:** open
- **Tier:** v0.1-nice-to-have

### `5d-decision-cross-reference` — note force_long_code post-processor in chunking_decision rustdoc

- **Surfaced:** Phase 5-D code review of `308b2e1`
- **Where:** `crates/wdm-codec/src/chunking.rs` `chunking_decision` rustdoc
- **What:** `force_long_code` selection logic now lives in two places: `chunking_decision` (which prefers Regular) and `encode.rs::encode` (which post-processes the plan to Long when `options.force_long_code` is set). Add a one-line comment in `chunking_decision`'s rustdoc directing readers to the post-processor.
- **Why deferred:** parallel 5-E task held `chunking.rs`; same reason as `5d-from-impl`.
- **Status:** open
- **Tier:** v0.1-nice-to-have

### `5e-checksum-correction-fallback` — `Correction.corrected = 'q'` for checksum-region corrections

- **Surfaced:** Phase 5-E code review of `7b7400b`; `// TODO(post-v0.1)` added inline at `decode.rs:119` in `111f176`
- **Where:** `crates/wdm-codec/src/decode.rs:115-127` (the BCH-correction → `Correction` translation). Real fix requires extending `crate::encoding::DecodedString` to expose the corrected `data+checksum` slice.
- **What:** When BCH ECC corrects a substitution within the 13/15-char checksum region, our decoder reports `Correction.corrected = ALPHABET[0]` (= `'q'`) as a placeholder because `decoded.data` (which has the checksum stripped) doesn't contain the corrected char. The displayed `corrected` value is silently wrong for diagnostic purposes; the underlying decode is correct (no data loss). User-facing impact: a recovery tool's "we corrected your transcription error from X to Y" message shows wrong Y for checksum-position corrections.
- **Why deferred:** the right fix touches Phase 1's encoding.rs API (`DecodedString` shape) — that's a wider refactor than 5-E's scope.
- **Status:** open
- **Tier:** v0.2

### `p3-decode-declaration-from-bytes` — `decode_declaration_from_bytes(&[u8]) -> Result<(DerivationPath, usize), Error>` ergonomic alt

- **Surfaced:** Phase 3.5' code review of `bdeb639`
- **Where:** `crates/wdm-codec/src/bytecode/path.rs`
- **What:** `decode_declaration` is currently `pub(crate)` and takes `&mut Cursor`. A `pub` slice-consuming variant returning `(DerivationPath, bytes_consumed)` would be friendlier for any future non-Cursor consumer (e.g., a one-off "parse this byte buffer" call site).
- **Why deferred:** Phase 5's framing wrapper (now Task 5-B) consumed it via Cursor without needing the slice variant. No v0.1 consumer exists.
- **Status:** open
- **Tier:** v0.2

### `p4-chunking-mode-enum` — `force_chunked: bool` → `ChunkingMode { Auto, ForceChunked }`

- **Surfaced:** Phase 4-D code review of `1fe9505`
- **Where:** `crates/wdm-codec/src/chunking.rs` `chunking_decision` signature
- **What:** Replacing the bool with a typed enum makes call sites self-documenting. v0.1 has only test call sites; if real consumers multiply (CLI, top-level encode), the readability win compounds.
- **Why deferred:** premature for v0.1; bool was correct at the time of the call-site count.
- **Status:** open
- **Tier:** v0.2

### `p4-chunking-rs-split` — split `chunking.rs` into a `chunking/` directory

- **Surfaced:** Phase 4-A and 4-D code reviews; Phase 4-E code review
- **Where:** `crates/wdm-codec/src/chunking.rs` (currently ~1500 lines)
- **What:** As of end of Phase 4, `chunking.rs` covers `ChunkHeader` (codec) + `ChunkCode`/`ChunkingPlan` + `chunking_decision` + `Chunk` (codec) + `chunk_bytes` + `reassemble_chunks` + `EncodedChunk` + `Correction`. One responsibility ("the chunking layer") but several distinct subsystems. Splitting into `chunking/header.rs`, `chunking/plan.rs`, `chunking/assembly.rs`, `chunking/types.rs` (for EncodedChunk + Correction) would reduce file size to ~300-500 lines per module. Tests stay inline in each.
- **Why deferred:** every prior reviewer agreed "current section-banner organization is navigable"; no consumer was inconvenienced. Defer until either Phase 6 (corpus tests) or Phase 7 (CLI) finds the file unwieldy.
- **Status:** open
- **Tier:** v0.1-nice-to-have

### `p2-decoded-template-hybrid` — hybrid `DecodedTemplate` decoder shape

- **Surfaced:** Phase 2 D-5 (`design/PHASE_2_DECISIONS.md`)
- **Where:** `crates/wdm-codec/src/bytecode/decode.rs`
- **What:** D-5 chose option (A) for `decode_template` (returns `Descriptor<DescriptorPublicKey>` directly via key substitution) over option (C) (returns a custom `DecodedTemplate` intermediate, with a separate `instantiate` adapter). If real callers ever need lazy key substitution (e.g., recovery flows that want to inspect the template before binding keys), add the intermediate type then.
- **Why deferred:** v0.1 has no such caller. Option (A)'s 2-arg API is the natural inverse of `encode_template(d, &map)`.
- **Status:** open
- **Tier:** v0.2

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

### `p10-miniscript-dep-audit` — release-readiness audit of the miniscript git pin

- **Surfaced:** Phase 5 D-1 (`design/PHASE_5_DECISIONS.md`); Phase 7 carry-forward CF-1 documents adjacent context
- **Where:** `crates/wdm-codec/Cargo.toml` (current `miniscript = { git = "...", rev = "..." }` pin to apoelstra fork) and root `Cargo.toml` (the workspace `[patch]` redirect)
- **What:** Before tagging `wdm-codec-v0.1.0` we MUST either (a) have the dependency pointed at a published miniscript release that includes `WalletPolicy`, or (b) explicitly document the git-dep status in `Cargo.toml`, the BIP draft, and the README. The hash-terminal patch (PR #1 to apoelstra) needs to either land upstream or be embedded in the released crate's history with a clear pin and rationale.
- **Why deferred:** explicitly a Phase 10 task.
- **Status:** open
- **Tier:** v0.1-blocker

### `external-pr-1-hash-terminals` — apoelstra/rust-miniscript PR #1

- **Surfaced:** Phase 5-B; submitted 2026-04-27
- **Where:** https://github.com/apoelstra/rust-miniscript/pull/1
- **What:** PR fixing `WalletPolicyTranslator` to support hash terminals (sha256/hash256/ripemd160/hash160). Until merged, our workspace `[patch]` redirects to a local clone of the patched fork.
- **Why deferred:** waiting for upstream maintainer review.
- **Status:** open
- **Tier:** external

### `6a-bytecode-roundtrip-path-mismatch` — encode→decode→encode is not byte-stable for template-only policies

- **Surfaced:** Phase 6 bucket A (corpus.rs idempotency test); Task 6.12 had to be reframed
- **Where:** `crates/wdm-codec/src/policy.rs::to_bytecode` (the BIP 84 fallback when `shared_path()` is None) and `from_bytecode` (which substitutes dummy keys whose origin path is `m/44'/0'/0'`)
- **What:** First-pass `encode(policy_from_template_str)` uses the BIP 84 mainnet path (`m/84'/0'/0'`) as the shared-path fallback for template-only policies (per Phase 5-B D-10). After `decode_bytecode`, the reconstructed `WalletPolicy` carries dummy keys with `m/44'/0'/0'` origin. Second-pass `encode(reconstructed)` therefore uses `m/44'/0'/0'` and produces a different path declaration. So `encode → decode → encode` is NOT byte-stable; only `(encode → decode → encode) → (decode → encode)` (i.e., second pass onward) is byte-stable. The Task 6.12 idempotency test now asserts second-pass equality plus structural equality only.
- **Why deferred:** v0.1 round-trips correctly at the structural level; byte-stability would require either (a) `WalletPolicy` storing the decoded shared path so re-encode reuses it, or (b) `from_bytecode` stashing the path on the WalletPolicy newtype. Both touch Phase 5-B's API surface.
- **Status:** open
- **Tier:** v0.2

### `6a-coldcard-corpus-shape` — Coldcard corpus entry uses representative shape (same as C2)

- **Surfaced:** Phase 6 bucket A; Task 6.11
- **Where:** `crates/wdm-codec/tests/corpus.rs::corpus_coldcard_bip388_export`
- **What:** The Coldcard-specific corpus entry uses `wsh(sortedmulti(2,@0/**,@1/**,@2/**))` — a representative Coldcard Mk4 export shape per Coldcard docs — but is structurally identical to corpus C2. If a more distinct Coldcard-specific shape becomes important (e.g., a 2-of-3 with explicit BIP 48 origin metadata in the policy string), the format may need a small extension.
- **Why deferred:** v0.1 corpus coverage is by *operator shape*, not by *export source*; C2's shape suffices.
- **Status:** open
- **Tier:** v0.1-nice-to-have

### `6c-encode-options-builder` — `EncodeOptions` `#[non_exhaustive]` blocks struct-update syntax from external tests

- **Surfaced:** Phase 6 bucket C; Task 6.18 (`natural_long_code_boundary`)
- **Where:** `crates/wdm-codec/src/options.rs::EncodeOptions`
- **What:** `EncodeOptions` is `#[non_exhaustive]` (correctly, for forward compat) but this means external integration tests cannot write `EncodeOptions { force_long_code: true, ..Default::default() }` — that syntax requires the type to be exhaustive at the call site. The bucket-C tests had to use conditional shape-detection (`if bytecode.len() > 48 && bytecode.len() <= 56`) instead of explicit force-long-code testing. Add a fluent builder: `EncodeOptions::default().with_force_chunking(true).with_force_long_code(true).with_seed(seed)` so external tests can exercise the option matrix directly.
- **Why deferred:** caught at integration-test time; non-blocking for v0.1 since internal unit tests can use struct-update via the same-crate exception.
- **Status:** open
- **Tier:** v0.1-nice-to-have

### `6d-rand-gen-keyword` — `rng.r#gen()` raw-identifier workaround for Rust 2024 reserved keyword

- **Surfaced:** Phase 6 bucket D; Task 6.20 (`many_substitutions_always_rejected`)
- **Where:** `crates/wdm-codec/tests/ecc.rs`
- **What:** Rust 2024 edition (which we're on, per `Cargo.toml`) reserved `gen` as a keyword for generators. `rand 0.8`'s `Rng::gen` method now requires `r#gen()` raw-identifier syntax to call. When `rand` migrates to a newer API (e.g., `rng.random::<u64>()` in `rand` 0.9+), this workaround can be removed.
- **Why deferred:** rand 0.9 migration is a separate concern; `r#gen()` works correctly today.
- **Status:** open
- **Tier:** v0.1-nice-to-have

### `6e-missing-children-unreachable` — `BytecodeErrorKind::MissingChildren` defined but never emitted

- **Surfaced:** Phase 6 bucket E; Task 6.21 — `rejects_invalid_bytecode_missing_children` is `#[ignore]`d
- **Where:** `crates/wdm-codec/src/error.rs` (the variant) and `crates/wdm-codec/src/bytecode/decode.rs` (where it should fire but currently UnexpectedEnd does instead)
- **What:** `BytecodeErrorKind::MissingChildren { expected, got }` exists in the enum (Phase 0.5 scaffolding) but no v0.1 code path produces it. When a `Multi`/`Thresh` operator's children loop exhausts the input mid-child, `Cursor::read_byte` surfaces `UnexpectedEnd` first, before any explicit arity check could fire. To make the variant reachable: add an explicit `count - emitted_children == got` check at the end of each variable-arity decoder branch, emitting `MissingChildren` if non-zero. The `rejects_invalid_bytecode_missing_children` conformance test is `#[ignore]`d until that arity check lands.
- **Why deferred:** the diagnostic gain (more specific error message for "stream truncated mid-children") is small; `UnexpectedEnd` correctly identifies the failure today.
- **Status:** open
- **Tier:** v0.1-nice-to-have

### `p2-fingerprints-block` — v0.2 fingerprints block support

- **Surfaced:** Phase 5-B; documented at `crates/wdm-codec/src/policy.rs:316-317` and `:668`
- **Where:** `crates/wdm-codec/src/bytecode/{header,decode}.rs`, `crates/wdm-codec/src/policy.rs::from_bytecode`
- **What:** The bytecode header has a fingerprints flag (bit 2) and a reserved tag `Tag::Fingerprints = 0x35`, but v0.1 rejects any input with the flag set via `Error::PolicyScopeViolation`. v0.2 should implement the full fingerprints-block format (per BIP §"Fingerprints block"): a count byte followed by `count` 4-byte master-key fingerprints in placeholder index order, allowing recovery tools to verify which seed corresponds to which `@i` placeholder before deriving.
- **Why deferred:** v0.1 spec scope. The recovery flow can match seeds to placeholders by trial derivation; the fingerprints block is an ergonomics + privacy improvement.
- **Status:** open
- **Tier:** v0.2

### `5e-skip-silent` — tests with size-conditional assertions skip silently

- **Surfaced:** Phase 5-E code review of `7b7400b`
- **Where:** `crates/wdm-codec/src/decode.rs:270` (`if bytecode.len() <= 56 { return; }`) and `decode.rs:530` (same pattern)
- **What:** Two tests gate their main assertion behind a size check; if the encoder ever shifts capacity (e.g., from a tag-table renumbering or LEB128 width change) the tests would silently pass without exercising the chunked path. Better: pass `EncodeOptions { force_chunking: true, ..Default() }` so the chunked path is exercised deterministically regardless of bytecode length.
- **Why deferred:** caught at review; non-blocking since the tests still pass on actual v0.1 byte counts.
- **Status:** open
- **Tier:** v0.1-nice-to-have

### `5e-dead-branch` — `decode_rejects_chunks_with_duplicate_indices` has unreachable fallback

- **Surfaced:** Phase 5-E code review of `7b7400b`
- **Where:** `crates/wdm-codec/src/decode.rs:418-450`
- **What:** The early `if backup.chunks.len() < 2` branch re-encodes the SAME 9-key multisig policy with `force_chunking: true` and asserts the same outcome as the fallthrough. Since the 9-key multisig already chunks under the default plan, the early branch is unreachable. Either remove the branch (simplify to the always-multi-chunk path) or restructure to test a smaller policy with `force_chunking` deliberately.
- **Why deferred:** caught at review; test still passes correctly.
- **Status:** open
- **Tier:** v0.1-nice-to-have

### `5e-correction-position-doc` — rustdoc cross-reference for `Correction.char_position` coordinate system

- **Surfaced:** Phase 5-E code review of `7b7400b`
- **Where:** `crates/wdm-codec/src/decode.rs` (the `decode` rustdoc); `crates/wdm-codec/src/chunking.rs:447` already documents `Correction.char_position` semantics
- **What:** `Correction.char_position` is a 0-indexed offset within the chunk's data part (after `wdm1` HRP+separator). The decode path consumes `corrected_positions` from `DecodedString`, which uses the same coordinate system. A one-line cross-reference in `decode`'s rustdoc reaffirming this would prevent confusion for callers building diagnostic UIs.
- **Why deferred:** documentation polish, not algorithmic.
- **Status:** open
- **Tier:** v0.1-nice-to-have

### `4a-from-bytes-shape` — reconsider `Chunk::from_bytes` shape (slice+usize vs `&mut Cursor`)

- **Surfaced:** Phase 4-A code review of `aefdf3f` (deferred to "after 4-E"); 4-E used the slice+usize shape unchanged
- **Where:** `crates/wdm-codec/src/chunking.rs::Chunk::from_bytes` (returns `Result<(Self, usize), Error>`)
- **What:** Phase 2/3's bytecode parsers use `&mut Cursor<'_>` for stream consumption; `Chunk::from_bytes` returns a slice + consumed-byte-count tuple. The two shapes do equivalent work but diverge stylistically. Consolidating on Cursor would let callers chain chunk parses inside a longer buffer; consolidating on slice+usize would let bytecode parsers expose simpler APIs. v0.1 has no caller that needs to switch shapes. Defer until either Phase 7 (CLI parsing of multi-chunk inputs) or any non-test consumer surfaces a need.
- **Why deferred:** premature; no consumer needs the conversion.
- **Status:** open
- **Tier:** v0.2

### `5e-five-bit-truncated-mapping` — `five_bit_to_bytes` failure error-variant choice

- **Surfaced:** Phase 5-E code review of `7b7400b`
- **Where:** `crates/wdm-codec/src/decode.rs:144-147`
- **What:** When `five_bit_to_bytes` returns `None` (data part isn't a multiple of 8 bits), the decode maps to `Error::InvalidBytecode { offset: 0, kind: Truncated }`. After a successful BCH-validated decode, this branch should be unreachable in practice — `unreachable!()` with a justification comment, or a dedicated error variant like `Error::FiveBitConversionFailed`, would be more honest than reusing `Truncated`. The current mapping is plausible (it IS a sense of "truncated") but the offset:0 is meaningless for this particular failure mode.
- **Why deferred:** caught at review; non-load-bearing diagnostic concern.
- **Status:** open
- **Tier:** v0.1-nice-to-have

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

---

## Convention notes for future agents

If you are an implementer or reviewer subagent dispatched on a task and you identify **minor items** (Important or Minor severity per the standard review rubric) that you are NOT fixing in your own commit, append an entry to this file in the same commit. Use a `<short-id>` like `<phase>-<keyword>` (e.g., `6c-corpus-fixture-helper`, `8a-vectors-schema-comment`).

If you are running in a **parallel batch** with sibling agents, do NOT write to this file directly — return your follow-up items in your final report and the controller will append them. Two parallel agents writing here cause merge conflicts.

If you are **closing** an item, edit its entry from `Status: open` → `Status: resolved <COMMIT>` and move the entry to the "Resolved items" section. Don't delete entries.
