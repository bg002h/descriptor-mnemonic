# R0 Review — Cycle E bitcoind differential (round 1)

Reviewer: Fable 5 architect agent (a0f3ac42c8e87eaf4), 2026-06-12.
Target: design/BRAINSTORM_stress_cycle_e_bitcoind_differential.md @ descriptor-mnemonic.
Persisted verbatim per CLAUDE.md convention.

## Verdict: YELLOW

The differential is PROVEN sound end-to-end (make-or-break passes: bitcoind ran here, md-codec's address == bitcoind's for all 10 corpus shapes byte-identical, incl. checksums). But the chosen network (regtest) is empirically BROKEN for this corpus and several OQs resolve opposite to the doc. Correctable on paper; fold the 2 Criticals + Importants + re-dispatch.

## Critical
- **C1 — regtest is the WRONG network; use offline `-chain=main` + mainnet `xpub` corpus.** bitcoind `-regtest` REJECTS every mainnet `xpub` (`"key 'xpub6...' is not valid"` — regtest wants `tpub`). md-codec's TLV→xpub reconstruction hardcodes `NetworkKind::Main` (derive.rs:57) and always renders `xpub...`; the whole corpus is mainnet xpub. So regtest can never consume md-codec's output without re-serializing every key to tpub (fragile, non-faithful). Proven-working path: offline `-chain=main` node (`bitcoind -chain=main -daemon -connect=0 -listen=0 -blocksonly=1`): no peers, no IBD, RPC-ready ~1s, 17MB datadir. `deriveaddresses`/`getdescriptorinfo` are pure functions of the descriptor — no chain sync. md-codec derives `Network::Bitcoin`; both sides agree on mainnet `bc1/1/3`. Rewrite the SPEC around offline `-chain=main`, NOT regtest.
- **C2 — `deriveaddresses` MUST get single-chain `/0/*` and `/1/*` descriptors, never `<0;1>`. Substitute per chain (hard invariant).** `getdescriptorinfo "wpkh(xpub.../<0;1>/*)"` → `error: Key path value '<0;1>' is not a valid uint32`. bitcoind v27 does NOT accept BIP-389 multipath here. md-codec's `to_miniscript_descriptor(d, chain)` ALREADY emits the single-chain descriptor with the chain alt substituted (`/0/*` chain=0, `/1/*` chain=1) — derive the bitcoind input from that EXACT rendered string (guarantees bitcoind sees the same descriptor md-codec derives from + the checksum round-trips).

## Important
- **I1 — Pin bitcoind v27.0 + checksum-verify the tarball.** `bitcoin-27.0-x86_64-linux-gnu.tar.gz`, sha256 `2a6974c5486f528793c79d42694b5987401e4a43c97f62b1383abf35bcee44a8` (matches published SHA256SUMS). v27.0 accepts the full corpus incl. `tr(key/NUMS, multi_a)`, `wsh(and_v(...older))`, `wsh(thresh)`. CI pins version AND verifies SHA. Not distro apt.
- **I2 — bitcoind rejects non-sane miniscript; corpus = {md-codec-derivable} ∩ {bitcoind-sane}.** `getdescriptorinfo "wsh(and_v(v:older(1),older(2)))"` → "is not sane: witnesses without signature exist". md-codec's `derive_address` does NOT enforce sanity. State the invariant so a future non-sane corpus addition fails as a REJECTION (not a mismatch) → false alarm avoided. All 10 current shapes are sane.
- **I3 — Self-tests mandatory: (a) per-shape checksum round-trip (`getdescriptorinfo` checksum == md-codec/miniscript `#csum` — verified for all 10, e.g. wpkh `#grgmpdvy`); (b) known-descriptor→pinned-address fails LOUD if bitcoind silent; (c) `panic!` if `BITCOIND_BIN` is set but the binary doesn't answer** (broken CI provisioning fails red, not green-by-skip).

## Minor
- **m1 — bitcoind RUNS in this sandbox** (download 48MB OK, `bitcoind --version` v27.0.0, offline mainnet starts, RPC answers, deriveaddresses works) → the implementer CAN verify locally; NOT CI-only. Document the local recipe (offline -chain=main, the SHA, per-chain substitution).
- **m2 — Home/shape fine:** `#[cfg(feature="derive")]` `#[ignore]`-gated test in `crates/md-codec/tests/bitcoind_differential.rs`, env-gated. In-process descriptor via `to_miniscript_descriptor(d, chain).to_string()` (no toolkit/cross-repo dep).
- **m3 — Scope:** test-only NO-BUMP, supplementary oracle. Divergence in BIP-44/49/84/86/48 DEFAULT shapes → fix-in-cycle (block); exotic-miniscript divergence → file + triage.
- **m4 — CI cadence:** push touching derive.rs/to_miniscript.rs/canonicalize.rs/encode.rs + schedule + dispatch; cache the tarball by SHA.

## Answers to open questions
MAKE-OR-BREAK (OQ1+OQ2): **PASS.** All 10 shapes: md-codec `derive_address(chain,index,Network::Bitcoin)` == bitcoind `deriveaddresses` byte-for-byte at (0,0),(0,1),(1,0),(0,5); all 10 checksums matched. wpkh BIP-84 A00 = `bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu` (= published BIP-84 vector) both sides; tr_nums_multi_a A00 = `bc1pg6u8q9mh0nc7ctd79uguq90dhkgqt5e4vwterkju6sm4d78ez9mquh3v9t` both.
1. Local feasibility: YES, reproducible locally.
2. Network: MAINNET offline `-chain=main` (regtest dead per C1).
3. Version: v27.0, SHA above; accepts all 10; rejects non-sane (I2). Corpus = 10 proven shapes.
4. Descriptor construction: in-process `to_miniscript_descriptor(d, chain).to_string()` (checksum matches getdescriptorinfo).
5. Multipath: bitcoind does NOT accept `<0;1>` — substitute per chain (C2).
6. CI: pinned checksum-verified tarball → offline -chain=main → env → --ignored test → stop.
7. Scope: NO-BUMP supplementary; default-shape divergence = block/fix-in-cycle, exotic = file+triage.

## Evidence log
- Download `bitcoin-27.0-x86_64-linux-gnu.tar.gz` (48MB), sha256 `2a6974c5…44a8` == published. `bitcoind --version` v27.0.0. Offline mainnet (`-chain=main -connect=0 -listen=0 -blocksonly=1`) ready ~1s, 17MB datadir.
- regtest rejected mainnet xpub: `getdescriptorinfo "wpkh(xpub6...)"` → "key not valid".
- Offline mainnet 10/10: each shape's `to_miniscript_descriptor(d,0/1).to_string()` → getdescriptorinfo (csum match) + deriveaddresses → A00/A01/A10 match. wpkh/pkh/sh_wpkh/tr_keypath/wsh_sortedmulti/sh_wsh_sortedmulti/tr_nums_multi_a/tr_key_multi_a/wsh_and_v_older/wsh_thresh all OK.
- Multipath rejected: `<0;1>` → "not a valid uint32". Non-sane rejected: `and_v(v:older,older)` → "not sane". Headroom: `tr(key, sortedmulti_a)` accepted by v27 (md-codec 13.0.0 can't render).
- Cleanup: scratch test removed, bitcoind stopped, /tmp/bcd deleted; tree as found.
