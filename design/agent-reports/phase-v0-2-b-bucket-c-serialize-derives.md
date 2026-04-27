# Phase B Wave 2 / v0.2 Bucket C — `7-serialize-derives` implementer report

- **Agent:** Opus 4.7 (1M ctx) — implementer subagent
- **Bucket:** Phase B Wave 2 / v0.2 Bucket C — solo
- **Task:** `7-serialize-derives` (replace hand-built `serde_json::json!{}` with `#[derive(Serialize)]` on output types)
- **Status:** **DONE**
- **Commit:** `231574dd614379ee78bcb223a982bf478b2f1e5f`
- **Branch:** `worktree-agent-a694ec5b756b4db2e`
- **Date:** 2026-04-27

## Summary

Replaced all `serde_json::json!{}` literals in the `wdm` CLI's `--json`
output paths with `#[derive(Serialize, Deserialize)]` wrapper types
defined in a new bin-private module `wdm_json`. Closes
FOLLOWUPS.md `7-serialize-derives`.

## Files changed

- **renamed** `crates/wdm-codec/src/bin/wdm.rs` → `crates/wdm-codec/src/bin/wdm/main.rs`
  (Cargo's bin-with-submodule convention; no source change beyond the `mod wdm_json;` declaration and the two `--json` blocks now using `EncodeJson::from(&backup)` / `DecodeJson::from(&result)`)
- **new** `crates/wdm-codec/src/bin/wdm/wdm_json.rs` (336 lines: wrapper types + `From<&LibraryType>` impls + 8 unit tests)
- **modified** `crates/wdm-codec/Cargo.toml` (`[[bin]] wdm` `path` updated `src/bin/wdm.rs` → `src/bin/wdm/main.rs`)
- **modified** `crates/wdm-codec/tests/cli.rs` (+2 integration tests guarding JSON-shape contract: `wdm_encode_json_shape_is_stable`, `wdm_decode_json_shape_is_stable`)

No library source files (`policy.rs`, `encoding.rs`, `chunking.rs`,
`decode.rs`, `decode_report.rs`, `lib.rs`, `options.rs`, etc.) were
touched. No public API surface change.

## Strategy + rationale

**Chose Option A** (local bin-private wrapper types) per the spec's
recommendation. Reasoning:

1. The FOLLOWUPS entry's gating reason — `WalletPolicy` wraps a
   non-Serialize miniscript `Descriptor<DescriptorPublicKey>` — has
   not changed. Adding `#[derive(Serialize)]` to `WalletPolicy` would
   require either gating it behind a `serde` feature flag (deferred
   per FOLLOWUPS as out-of-scope for v0.1) or adding a custom
   `Serialize` impl that emits the canonical string. The wrapper
   approach captures the same intent (`policy: String =
   p.to_canonical_string()`) without library churn.
2. The v0.1.1 hand-built JSON used `format!("{:?}", confidence)` /
   `format!("{:?}", outcome)` for `Confidence` and `DecodeOutcome`.
   Replicating that contract on the library types would require
   `#[serde(rename_all = "PascalCase")]` plus marking enums
   `non_exhaustive` (which they already are) — but that couples a
   stable JSON contract to library-internal Debug formatting, which
   is fragile. The wrappers stamp it down explicitly.
3. The `BchCode` enum is currently rendered as lowercase
   `"regular"` / `"long"`. Adding `#[serde(rename_all = "lowercase")]`
   to the library `BchCode` would work, but it's a meaningful
   semantic addition to a public type for solely-CLI benefit.
4. Keeping serde concerns colocated with the CLI (`bin/wdm/wdm_json.rs`)
   is the minimal change.

The wrapper module is bin-private (`pub(crate)` within the bin crate)
so external library consumers see no API change.

## Inventory of JSON output paths in `bin/wdm.rs`

Three subcommands emit JSON:

| Subcommand                | Output struct (v0.1.1 `json!{}`)                                               | Library types involved                                                                | Wrapper type                          |
| ------------------------- | ------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------- | ------------------------------------- |
| `wdm encode --json`       | `{ chunks: [...], wallet_id_words: String }`                                   | `WdmBackup`, `EncodedChunk`, `BchCode`, `WalletIdWords`                               | `EncodeJson` / `EncodedChunkJson` / `BchCodeJson` |
| `wdm decode --json`       | `{ policy: String, report: { outcome, confidence, corrections, verifications } }` | `DecodeResult`, `DecodeReport`, `DecodeOutcome`, `Confidence`, `Correction`, `Verifications`, `WalletPolicy` (rendered via `to_canonical_string()`) | `DecodeJson` / `DecodeReportJson` / `CorrectionJson` / `VerificationsJson` |
| `wdm vectors`             | `TestVectorFile` (already `#[derive(Serialize)]` since Phase 8)                | `TestVectorFile`, `Vector`, `NegativeVector`                                          | (unchanged — already derived)         |

`wdm verify`, `wdm inspect`, `wdm bytecode` do not emit JSON, so they
are unaffected.

## Wrapper type list

In `crates/wdm-codec/src/bin/wdm/wdm_json.rs`:

- `EncodeJson` (top-level encode output)
- `EncodedChunkJson`
- `BchCodeJson` enum with `#[serde(rename_all = "lowercase")]`
- `DecodeJson` (top-level decode output)
- `DecodeReportJson`
- `CorrectionJson`
- `VerificationsJson`

Each wrapper:
- Derives `Debug, Clone, Serialize, Deserialize, PartialEq, Eq` (the
  `Deserialize` derive is what makes the symmetric round-trip test
  possible).
- Declares fields in alphabetical order — the v0.1.1 `serde_json::json!{}`
  literal produced JSON with alphabetically-sorted keys because
  `serde_json::Map` is `BTreeMap`-backed by default. Mirroring the
  declaration order makes `serde_json::to_string_pretty` produce
  byte-identical output.
- Has an explicit `From<&LibraryType> for WrapperType` impl that
  performs the conversion (e.g., `WalletPolicy::to_canonical_string()`,
  `Confidence` → Debug-repr String, `BchCode` → `BchCodeJson`).

## Before/after JSON-output diff

All three captured baselines (encode, encode-chunked, decode) compared
byte-for-byte against post-change output:

```
$ diff /tmp/wdm-encode-before.json /tmp/wdm-encode-final.json
$ diff /tmp/wdm-decode-before.json /tmp/wdm-decode-final.json
$ diff /tmp/wdm-encode-chunked-before.json /tmp/wdm-encode-chunked-final.json
```

All three diffs are empty: the v0.1.1 contract is preserved byte-for-byte.

The single sample of each output (for the report record):

`wdm encode --json wsh(pk(@0/**))`:
```json
{
  "chunks": [
    {
      "chunk_index": 0,
      "code": "regular",
      "raw": "wdm1qqqqqvcrq5xpkvsqtkqefq4vkzef2",
      "total_chunks": 1
    }
  ],
  "wallet_id_words": "secret scorpion truly forum van cinnamon hybrid public fun during bottom clock"
}
```

`wdm decode --json <chunk>`:
```json
{
  "policy": "wsh(pk(@0/<0;1>/*))",
  "report": {
    "confidence": "Confirmed",
    "corrections": [],
    "outcome": "Clean",
    "verifications": {
      "bytecode_well_formed": true,
      "cross_chunk_hash_ok": true,
      "total_chunks_consistent": true,
      "version_supported": true,
      "wallet_id_consistent": true
    }
  }
}
```

## Tests

### New tests

In `crates/wdm-codec/src/bin/wdm/wdm_json.rs::tests` (8):
- `bch_code_json_renders_lowercase`
- `encoded_chunk_json_field_order_is_alphabetical`
- `encode_json_round_trip_via_serde` (real `WdmBackup` → JSON → `EncodeJson`)
- `decode_json_round_trip_via_serde` (real `DecodeResult` → JSON → `DecodeJson`)
- `confidence_debug_matches_v011_contract`
- `outcome_debug_matches_v011_contract`
- `verifications_from_library_type` (From-impl unit test)
- `correction_from_library_type` (From-impl unit test)

In `crates/wdm-codec/tests/cli.rs` (2):
- `wdm_encode_json_shape_is_stable` — invokes the binary, parses
  `--json` output, asserts the v0.1.1-contract field set is present
  with correct types (e.g., `code` is lowercase string, `wallet_id_words`
  is 12 words).
- `wdm_decode_json_shape_is_stable` — invokes the binary, parses
  `--json` output, asserts `outcome == "Clean"`,
  `confidence == "Confirmed"`, all five `verifications.*` flags are
  bools.

### Test counts

- Baseline (before): 472 tests passing
- After: **482 tests passing** (+10: 8 unit + 2 integration)

### Quality gates (all PASS)

- `cargo test -p wdm-codec`: 482 passed, 0 failed
- `cargo clippy --workspace --all-targets -- -D warnings`: clean
- `cargo fmt --all --check`: clean
- `RUSTDOCFLAGS="-D warnings -D missing_docs" cargo doc --workspace --no-deps --document-private-items`: clean
- `cargo run --bin gen_vectors -- --verify crates/wdm-codec/tests/vectors/v0.1.json`: PASS (wire format unchanged — `gen_vectors: PASS — committed file matches regenerated vectors (10 positive, 30 negative)`)

## Out-of-scope items honoured

- No public API change to `WalletPolicy`, `EncodeOptions`, `WdmBackup`,
  `DecodeResult`, `Confidence`, etc.
- No serde derives added to miniscript types.
- No changes to `encoding.rs`, `decode.rs`, `chunking.rs`, `policy.rs`
  cores, or `decode_report.rs`. The library is unchanged below the
  CLI binary.
- Wire format unchanged (verify-vectors PASS).

## Worktree-specific build note (not blocking; minor)

The workspace `[patch."https://github.com/apoelstra/rust-miniscript"]
miniscript = { path = "../rust-miniscript-fork" }` redirect in
`Cargo.toml` is path-relative to the workspace root. From this
worktree (`.claude/worktrees/agent-...`), `../rust-miniscript-fork`
resolves to `.claude/worktrees/rust-miniscript-fork`, which does not
exist — the actual sibling clone is at
`/scratch/code/shibboleth/rust-miniscript-fork` (peer of the main
repo). To build/test in the worktree I used the cargo `--config`
override:

```
cargo {build,test,clippy,doc} --config 'patch."https://github.com/apoelstra/rust-miniscript".miniscript.path="/scratch/code/shibboleth/rust-miniscript-fork"' …
```

This is environmental (a worktree-vs-main-repo pathing artifact, not a
bug in this change). The patch redirect itself disappears once
`apoelstra/rust-miniscript#1` lands per the WDM upstream PR memory; no
further action needed for this bucket.

## Deferred minor items

None surfaced during this work. The wrapper-type approach is a clean
1:1 mirror of the v0.1.1 JSON shape; there's no follow-up debt.

(One tangential observation, not a follow-up: the `BchCode` library
enum could derive `#[serde(rename_all = "lowercase")] Serialize,
Deserialize` cleanly today — its variants are unit, no internal
dependencies. But no library consumer asks for it and the wrapper
already pins the contract. Leaving it untouched per the "do not
expand library serde surface" out-of-scope rule.)

## FOLLOWUPS.md update needed (controller action)

Move the `7-serialize-derives` entry from the **Open items** section
to the **Resolved items** section with status:

```
- **Status:** resolved 231574dd614379ee78bcb223a982bf478b2f1e5f
```

Per the WDM subagent workflow rules, the controller (not this agent)
performs the FOLLOWUPS.md aggregation.

## Self-review notes

- The strategy decision is documented in the commit body so the
  reviewer can audit it without reading this report.
- Field-order alphabetical-ness is enforced both by alphabetical
  declaration AND by the unit test
  `encoded_chunk_json_field_order_is_alphabetical`. If a future
  contributor reorders fields, the test breaks.
- Round-trip-via-serde unit tests exercise both directions of the
  `Serialize` + `Deserialize` derives — important because if someone
  later removes `Deserialize` "since the binary doesn't deserialize",
  these tests fail and explain why we keep both.
- The `format!("{:?}", confidence)` / `format!("{:?}", outcome)`
  pattern from v0.1.1 is preserved verbatim — these are fragile (rely
  on Debug repr stability) but matching them is the explicit
  byte-identical contract for v0.2. A future cleanup could introduce
  `Display` impls on `Confidence` / `DecodeOutcome` and switch the
  wrappers to use them, but that's a separate concern (and not a
  FOLLOWUPS item the spec asked for).
