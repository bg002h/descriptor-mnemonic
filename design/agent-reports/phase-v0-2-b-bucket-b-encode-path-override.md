# Phase v0.2 Phase B — Bucket B: `7-encode-path-override`

**Status:** DONE

**Commit:** `0993dc05af3535f5df0bc18b8386b484e8602b01`
(`feat(encode)!: EncodeOptions::shared_path override + CLI --path wired (closes 7-encode-path-override)`)

**Date:** 2026-04-27

## Summary

Wired the v0.1.1 CLI `--path` flag through to actually take effect on the
encoded bytecode. Added `EncodeOptions::shared_path: Option<DerivationPath>`
as tier-0 of the shared-path precedence chain, extending the Phase A rule
established by `6a-bytecode-roundtrip-path-mismatch`.

Final precedence (in `WalletPolicy::to_bytecode`):

```
EncodeOptions::shared_path           (tier 0; Phase B; NEW)
  > WalletPolicy.decoded_shared_path (tier 1; Phase A)
  > WalletPolicy.shared_path()       (tier 2; real-key origin)
  > BIP 84 mainnet fallback m/84'/0'/0' (tier 3; v0.1)
```

Acknowledged breaking change: `WalletPolicy::to_bytecode(&self)` →
`WalletPolicy::to_bytecode(&self, opts: &EncodeOptions)`.

## Files changed

| File | Purpose |
| --- | --- |
| `crates/wdm-codec/src/options.rs` | Added `shared_path` field, `with_shared_path` builder, builder test. Removed `Copy` derive (DerivationPath is not Copy); kept `Clone + Default + PartialEq + Eq`. |
| `crates/wdm-codec/src/policy.rs` | Changed `to_bytecode` signature; updated precedence-chain rustdoc on `WalletPolicy.decoded_shared_path` and `to_bytecode` to reflect tier-0 addition; updated 12 test-site call signatures; added 3 new precedence tests. |
| `crates/wdm-codec/src/encode.rs` | Pipeline now passes `options` through to `to_bytecode(options)`; updated 4 test-site call signatures. |
| `crates/wdm-codec/src/lib.rs` | `encode_bytecode` wrapper passes `&EncodeOptions::default()`. |
| `crates/wdm-codec/src/wallet_id.rs` | `compute_wallet_id_for_policy` passes `&EncodeOptions::default()`. |
| `crates/wdm-codec/src/vectors.rs` | Vector builder passes `&EncodeOptions::default()`. |
| `crates/wdm-codec/src/bin/wdm.rs` | Replaced v0.1.1 `--path is parsed but not applied` warning with actual `opts.shared_path = parsed`; updated `cmd_bytecode` call site. |
| `crates/wdm-codec/tests/chunking.rs` | 3 test-site call signature updates. |
| `crates/wdm-codec/tests/conformance.rs` | 1 test-site call signature update. |
| `crates/wdm-codec/tests/cli.rs` | New integration test `wdm_encode_path_override_bip48_takes_effect` asserts the BIP 48 indicator (`0x05`) appears in the encoded bytecode and the v0.1.1 warning is gone. |

## `to_bytecode` call-site count

22 call sites updated to the new `(&EncodeOptions)` signature:

- 1 pipeline call (`encode.rs::encode`)
- 1 wrapper (`lib.rs::encode_bytecode`)
- 1 wallet-id helper (`wallet_id.rs::compute_wallet_id_for_policy`)
- 1 vector builder (`vectors.rs::build_positive_vectors`)
- 1 CLI handler (`bin/wdm.rs::cmd_bytecode`)
- 12 in-policy unit tests (`policy.rs::tests`)
- 4 in-encode unit tests (`encode.rs::tests`)
- 3 in-chunking integration tests (`tests/chunking.rs`)
- 1 in-conformance integration test (`tests/conformance.rs`)

The CLI `cmd_encode` path goes through `encode()` → `policy.to_bytecode(options)`,
so it does not appear in the count above as a direct call site.

## New tests

| Test | What it pins |
| --- | --- |
| `options::tests::encode_options_with_shared_path_sets_field` | builder method sets the field; other defaults preserved. |
| `policy::tests::to_bytecode_honors_encode_options_shared_path_override` | tier-0 override lands in the on-wire `Tag::SharedPath` byte (`m/48'/0'/0'/2'` → indicator `0x05`, not the default `0x03`). |
| `policy::tests::to_bytecode_override_wins_over_decoded_shared_path` | tier-0 beats tier-1 (`decoded_shared_path = m/84'/0'/0'` overridden by `shared_path = m/48'/0'/0'/2'`). |
| `policy::tests::to_bytecode_default_options_still_consult_decoded_shared_path` | regression guard: tier-1 still works when tier-0 is `None`; encode-decode-encode is byte-identical. |
| `cli::wdm_encode_path_override_bip48_takes_effect` | CLI integration: `wdm encode --path bip48 wsh(pk(@0/**))` produces a chunk whose decoded bytecode has shared-path indicator `0x05`; the v0.1.1 warning is suppressed. |

## Quality gates

| Gate | Result |
| --- | --- |
| `cargo test -p wdm-codec` | 489 lib + integration tests pass; 5 doctests pass. |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo fmt --all --check` | clean (rustfmt reformatted 4 multi-line chained calls; subsequently applied). |
| `RUSTDOCFLAGS="-D warnings -D missing_docs" cargo doc --workspace --no-deps --document-private-items` | clean |
| `cargo run --bin gen_vectors -- --verify crates/wdm-codec/tests/vectors/v0.1.json` | PASS — wire format unchanged for the default-path case (10 positive, 30 negative vectors all match committed JSON byte-for-byte). |

## Coordination notes

- Bucket A's WIP (`decode.rs` + `encoding.rs` for `5e-checksum-correction-fallback`)
  was present in the working tree at multiple points during this bucket's
  development. Followed the v0.2 Phase A pattern: stashed Bucket A's WIP,
  did clean work, committed, then verified Bucket A's commit had already
  landed by the time I committed (no orphaned stashes remain).
- The auto-revert behavior of the harness (system-reminder triggered file
  rewrites of edited files) caused several rounds of stash-pop cycles. All
  intermediate state was recovered from stashes; no work was lost.

## Out-of-scope items honored

- Did not touch `encoding.rs` or `decode.rs` (Bucket A's scope).
- Did not replace `serde_json::json!{}` JSON construction in `bin/wdm.rs`
  (Bucket C's scope: `7-serialize-derives`).
- Did not add `EncodeOptions::fingerprints` (Phase E).
- Did not modify `design/FOLLOWUPS.md` (parallel-batch rule).

## Deferred minor items

None surfaced during this bucket. The acknowledged design choice — removing
`Copy` from `EncodeOptions` because `DerivationPath` is not `Copy` — is in
line with the v0.2 plan's "additive on `#[non_exhaustive]`" framing and does
not need a FOLLOWUPS entry; downstream callers were never relying on `Copy`
(no `let x = opts;` patterns in the codebase use the moved value afterward,
verified via the clean test build).
