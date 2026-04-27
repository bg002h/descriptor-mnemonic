# Phase 7 — CLI binary (Tasks 7.1-7.6, 7.8)

**Status:** DONE_WITH_CONCERNS
**Commit:** b5c6fcb
**File(s):**
- `crates/wdm-codec/src/bin/wdm.rs` — full implementation (was a 5-line stub)
- `crates/wdm-codec/Cargo.toml` — added `anyhow = { version = "1.0", optional = true }` to `[dependencies]` and to the `cli` feature
- `design/FOLLOWUPS.md` — appended three new open items
- `design/agent-reports/phase-7-task-cli.md` — this report
**Role:** implementer

## Summary

Implemented the `wdm` CLI binary with five active subcommands (`encode`, `decode`, `verify`, `inspect`, `bytecode`) and one placeholder (`vectors`, deferred to Phase 8). The binary uses clap derive macros, manual JSON construction, and a three-form path argument parser. All five gates (build, smoke run, tests, clippy, fmt, rustdoc) passed cleanly.

## Implementation notes

### JSON output approach (option b — manual construction)

Per the spec and existing library constraints: `WalletPolicy` wraps a miniscript `InnerWalletPolicy` that does not derive `Serialize`, so adding `#[derive(Serialize)]` to `WdmBackup`/`DecodeResult` would require either a `serde` feature flag or manual impls. Used `serde_json::json!{}` macros in all JSON output paths instead. This keeps `bin/wdm.rs` as a pure consumer of the library with no source-level changes. Tracked as `7-serialize-derives` in FOLLOWUPS.md.

### Path argument parser (Task 7.8)

`parse_path_arg(s: &str) -> Result<DerivationPath, anyhow::Error>` tries three forms in order:

1. **Name lookup** (case-insensitive): matches against a `&[(&str, u8)]` table with 12 entries covering mainnet (bip44, bip49, bip84, bip86, bip48, bip87) and testnet (`*t` suffix) variants. Resolves via `wdm_codec::bytecode::path::indicator_to_path`.
2. **Hex indicator**: `0x??` → parses as u8, special-cases `0xFE` with a helpful message explaining that 0xFE selects the explicit encoding and the user should supply a literal path. Unknown indicators produce a clear error listing the valid range.
3. **Literal derivation path**: `DerivationPath::from_str` via the bitcoin crate.

### Path override in v0.1 (concern)

The `--path` flag is fully parsed and validated, but `EncodeOptions` does not have a `shared_path` field (Phase 5 decision D-10 deferred it). The path argument is accepted at the CLI to make the surface spec-complete, but it has no effect on the encoder in v0.1. A warning is emitted to stderr. Tracked as `7-encode-path-override`. This is the primary concern driving DONE_WITH_CONCERNS status — the flag exists but silently has no effect (beyond the warning).

### EncodeOptions construction

`EncodeOptions` is `#[non_exhaustive]` with public fields. The binary is a separate compilation unit from the library, so struct literal syntax is rejected by E0639. Used field mutation (`let mut opts = EncodeOptions::default(); opts.force_chunking = ...;`) instead.

### anyhow as a dep

Added `anyhow = { version = "1.0", optional = true }` gated on the `cli` feature (same as clap). Library consumers who disable `cli` do not pull anyhow.

### clap subcommand structure

Five real subcommands + one `Vectors` placeholder. The `Vectors` arm in the match prints a deferred message to stderr and exits with code 1. The clap derive generates `--help` text including the deferred status note.

## Smoke test results

