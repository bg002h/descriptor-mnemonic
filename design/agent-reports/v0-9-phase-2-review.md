# v0.9.0 P2 review (opus)

**Date:** 2026-04-29
**Commit:** e622540
**Reviewer:** opus-4.7

## Summary

**needs-fixes-then-proceed** — The high-stakes correctness question (Q7) is **answered correctly**: T1 genuinely exercises the new `0x16` dictionary entry. The vector's `expected_bytecode_hex` is `0034160305090203330033013302`, where the third byte `0x16` is the single-byte dictionary form of the testnet path — not the explicit-path fallback. The encoder code path is `to_bytecode → encode_declaration → encode_path → path_to_indicator → vec![0x16]`, confirmed by source inspection at `crates/md-codec/src/policy.rs:418` and `crates/md-codec/src/bytecode/path.rs:69-71`. Byte-for-byte, T1 differs from M2 only in the path byte (`0x06` mainnet → `0x16` testnet), exactly as intended.

All 448 lib tests + integration + doctests pass. Bytecode-path test set is 35-passed/0-failed including the new `indicator_0x16_decodes_to_bip48_testnet_p2sh_p2wsh`. Dictionary array, FIXTURE table, both negative arrays, BIP table, SHA pin (`750d3d15…`), and corpus count assertion (44) are all consistent.

Three **minor cosmetic/cleanup issues** found (none affect wire format or test correctness). Recommend a 5-minute fixup commit before P3, but P3 can also proceed in parallel.

## Findings

### F1 — Stale rustdoc comments: "13 well-known"/"13 v0 dictionary entries" (cosmetic; minor)

`crates/md-codec/src/bytecode/path.rs:3` says "Maps the 13 well-known indicator bytes". Line 13 says "The 13 v0 dictionary entries". Both should now say `14`. The array literal at line 14 is correctly typed `[(u8, DerivationPath); 14]`, so this is purely a doc-comment drift, not a code/data inconsistency.

```rust
//! Maps the 13 well-known indicator bytes defined in BIP §"Path dictionary"
                ^^ should be 14
/// The 13 v0 dictionary entries, parsed once on first access.
        ^^ should be 14
```

### F2 — Stale rustdoc tag-byte references: `Tag::SharedPath` documented as `0x33` (cosmetic; pre-existing)

`crates/md-codec/src/bytecode/path.rs` lines 63, 102, 168, 173, 188, 199, 260 all say `Tag::SharedPath` is `0x33`. The actual current value is `0x34` (see `crates/md-codec/src/bytecode/tag.rs:122`; the v0.5→v0.6 renumber bumped Placeholder→0x33 and SharedPath→0x33→0x34). This is a **pre-existing** P2-orthogonal bug (P2 didn't introduce it; P2 didn't fix it either). Mention only because P2 touched this file. Out of scope for P2; could be a P3 cleanup or its own one-line FOLLOWUPS entry.

### F3 — One P1 straggler missed: test name `reassemble_policy_id_mismatch` (cleanup; minor)

`crates/md-codec/src/chunking.rs:1489` still has:

```rust
#[test]
fn reassemble_policy_id_mismatch() {
```

The body is correct (uses `ChunkSetId::new(...)`, asserts `ChunkSetIdMismatch`). Only the test fn name lags the rename. The implementer caught the v0.5 negative-fixture stragglers (`generate_n08_*`, `generate_n15_*`) but missed this one. A simple `s/reassemble_policy_id_mismatch/reassemble_chunk_set_id_mismatch/` and recompile.

### F4 — FOLLOWUPS entry for `md-path-dictionary-0x16-gap` not yet marked resolved (process; minor)

`design/FOLLOWUPS.md` still shows `Status: open` for the `md-path-dictionary-0x16-gap` entry. Per the "Cross-repo coordination" convention in CLAUDE.md, P2 shipping should flip this to `resolved e622540` and the mk1 companion should reference the same commit. This is the kind of step normally bundled with the release commit, so it may be intentional to defer — but flag for the implementer to decide.

## Confirmations

