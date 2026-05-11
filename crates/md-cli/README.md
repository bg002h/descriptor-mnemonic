# md-cli

`md` — command-line interface for the **Mnemonic Descriptor (MD)** format.
Encode, decode, verify, and inspect engravable backups of [BIP 388 wallet
policies][bip388].

The codec library lives in the sibling [`md-codec`](../md-codec/) crate;
md-cli is a thin CLI on top of it. The `md` binary's source moved out of
md-codec into this crate at md-codec v0.16.0 / md-cli v0.1.0 (split was
wire-format-neutral). The current wire format is v0.30 (a clean break
from v0.x — see [`MIGRATION.md`](../../MIGRATION.md) and
`design/SPEC_v0_30_wire_format.md`).

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
| `md compile <EXPR> --context tap\|segwitv0 [--unspendable-key <KEY>]` | Compile a sub-Miniscript-Policy expression into a BIP 388 template. Requires `cli-compiler` feature. `--unspendable-key` is a tap-context-only fallback hint; defaults to BIP-341 NUMS H-point when omitted. |

### Compile examples

```text
# Single-key tap (key-path-only):
md compile 'pk(@0)' --context tap
# → tr(@0)

# Inheritance / timelock pattern (extract wins; @0 is internal key,
# the timelocked branch becomes the script-path leaf):
md compile 'or(pk(@0),and(pk(@1),older(144)))' --context tap
# → tr(@0,and_v(v:pk(@1),older(144)))

# 2-of-3 hardware-wallet multisig (auto-NUMS internal key —
# script-path-only spending via multi_a):
md compile 'thresh(2,pk(@0),pk(@1),pk(@2))' --context tap
# → tr(50929b74...ce803ac0,multi_a(2,@0,@1,@2))

# Force script-path-only with explicit NUMS (rare; for
# extractable-key policies the auto-NUMS default already kicks in
# only when extraction fails, so explicit NUMS is identity for
# extractable policies):
md compile 'and(pk(@0),pk(@1))' --context tap \
    --unspendable-key 50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0
# → tr(50929b74...ce803ac0,and_v(v:pk(@0),pk(@1)))

# Segwitv0 wsh (policy `thresh` compiles to miniscript `multi`):
md compile 'thresh(2,pk(@0),pk(@1),pk(@2))' --context segwitv0
# → wsh(multi(2,@0,@1,@2))
```

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

MIT License — see [`../../LICENSE`](../../LICENSE).
