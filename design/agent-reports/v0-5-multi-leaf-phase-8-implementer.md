# Phase 8 Implementer Report — CLI Integration Test

**Date:** 2026-04-28
**Branch:** feature/v0.5-multi-leaf-taptree
**Commit:** a77c914

## Status: DONE

## Tasks Completed

### Task 8.1 — Manual CLI Verification

Ran `md encode "tr(@0/**,{pk(@1/**),pk(@2/**)})"` (no fingerprints needed).
Output: `md1qqqqqvcrqceqqzqvrveqzrqmxgpq047xqk42r234a`

Then decoded:
```
tr(@0/<0;1>/*,{pk(@1/<0;1>/*),pk(@2/<0;1>/*\)})
Outcome: Clean, Confidence: Confirmed, all verifications true
```

Key finding: The plan's `--fingerprints 00000000:11111111:22222222` flag format does not exist.
The actual CLI flag is `--fingerprint @N=hex` (repeatable). However, fingerprints are optional
and the encode works without them, so the integration test uses the simpler no-fingerprint form.

### Task 8.2 — CLI Integration Test

Created `crates/md-codec/tests/v0_5_cli.rs` with `cli_encode_decode_multi_leaf_taptree`.

Test structure:
1. Encodes `tr(@0/**,{pk(@1/**),pk(@2/**\)})` via `md encode`
2. Asserts exit 0 and `md1` prefix in stdout
3. Decodes the resulting chunk via `md decode`
4. Asserts exit 0 and that stdout contains `tr(`, `@0`, `@1`, `@2`, `pk(@1`, `pk(@2`

Adaptation from plan template: plan used a full key-origin descriptor as the policy string;
actual CLI takes BIP 388 template notation. Used the simpler template form which matches
what all other CLI tests use.

### Task 8.3 — Commit

Committed as `a77c914` with the plan's exact commit message.

## Self-Review Gates

- `cargo test --workspace --no-fail-fast`: 634 tests, 0 failed, 0 ignored (was 633 before this phase)
- `cargo fmt --check` (stable): clean
- `cargo clippy --workspace --all-targets -- -D warnings` (stable): clean

## File Created

- `/scratch/code/shibboleth/descriptor-mnemonic-v0.5/crates/md-codec/tests/v0_5_cli.rs` (61 lines)
