# SPEC v0.15 — `md` CLI

Date: 2026-05-03 (revised after architect review)
Status: design approved, plan pending
Crate: `md-codec` v0.15.0 (additive — no library API breakage)

## Goal

Restore command-line functionality for `md-codec`. Users can encode, decode,
verify, inspect, and compile BIP 388 wallet policies from a terminal. CLI is
the primary deliverable; library API is unchanged.

## Crate layout

CLI lives in `crates/md-codec` as `[[bin]] name = "md"`.

```
crates/md-codec/
├── Cargo.toml
└── src/
    ├── lib.rs                        # unchanged public API
    └── bin/md/
        ├── main.rs                   # clap dispatch
        ├── cmd/{encode,decode,verify,inspect,bytecode,vectors,compile}.rs
        ├── parse/{template,keys,path}.rs
        ├── format/{text,json}.rs
        └── compile.rs                # cfg(feature = "cli-compiler")
```

## Cargo features

| Feature        | Default | Pulls in                                  | Enables |
|----------------|---------|-------------------------------------------|---------|
| `cli`          | yes     | `clap`, `anyhow`, `miniscript = "13.0.0"` (crates.io, no `compiler` feature) | `md` binary; encode/decode/verify/inspect/bytecode/vectors |
| `json`         | yes     | `serde`, `serde_json`                     | `--json` on every read/write subcommand |
| `cli-compiler` | no      | `miniscript/compiler`                     | `compile` subcommand and `encode --from-policy` |

`miniscript` is pinned to crates.io `13.0.0` (released 2025-10-22, `bitcoin 0.32` compatible). The v0.11 apoelstra git fork is not needed: the template parser substitutes `@i` placeholders with synthetic xpubs before invoking miniscript, so only standard miniscript shape parsing is required.

Library-only consumers: `default-features = false`.
Cargo categories regain `command-line-utilities`.

## Subcommand surface

```
md encode <TEMPLATE> [--path <PATH>] [--key @i=<XPUB>]... [--fingerprint @i=<HEX>]...
                    [--force-chunked] [--force-long-code]
                    [--policy-id-fingerprint] [--json]
md encode --from-policy <EXPR> --context <tap|segwitv0> [...same opts...]   # cli-compiler
md decode <STRING>... [--json]
md verify <STRING>... --template <T> [--key @i=<XPUB>]... [--fingerprint @i=<HEX>]...
md inspect <STRING>... [--json]
md bytecode <STRING>... [--json]
md vectors [--out <DIR>]
md compile <EXPR> --context <tap|segwitv0> [--json]                         # cli-compiler
```

- `<TEMPLATE>` = BIP 388 template, e.g. `wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))`.
- `--key @i=<XPUB>` populates `tlv.pubkeys` (v0.13 wallet-policy-with-keys mode).
- `--fingerprint @i=<HEX>` populates fingerprints TLV. Independent of `--key`.
- `--path <PATH>` accepts a name (`bip48`), hex (`0x05`), or literal path
  (`m/48'/0'/0'/2'`).
- Unknown subcommand or arg → exit 2.
- `verify` re-encodes and compares; prints `OK` or `MISMATCH: <reason>`.
- `vectors` writes corpus to `--out <DIR>` (default `crates/md-codec/tests/vectors/`).

### Exit codes

| Code | Meaning |
|------|---------|
| 0    | success |
| 1    | user error (bad input, `verify` mismatch, decode failure) |
| 2    | clap usage error / internal error |

### Help-text contract

Every subcommand has an `after_long_help` block with one worked example: a
literal command line and the literal expected stdout. The harness in
`tests/help_examples.rs` runs each example and asserts byte-equal stdout, so
help cannot drift from behavior. `-h` shows usage + flags; `--help` adds the
example block.

## Template → Descriptor bridge (`parse/template.rs`)

