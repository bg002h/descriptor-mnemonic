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
> opt-in signer-aware validation can invoke `validate_tap_leaf_subset`
> explicitly (retained as a `pub fn` in `bytecode::encode`); see the BIP
> draft §"Signer compatibility (informational)" for the full framing.

See the [BIP draft](../../bip/bip-mnemonic-descriptor.mediawiki) for
the format specification and the
[design notes](../../design/POLICY_BACKUP.md) for the rationale.

[bip388]: https://github.com/bitcoin/bips/blob/master/bip-0388.mediawiki

## Quickstart

Add to `Cargo.toml`:

```toml
[dependencies]
md-codec = "0.3"
```

Encode a wallet policy and decode it back:

```rust
use std::str::FromStr;
use md_codec::{decode, encode, DecodeOptions, EncodeOptions, WalletPolicy};

let policy = WalletPolicy::from_str("wsh(pk(@0/**))")?;
let backup = encode(&policy, &EncodeOptions::default())?;

// `backup.chunks` holds 1+ codex32-derived strings ready to engrave.
println!("Wallet ID: {}", backup.wallet_id_words);
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
two-WalletId story, scope), see the [crate-level rustdoc][rustdoc-crate].

[rustdoc-crate]: https://docs.rs/md-codec

## Library-only consumers

The `cli` Cargo feature is enabled by default; disable it to avoid pulling
`clap` and `anyhow`:

```toml
[dependencies]
md-codec = { version = "0.3", default-features = false }
```

## CLI

This crate ships an `md` binary for ad-hoc encoding, decoding, and
inspection:

| Command | Purpose |
|---|---|
| `md encode <policy>` | Encode a BIP 388 wallet policy to one or more MD strings |
| `md decode <string>...` | Decode MD strings back to a wallet policy + report |
| `md verify <string>... --policy <policy>` | Verify decode matches expected policy |
| `md inspect <string>` | Show parsed chunk header (no full decode) |
| `md bytecode <policy>` | Hex-dump canonical bytecode for a policy |
| `md vectors` | Print the test-vector JSON to stdout |

Run as a one-shot from the workspace root:

```bash
cargo run -p md-codec --bin md -- encode 'wsh(pk(@0/**))'
```

…or install:

```bash
cargo install --path crates/md-codec
md encode 'wsh(pk(@0/**))'
```

## Test vectors

A reference test-vector file is committed at
[`tests/vectors/v0.1.json`](tests/vectors/v0.1.json) — 10 positive
round-trip vectors covering the canonical corpus plus 30 negative vectors
covering each `Error` variant. Cross-implementations should consume this
file directly; the schema lives in [`src/vectors.rs`](src/vectors.rs).

Regenerate the file with:

```bash
cargo run -p md-codec --bin gen_vectors -- --output crates/md-codec/tests/vectors/v0.1.json
```

Verify a candidate file structurally with:

```bash
cargo run -p md-codec --bin gen_vectors -- --verify crates/md-codec/tests/vectors/v0.1.json
```

## Status

`v0.3.0`. Tracks BIP 388 segwit-v0 and taproot wallet policies. The current scope:

- Single user holding all seeds (no foreign xpubs)
- All `@i` placeholders share one derivation path
- `wsh()` segwit-v0 and `tr()` taproot top-level

MuSig2, foreign xpubs, per-placeholder paths, and BIP 393 recovery
annotations are deferred to v1+. See
[`design/FOLLOWUPS.md`](../../design/FOLLOWUPS.md) for the full deferral
catalog.

565 unit + integration tests, BCH known-vectors verified against an
independent Python implementation, corpus round-trips and negative
conformance vectors locked in `tests/vectors/v0.1.json` and `tests/vectors/v0.2.json`.

## License

CC0-1.0 — see [`../../LICENSE`](../../LICENSE).
