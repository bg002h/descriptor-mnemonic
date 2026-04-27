# Wallet Descriptor Mnemonic (WDM)

> **Status: Pre-Draft, AI only, not yet human reviewed.** This is an
> early specification produced by an AI assistant in collaboration with
> the author, but has not yet been reviewed end-to-end by a human or by
> the broader bitcoin community. Tag values, header layout, HRP, and
> structural decisions are subject to change. No reference implementation
> exists yet. Treat as a working sketch, not a finalized spec.

A specification for backing up bitcoin wallet policies on durable media
(paper, steel) in a form that is compact, hand-transcribable, and
strongly error-correcting.

WDM is to **wallet structure** what BIP 39 is to **seed entropy**: a
canonical engravable backup format. Where a 24-word BIP 39 phrase
restores a wallet's keys, a WDM string restores a wallet's spending
policy — the miniscript template, derivation paths, and (in future
versions) cosigners' extended public keys.

## What this repository contains

This repository is the format specification only. A reference
implementation will be developed in a separate phase and released under
matching terms.

```
.
├── bip/
│   └── bip-wallet-descriptor-mnemonic.mediawiki   ← the formal BIP draft
├── design/
│   ├── POLICY_BACKUP.md   ← design rationale, decisions log, open items
│   ├── PRIOR_ART.md       ← survey of related encoding schemes
│   └── CORPUS.md          ← reference miniscript test corpus
├── LICENSE
└── README.md
```

## Where to start reading

- **For format users / implementers:** `bip/bip-wallet-descriptor-mnemonic.mediawiki` is the canonical spec.
- **For why the spec is the way it is:** `design/POLICY_BACKUP.md` documents the design decisions and tradeoffs in detail.
- **For comparison with existing formats:** `design/PRIOR_ART.md`.
- **For what real miniscripts look like under WDM encoding:** `design/CORPUS.md`.

## What WDM is for

Bitcoin wallets that use arbitrary miniscript spending policies — multisig, timelocks, decaying conditions, inheritance schemes — must back up the policy structure separately from the seed. The seed alone is insufficient to recover funds because the wallet doesn't know what spending conditions to enforce.

Existing approaches (JSON exports, Liana `.bed` files, Coldcard multisig configs, Bytewords/UR) all have shortcomings for the durable-storage case: they're too verbose for engraving, lack strong error correction, depend on digital infrastructure, or don't cover arbitrary miniscript.

WDM addresses this with:

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

This specification is in **Pre-Draft, AI only, not yet human reviewed** status. The structure of the spec is in place but has not been validated by independent review or by working implementations. Open decisions and unresolved questions are tracked in `design/POLICY_BACKUP.md` §8.

The `Draft` status (the first formal BIP 2 status) will be claimed only after the spec has been reviewed by at least one human contributor end-to-end.

The next development steps are tracked in `design/POLICY_BACKUP.md` §10:

1. Reference implementation (Rust)
2. Concrete test vectors for the corpus
3. Implementation experience to refine the spec
4. Submission to bitcoin-dev for community review
5. Formal BIP submission

## License

The specification text in this repository is dedicated to the public
domain under [CC0-1.0](LICENSE). Reference implementation code (when
added) will be released under a permissive license.

## Contact

bg002h · `bcg@pm.me`

## Related work

- [BIP 388 — Wallet Policies for Descriptor Wallets](https://github.com/bitcoin/bips/blob/master/bip-0388.mediawiki)
- [BIP 93 — codex32](https://github.com/bitcoin/bips/blob/master/bip-0093.mediawiki)
- [BIP 380 — Output Descriptors](https://github.com/bitcoin/bips/blob/master/bip-0380.mediawiki)
- [Pieter Wuille's miniscript reference](https://bitcoin.sipa.be/miniscript/)
- [`descriptor-codec` (Josh Doman)](https://github.com/joshdoman/descriptor-codec)
- [Liana wallet (Wizardsardine)](https://github.com/wizardsardine/liana)
