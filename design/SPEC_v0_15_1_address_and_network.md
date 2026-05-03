# SPEC v0.15.1 — `md address` + `--network`

Date: 2026-05-03
Status: design proposed; awaiting iterative review
Crate: `md-codec` (patch release; library API unchanged; wire format unchanged)
Anchored to: brainstormed scope captured in plan-mode artifact (post-review)

## Goal

Two additions, shipped together:

1. **`md address`** subcommand — derive bitcoin addresses from a wallet-policy
   mode descriptor. Wraps the existing `Descriptor::derive_address(chain, index, network)`
   library API at `crates/md-codec/src/derive.rs:250` (no library changes).
2. **`--network mainnet|testnet|signet|regtest`** flag (default `mainnet`)
   on every CLI surface that consumes or produces real xpubs:
   `md encode`, `md verify`, `md address`. Decode/inspect/bytecode stay
   network-agnostic — the wire payload carries 65-byte chain-code‖pubkey
   only, no network bit.

Patch-release (additive). No source changes required for downstream library
consumers. v0.15.x wire format unchanged.

## Non-goals

- xpriv / mnemonic input. `md address` only consumes already-derived xpubs.
- `--account` flag. Account level is baked into the supplied xpub.
- `--network` on decode/inspect/bytecode (no semantic effect).
- Re-deriving from partial-keys descriptors. `derive_address` requires every
  `@N` to have a TLV pubkey; partial-keys is not implemented in the library.
- Wire format changes.
- Address validation / `--require-network` defensive recheck.

## Subcommand surface (delta from v0.15.0)

```
md encode  ... [--network <NET>] [--key @i=<XPUB|TPUB>]...
md verify  ... [--network <NET>] [--key @i=<XPUB|TPUB>]...
md address <STRING>...                                     # mode (a)
md address --template <T>                                  # mode (b)
           [--key @i=<XPUB|TPUB>]... [--fingerprint @i=<HEX>]...
           [--network <NET>]
           [--chain N | --change] [--index N] [--count K]
           [--json]
```

`<NET>` is one of `mainnet|testnet|signet|regtest`. Default `mainnet`.

### `md address` arg semantics

| Arg | Default | Notes |
|---|---|---|
| `<STRING>...` (positional) | — | One or more md1 phrases. Mutually exclusive with `--template` at clap level (clap `ArgGroup::required(true)`). |
| `--template <T>` | — | BIP 388 template; same shape as `md encode`. Requires at least one `--key`. Mutually exclusive with positional strings. |
| `--key @i=<XPUB\|TPUB>` (repeatable) | — | Concrete xpubs that get baked into `tlv.pubkeys`. Required iff `--template` given. Validated against `--network`. |
| `--fingerprint @i=<HEX>` (repeatable) | — | Optional master-key fingerprints for `tlv.fingerprints`. Requires `--template`. |
| `--network <NET>` | `mainnet` | Routes xpub-version validation in `parse_key` AND chooses the address HRP/version in the final `Address::p2*(_, network)` call. |
| `--chain N` | `0` | Multipath alternative selector. For canonical `<0;1>/*`: 0 = receive, 1 = change. Conflicts with `--change` at clap level. |
| `--change` | false | Sugar for `--chain 1`. |
| `--index N` | `0` | **Starting** index along the wildcard. With `--count > 1`, addresses are derived for indices `[index, index + count)`. |
| `--count K` | `1` | Number of consecutive addresses to derive. clap `value_parser = clap::value_parser!(u32).range(1..=1000)`. |
| `--json` | false | Emit JSON output (schema `md-cli/1`). |

### Default text output (one per line)

```
$ md address $PHRASE --change --index 0 --count 3
bc1q...                                  # change/0
bc1q...                                  # change/1
bc1q...                                  # change/2
```

### JSON output

```json
{
  "schema": "md-cli/1",
  "network": "mainnet",
  "addresses": [
    { "chain": 1, "index": 0, "address": "bc1q..." },
    { "chain": 1, "index": 1, "address": "bc1q..." },
    { "chain": 1, "index": 2, "address": "bc1q..." }
  ]
}
```

The `network` field is the same string the user passed to `--network` (or
the default `"mainnet"`). It mirrors the CLI flag vocabulary, NOT
`bitcoin::Network`'s `Display` (which is `"bitcoin"` / `"testnet"` /
`"signet"` / `"regtest"` — the `bitcoin` vs `mainnet` discrepancy would
confuse JSON consumers).

