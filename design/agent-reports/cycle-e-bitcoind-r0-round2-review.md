# R0 Review — Cycle E bitcoind differential (round 2)

Reviewer: Fable 5 architect agent (a52be1c3f088fa0fa), 2026-06-12.
Target: design/BRAINSTORM_stress_cycle_e_bitcoind_differential.md (R1 fold).
Persisted verbatim per CLAUDE.md convention.

## Verdict: YELLOW

The make-or-break is re-proven end-to-end against the PINNED Bitcoin Core v27.0 (SHA-verified, fresh this round): all 10 corpus shapes × 2 chains = 20/20 byte-identical (checksum + address), 0 mismatches, 0 rejections. Both contested shapes (`wsh(and_v(v:pk,older))`, `wsh(thresh)`) are bitcoind-sane and matched. C2/I1/I2/I3 fold correctly. But C1 was folded INCOMPLETELY (two `-regtest` residuals survive in §Home+shape), and the bitcoind ownership/wiring contract is under-specified. Both fixable on paper; re-dispatch.

## Critical
- (none)

## Important
- **E-I1 — Residual regtest contradicts the C1 fold.** §Proposed design → Home+shape still reads "queries a running `-regtest` bitcoind" and "starts `-regtest -daemon`". That is the network C1 declared DEAD (regtest rejects mainnet xpubs → 100% corpus failure). Every other section correctly says offline `-chain=main`, so the spec is internally inconsistent. Fold: rewrite the Home+shape paragraph to offline `-chain=main`, and align with the connect-only model (E-I2). Re-confirmed: regtest dead, offline mainnet accepts all 10.
- **E-I2 — bitcoind ownership + wiring contract ambiguous (the make-or-break for implementability).** The spec implies "CI starts/polls/stops bitcoind; the test connects via `bitcoin-cli`" but under-specifies: only `BITCOIND_BIN` (a binary path) is named, yet the test needs the **bitcoin-cli path + the running node's `-datadir` + `-rpcport`** to connect, and the **auth handshake** is unspecified. Verified: default auth (no rpcuser) writes `<datadir>/.cookie`, and `bitcoin-cli -chain=main -datadir=… -rpcport=…` reads it (datadir+rpcport suffice). Fold: pick ONE contract — RECOMMENDED *CI owns lifecycle; test connects as a cookie client*: CI starts `bitcoind -chain=main -datadir=$RUNNER_TEMP/bcd -rpcport=18999 …` (no rpcuser/pass → cookie), exports `BITCOIND_DATADIR`, `BITCOIND_RPCPORT`, `BITCOINCLI_BIN`; the test shells `bitcoin-cli -chain=main -datadir=$BITCOIND_DATADIR -rpcport=$BITCOIND_RPCPORT …`. The local recipe also starts the node by hand → the test must accept a PRE-RUNNING node (connect-only, never spawns). State the env var names + cookie auth explicitly.

## Minor
- **E-m1 — `deriveaddresses` on a ranged (`/*`) descriptor REQUIRES a range arg `[start,end]`** — without it, RPC `error -8` (a REJECT, not a match). The corpus is all-ranged → the range arg is mandatory on every call. State as an invariant (a missing-range error must not be misclassified as a corpus bug).
- **E-m2 — drop "origin-annotated".** md-codec's TLV→xpub path (`xpub_from_tlv_bytes`, derive.rs:55-62: depth=0, parent_fp=default) renders BARE depth-0 xpubs with NO `[fp/path]` prefix (verified all 20: e.g. `wpkh(xpub661My…/0/*)#grgmpdvy`). Differential unaffected (bitcoind consumes md-codec's own rendered string), but "origin-annotated" misleads.
- **E-m3 — qualify `md_codec::to_miniscript::to_miniscript_descriptor(&d, chain)`** (pub fn to_miniscript.rs:53 under pub mod to_miniscript lib.rs:35, NOT re-exported at crate root). Public API confirmed (scratch `use md_codec::to_miniscript::to_miniscript_descriptor;` compiled). ALSO: the sandbox `/usr/bin/bitcoind` on PATH = **Bitcoin Satellite v0.2.4 (a fork, NOT Core)** — the differential MUST use the pinned tarball binary, never PATH `bitcoind`. Reinforces I1.

## Fold verification table
| Round-1 finding | Resolved? | Notes |
|---|---|---|
| C1 offline -chain=main, no regtest | PARTIAL | Correct in §provisioning/§decisions/§CI + re-proven; two `-regtest` residuals at §Home+shape → E-I1. |
| C2 per-chain single-chain descriptor | YES | Proven: `to_miniscript_descriptor(&d,0/1).to_string()` → `/0/*#…`, `/1/*#…`, no `<0;1>`; bitcoind rejects `<0;1>`. |
| I1 pin v27.0 + sha + verify | YES | Re-downloaded; sha256 == `2a6974c5…44a8`; binary v27.0.0. |
| I2 corpus 10 sane shapes; reject≠mismatch | YES | 10 labels == round-1 evidence; all sane+matched; reject classes (code -5/-8) separable from divergence. |
| I3 self-tests | YES (design) | All three specified; checksum RT re-proven; (b)/(c) depend on E-I2 wiring being nailed. |
| m1-m4 | YES | local recipe / in-process / scope / CI cadence present; m4 per-test wiring = the E-I2 gap. |

## Evidence log
- Pin: curl bitcoin-27.0-x86_64-linux-gnu.tar.gz (48,849,225 B); sha256 == `2a6974c5…44a8`; `bitcoind --version` v27.0.0.
- Offline mainnet: `-chain=main -connect=0 -listen=0 -blocksonly=1` ready ~2s, getblockchaininfo chain=main blocks=0. Cookie: default writes `<datadir>/.cookie`; bitcoin-cli connects with datadir+rpcport.
- 2b mechanic: `use md_codec::to_miniscript::to_miniscript_descriptor;` compiled; `(&d,0).to_string()` = `wpkh(xpub661My…/0/*)#grgmpdvy`, chain1 `…/1/*#ehd6ucuu`. Public, real.
- Full differential: 20/20 (chain,index) PASS vs pinned v27.0 — md checksum == getdescriptorinfo AND derive_address == deriveaddresses [0,0] byte-identical, all 10 labels (pkh, sh_wpkh, wpkh, tr_keypath, wsh_sortedmulti, sh_wsh_sortedmulti, tr_nums_multi_a, tr_key_multi_a, wsh_and_v_older, wsh_thresh). wpkh/0 = bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu (BIP-84 vector). wsh_and_v_older/0, wsh_thresh/0 both sane+matched.
- Reject classes: `<0;1>`→-5 "not a valid uint32"; non-sane→-5 "not sane"; bad xpub→-5; ranged-no-range→-8. All RPC errors, separable from divergence.
- Source: `derive_address(&self, chain: u32, index: u32, network: Network) -> Result<Address<NetworkUnchecked>, Error>` (derive.rs:93-97); Network::Bitcoin = mainnet bc1. 4 trigger-path files exist.
- `/usr/bin/bitcoind` = Bitcoin Satellite v0.2.4 (fork) — pre-existing, NOT Core; use pinned tarball.
- Cleanup: scratch deleted, pinned bitcoind stopped, /tmp/bcd_r0e*+/tmp/bitcoin-27.0+tarball removed; tree as found.

GREEN requires E-I1 (delete regtest residuals) + E-I2 (name the wiring contract + auth). Differential proven sound.
