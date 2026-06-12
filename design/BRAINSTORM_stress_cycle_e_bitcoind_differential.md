# BRAINSTORM — stress Cycle E: Bitcoin Core address differential

Status: R4 **GREEN (0C/0I)** — cleared for implementation. 2026-06-12.
Reviews (persisted verbatim): cycle-e-bitcoind-r0-round1-review.md (YELLOW
2C/3I — folded `[C1]`/`[C2]`+I1-I3,m1-m4), …-round2-review.md (YELLOW 0C/2I —
folded `[E-I1]`/`[E-I2]`+E-m1-3), …-round3-review.md (YELLOW 0C/1I — folded
`[E3-I1]` env-var typo + `[E3-m1]` stale recon line), …-round4-review.md
(GREEN 0C/0I). **Make-or-break PROVEN (twice):** the PINNED bitcoind v27.0
ran in-sandbox and md-codec's `derive_address` == `bitcoind deriveaddresses`
BYTE-FOR-BYTE for all 10 corpus shapes × 2 chains = 20/20 (incl. checksums).
Program: Cycle E of the 6-cycle stress program (A/B/C/D shipped). Home repo:
descriptor-mnemonic (md-codec owns the funds-critical `derive_address`).
Heaviest infra (bitcoind in CI).

## Problem / charter

The constellation's address derivation
(`md_codec::Descriptor::derive_address` → `to_miniscript_descriptor` →
`miniscript::Descriptor::at_derivation_index().address()`,
derive.rs:92-132) is validated TODAY only against:
- rust-miniscript itself (the converter's own backend), and
- hardcoded BIP-44/49/84/86 spec golden vectors (address_derivation.rs).

Both share a common dependency family (rust-bitcoin / rust-miniscript). A
bug in rust-miniscript's address computation — or in md-codec's
AST→miniscript converter that rust-miniscript happens to also mis-handle —
would pass undetected. **Bitcoin Core (`bitcoind`) is an INDEPENDENT C++
implementation**: cross-checking the constellation's derived addresses
against `bitcoind deriveaddresses` catches exactly the class of bug a
same-ecosystem oracle can't. This is the highest-assurance differential in
the program (external ground truth for the funds-critical output: the
address a user sends coins to).

## Goal

A CI-gated differential: for a corpus of descriptors, derive addresses via
md-codec AND via `bitcoind deriveaddresses`, assert byte-equal per (chain,
index). Test-only, NO-BUMP. bitcoind SUPPLEMENTS (does not replace) the
existing rust-miniscript + golden oracles.

## Recon facts (2026-06-12; R0 to re-verify load-bearing ones)

- **Constellation derive surface:** `md_codec::Descriptor::derive_address`
  (derive.rs:92-132). md-cli `md address <md1> --chain --index --count` and
  `md address --template <T> --key @i=<xpub>` shell over it.
- **bitcoind consumption:** `bitcoind deriveaddresses "<descriptor>"
  "[a,b]"` needs a CONCRETE descriptor WITH a BIP-380 checksum (from
  `getdescriptorinfo`). `deriveaddresses`/`getdescriptorinfo` are RPCs →
  need a RUNNING bitcoind, but an OFFLINE `-chain=main` node is instant
  (genesis only, no IBD/chain/network — see [C1]: regtest is DEAD here, it
  rejects mainnet xpubs; mainnet is the only viable network for this corpus).
- **Concrete checksummed descriptor:** `miniscript::Descriptor::from_str`
  computes the BIP-380 checksum; the harness can build the concrete
  descriptor string itself (it owns the corpus xpubs/templates — same as
  address_derivation.rs builds Descriptors programmatically). The toolkit's
  `restore --format descriptor` also emits one, but the harness building it
  in-process avoids the cross-repo dependency.
- **Existing oracle:** address_derivation.rs (md-codec) already has BIP
  golden vectors + the rust-miniscript path for single-sig/multisig/taproot
  shapes, using the abandon-mnemonic ("abandon…about", mfp 73c5da0a).
- **No bitcoind in either repo's CI today.**
- **Version:** rust-bitcoin 0.32 / miniscript pinned; bitcoind needs ≥ v24
  for stable taproot + `multi_a`/miniscript-in-tr; v25+ recommended.

## Proposed design

### Home + shape

A `#[cfg(feature = "derive")]` `#[ignore]`-by-default integration test in
**md-codec** (`crates/md-codec/tests/bitcoind_differential.rs`). It derives
via md-codec for a corpus, queries a PRE-RUNNING offline `-chain=main`
bitcoind via `bitcoin-cli`, and asserts address equality. A
`bitcoind-differential.yml` CI job installs the PINNED v27.0 bitcoind
(sha-verified), starts an offline `-chain=main -daemon` node, exports the
wiring env vars (below), runs the `--ignored` test, stops bitcoind.

**bitcoind ownership/wiring contract [E-I2] — CI owns the lifecycle; the
test is CONNECT-ONLY (never spawns bitcoind).** CI starts
`bitcoind -chain=main -daemon -datadir=$RUNNER_TEMP/bcd -rpcport=18999
-connect=0 -listen=0 -blocksonly=1` with DEFAULT auth (no `-rpcuser`/
`-rpcpassword` → bitcoind writes `<datadir>/.cookie`) and exports three env
vars the test reads:
- `BITCOINCLI_BIN` — path to the pinned `bitcoin-cli` (NOT PATH `bitcoind`,
  which in some environments is a non-Core fork, e.g. Bitcoin Satellite —
  always the pinned tarball binary [E-m3]).
- `BITCOIND_DATADIR` — the running node's datadir (so bitcoin-cli finds the
  `.cookie`).
