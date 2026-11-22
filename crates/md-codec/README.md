# md-codec

Reference implementation of the **Mnemonic Descriptor (MD)** format —
an engravable backup format for [BIP 388 wallet policies][bip388].

MD is to *wallet structure* what BIP 39 is to *seed entropy*: a canonical
engravable backup format. A 24-word BIP 39 phrase restores a wallet's keys; an
MD string restores a wallet's spending policy — the miniscript template, the
shared derivation path, and (in future versions) cosigner xpubs.

> **Scope note (v0.6+):** MD is *neutral* on hardware-signer compatibility.
> An MD-encoded backup is structurally well-formed if and only if the policy
> parses under BIP 388 + BIP 379; whether the policy is signable on a
> particular hardware signer is a separate concern handled by your wallet
> software and your signer's firmware. **You are responsible for ensuring
> your policy is signable on your target signer.** Callers who want
> opt-in signer-aware validation can either:
>
> - call `bytecode::encode::validate_tap_leaf_subset_with_allowlist(ms, &allowlist, leaf_index)`
>   directly with their own operator allowlist, or
> - depend on the sibling [`md-signer-compat`](../md-signer-compat/) crate
>   (v0.7.0+) for named hardware-signer subsets (`COLDCARD_TAP`, `LEDGER_TAP`)
>   plus a `validate_tap_tree(subset, tap_tree)` walker that threads
>   DFS-pre-order leaf indices through each per-leaf check.
>
> See the BIP draft §"Signer compatibility (informational)" for the full framing.

See the [BIP draft](../../bip/bip-mnemonic-descriptor.mediawiki) for
the format specification and the
[design notes](../../design/POLICY_BACKUP.md) for the rationale.

## CLI

The `md` CLI ships in the sibling [`md-cli`](../md-cli/) crate. As of
md-codec v0.16.0, this crate is library-only — `cargo install md-codec`
no longer produces a binary. To install the CLI:

```sh
cargo install --path crates/md-cli
```

See [`crates/md-cli/README.md`](../md-cli/README.md) for the subcommand
reference, network-selection notes, and feature flags.

[bip388]: https://github.com/bitcoin/bips/blob/master/bip-0388.mediawiki

## Quickstart

Add to `Cargo.toml`:

```toml
[dependencies]
md-codec = "0.48"
```

Decode an engraved md1 card and encode it back:

```rust
use md_codec::{decode_md1_string, descriptor_to_template, encode_md1_string};

// A single-string card: `wpkh(@0/<0;1>/*)` with origin m/84'/0'/0'.
let card = "md1yq802gggqpsqwgtua24e7ssf3";
let descriptor = decode_md1_string(card)?;
println!("template: {}", descriptor_to_template(&descriptor)?);

// Encoding is canonical: the decoded descriptor re-encodes to the same card.
assert_eq!(encode_md1_string(&descriptor)?, card);
# Ok::<(), Box<dyn std::error::Error>>(())
```

For the full module-level overview (pipeline diagram, type-state graph,
two-PolicyId story, scope), see the [crate-level rustdoc][rustdoc-crate].

[rustdoc-crate]: https://docs.rs/md-codec

## BCH error correction (v0.34.0+)

md-codec v0.34.0 exposes a `decode_with_correction` wrapper that runs
BCH error-correction (`BCH(93,80,8)`, `t=4`: up to four substitution
errors per chunk) before the standard decode pipeline. Use it when a
backup card may have a small number of damaged characters:

```rust
use md_codec::decode_with_correction;

// The card above with one character damaged (`gggq` engraved as `ggqq`).
let inputs = ["md1yq802ggqqpsqwgtua24e7ssf3"];
let (descriptor, corrections) = decode_with_correction(&inputs)?;
for c in &corrections {
    println!("chunk {} pos {}: {:?} -> {:?}", c.chunk_index, c.position, c.was, c.now);
}
assert_eq!(corrections.len(), 1);
assert_eq!(md_codec::encode_md1_string(&descriptor)?, "md1yq802gggqpsqwgtua24e7ssf3");
# Ok::<(), md_codec::Error>(())
```

The underlying BCH primitives (`bch::polymod_run`, `bch::hrp_expand`,
`bch::MD_REGULAR_CONST`, etc.) are also `pub` for downstream consumers
that need direct access (e.g., the `mnemonic-toolkit` `repair.rs`
delegates to `decode_with_correction` rather than reimplementing the
BCH arithmetic). The newly-exposed `bch_decode` module contains the
Berlekamp-Massey + Chien-search + Forney port (~450 LOC) that powers
the wrapper.

Multi-chunk inputs are processed atomically: if ANY chunk exceeds
the `t=4` correction capacity, the call returns
`Error::TooManyErrors { chunk_index, bound: 8 }` and no partial
result is emitted (the failing chunk's index is named for diagnostics).

A single non-chunked md1 string (the form plain `encode` emits for small payloads) is corrected the same way
(md-codec v0.35.0+).

## Cargo features

| Feature | Default? | Purpose |
|---|---|---|
| `derive` | yes | Enable `Descriptor::derive_address` (pulls in `miniscript` as a dep for the v0.32 AST → `miniscript::Descriptor` converter). |

md-codec became library-only at v0.16.0; the original CLI features
(`cli`, `cli-compiler`, `json`) moved to the sibling `md-cli` crate.
The `derive` feature was added at v0.32.0 — pure-codec consumers who
don't need address derivation can opt out with `default-features = false`:

```toml
[dependencies]
md-codec = { version = "0.32", default-features = false }
```

## License

MIT License — see [`../../LICENSE`](../../LICENSE).
