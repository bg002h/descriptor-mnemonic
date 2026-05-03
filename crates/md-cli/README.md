# md-cli

`md` — command-line interface for the **Mnemonic Descriptor (MD)** format.
Encode, decode, verify, and inspect engravable backups of [BIP 388 wallet
policies][bip388].

The codec library lives in the sibling [`md-codec`](../md-codec/) crate;
md-cli is a thin CLI on top of it. The `md` binary's source moved out of
md-codec into this crate at md-codec v0.16.0 / md-cli v0.1.0; the wire
format is unchanged.

[bip388]: https://github.com/bitcoin/bips/blob/master/bip-0388.mediawiki

## Install

In-repo (recommended while md-cli is pre-1.0 and unpublished):

```sh
cargo install --path crates/md-cli
```

This produces an `md` binary in `~/.cargo/bin/`. To enable the policy
compiler subcommand (`md compile`, `md encode --from-policy`):

```sh
cargo install --path crates/md-cli --features cli-compiler
```

To build without the JSON output paths (smaller dep set):

```sh
cargo install --path crates/md-cli --no-default-features
```

## Subcommands

| Subcommand | Purpose |
|---|---|
| `md encode <TEMPLATE>` | Encode a BIP 388 wallet policy template into one or more MD backup strings. |
| `md decode <STRING>...` | Decode one or more MD strings back to the template. |
| `md verify <STRING>... --template <T>` | Re-encode the template and assert it matches the strings. Exit 0 on match, 1 on mismatch. |
| `md inspect <STRING>...` | Pretty-print everything the codec sees: template, identity hashes, TLV blocks. |
| `md bytecode <STRING>...` | Annotated dump of the raw payload bytes. |
| `md address <STRING>...` (or `--template <T> --key @i=<XPUB>`) | Derive bitcoin addresses from a wallet-policy-mode descriptor. `--chain N` / `--change`, `--index N`, `--count K`, `--network mainnet\|testnet\|signet\|regtest`, `--json`. |
| `md vectors [--out DIR]` | Regenerate the project's deterministic test-vector corpus (maintainer tool). |
| `md compile <EXPR> --context tap\|segwitv0` | Compile a sub-Miniscript-Policy expression into a BIP 388 template. Requires `cli-compiler` feature. |

`encode`, `decode`, `inspect`, `bytecode`, `address`, and `compile` accept
`--json` for structured output (schema versioned as `md-cli/1`). `verify`
reports match/mismatch via exit code (0 = match, 1 = mismatch). Each
subcommand's `--help` shows a worked example.

### Network selection

`md encode`, `md verify`, and `md address` accept
`--network mainnet|testnet|signet|regtest` (default `mainnet`). The wire
format does not carry network — it's a CLI-side convenience for
xpub/tpub validation and address rendering. `md decode`/`inspect`/`bytecode`
are network-agnostic; pass `--network` to `md address` when rendering
addresses from a phrase that was originally built with non-mainnet keys.

## Cargo features

| Feature | Default? | Purpose |
|---|---|---|
| `json` | yes | Enable `--json` output paths; pulls in `serde` + `serde_json`. |
| `cli-compiler` | no | Enable `md compile` and `md encode --from-policy` (pulls `miniscript/compiler`). |

## License

CC0-1.0 — see [`../../LICENSE`](../../LICENSE).
