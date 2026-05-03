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
md-codec = "0.16"
```

Encode a wallet policy and decode it back:

```rust
use std::str::FromStr;
use md_codec::{decode, encode, DecodeOptions, EncodeOptions, WalletPolicy};

let policy = WalletPolicy::from_str("wsh(pk(@0/**))")?;
let backup = encode(&policy, &EncodeOptions::default())?;

// `backup.chunks` holds 1+ codex32-derived strings ready to engrave.
println!("Policy ID: {}", backup.policy_id_words);
for (i, chunk) in backup.chunks.iter().enumerate() {
    println!("chunk {i}: {}", chunk.raw);
}

// Decode round-trip:
let inputs: Vec<&str> = backup.chunks.iter().map(|c| c.raw.as_str()).collect();
let result = decode(&inputs, &DecodeOptions::new())?;
assert_eq!(result.policy.to_canonical_string(), policy.to_canonical_string());
# Ok::<(), md_codec::Error>(())
```

For the full module-level overview (pipeline diagram, type-state graph,
two-PolicyId story, scope), see the [crate-level rustdoc][rustdoc-crate].

[rustdoc-crate]: https://docs.rs/md-codec

## Cargo features

None. md-codec is library-only as of v0.16.0; the previous `cli`,
`cli-compiler`, and `json` features moved to `md-cli` along with the
binary. Library consumers depend on the crate without a feature flag:

```toml
[dependencies]
md-codec = "0.16"
```

## License

CC0-1.0 — see [`../../LICENSE`](../../LICENSE).