- `BITCOIND_RPCPORT` — the rpc port.
The test shells `$BITCOINCLI_BIN -chain=main -datadir=$BITCOIND_DATADIR
-rpcport=$BITCOIND_RPCPORT <rpc> …` (cookie auth, no credentials needed).
**Fail-LOUD [I3b]:** if the three env vars ARE set but bitcoin-cli doesn't
answer (`getblockchaininfo` fails), the test `panic!`s — broken CI
provisioning fails RED, never green-by-skip. If the env vars are UNSET,
the test no-ops (skips) — the standard `#[ignore]` local default. The local
recipe (above) starts the node by hand and exports the same three vars.

### bitcoind provisioning [I1] — PINNED v27.0, OFFLINE MAINNET [C1]

Pin **bitcoind v27.0**, install via the official tarball
`bitcoin-27.0-x86_64-linux-gnu.tar.gz`, **sha256-verify**
`2a6974c5486f528793c79d42694b5987401e4a43c97f62b1383abf35bcee44a8`
(matches the published SHA256SUMS) — NOT distro apt (versions lag/vary). v27
accepts the full corpus (incl. `tr(key/NUMS, multi_a)`, miniscript-in-wsh).

**Network = MAINNET, OFFLINE `-chain=main` [C1] (NOT regtest).** Regtest is
DEAD for this corpus: bitcoind `-regtest` rejects mainnet `xpub`s
(`"key not valid"` — regtest wants `tpub`), but md-codec's TLV→xpub
reconstruction hardcodes `NetworkKind::Main` (derive.rs:57) and always
renders `xpub...`, so the whole corpus is mainnet xpub. Start
`bitcoind -chain=main -daemon -datadir=<tmp> -connect=0 -listen=0
-blocksonly=1 -rpcport=<p>`: no peers, no IBD, RPC-ready in ~1s, ~17MB
datadir (genesis only). `deriveaddresses`/`getdescriptorinfo` are pure
functions of the descriptor — no chain sync. md-codec derives
`Network::Bitcoin`; both sides agree on mainnet `bc1`/`1`/`3`. Teardown
`bitcoin-cli stop`. Pin the version so a Core release can't silently move
the oracle.

**Local-verification recipe [m1] (bitcoind RUNS in-sandbox — NOT CI-only):**
download + sha-verify the tarball, extract `bitcoind`/`bitcoin-cli`, start
the offline `-chain=main` node above, run the `--ignored` test with the
binary path env. The implementer MUST verify locally.

### Corpus [I2] — the 10 R0-PROVEN shapes (md-codec-derivable ∩ bitcoind-SANE)

bitcoind REJECTS non-sane miniscript (`"is not sane: witnesses without
signature exist"`) and md-codec's `derive_address` does NOT enforce sanity —
so the corpus is the intersection {md-codec-derivable} ∩ {bitcoind-sane}.
A future non-sane addition would fail as a REJECTION (not a mismatch) → a
false alarm; the harness must classify a bitcoind PARSE-REJECT distinctly
from an address MISMATCH. The 10 R0-proven (all sane, all matched):
- single-sig: `pkh`, `sh(wpkh)`, `wpkh`, `tr` keypath (BIP-44/49/84/86)
- multisig: `wsh(sortedmulti)`, `sh(wsh(sortedmulti))`
- taproot: `tr(NUMS, multi_a(k,…))`, `tr(<key>, multi_a)`
- sane miniscript-in-wsh: `wsh(and_v(v:pk, older))`, `wsh(thresh(...))`
Each from the abandon-mnemonic mainnet xpub vectors (reuse
address_derivation.rs's). NOTE [E-m2]: md-codec's TLV→xpub path
(`xpub_from_tlv_bytes`, derive.rs:55-62) renders BARE depth-0 xpubs with NO
`[fp/path]` origin prefix (e.g. `wpkh(xpub661My…/0/*)#grgmpdvy`) — the
differential is unaffected (bitcoind consumes md-codec's own rendered
string), so do NOT try to inject origins the wire path doesn't produce.

### Oracle [C2][I3]

For each corpus entry × chain ∈ {0,1} × index ∈ {0..N}:
- md-codec: `derive_address(chain, index, Network::Bitcoin)`.
- **Descriptor for bitcoind [C2]:** the SINGLE-CHAIN string
  `to_miniscript_descriptor(d, chain).to_string()` — md-codec already
  substitutes the chain alt (`/0/*` for chain 0, `/1/*` for chain 1) and
  includes the BIP-380 `#csum`. bitcoind does NOT accept the `<0;1>`
  multipath form (`"not a valid uint32"`), so per-chain substitution is a
  HARD invariant. Deriving bitcoind's input from md-codec's OWN rendered
  string guarantees bitcoind sees exactly the descriptor md-codec derives
  from.
- bitcoind: `deriveaddresses "<that string>" "[0,N]"` → array; pick index.
  Assert byte-equal to md-codec's. **The range arg `[0,N]` is MANDATORY
  [E-m1]** — `deriveaddresses` on a ranged (`/*`) descriptor with NO range
  returns RPC error -8; a missing-range error is a HARNESS bug, not a corpus
  reject.
- The bitcoind input is `md_codec::to_miniscript::to_miniscript_descriptor(&d,
  chain).to_string()` (the fully-qualified path [E-m3]; pub fn, not
  re-exported at the crate root).
- **Self-tests [I3] (mandatory):**
  - (a) per-shape **checksum round-trip**: `getdescriptorinfo`'s computed
    checksum MUST equal md-codec/miniscript's `#csum` (verified all 10,
    e.g. wpkh `#grgmpdvy`) — catches canonicalization drift before
    deriveaddresses.
  - (b) **fail-LOUD if set-but-silent** (the §Home+shape contract): if the
    three wiring vars (`BITCOINCLI_BIN`/`BITCOIND_DATADIR`/`BITCOIND_RPCPORT`)
    are SET but `bitcoin-cli getblockchaininfo` fails, `panic!` — broken CI
    provisioning fails RED, never green-by-skip. (Unset → skip, the
    `#[ignore]` local default.)
  - (c) a pinned known-descriptor→known-`bc1` golden so a silently-wrong
    bitcoind fails loud.
