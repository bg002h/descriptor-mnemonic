# Mnemonic Descriptor (MD)

> **Note**: This crate was renamed from `wdm-codec` (HRP `wdm`) to `md-codec` (HRP `md`) in v0.3.0. See [`MIGRATION.md`](./MIGRATION.md#v02x--v030) for upgrade guidance and [`CHANGELOG.md`](./CHANGELOG.md) for the v0.3.0 entry. The repository URL is unchanged.

> **Status: Pre-Draft, AI + reference implementation, awaiting human review.**
> This specification was produced by an AI assistant in collaboration with
> the author, and is shipped alongside a working reference implementation
> split across two Rust crates (CC0-1.0): the codec library at
> [`crates/md-codec/`](crates/md-codec/) and the `md` CLI at
> [`crates/md-cli/`](crates/md-cli/), locking the v0.1 wire format with
> committed test vectors. The spec and impl have
> not yet been reviewed end-to-end by a human or by the broader bitcoin
> community. Tag values, header layout, HRP, and structural decisions
> remain subject to change pending that review. Treat as a working sketch
> with concrete artifacts, not a finalized spec.

A specification for backing up bitcoin wallet policies on durable media
(paper, steel) in a form that is compact, hand-transcribable, and
strongly error-correcting.

MD is to **wallet structure** what BIP 39 is to **seed entropy**: a
canonical engravable backup format. Where a 24-word BIP 39 phrase
restores a wallet's keys, an MD string restores a wallet's spending
policy — the miniscript template, derivation paths, and (in future
versions) cosigners' extended public keys.

> **Scope note (v0.6+):** MD is *neutral* on hardware-signer compatibility.
> An MD-encoded backup is structurally well-formed if and only if the
> policy parses under BIP 388 + BIP 379; whether the policy is signable
> on a particular hardware signer is a separate concern handled by
> wallet software (above MD) and by signer firmware (below MD). The
> responsibility chain is: wallet software constructs a signer-compatible
> policy → MD encodes it losslessly → user recovers → recovery wallet
> pairs with matching signer to spend. **You are responsible for
> ensuring your policy is signable on your target signer.** See the
> BIP draft §"Signer compatibility (informational)" for the full
> framing.

## What this repository contains

```
.
├── bip/
│   ├── README.md                                  ← BIP draft index
│   └── bip-mnemonic-descriptor.mediawiki          ← the formal BIP draft
├── crates/
│   ├── md-codec/                                  ← Rust reference codec library (v0.16+: library-only)
│   ├── md-cli/                                    ← `md` CLI binary (v0.1+; ships the binary extracted from md-codec 0.15.x)
│   └── md-signer-compat/                          ← layered named-signer-subset validator (v0.7+)
├── design/
│   ├── POLICY_BACKUP.md   ← design rationale, decisions log, open items
│   ├── PRIOR_ART.md       ← survey of related encoding schemes
│   ├── CORPUS.md          ← reference miniscript test corpus
│   ├── FOLLOWUPS.md       ← deferred items + closure log
│   └── agent-reports/     ← per-phase Opus review reports
├── LICENSE
└── README.md
```

## Where to start reading

- **For format users / implementers:** `bip/bip-mnemonic-descriptor.mediawiki` is the canonical spec.
- **For the codec library:** [`crates/md-codec/README.md`](crates/md-codec/README.md) — quickstart and library API.
- **For the `md` CLI:** [`crates/md-cli/README.md`](crates/md-cli/README.md) — install instructions, subcommand reference, network selection.
- **For why the spec is the way it is:** `design/POLICY_BACKUP.md` documents the design decisions and tradeoffs in detail.
- **For comparison with existing formats:** `design/PRIOR_ART.md`.
- **For what real miniscripts look like under MD encoding:** `design/CORPUS.md` and the locked test vectors at `crates/md-codec/tests/vectors/v0.1.json`.

## What MD is for

Bitcoin wallets that use arbitrary miniscript spending policies — multisig, timelocks, decaying conditions, inheritance schemes — must back up the policy structure separately from the seed. The seed alone is insufficient to recover funds because the wallet doesn't know what spending conditions to enforce.

Existing approaches (JSON exports, Liana `.bed` files, Coldcard multisig configs, Bytewords/UR) all have shortcomings for the durable-storage case: they're too verbose for engraving, lack strong error correction, depend on digital infrastructure, or don't cover arbitrary miniscript.

MD addresses this with:

- **Codex32-derived encoding** (BIP 93 BCH error correction; bech32 alphabet)
- **Compact bytecode** for BIP 380 descriptors and BIP 388 wallet policies (extending `descriptor-codec`)
- **Multi-string chunking** for arbitrary-length policies
- **Mandatory guided recovery** combining BCH ECC with structural knowledge

## What's covered in v0

The v0 specification scope targets the most common self-custody case:

- A single user holding all seeds referenced by the policy (no foreign xpubs)
- Shared paths (all `@i` placeholders on one derivation path) **and per-`@N` divergent paths** (one path per placeholder, in placeholder-index order) — v0.10+
- `wsh()` segwit v0 or `tr()` taproot script types

This covers single-key wallets, all common multisig configurations (including those where each cosigner derives from a distinct BIP 48 account), decaying multisig, simple inheritance, and timelock-based recovery — every Liana wallet template plus typical Coldcard self-custody setups.

Foreign xpubs (multi-party multisig where you don't hold all seeds) are deferred to v1+. Per-`@N` divergent paths shipped in v0.10 (header bit 3 reclaimed; `Tag::OriginPaths = 0x36`); see [`CHANGELOG.md`](CHANGELOG.md) and [`MIGRATION.md`](MIGRATION.md) for the v0.10 wire-format break.

## Status

This specification is in **Pre-Draft, AI + reference implementation, awaiting human review** status. The structure of the spec is in place and a reference implementation ships at [`crates/md-codec/`](crates/md-codec/) with 673+ tests passing across the workspace. Independent human review of both the spec and the impl is the remaining gate. Open spec questions are tracked in `design/POLICY_BACKUP.md` §8; deferred work is tracked in [`design/FOLLOWUPS.md`](design/FOLLOWUPS.md).

The Rust reference implementation implements the current scope:

- Full encode → bytecode → chunking → codex32 → BCH-checksummed string round-trip.
- Decode pipeline with per-stage diagnostics (`DecodeReport` + `Confidence`).
- 673+ unit + integration tests across `md-codec` + `md-signer-compat`, including corpus round-trips, negative conformance vectors, hand-AST defensive coverage, and BCH known-vectors cross-checked against an independent Python implementation.
- v0.1 + v0.2 test vectors locked in `crates/md-codec/tests/vectors/` (schemas in `src/vectors.rs`).
- An `md` CLI (in [`crates/md-cli/`](crates/md-cli/)) for ad-hoc encode/decode/verify/inspect/bytecode/vectors operations, plus a `from-policy` mode (behind opt-in `cli-compiler` feature) wrapping rust-miniscript's policy compiler.
- A sibling [`md-signer-compat`](crates/md-signer-compat/) crate (v0.7.0+) shipping named hardware-signer subsets (`COLDCARD_TAP`, `LEDGER_TAP`) with a `validate_tap_tree` walker, plus a `md-signer-compat validate --signer <name> ...` CLI binary.

The `Draft` status (the first formal BIP 2 status) will be claimed only after the spec has been reviewed by at least one human contributor end-to-end.

The next development steps are tracked in `design/POLICY_BACKUP.md` §10:

1. ~~Reference implementation (Rust)~~ — done; see `crates/md-codec/`.
2. ~~Concrete test vectors for the corpus~~ — done; locked in `tests/vectors/v0.1.json`.
3. Implementation experience to refine the spec
4. Submission to bitcoin-dev for community review
5. Formal BIP submission

## License

The specification text in this repository is dedicated to the public
domain under [CC0-1.0](LICENSE). The reference implementation in
`crates/md-codec/` is released under the same CC0-1.0 license.

## Contact

bg002h · `bcg@pm.me`

## Related work

- [BIP 388 — Wallet Policies for Descriptor Wallets](https://github.com/bitcoin/bips/blob/master/bip-0388.mediawiki)
- [BIP 93 — codex32](https://github.com/bitcoin/bips/blob/master/bip-0093.mediawiki)
- [BIP 380 — Output Descriptors](https://github.com/bitcoin/bips/blob/master/bip-0380.mediawiki)
- [Pieter Wuille's miniscript reference](https://bitcoin.sipa.be/miniscript/)
- [`descriptor-codec` (Josh Doman)](https://github.com/joshdoman/descriptor-codec)
- [Liana wallet (Wizardsardine)](https://github.com/wizardsardine/liana)
