# Mnemonic Descriptor (MD)

> **Note**: This crate was renamed from `wdm-codec` (HRP `wdm`) to `md-codec` (HRP `md`) in v0.3.0. See [`MIGRATION.md`](./MIGRATION.md#v02x--v030) for upgrade guidance and [`CHANGELOG.md`](./CHANGELOG.md) for the v0.3.0 entry. The repository URL is unchanged.

> **Status: Pre-Draft, AI + reference implementation, awaiting human review.**
> This specification was produced by an AI assistant in collaboration with
> the author, and is shipped alongside a working reference implementation
> at [`crates/md-codec/`](crates/md-codec/) (Rust, CC0-1.0) that locks
> the v0.1 wire format with committed test vectors. The spec and impl have
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

## What this repository contains

```
.
├── bip/
│   ├── README.md                                  ← BIP draft index
│   └── bip-mnemonic-descriptor.mediawiki          ← the formal BIP draft
├── crates/
│   └── md-codec/                                  ← Rust reference implementation
├── design/
│   ├── POLICY_BACKUP.md   ← design rationale, decisions log, open items
│   ├── PRIOR_ART.md       ← survey of related encoding schemes
│   └── CORPUS.md          ← reference miniscript test corpus
├── LICENSE
└── README.md
```

## Where to start reading

- **For format users / implementers:** `bip/bip-mnemonic-descriptor.mediawiki` is the canonical spec.
- **For the reference implementation:** [`crates/md-codec/README.md`](crates/md-codec/README.md) — quickstart, CLI, library API.
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
- All `@i` placeholders share one derivation path
- `wsh()` segwit v0 or `tr()` taproot script types

This covers single-key wallets, all common multisig configurations, decaying multisig, simple inheritance, and timelock-based recovery — every Liana wallet template plus typical Coldcard self-custody setups.

Foreign xpubs (multi-party multisig where you don't hold all seeds) and per-placeholder paths are deferred to v1+.

## Status

This specification is in **Pre-Draft, AI + reference implementation, awaiting human review** status. The structure of the spec is in place and a reference implementation ships at [`crates/md-codec/`](crates/md-codec/) with 565 tests passing and ~95% library line coverage. Independent human review of both the spec and the impl is the remaining gate. Open spec questions are tracked in `design/POLICY_BACKUP.md` §8; deferred work is tracked in [`design/FOLLOWUPS.md`](design/FOLLOWUPS.md).

The Rust reference implementation in [`crates/md-codec/`](crates/md-codec/) implements the v0.1 scope:

- Full encode → bytecode → chunking → codex32 → BCH-checksummed string round-trip.
- Decode pipeline with per-stage diagnostics (`DecodeReport` + `Confidence`).
- 565 unit + integration tests, including 12 corpus round-trips, 30+ negative conformance vectors, and BCH known-vectors cross-checked against an independent Python implementation.
- v0.1 test vectors locked in `crates/md-codec/tests/vectors/v0.1.json` (schema in `src/vectors.rs`).
- An `md` CLI for ad-hoc encode/decode/verify/inspect/bytecode/vectors operations.

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
