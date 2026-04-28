# Phase v0.2 E — Fingerprints block (`p2-fingerprints-block`)

**Status:** DONE

**Commit SHA:** `6559c17`

**Branch:** `worktree-agent-a00b5f5adba8b9b4e` (worktree)

## Summary

Removed the v0.1 `Error::PolicyScopeViolation` rejection of bytecode header
bit 2 = 1 and shipped full encoder + decoder support for the BIP
§"Fingerprints block" optional element. Library API: callers opt in via
`EncodeOptions::with_fingerprints(...)`; round-trip surfaces are
`WdmBackup.fingerprints` (encode-side) and `DecodeResult.fingerprints`
(decode-side). Decisions E-1 through E-12 from
`design/PHASE_v0_2_E_DECISIONS.md` are honoured; E-9 (MIGRATION.md note)
and E-10 (CLI flag) are deferred per the decision log.

Wire format for the no-fingerprints path is byte-identical to v0.1 — the
v0.1 vectors verify byte-for-byte (`gen_vectors --verify` PASS, 10
positive + 30 negative).

## Files changed

| File | Δ | Notes |
|---|---|---|
| `crates/wdm-codec/src/bytecode/tag.rs` | modified | Added `Tag::Fingerprints = 0x35` enum variant + `from_byte` arm; rustdoc updated to mark v0.2 implemented; existing tests adjusted for the new tag and the 0x34 reserved gap. |
| `crates/wdm-codec/src/error.rs` | modified | New `Error::FingerprintsCountMismatch { expected, got }` variant with full BIP-MUST-clause rationale rustdoc; pipeline-stage doc updated for both stage 5 (decode) and encode-side. |
| `crates/wdm-codec/src/options.rs` | modified (~+45 LOC) | New `EncodeOptions::fingerprints: Option<Vec<bitcoin::bip32::Fingerprint>>` field + `with_fingerprints` builder method; `# Privacy` rustdoc clause per E-7; default test pins all-off; new `encode_options_with_fingerprints_sets_field` unit test. |
| `crates/wdm-codec/src/policy.rs` | modified (~+170 LOC) | Encoder: validates `fps.len() == placeholder_count` up front, emits header byte `0x04` + `[Tag::Fingerprints][count][4·n]` after the path declaration. Decoder: removed the v0.1 PolicyScopeViolation rejection; reads tag/count/fingerprint bytes; validates count against the reconstructed template's `key_count()` (E-12 helper reuse). New public `from_bytecode_with_fingerprints` returning `(WalletPolicy, Option<Vec<Fingerprint>>)`; existing `from_bytecode` is now a thin wrapper that discards the parsed fingerprints. `WdmBackup.fingerprints` field added. Repurposed `from_bytecode_rejects_fingerprints_flag` test as `from_bytecode_with_fingerprints_flag_no_block_is_truncated` (E-6). |
| `crates/wdm-codec/src/encode.rs` | modified | `encode()` threads `options.fingerprints.clone()` onto `WdmBackup.fingerprints` so encode-side state is observable without a re-decode. |
| `crates/wdm-codec/src/decode.rs` | modified | `decode()` calls `WalletPolicy::from_bytecode_with_fingerprints` and populates `DecodeResult.fingerprints`; rustdoc Errors table updated to add `FingerprintsCountMismatch`. |
| `crates/wdm-codec/src/decode_report.rs` | modified | Added public `DecodeResult.fingerprints: Option<Vec<Fingerprint>>` field with privacy-flag rustdoc; existing struct-construction test updated. |
| `crates/wdm-codec/tests/fingerprints.rs` | NEW (~270 LOC) | 8 tests: round-trip with + without fingerprints, encoder count-mismatch (asymmetric "too many" direction), 4 decoder hand-crafted-bytecode rejections (missing tag, count mismatch, mid-block truncation, missing count byte), and `fingerprints_block_byte_layout_matches_bip_example` which pins the exact hex used in the BIP example so any upstream encoding drift surfaces on CI. |
| `crates/wdm-codec/tests/conformance.rs` | modified | New Layer 8 `rejects_fingerprints_count_mismatch` rejection test. |
| `crates/wdm-codec/tests/error_coverage.rs` | modified | `FingerprintsCountMismatch` registered in the `ErrorVariantName` exhaustiveness mirror enum. |
| `bip/bip-wallet-descriptor-mnemonic.mediawiki` | modified | (1) §"Fingerprints block" — added "**Privacy.**" normative paragraph (E-7) + new `=====Byte-layout example=====` subsection with annotated bytecode reproduced byte-for-byte from the live encoder. (2) Tag-table 0x35 row: "(implemented v0.2)" annotation. (3) No change at line 220 area — both `0x00` and `0x04` were already documented. |