- Dictionary entry: `(0x16, m/48'/1'/0'/1')` correctly placed between `0x15` and `0x17` in the `DICT` array (`path.rs:30`). Array size is `[...; 14]` (`path.rs:14`).
- `FIXTURE` table (`path.rs:494-509`) lists all 14 entries; 0x16 row in the right place.
- New positive test `indicator_0x16_decodes_to_bip48_testnet_p2sh_p2wsh` (`path.rs:366-371`) asserts both `indicator_to_path(0x16)` and `path_to_indicator(...)`. Comments cite FOLLOWUPS entry id `md-path-dictionary-0x16-gap` correctly. Ordering is natural: it sits after the canonical-byte-sequences test and before the reserved-indicator negative test, paralleling the natural reading flow "positive cases → newly-added positive case → negative cases".
- Negative-test arrays (`decode_rejects_reserved_indicator` line 378, `unknown_indicator_returns_none` line 528) both list `[0x00, 0x08, 0x10, 0x18, 0xFD, 0xFF]` (resp. `[0x00, 0x08, 0x10, 0x18, 0xFD]`). 0x16 correctly removed from both. The 0xFF case in the rejected-reserved test is preserved as expected.
- Vector builder `build_v0_9_testnet_p2sh_p2wsh_vector()` (`vectors.rs:1072-1112`): template `sh(wsh(sortedmulti(2,@0/**,@1/**,@2/**)))` is byte-identical to M2's template (parallel testnet sibling). Override is `EncodeOptions::default().with_shared_path(DerivationPath::from_str("m/48'/1'/0'/1'").unwrap())`.
- **Q7 critical confirmation:** `expected_bytecode_hex = "0034160305090203330033013302"` decodes as:
  - `00` = header (no fingerprints)
  - `34` = `Tag::SharedPath`
  - `16` = path indicator (single-byte dictionary form for `m/48'/1'/0'/1'`)
  - `0305090203330033013302` = `sh(wsh(sortedmulti(2,@0,@1,@2)))` template tree
  - Comparison with M2 mainnet: `0034060305090203330033013302` — byte-for-byte identical except `06`→`16`. **The new dictionary entry is genuinely exercised.**
- BIP table `bip/bip-mnemonic-descriptor.mediawiki:351` has the row `| 0x16 || m/48'/1'/0'/1' || BIP 48 testnet multisig P2SH-P2WSH` between 0x15 and 0x17, mirroring mainnet 0x06 description. Format and content correct.
- `tests/vectors_schema.rs:251` `V0_2_SHA256 = "750d3d15…9646"` matches `sha256sum crates/md-codec/tests/vectors/v0.2.json` output exactly.
- `build_test_vectors_has_expected_corpus_count` (`tests/vectors_schema.rs:69`) asserts `v2.vectors.len() == 44` with message citing the v0.9/0x16 reason — bumped 43→44 with appropriate provenance.
- P1 stragglers in vectors.rs caught by P2: `generate_n08_reserved_chunk_set_id_bits_set` (line 1252), `generate_n15_chunk_set_id_mismatch` (line 1358), and dispatch table at lines 1129/1136. Both `n08` and `n15` ids and bodies consistent.
- `rg 'policy_id_consistent|wallet_id|wid_[ab]|WalletId\b' crates/md-codec/` — only one hit, in `policy_id.rs:218,219` documenting the historical v0.7→v0.8 `WalletId → PolicyId` rename in a doc-comment about `WalletInstanceId`. This is correct historical commentary, not a leftover.
- `rg 'ChunkPolicyId|chunk_policy_id|PolicyIdSeed|policy_id_seed|PolicyIdMismatch|ReservedPolicyIdBitsSet' crates/md-codec/` — zero hits. Clean.
- Test run: 448/448 lib tests pass; 35/35 bytecode::path tests pass including the new positive test; doctests pass. Implementer's "678 total tests passing" claim is consistent.

## Open questions for the implementer

1. Should F4 (FOLLOWUPS resolution stamp) ship now in a small fixup, or be batched with the v0.9.0 release commit? Either is fine; the existing convention is to mark `resolved <commit>` close to the resolving commit so reviewers can grep, suggesting "now" is preferred.
2. F2 is pre-existing technical debt unrelated to P2. Open a one-line FOLLOWUPS entry, or let it ride into P3's stewardship prose work?