v0.14's `Descriptor` is field-level only; CLI builds a parser on top of
rust-miniscript. **Two passes** are required because the per-`@i` multipath
suffix `<M;N>/*` cannot be recovered from the miniscript AST alone — the
synthetic-key substitution that lets miniscript parse the template also
strips that information.

### Data flow

```
raw template
   "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))"
        │
        ├─ Pass A — placeholder lexer (regex over raw string, before any substitution)
        │     extracts per-occurrence tuple:
        │       (index: u8, origin_path: Option<DerivationPath>,
        │        multipath_alts: Vec<Alternative>, wildcard_hardened: bool)
        │     → fills path_decl, use_site_path, tlv.use_site_path_overrides
        │
        ├─ Pass B — synthetic substitution + miniscript parse + AST walk
        │     each `@i/<M;N>/*` → deterministic synthetic xpub keyed by `i`
        │     → miniscript AST → walker fills tree, n
        │
        └─ join: synthetic-xpub identities ↔ @i indices ↔ leaf positions in `tree`
```

### Pass A — placeholder extraction

Custom tokenizer (regex + small validator), ~50 lines. Edge cases the
tokenizer must handle:

| Template fragment           | Meaning |
|-----------------------------|---------|
| `@i/*`                      | single-path, non-hardened wildcard, `multipath: None` |
| `@i/*'`                     | single-path, hardened wildcard, `wildcard_hardened: true` |
| `@i/<M;N>/*`                | multipath `{M, N}`, non-hardened wildcard |
| `@i/<M;N>/*'`               | multipath `{M, N}`, hardened wildcard |
| `<a;b;c>` arity > 2         | accepted (`UseSitePath.multipath: Option<Vec<Alternative>>` is variable-length per BIP 389) |
| same `@i` twice, identical `<M;N>` | accepted; one `use_site_path` entry |
| same `@i` twice, **different** `<M;N>` | **rejected as malformed** — divergence is per-placeholder, not per-leaf |
| different `@i`s with different `<M;N>` | divergent mode → **`@0`'s** multipath fills `use_site_path`; every `@i` (i>0) whose multipath differs from `@0`'s goes into `tlv.use_site_path_overrides[i]`. The encoder's `canonicalize_placeholder_indices` (`canonicalize.rs`) builds a permutation from first-occurrence order in the tree and atomically applies it to the tree, divergent path-decl, and every per-`@N` TLV map (including `use_site_path_overrides`); the wire output is always **canonical** for the policy regardless of the CLI's pre-canonicalization key-assignment order. |

### Pass B — AST walker

After substitution, the miniscript-parsed AST drives `Descriptor.tree`
construction:

- script context (`wsh`/`tr`/`sh`/`pkh`/`wpkh`) → `tree.tag` root
- inner expressions → `tree.body` recursively (one match arm per `Tag`)
- placeholder index recovered by inverting the synthetic-xpub map → fills `n` = max(i)+1 and per-leaf placeholder refs

### Key/fingerprint substitution (after both passes)

- `--key @i=<XPUB>` → `tlv.pubkeys`. See "Xpub parsing" below for validation.
- `--fingerprint @i=<HEX>` → `tlv.fingerprints`.

### Tag mapping

A single match expression in `parse/template.rs`, one arm per `Tag` variant.
Names line up with miniscript AST nodes. The plan enumerates each arm as a
TDD checkpoint.

### Xpub parsing (`parse/keys.rs`)

`--key @i=<XPUB>` flow:

1. Split `@i=XPUB` on `=`; strip optional leading `@`; parse `i` as `u8`.
2. Base58check-decode the xpub string. Reject on checksum failure.
3. Verify decoded length is exactly **78 bytes**.
4. Verify the 4-byte version field is `0x0488B21E` (mainnet xpub).
   **Mainnet only** in v0.15.0. Testnet `tpub` (`0x043587CF`) and regtest
   are rejected. (v0.16+ may add a `--network` flag.)
5. Verify depth matches the script context per BIP 388. The depth byte
   lives at xpub byte offset 4 (per BIP 32):
   - `wpkh`/`pkh` → depth 3 (account-level for single-sig)
   - `wsh`/`sh-wsh`/`tr` → depth 4 (account-level for multisig)
6. Slice bytes `[13..78]` → `[u8; 65]` payload (32-byte chain code ‖ 33-byte
   compressed pubkey). Push `(i, payload)` into `tlv.pubkeys`.

### Errors — `CliError` in the binary

The library `Error` enum is **not** `#[non_exhaustive]` (`error.rs:7`), so
adding a variant would be a semver break. The CLI defines its own error type
in the binary that wraps `md_codec::Error`:

```rust
// in src/bin/md/error.rs
pub enum CliError {
    Codec(md_codec::Error),
    TemplateParse(String),     // pass A and pass B failures
    BadXpub { i: u8, why: String },
    BadFingerprint { i: u8, why: String },
    Compile(String),           // cli-compiler only
    Mismatch(String),          // verify
    BadArg(String),            // residual user-input errors
}
```

`From<md_codec::Error> for CliError` for transparent `?` flow. Library
exports remain identical — no new variants, no `#[non_exhaustive]` change.

## JSON wrapper (`format/json.rs`)

Library stays serde-free. CLI defines shadow types with `#[derive(Serialize)]`
mirroring `Descriptor`, `PathDecl`, `UseSitePath`, `Node`, `TlvSection`,
`Md1EncodingId`, `WalletDescriptorTemplateId`, `WalletPolicyId`, `Header`,
`ChunkHeader`. `From<&md_codec::X> for JsonX` for each. No `Deserialize`.

### Schema rules

- `[u8; N]` and `Vec<u8>` → lowercase hex.
- `DerivationPath` → BIP-style string (`"m/48'/0'/0'/2'"`).
- Enums → adjacent-tagged: `{"tag": "...", "data": {...}}`.
- Identity hashes → `{"hex": "..."}`; `WalletPolicyId` adds `"fingerprint": "0x..."`.
- Every JSON output carries top-level `"schema": "md-cli/1"`.

### Per-subcommand outputs

- `encode --json` → `{"phrase": "...", "chunk_set_id": "0x...", "policy_id_fingerprint": "0x...?"}` (last field present iff `--policy-id-fingerprint` was passed).
- `decode --json` → full `Descriptor` shadow + identity hashes.
- `inspect --json` → full `Descriptor` shadow + identity hashes + chunk metadata + decoded TLV blocks.
- `bytecode --json` → labeled byte arrays per layout region.

Schema list lives in `docs/json-schema-v1.md` (one file, terse).

## Compiler integration (`compile.rs`, gated `cli-compiler`)

```rust
pub fn compile_policy_to_template(
    expr: &str,
    ctx: ScriptContext,
) -> Result<String, CompileError>;
```

Pipeline: parse `expr` as `miniscript::policy::concrete::Policy<String>` →
`.compile::<Ctx>()` → wrap as `wsh(...)` or `tr(...)` per `ctx` → return
template string. `String` key type passes `@i` placeholders through verbatim.

`md compile` prints the template (one line). `--json` →
`{"template": "...", "context": "tap|segwitv0"}`.

`md encode --from-policy` is a thin wrapper: compile, then dispatch to the
normal encode codepath with the resulting string. All other encode flags
compose unchanged.

```rust
// in src/bin/md/compile.rs
pub enum CompileError {
    Parse(String),       // miniscript policy parser error
    Compile(String),     // miniscript compiler error
    BadContext(String),  // unsupported / unrecognized script context
}
impl std::fmt::Display for CompileError { /* "<variant>: <msg>" */ }

