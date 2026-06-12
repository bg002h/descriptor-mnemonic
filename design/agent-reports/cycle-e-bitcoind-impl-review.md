# Implementation Review — Cycle E bitcoind differential (self-review)

Reviewer: orchestrator (Fable 5), 2026-06-12. Verified against the GREEN R4 spec.

## Verdict: GREEN (0C/0I)

Test-only / NO-BUMP. Verified by the implementer (full local run) + orchestrator spot-check:

- **Locally PROVEN:** `BITCOINCLI_BIN=… BITCOIND_DATADIR=… BITCOIND_RPCPORT=18999 cargo test -p md-codec --features derive --test bitcoind_differential -- --ignored` → PASS. 10 shapes × 2 chains × 5 indices = **100 address checks + 20 checksum round-trips, all byte-identical vs pinned bitcoind v27.0** (sha `2a6974c5…44a8` verified). Reproduces R0's 20/20 with more index coverage. `wpkh` chain-0 csum `grgmpdvy` matches R0 evidence.
- **All four behaviors confirmed:** env set + node alive → PASS; env unset → skips cleanly; no `--ignored` → ignored; env set + node dead → `panic!` (fail-loud, proves the I3(b) guard). Plus a partial-env guard (panic if the 3 vars are partially set).
- **Oracle correctness (spot-checked the test source):** per shape×chain, bitcoind input = `md_codec::to_miniscript::to_miniscript_descriptor(&d, chain).to_string()` (per-chain `/0/*`|`/1/*`, never `<0;1>`); checksum self-test (`getdescriptorinfo .checksum` == the `#csum` in desc) [I3a]; `deriveaddresses "<desc>" "[0,N]"` (mandatory range); md-codec `derive_address(chain, index, Network::Bitcoin).assume_checked()`; byte-equal assert with a FUNDS-CRITICAL message. The `bitcoin_cli` helper `panic!`s on any RPC/process error → a bitcoind reject is a loud harness/corpus bug, never a silent match [I2].
- **Anti-vacuity golden [I3c]:** `wpkh` chain0 idx0 asserted == `bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu` (BIP-84 vector), with a `golden_asserted` flag checked at the end (the golden MUST have fired) — a silently-wrong bitcoind can't make the test vacuously pass.
- **Corpus = the 10 R0-proven shapes** (pkh/sh-wpkh/wpkh/tr-keypath; wsh-sortedmulti/sh-wsh-sortedmulti; tr-NUMS-multi_a/tr-key-multi_a; wsh-and_v-older/wsh-thresh). The two not verbatim in address_derivation.rs (sh(wpkh) BIP-49, tr(NUMS,multi_a)) were composed from the same primitives; both matched bitcoind first try (no weakened assertion).
- **CI workflow** `.github/workflows/bitcoind-differential.yml`: pinned v27.0 tarball + sha256 verify + cache-by-sha → extract pinned bitcoind/bitcoin-cli (NOT PATH; sandbox PATH `bitcoind` is Bitcoin Satellite, a fork) → offline `-chain=main -connect=0 -listen=0 -blocksonly=1` port 18999 + 60s readiness poll → export the 3 wiring vars → `--ignored` test → `bitcoin-cli stop` (`if: always()`) → upload-artifact@v5 on failure. Triggers: push/PR on derive.rs/to_miniscript.rs/canonicalize.rs/encode.rs + test + workflow; daily schedule; workflow_dispatch. actionlint CLEAN.
- `cargo fmt --all --check` clean; `cargo clippy --tests -D warnings` clean; address_derivation suite still 21/21. NO src change; `serde_json` already a dev-dep.

Cleared to commit. This is the external (non-rust-miniscript) ground-truth oracle for the funds-critical address output — the highest-assurance differential in the program, and it AGREES with Bitcoin Core across the whole corpus.
