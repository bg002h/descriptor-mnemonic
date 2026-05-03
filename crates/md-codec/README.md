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

`cargo install md-codec` produces an `md` binary.

| Subcommand | Purpose |
|---|---|
| `md encode <TEMPLATE>` | Encode a BIP 388 wallet policy template into one or more MD backup strings. |
| `md decode <STRING>...` | Decode one or more MD strings back to the template. |
| `md verify <STRING>... --template <T>` | Re-encode the template and assert it matches the strings. Exit 0 on match, 1 on mismatch. |
| `md inspect <STRING>...` | Pretty-print everything the codec sees: template, identity hashes, TLV blocks. |
| `md bytecode <STRING>...` | Annotated dump of the raw payload bytes. |
| `md vectors [--out DIR]` | Regenerate the project's deterministic test-vector corpus (maintainer tool). |
| `md compile <EXPR> --context tap\|segwitv0` | Compile a sub-Miniscript-Policy expression into a BIP 388 template. Requires `cli-compiler` feature. |

`encode`, `decode`, `inspect`, `bytecode`, and `compile` accept `--json` for structured output (schema versioned as `md-cli/1`). `verify` reports match/mismatch via exit code (0 = match, 1 = mismatch). Each subcommand's `--help` shows a worked example.

To build without the CLI: `cargo build --no-default-features`.

[bip388]: https://github.com/bitcoin/bips/blob/master/bip-0388.mediawiki

## Quickstart

Add to `Cargo.toml`:

```toml
[dependencies]
md-codec = "0.7"
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

| Feature | Default? | Purpose |
|---|---|---|
| `cli` | yes | Build the `md` binary; pulls in `clap`, `anyhow`, `regex`, `miniscript`. |
| `json` | yes | Enable `--json` output on the CLI; pulls in `serde` + `serde_json`. |
| `cli-compiler` | no | Enable `md compile` and `md encode --from-policy` (pulls `miniscript/compiler`). Implies `cli`. |

Library-only consumers:

```toml
[dependencies]
md-codec = { version = "0.15", default-features = false }
```

## License

CC0-1.0 — see [`../../LICENSE`](../../LICENSE).