- **Reject vs mismatch:** a bitcoind PARSE error on a corpus entry = a
  corpus/harness bug (loud), NOT a "match" and NOT an address divergence.

### In-process descriptor construction [m2]

Build the Descriptor in-process (md-codec test deps already build
`miniscript::Descriptor` strings in address_derivation.rs); the bitcoind
input is `to_miniscript_descriptor(d, chain).to_string()`. No toolkit /
cross-repo dependency.

### What this could surface

Any (chain,index,shape) where md-codec's address ≠ bitcoind's = a real
funds-critical finding (a user would send to the wrong address). File LOUD.
Expected outcome (R0-confirmed today): full agreement. **On a divergence
[m3]:** in a BIP-44/49/84/86/48 DEFAULT shape → BLOCK / fix-in-cycle
(funds-critical default path); in an exotic-miniscript shape → file a
funds-critical FOLLOWUP + triage.

## Resolved decisions (round-1 R0 answers, adopted)

1. Local feasibility: YES — bitcoind v27.0 runs in-sandbox; implementer
   verifies locally (NOT CI-only). [m1]
2. Network: MAINNET offline `-chain=main` (regtest dead — rejects mainnet
   xpubs). [C1]
3. Version: v27.0, sha `2a6974c5…44a8`; accepts all 10 shapes; rejects
   non-sane. [I1][I2]
4. Descriptor: in-process via `to_miniscript_descriptor(d, chain).to_string()`.
   [m2]
5. Multipath: bitcoind rejects `<0;1>` → substitute per chain (the rendered
   single-chain string already does). [C2]
6. CI: pinned sha-verified tarball → offline `-chain=main` → env → `--ignored`
   test → stop; path trigger on derive.rs/to_miniscript.rs/canonicalize.rs/
   encode.rs + schedule + dispatch; cache by sha. [m4]
7. Scope: test-only NO-BUMP supplementary; default-shape divergence =
   block/fix-in-cycle, exotic = file + triage. [m3]

## CI shape [m4]

`bitcoind-differential.yml`: (1) download `bitcoin-27.0-x86_64-linux-gnu.tar.gz`,
verify sha256 `2a6974c5486f528793c79d42694b5987401e4a43c97f62b1383abf35bcee44a8`,
cache by that sha; (2) extract → pinned `bitcoind`/`bitcoin-cli` (NOT PATH);
(3) start `<pinned>/bitcoind -chain=main -daemon -datadir=$RUNNER_TEMP/bcd
-rpcport=18999 -connect=0 -listen=0 -blocksonly=1` (default auth → `.cookie`);
(4) poll `<pinned>/bitcoin-cli -chain=main -datadir=$RUNNER_TEMP/bcd
-rpcport=18999 getblockchaininfo` until ready; (5) export `BITCOINCLI_BIN`
(pinned path), `BITCOIND_DATADIR=$RUNNER_TEMP/bcd`, `BITCOIND_RPCPORT=18999`
and run `cargo test -p md-codec --features derive --test bitcoind_differential
-- --ignored`; (6) `<pinned>/bitcoin-cli … stop`.
Triggers: push/PR touching `crates/md-codec/src/{derive,to_miniscript,
canonicalize,encode}.rs` + the test + the workflow; schedule (daily/weekly);
workflow_dispatch. actionlint-clean. upload-artifact@v5 on failure
(the diverging descriptor + both addresses).