```
$ cargo run -p wdm-codec --bin wdm -- encode "wsh(pk(@0/**))"
wdm1qqqqqvcrq5xpkvsqtkqefq4vkzef2

Wallet ID: secret scorpion truly forum van cinnamon hybrid public fun during bottom clock

$ cargo run -p wdm-codec --bin wdm -- bytecode "wsh(pk(@0/**))"
003303050c1b3200

$ cargo run -p wdm-codec --bin wdm -- decode "wdm1qqqqqvcrq5xpkvsqtkqefq4vkzef2"
wsh(pk(@0/<0;1>/*))

Outcome:     Clean
Confidence:  Confirmed
Corrections: 0
Verifications:
  cross_chunk_hash_ok:    true
  wallet_id_consistent:   true
  total_chunks_consistent:true
  bytecode_well_formed:   true
  version_supported:      true

$ cargo run -p wdm-codec --bin wdm -- verify "wdm1qqqqqvcrq5xpkvsqtkqefq4vkzef2" --policy "wsh(pk(@0/**))"
OK — decoded policy matches expected policy.
Policy: wsh(pk(@0/<0;1>/*))

$ cargo run -p wdm-codec --bin wdm -- verify "wdm1qqqqqvcrq5xpkvsqtkqefq4vkzef2" --policy "wsh(pk(@1/**))"
MISMATCH
  Decoded:  wsh(pk(@0/<0;1>/*))
  Expected: wsh(pk(@1/<0;1>/*))
[exit 1]

$ cargo run -p wdm-codec --bin wdm -- inspect "wdm1qqqqqvcrq5xpkvsqtkqefq4vkzef2"
BCH code:        Regular (13-char checksum)
BCH corrections: 0
Fragment length: 8 bytes
Type:            SingleString
Version:         0

$ cargo run -p wdm-codec --bin wdm -- encode "wsh(pk(@0/**))" --json
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

$ cargo run -p wdm-codec --bin wdm -- encode "wsh(multi(5,@0/**,@1/**,@2/**,@3/**,@4/**,@5/**,@6/**,@7/**,@8/**))"
wdm1qqqqqvcrq5vs2zfjqqeqzvszxgpnyppjq5eqvvs8xgyqtxryu4g3faaml

Wallet ID: jeans control add only opera hurry pair aim napkin large direct grass

$ cargo run -p wdm-codec --bin wdm -- encode "wsh(pk(@0/**))" --path bip48
warning: --path is parsed but the shared-path override is not yet applied to the bytecode encoder (deferred to v0.2; see FOLLOWUPS.md 7-encode-path-override)
[...output...]

$ cargo run -p wdm-codec --bin wdm -- encode "wsh(pk(@0/**))" --force-chunked
wdm1qqqsc2vzqyqqqvcrq5xpkvsqc2vz0fgl7qkuehyen6wx
[...wallet id...]

$ cargo run -p wdm-codec --bin wdm -- vectors
wdm vectors: not yet implemented — use gen_vectors directly until v0.2 (Task 7.7 deferred to Phase 8)
[exit 1]
```

## Test gates

- **Build**: clean (`cargo build -p wdm-codec --bin wdm` exits 0)
- **Tests**: 361 lib + 68 integration + 1 doctest = 430 passing, 1 ignored, 0 failed (unchanged from baseline)
- **Clippy**: clean (`cargo clippy --workspace --all-targets -- -D warnings`)
- **Fmt**: clean (`cargo fmt --check`)
- **Rustdoc**: clean (`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items`)

## Follow-up items (added to FOLLOWUPS.md inline)

- `7-cli-integration-tests`: CLI integration tests via `assert_cmd` — deferred to v0.2 (option b per spec)
- `7-encode-path-override`: `--path` flag parses but does not affect the bytecode encoder; deferred to v0.2 when `EncodeOptions::shared_path` is added
- `7-serialize-derives`: manual JSON construction vs `#[derive(Serialize)]` on library types — deferred to v0.2

## Concerns / deviations

1. **`--path` has no effect on encode output**: The option is validated (parse errors surface correctly) and the warning message directs users to the follow-up, but a user who relies on `--path` to control the bytecode path declaration will get the default path (m/84'/0'/0' for template-only policies). This is the primary concern for DONE_WITH_CONCERNS. The fix is straightforward in v0.2 once `EncodeOptions::shared_path` is added.

2. **`wdm vectors` placeholder**: The subcommand is present in the clap definition (so `--help` lists it) but exits 1 with an explanatory message. This is intentional per the spec (7.7 deferred to Phase 8).

3. **Line count**: `wdm.rs` is 453 lines, just past the 400-line threshold mentioned in the spec. The path-arg parser is inline in `wdm.rs` rather than split into a sibling module because the current size is still readable and the spec says to split "if it grows past ~400 lines" (guidance, not hard limit). No split performed.