impl From<CompileError> for CliError {
    fn from(e: CompileError) -> Self { CliError::Compile(e.to_string()) }
}
```

The `compile.rs` module is testable in isolation; the lossy
`Display`-stringification happens at the `CliError` boundary.

miniscript dep pinned to crates.io `13.0.0` in workspace `Cargo.toml`; the
`compiler` feature is enabled only when `cli-compiler` is on.

`--help` carries the caveat: rust-miniscript's compiler is heuristic for Tap
context; output may change across versions.

## `vectors` subcommand

Manifest: `crates/md-codec/tests/vectors/manifest.rs` — Rust source listing
`(name, template, keys?, fingerprints?, options)`.

Output (deterministic; sorted writes; LF line endings; no timestamps):

- `<name>.template`        — input template string
- `<name>.descriptor.json` — decoded `Descriptor` JSON (schema v1)
- `<name>.bytes.hex`       — payload bytes
- `<name>.phrase.txt`      — final codex32 phrase(s); one per line for chunked

Coverage: ~12 entries — single-sig (`wpkh`, `pkh`), multisig (`wsh(multi)`,
`wsh(sortedmulti)`), taproot (`tr(@0)`, `tr(@0,{...})`), divergent paths,
wallet-policy-with-keys, fingerprints-only, multi-chunk, long-code.

CI runs `md vectors --out tmp/`, then `diff -r tmp/ tests/vectors/`. Drift
fails the build. Integration tests under `tests/template_roundtrip.rs`
consume the same manifest directly; no shelling out.

## Testing

| Layer | File | What |
|-------|------|------|
| unit       | colocated under `src/bin/md/parse/*` and `format/*` | per-arm walker tests, arg parsers |
| JSON       | `tests/json_snapshots.rs` (insta) | one snapshot per (subcmd, fixture) |
| help drift | `tests/help_examples.rs` (assert_cmd) | example block matches stdout byte-equal |
| round-trip | `tests/template_roundtrip.rs` | template → encode → decode → re-string equality |
| corpus     | `tests/vector_corpus.rs` | `md vectors` matches committed corpus |
| compiler   | `tests/compile.rs` (cfg `cli-compiler`) | `(policy, ctx) → template` golden table |
| exit codes | `tests/exit_codes.rs` | spot-check per-subcommand codes |

TDD per repo convention: tests precede impl in each phase of the plan.

Dev-deps added: `assert_cmd`, `predicates`, `insta`, `tempfile` —
workspace-pinned.

## Docs and release

- **README**: new `## CLI` section near the top — install line, one-line
  per subcommand, pointer to `--help` for examples. Tight.
- **MIGRATION.md**: new section `## v0.14.x → v0.15.0` covering binary
  return, default features, `cli-compiler` opt-in, no library breakage.
- **CHANGELOG.md**: standard `0.15.0` entry — bullet per subcommand,
  deps disclosed.
- **No wire-format spec change**: behavior is documented in `--help` and
  this doc. Wire format is unchanged from v0.13/v0.14.
- **Versioning**: crate → `0.15.0`. Library API additive (no breakage).
- **Cross-repo**: no mk1 impact. No companion in `design/FOLLOWUPS.md`.

## Out of scope

- HSM / hardware-wallet integration.
- Network access (no `electrum`, no RPC).
- Address derivation in the CLI (library has it; CLI subcommand can land in
  a later release if demand emerges).
- A separate `gen_vectors` binary — folded into `md vectors`.
- Library serde support — JSON stays a CLI-only concern.
- `--seed` flag for chunk-set-id override — v0.11 had it; dropped in v0.15
  because v0.14's `derive_chunk_set_id` is fully deterministic from the
  payload and adding an override hook would require a library API addition
  to `chunk::split`. The `vectors` subcommand serves the deterministic-id
  use case.
- Testnet / regtest xpubs in `--key` — mainnet `xpub` (`0x0488B21E`) only
  in v0.15.0. A `--network` flag can land in a follow-up.
- Adding `#[non_exhaustive]` to library `Error` — would itself be a semver
  break. Deferred to a future major.

## Style

Per repo convention (see `MEMORY.md` → `feedback_terse_code.md`): short
doc-comments, no narrative module headers, no inline comments restating
behavior. clap `///` arg help is one short sentence each.