## Placeholder-count helper choice (E-12)

Reused the existing `WalletPolicy::key_count()` method (in
`crates/wdm-codec/src/policy.rs:238`). It already does what E-12 specifies
— scans the BIP 388 template form for `@N` tokens and returns
`max_index + 1` — and is used uniformly by both the encoder
(pre-validation against `opts.fingerprints.len()`) and the decoder
(post-template-decode validation against the bytecode's count byte). No
new helper was introduced.

## DecodeResult vs WalletPolicy attachment choice

Chose to add `DecodeResult.fingerprints: Option<Vec<Fingerprint>>` (a
public field on the additive, `non_exhaustive` `DecodeResult` struct).
Internally, the decoder routes through a new public
`WalletPolicy::from_bytecode_with_fingerprints` that returns
`(WalletPolicy, Option<Vec<Fingerprint>>)`; the legacy `from_bytecode`
is preserved as a thin wrapper that discards the parsed fingerprints.

**Rationale:** attaching fingerprints as a private field on
`WalletPolicy` would have re-introduced the same `PartialEq` caveat as
the existing `decoded_shared_path` field — two logically-equivalent
policies (one parsed, one decoded) would compare unequal. Threading the
fingerprints out as a separate return value keeps `WalletPolicy`
construction-path-agnostic and makes the surface point on the public
API (`DecodeResult.fingerprints`) match the BIP description ("decoders
that surface the parsed block to the user"). The change is additive on
both `DecodeResult` (new field, `non_exhaustive`) and `WalletPolicy`
(no change to public state).

## Test coverage breakdown

| Bucket | Count | Tests |
|---|---|---|
| Round-trip positives | 2 | `round_trip_with_fingerprints_two_keys`, `round_trip_without_fingerprints_two_keys` (`tests/fingerprints.rs`) |
| Encoder rejection | 2 | `rejects_fingerprints_count_mismatch` (`tests/conformance.rs`, exhaustiveness-gated, Layer 8); `encoder_rejects_fingerprints_too_many` (`tests/fingerprints.rs`, asymmetric-direction) |
| Decoder rejection | 4 | `decoder_rejects_missing_fingerprints_tag`, `decoder_rejects_fingerprints_count_mismatch`, `decoder_rejects_fingerprints_truncated_mid_block`, `decoder_rejects_fingerprints_missing_count_byte` (`tests/fingerprints.rs`) |
| Byte-layout pin | 1 | `fingerprints_block_byte_layout_matches_bip_example` (`tests/fingerprints.rs`) — pins `0433033502deadbeefcafebabe0519020232003201` exactly so any upstream encoding drift surfaces on CI before the BIP example goes stale |
| Behavioural-break test cleanup (E-6) | 1 | `from_bytecode_with_fingerprints_flag_no_block_is_truncated` — replaces the old `from_bytecode_rejects_fingerprints_flag` |
| Unit / smoke updates | 3 | `encode_options_with_fingerprints_sets_field`, `encode_options_default_is_all_off` (extended), `tag_round_trip_all_defined` / `tag_rejects_unknown_bytes` / `tag_specific_values` (updated for 0x35) |

Total wdm-codec tests: **554 passing** (up from 544+).

## BIP edits

The BIP draft (`bip/bip-wallet-descriptor-mnemonic.mediawiki`) gained:

1. **Privacy normative paragraph** (E-7) appended to §"Fingerprints
   block (optional)":
   > **Privacy.** The fingerprints block leaks which seeds match which
   > `@i` placeholders. The block is **optional** — implementations
   > SHOULD NOT emit it by default. Recovery tools MUST warn the user
   > before encoding fingerprints, especially for solo-user single-seed
   > wallets where the disclosure is unnecessary. Decoders that surface
   > the parsed block to the user MUST flag it as a privacy-sensitive
   > field.

2. **Byte-layout example subsection** (E-11) generated from the live
   encoder via the `fingerprints_block_byte_layout_matches_bip_example`
   test. The example shows
   `wsh(multi(2, @0/**, @1/**))` with fingerprints
   `[0xdeadbeef, 0xcafebabe]` and the BIP 84 mainnet shared path:

   ```
   04 33 03 35 02 deadbeef cafebabe 05 19 02 02 32 00 32 01
   ```

   Each pair of bytes is annotated against the format table (header,
   path declaration, fingerprints block header, fps[0], fps[1],
   operator path, multi parameters, @0, @1).

3. **Tag-table annotation** (line 374): the `0x35` row now reads
   "Fingerprints block (implemented v0.2)".

4. **No change at line 220** — both `0x00` and `0x04` were already
   documented as valid v0 header values.

## Quality gates

| Gate | Result |
|---|---|
| `cargo test -p wdm-codec` | 554 passed, 0 failed |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo fmt --all --check` | clean |
| `RUSTDOCFLAGS="-D warnings -D missing_docs" cargo doc --workspace --no-deps --document-private-items` | clean |
| `cargo run --bin gen_vectors -- --verify crates/wdm-codec/tests/vectors/v0.1.json` | PASS — committed file matches regenerated vectors (10 positive, 30 negative); wire format unchanged for the no-fingerprints path |

## Deferred minor items (for the controller's FOLLOWUPS aggregation)

These are listed for the controller per the per-task workflow contract;
the agent does NOT write to `design/FOLLOWUPS.md`.

1. **`phase-e-cli-fingerprint-flag`** (E-10) — `wdm encode` CLI does not
   yet expose a `--fingerprint @0=<hex>` flag. The library API is fully
   functional via `EncodeOptions::default().with_fingerprints(...)`;
   CLI users get fingerprints support in v0.2.1+. No
   regression — pre-existing CLI tests pass unchanged.

2. **`phase-e-fingerprints-behavioral-break-migration-note`** (E-9) —
   v0.1 callers that pattern-matched `Error::PolicyScopeViolation` for
   header-bit-2 = 1 inputs will no longer see that variant. To be
   captured in `MIGRATION.md` at Phase G. Also: any third-party tooling
   that asserted `Error::PolicyScopeViolation` for the on-the-wire byte
   `0x04` will need to be updated (pre-Phase-E test
   `from_bytecode_rejects_fingerprints_flag` was repurposed as
   `from_bytecode_with_fingerprints_flag_no_block_is_truncated` — that
   conversion is a useful template for migrating any external test of
   the same shape).

## Notes

- The decoder validates the fingerprints-block count byte against the
  reconstructed template's `key_count()` (i.e. *after* the tree is
  parsed). This was the simpler of the two options the decision log
  mentions: validating before the tree is parsed would have required
  re-implementing the placeholder scan, and the existing `key_count()`
  helper is authoritative.

- Encoder fail-fast: the `fps.len() != count` check runs before the
  (relatively expensive) descriptor materialization, so a count
  mismatch surfaces immediately without paying for dummy-key
  substitution.

- The `Fingerprint::as_bytes()` method (a `&[u8; 4]` newtype accessor
  from `bitcoin::bip32` 0.32) is the canonical way to get the raw
  4-byte form for `extend_from_slice`.

- No changes to `chunking.rs`, `encoding.rs::bch_*`, `encoding.rs::polymod_*`,
  `bytecode/encode.rs`, or `bytecode/decode.rs` — staying within the
  scope ringfence in the prompt.