### `encode --json` gains a `network` field

```json
{
  "schema": "md-cli/1",
  "network": "testnet",        // NEW; always present, defaults to "mainnet"
  "phrase": "md1q..."
}
```

Always-present (not iff non-default) for downstream-consumer simplicity. The
phrase doesn't carry network on the wire, but `encode --json | jq .network`
gives a script the network the user originally targeted, avoiding silent
mainnet-fallback when piping into `address --json`. `verify` does not emit
JSON; no change there.

## Exit codes (no change to existing semantics)

- `0` — success.
- `1` — runtime error (codec error, derivation error, mismatch).
- `2` — usage error (clap rejection, `CliError::BadArg`).

New `CliError::BadArg` triggers introduced by `md address` — two distinct
checks at two distinct layers:

- **Runtime**, after descriptor construction: `"address requires wallet-policy mode (Pubkeys TLV); supply --key @i=XPUB or use a wallet-policy-mode phrase"`. Fires when the constructed descriptor has empty/absent `Pubkeys` TLV — typically because the user supplied a phrase that was encoded in template-only mode.
- **Runtime**, after clap parse but before descriptor construction: `"--key @i=<XPUB> required when --template is supplied"`. Fires when `--template` is given but `--key` Vec is empty. (clap `requires` only enforces presence of the arg, not non-empty repetition; this is an explicit runtime check.)
- **Clap-level** (exit 2 with clap's standard messages): `--chain` + `--change` together (`conflicts_with`); `--count` outside `1..=1000` (`value_parser` range); positional `<STRING>...` together with `--template` (`ArgGroup`); `--key`/`--fingerprint` without `--template` (`requires`).

## Implementation surface

### File-level changes

- **Modify** `crates/md-codec/src/bin/md/parse/keys.rs`:
  - Add `pub(crate) const TESTNET_XPUB_VERSION: [u8; 4] = [0x04, 0x35, 0x87, 0xCF];` (per BIP 32; same constant for testnet/signet/regtest).
  - Change `parse_key(arg: &str, ctx: ScriptCtx) → parse_key(arg: &str, ctx: ScriptCtx, network: bitcoin::Network)`. Branch the version-byte check: `Network::Bitcoin` → mainnet bytes; everything else → testnet bytes. Update existing tests to pass `Network::Bitcoin`. Add `accepts_tpub_under_testnet`, `rejects_xpub_under_testnet`, `rejects_tpub_under_mainnet`.
- **Modify** `crates/md-codec/src/bin/md/parse/template.rs`:
  - `parse_template` signature does **not** change. The synthetic xpub generator (`synthetic_xpub_for`) stays mainnet-only — it's miniscript-parseable scaffold, never emitted, and miniscript ignores xpub version bytes for curve membership. Network only flows through the call sites' `parse_key` invocations, which `parse_template` does not perform itself (callers do).
- **Modify** `crates/md-codec/src/bin/md/cmd/encode.rs`:
  - `EncodeArgs` gains `pub network: bitcoin::Network` and `pub network_str: &'static str`.
  - Pass `args.network` into `parse_key` calls.
  - JSON output gains `"network": args.network_str` at top level (always present). **Critical:** must serialize the CLI vocabulary string (`"mainnet"` for `bitcoin::Network::Bitcoin`), NOT `bitcoin::Network`'s `Display` (which emits `"bitcoin"`). The simplest implementation routes the kebab-cased `CliNetwork` variant string through `EncodeArgs::network_str`; an alternative is a `cli_network_str(bitcoin::Network) -> &'static str` helper in `main.rs`.
- **Modify** `crates/md-codec/src/bin/md/cmd/verify.rs`:
  - `VerifyArgs` gains `pub network: bitcoin::Network`. Pass through to `parse_key`. No JSON output (verify reports via exit code).
- **Create** `crates/md-codec/src/bin/md/cmd/address.rs`. Public surface:
  ```rust
  pub struct AddressArgs<'a> {
      pub phrases: &'a [String],
      pub template: Option<&'a str>,
      pub keys: &'a [String],
      pub fingerprints: &'a [String],
      pub network: bitcoin::Network,
      pub network_str: &'static str,   // for JSON: kebab-cased CliNetwork name
      pub chain: u32,
      pub index: u32,
      pub count: u32,
      pub json: bool,
  }
  pub fn run(args: AddressArgs<'_>) -> Result<(), CliError>;
  ```
  - **Construction pattern**: identical to `EncodeArgs<'a>` — `AddressArgs<'a>` is constructed inline in `main.rs`'s dispatch arm, borrowing from the clap-parsed `Args` fields (no owned-field variant). Mirror the encode dispatch site verbatim.
  - Build `Descriptor` from either input mode (mirroring `cmd::decode::run` for phrases and `cmd::encode::run`'s template+key path).
  - Reject if `!descriptor.is_wallet_policy()` with the runtime wallet-policy `BadArg` message (see §Exit codes for distinct text).
  - Loop `index..(index+count)`, calling `descriptor.derive_address(args.chain, idx, args.network)?.assume_checked()`. Print one per line (text mode) or accumulate into JSON.
- **Modify** `crates/md-codec/src/bin/md/cmd/mod.rs`: `pub mod address;`.
- **Modify** `crates/md-codec/src/bin/md/main.rs`:
  - Define `#[derive(Copy, Clone, Debug, clap::ValueEnum)] enum CliNetwork { Mainnet, Testnet, Signet, Regtest }` with `impl From<CliNetwork> for bitcoin::Network` (Mainnet → `Network::Bitcoin`, others → matching variants).
  - Add `network: CliNetwork` (`#[arg(long, value_enum, default_value_t = CliNetwork::Mainnet)]`) to `Encode`, `Verify`, `Address` variants.
  - Add `Address` variant with all args from the table above. Use `clap::ArgGroup::new("input").required(true).args(["phrases", "template"])` to enforce one-of-two.
  - Dispatch arm: collapse `change → chain = 1` once, then call `cmd::address::run`.
- **Create** `crates/md-codec/tests/cmd_address.rs` (golden vectors at the CLI layer). See Testing.
- **Modify** `crates/md-codec/Cargo.toml`: version bump `0.15.0 → 0.15.1`. (Workspace `Cargo.toml` carries no `version` field; the crate manifest is the only place to bump.)
- **Modify** `CHANGELOG.md`: new `## [0.15.1]` section.
- **Modify** `MIGRATION.md`: short `v0.15.0 → v0.15.1` note (additive).
- **Modify** `crates/md-codec/README.md`: add `md address` row to the CLI subcommand table; one-paragraph quickstart with mainnet + testnet examples.
- **Modify** `docs/json-schema-v1.md`: add `address --json` section; add `network` field to `encode --json` section.

### Reusable APIs (verified by Explore)

- `Descriptor::derive_address(chain: u32, index: u32, network: Network) -> Result<Address<NetworkUnchecked>, Error>` at `crates/md-codec/src/derive.rs:250`. Caller uses `.assume_checked()` (matches existing pattern at `tests/address_derivation.rs:87`).
- `Descriptor::is_wallet_policy() -> bool` at `crates/md-codec/src/encode.rs:47`.
- `decode_md1_string(s) / reassemble(refs)` fork pattern at `cmd/decode.rs:7-12` and `cmd/inspect.rs:8-13`.
- `parse_template(template, &parsed_keys, &parsed_fps)` (defined in `parse/template.rs`; called from `cmd/encode.rs:24`) — populates `tlv.pubkeys` when `parsed_keys` non-empty.
- JSON `SCHEMA = "md-cli/1"` at `format/json.rs:6`.

## Network handling — exhaustive table

| Surface | Reads `--network`? | Where it matters |
|---|---|---|
| `parse_key` (encode/verify/address) | yes | xpub-version validation: mainnet→xpub, others→tpub |
| `synthetic_xpub_for` (parse_template) | NO | Curve-membership only; mainnet-only scaffold is fine |
| `xpub_from_tlv_bytes` (derive.rs) | NO | Consumes chain code + pubkey; ignores network |
| `Descriptor::derive_address(_, _, network)` | yes | Final `Address::p2*(_, network)` HRP/version selection |
| Wire format (chunks, payload) | NO | 65-byte payload is curve material only |

The asymmetry: encoding-side network only matters for parsing the user's
xpub strings; the on-wire bytes are network-agnostic. Decoding-side
network only matters for address rendering. This is why `--network` lives
on encode/verify/address but not decode/inspect/bytecode.

## Testing

### Unit tests (in-module)

- `parse/keys.rs`:
  - Existing tests gain explicit `Network::Bitcoin` arg.
  - New: `accepts_tpub_under_testnet`, `rejects_xpub_under_testnet`, `rejects_tpub_under_mainnet`. Fixtures: derive a tpub via `bip32::Xpub::from_priv` with `Network::Testnet` from the abandon-mnemonic at `m/84'/1'/0'` (depth 3, single-sig context).

### Integration tests

- `tests/cmd_address.rs` (new):
  - **Mainnet wpkh receive 0 and 1**: encode the abandon-mnemonic at `m/84'/0'/0'` via the CLI (template + `--key`), then `md address` against the resulting phrase. Pin BIP 84's published vectors:
    - `bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu` (receive/0)
    - `bc1qnjg0jd8228aq7egyzacy8cys3knf9xvrerkf9g` (receive/1)
  - **Mainnet wpkh first change**: `--change --index 0` → `bc1q8c6fshw2dlwun7ekn9qwf37cu2rn755upcp6el` (BIP 84 published).
  - **Mainnet wpkh `--count 2 --index 0`**: assert exactly two newline-separated addresses matching receive/0 and receive/1.
  - **Testnet wpkh receive 0**: `--network testnet --key @0=<tpub at m/84'/1'/0'>`. BIP 84 publishes only mainnet vectors for the abandon-mnemonic; derive the expected testnet address in-test via the same `bip32::Xpub::from_priv` recipe used in `tests/address_derivation.rs:32-44` and assert the CLI matches that independently-derived value (an end-to-end CLI assertion against a trusted secondary path; mirrors how `address_derivation.rs` cross-checks multisig vectors).
  - **wsh-multi receive 0**: 2-of-2 wsh-multi at `m/48'/0'/0'/2'` from two abandon-derivative xpubs; cross-check via rust-bitcoin's descriptor derivation.
  - **`--count 1000` succeeds; `--count 1001` exits 2** (clap rejection).
  - **Template-only without `--key`**: exits 2 with `"requires wallet-policy mode"` substring.
  - **`--chain 5` on `<0;1>/*`**: exits 1 with stderr containing `"out of range"` (codec `ChainIndexOutOfRange`).
  - **`--change` + `--chain 1` together**: exits 2 (clap conflict).
- `tests/cmd_encode.rs`: extend with `encode_json_includes_network_field` (asserts `network` key always present, defaults to `"mainnet"`).
- `tests/cmd_address_json.rs` (new) **OR** extend `tests/json_snapshots.rs`: insta snapshots for at least `wpkh_mainnet_receive_0_to_2`, `wpkh_mainnet_change_0`, `wpkh_testnet_receive_0`, `wsh_2of2_mainnet_receive_0`. Same `with_settings!`/`assert_snapshot!` pattern as `tests/json_snapshots.rs:22-25`. Fixtures pinned in the same commit as the test code.

### Test count expectation

Baseline 340 (re-confirmed in v0.15.1 worktree at branch creation:
`cargo test --workspace --features cli,json,cli-compiler` → `Total ok: 340`).
v0.15.1 adds approximately:

- ~3 parse/keys network-routing unit tests
- ~7 cmd_address integration tests
- ~1 cmd_encode network-field integration test
- ~4 JSON snapshot tests (one per snapshot)

Expected total: ~355. The IMPLEMENTATION_PLAN will pin exact counts per
phase.

## Style and process

- TDD discipline: failing test first, then minimal impl, then commit.
- Every commit must pass `cargo test --workspace --features cli,json,cli-compiler` and `cargo clippy --workspace --features cli,json,cli-compiler --all-targets -- -D warnings`.
- Snapshot/golden vectors pinned in the same commit as the test code; `cargo test` must pass without `INSTA_UPDATE=always` or equivalent overrides.
- Per-phase code review per the standing rule (see plan-mode artifact); reports persist to `design/agent-reports/v0.15.1-phase-N-review.md`. Critical/important findings fixed in-session; nits to `design/FOLLOWUPS.md` under tier `v0.15.2`.

## Out-of-scope items captured for FOLLOWUPS

(Pre-emptively listed here so the SPEC reviewer doesn't surface them as
gaps; they're explicit deferrals to v0.15.2 or beyond.)

- Address validation subcommand (`md validate-address <ADDR>`). Defer.
- Address derivation from xpriv/mnemonic. Defer to a separate release.
- `bech32m` checksum validation flag. Already handled by rust-bitcoin's
  `Address` constructors.
- `--network` on `md decode/inspect/bytecode --json` to label output for
  scripted pipelines. Defer; current decision is "no semantic effect".
- Multi-network parsing (accept either xpub or tpub regardless of
  `--network`). Defer; explicit network selection is intentional.
