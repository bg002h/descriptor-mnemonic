# SPEC — md-codec v0.16.0 (library-only) + new `md-cli` crate

Date: 2026-05-03
Status: design approved by user; brainstorm-stage and spec-stage architect reviews complete; user revision (preserve `json` feature on md-cli) applied; awaiting user spec review before plan-writing
Crates: `md-codec` v0.15.2 → v0.16.0 (breaking — `[[bin]]` and CLI features removed)
        `md-cli` v0.1.0 (new crate; ships the `md` binary, inherits the `json` feature flag)

## Goal

Move the `md` binary out of `md-codec` into a new in-repo crate `md-cli`. Pure
code-move refactor: no new functionality, no wire-format change, no behavior
change. `md-codec` becomes a library-only crate; `md-cli` becomes the user-facing
binary.

This is **step 1 of a staged plan (B → C)** to eventually host the binary in a
third sibling repo where it depends on `md-codec`, `mk-codec`, and a future
codex32 (BIP-93 seed shares) crate as published artifacts. Step 1 is in-repo
extraction; the C-state transplant is deferred to its own brainstorm/spec/plan
cycle once the API has stabilized through real cross-crate use.

## Non-goals (out of scope for this step)

- mk-codec dependency, HRP detection, or input-routing scaffolding.
- codex32 (BIP-93 seed shares) crate extraction or dependency. (Today's
  `crates/md-codec/src/codex32.rs` is the BCH-layer adapter for md1 — unchanged.)
- Public-API stability commitments on `md-codec` (no `#[non_exhaustive]` audit,
  no `cargo public-api` baseline). Pre-1.0 churn allowed.
- Transplant of `md-cli` to a third sibling repo. Deferred to the C-state cycle.
- New CLI commands, output format changes, or argument ergonomics.
- Renaming the `md` binary. The user-facing command stays `md`.
- Any change to `crates/md-codec/src/codex32.rs` (the BCH-layer adapter,
  confusingly named relative to BIP-93 codex32; rename not in scope).
- Documentation rewrites beyond Cargo.toml `description`/`categories` field
  updates (deferred to FOLLOWUPS).

## End state

```
descriptor-mnemonic/
├── Cargo.toml                  # workspace; members = ["crates/md-codec", "crates/md-cli"]
├── crates/
│   ├── md-codec/               # library only, v0.16.0
│   │   ├── Cargo.toml          # no [[bin]]; default = []; no features defined
│   │   ├── src/                # unchanged: lib.rs + 19 modules
│   │   └── tests/              # library tests + reference vectors corpus; CLI integration tests removed
│   └── md-cli/                 # binary crate, v0.1.0 (new)
│       ├── Cargo.toml          # [[bin]] name = "md", path = "src/main.rs"
│       ├── src/                # flat — no bin/md/ nesting
│       │   ├── main.rs         # carries #![allow(missing_docs)]
│       │   ├── cmd/            # encode, decode, verify, inspect, bytecode, vectors, compile, address
│       │   ├── compile.rs      # cfg(feature = "cli-compiler")
│       │   ├── error.rs        # CliError
│       │   ├── format/         # text, json
│       │   └── parse/          # keys, template, path
│       └── tests/              # CLI integration tests + snapshots
```

## Manifest changes

### `crates/md-codec/Cargo.toml`

Remove:

- `[[bin]] name = "md"` stanza.
- `default = ["cli", "json"]`.
- `[features]` block entirely (`cli`, `cli-compiler`, `json` all gone).
- Optional deps: `clap`, `anyhow`, `miniscript`, `regex`, `serde`, `serde_json`.
- Dev-deps used only by CLI integration tests: `assert_cmd`, `predicates`,
  `tempfile`. `insta` is also dropped **iff Phase 0 confirms** it is used only
  by tests that move to md-cli; if any retained lib test still uses it,
  `insta` stays in md-codec's dev-deps. Phase 0 produces a definitive verdict;
  no FOLLOWUPS deferral.

Result:

```toml
[package]
name = "md-codec"
version = "0.16.0"
# ... (unchanged metadata; description and categories field updates deferred to FOLLOWUPS)

[lib]
name = "md_codec"

[dependencies]
bitcoin = "0.32"
thiserror = "2.0"
bip39 = "2.2.2"

[dev-dependencies]
# Phase 0 produces this list. If any retained lib test imports `insta`, it
# stays. Otherwise dev-dependencies is empty.
```

No features defined. `default = []` is implicit when the `[features]` block is absent.

### `crates/md-cli/Cargo.toml` (new)

```toml
[package]
name = "md-cli"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "CLI for the Mnemonic Descriptor (MD) engravable BIP 388 wallet policy backup format"
keywords = ["bitcoin", "bip388", "wallet", "descriptor", "bech32"]
categories = ["cryptography::cryptocurrencies", "command-line-utilities"]

[lints]
workspace = true

[[bin]]
name = "md"
path = "src/main.rs"

[features]
default = ["json"]
json = ["dep:serde", "dep:serde_json"]
cli-compiler = ["miniscript/compiler"]

[dependencies]
md-codec = { path = "../md-codec" }
clap = { version = "4.5", features = ["derive"] }
anyhow = "1.0"
regex = "1.10"
bitcoin = "0.32"
bip39 = "2.2.2"
serde = { version = "1.0", features = ["derive"], optional = true }
serde_json = { version = "1.0", optional = true }
miniscript = { workspace = true }

[dev-dependencies]
assert_cmd = "2.0"
predicates = "3.1"
insta = { version = "1.40", features = ["json"] }
tempfile = "3.13"
```

**Notes:**

- **`clap`/`anyhow`/`regex`** are unconditional — `md-cli`'s reason for
  existing is to *be* the CLI; arg parsing always compiles in.
- **`json` feature carries over from md-codec** (per user direction; future
  md-cli iterations will grow additional JSON-related capabilities atop this
  flag). `default = ["json"]` preserves today's install-time behavior:
  `cargo install md-cli` matches today's `cargo install md-codec` for users on
  default features. `cargo install md-cli --no-default-features` builds the
  binary without `serde`/`serde_json`, mirroring today's
  `cargo install md-codec --no-default-features --features cli`.
- **`serde`/`serde_json` are optional**, gated behind `json`. The bin's
  existing `#[cfg(feature = "json")]` gates (in `format/json.rs` consumers,
  `cmd/vectors.rs`, etc.) carry over unchanged.
- **`miniscript`** is `workspace = true` (unconditional). The bin source
  uses `miniscript::*` in `parse/template.rs` for descriptor parsing across
  every command path (encode, decode, verify, address, ...), not just the
  policy compiler. The `cli-compiler` feature only adds the `compiler`
  feature flag to miniscript (gating the policy → miniscript compilation
  in `compile.rs`). This matches today's behavior — old md-codec had
  `default = ["cli", "json"]` with `cli = ["dep:miniscript", ...]`, so
  miniscript was always present in the default install.
- **`bitcoin` and `bip39`** are direct deps of CLI code (e.g.
  `bitcoin::bip32::DerivationPath` in `parse/template.rs`).

**Feature mapping from today's md-codec:**

| Today (md-codec)                                   | After (md-cli)             |
|----------------------------------------------------|----------------------------|
| `cli` (gates the binary itself)                    | (subsumed — md-cli *is* the CLI; `clap`/`anyhow`/`regex` unconditional) |
| `json` (gates `--json` output via serde)           | `json` (preserved verbatim)|
| `cli-compiler` (gates `compile`/`encode --from-policy`) | `cli-compiler` (preserved verbatim) |
| `default = ["cli", "json"]`                        | `default = ["json"]`       |

### `Cargo.toml` (workspace)

```toml
[workspace]
resolver = "3"
members = ["crates/md-codec", "crates/md-cli"]
```

(Workspace deps unchanged — `miniscript = { version = "13.0.0", default-features = false, features = ["std"] }` stays.)

## Source-tree changes

### Move

- `git mv crates/md-codec/src/bin/md/main.rs       crates/md-cli/src/main.rs`
- `git mv crates/md-codec/src/bin/md/error.rs      crates/md-cli/src/error.rs`
- `git mv crates/md-codec/src/bin/md/compile.rs    crates/md-cli/src/compile.rs`
- `git mv crates/md-codec/src/bin/md/cmd/          crates/md-cli/src/cmd/`
- `git mv crates/md-codec/src/bin/md/format/       crates/md-cli/src/format/`
- `git mv crates/md-codec/src/bin/md/parse/        crates/md-cli/src/parse/`

The `bin/md/` nesting flattens; `crates/md-cli/src/main.rs` is the binary entry.

### Edit `cmd/vectors.rs`

One edit. Replace the cross-tree `#[path]` reach with a portable `include!`:

```rust
// Before (in crates/md-codec/src/bin/md/cmd/vectors.rs):
#[path = "../../../../tests/vectors/manifest.rs"]
mod manifest;
use manifest::MANIFEST;

// After (in crates/md-cli/src/cmd/vectors.rs):
mod manifest {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../md-codec/tests/vectors/manifest.rs"));
}
use manifest::MANIFEST;
```

The manifest itself stays at `crates/md-codec/tests/vectors/manifest.rs` —
it's part of the format's reference corpus and belongs with the codec.

The existing `#[cfg(feature = "json")]` guard on the `.descriptor.json`
emission block (around line 41) is **preserved** — md-cli inherits the
`json` feature flag, so the gate continues to mean what it meant before:
"emit JSON when serde is available." Default install behavior unchanged
(`default = ["json"]`).

The runtime default `out_dir` of `"crates/md-codec/tests/vectors"` (currently a
CWD-relative path) is **left unchanged** in this refactor; it was already a CWD
assumption pre-refactor. A FOLLOWUPS entry will track making it configurable.

### Carry `#![allow(missing_docs)]`

`main.rs` line 1 today is `#![allow(missing_docs)]`. The workspace lint
`missing_docs = "warn"` applies to all members; the suppression must move with
the file. Verified post-Phase-2 by `cargo check -p md-cli --all-targets`
returning zero warnings.

## Test handling

**Phase 0 confirms this list by re-reading each file; the architect spot-checked
the two ambiguous cases (`smoke.rs` and `template_roundtrip.rs`) and the
classifications below reflect ground truth.**

**Move to `md-cli/tests/`** (CLI integration tests; use `assert_cmd`):

- `cmd_address.rs`, `cmd_address_json.rs`, `cmd_bytecode.rs`, `cmd_compile.rs`,
  `cmd_decode.rs`, `cmd_encode.rs`, `cmd_inspect.rs`, `cmd_verify.rs`
- `compile.rs`, `exit_codes.rs`, `help_examples.rs`, `json_snapshots.rs`,
  `scaffold.rs`
- `template_roundtrip.rs` (uses `cargo_bin("md")` on line 10)
- `vector_corpus.rs` (uses `cargo_bin("md")` on line 10 to regenerate the
  corpus and `diff -r` against the committed tree — Phase 0 audit
  reclassified this from lib-only to CLI; corpus stays in md-codec but
  the test reaches it via `CARGO_MANIFEST_DIR/../md-codec/tests/vectors`)

**Stay in `md-codec/tests/`** (library tests; no `assert_cmd` / `cargo_bin`):

- `address_derivation.rs`, `chunking.rs`, `wallet_policy.rs`,
  `forward_compat.rs`, `smoke.rs` (Phase 0 audit confirmed: pure library
  calls, no `assert_cmd`)

**Move with the bin:**

- `crates/md-codec/tests/snapshots/` → `crates/md-cli/tests/snapshots/`

**Stay with md-codec:**

- `crates/md-codec/tests/vectors/` (the reference corpus)

After the move, no test in `md-codec/tests/` calls `Command::cargo_bin("md")`,
and the `md` binary is unambiguously defined by `md-cli` only.

## Public-API surface on md-codec

Phase 0 audit confirms zero new public items needed. The bin's `md_codec::`
imports already resolve to publicly-accessible items.

These items are reachable as flat re-exports at the crate root (per
`pub use ...` in `lib.rs`):

- `decode::decode_md1_string`
- `chunk::{reassemble, ChunkHeader, derive_chunk_set_id, split}`
- `encode::{Descriptor, encode_md1_string, encode_payload}`
- `header::Header`
- `identity::{compute_md1_encoding_id, compute_wallet_descriptor_template_id,
  compute_wallet_policy_id, Md1EncodingId, WalletDescriptorTemplateId,
  WalletPolicyId}`
- `tag::Tag`
- `tlv::TlvSection`
- `origin_path::{OriginPath, PathComponent, PathDecl, PathDeclPaths}`

These items are reachable via their `pub mod` paths (no flat re-export, but
the bin code uses the module-path form so this is fine):

- `tree::{Body, Node}` — accessible as `md_codec::tree::Body`/`Node`
- `use_site_path::{Alternative, UseSitePath}` — accessible as
  `md_codec::use_site_path::Alternative`/`UseSitePath`

If Phase 0 finds any private reach, the offending item is promoted to `pub` in
Phase 0's commit. Promotion is additive and does not on its own require the
v0.16.0 bump (a v0.15.3 release would suffice). The breaking change driving
v0.16.0 is the removal of `[[bin]]` and the `cli`/`cli-compiler`/`json` features.

## `serde` / `json` policy

**md-codec (library):** library types carry no `serde` derives by design at
this stage. The pre-1.0 format spec is still evolving, and exposing `serde`
at the library layer would ossify field names and shapes that the wire format
may yet revise. The `json` feature is removed from `md-codec` entirely (no
optional deps, no feature flag). If a downstream library consumer eventually
wants `serde` on the public types, that's a separate discussion (likely
deferred until v1.0).

**md-cli (binary):** the `json` feature flag survives, gating `serde` /
`serde_json` and the `--json` CLI output paths. `default = ["json"]` matches
today's md-codec default; future md-cli iterations will grow additional
JSON-related capabilities atop this flag (per user direction). Users can
build a serde-free binary via `cargo install md-cli --no-default-features`,
mirroring today's `cargo install md-codec --no-default-features --features
cli`.

## Phase plan

Five phases, one feature branch, one PR. Each phase is its own commit. Each
phase commit gets its own iterative-agent review per repo convention; reports
persist to `design/agent-reports/`. Critical+important fixed inline; low/nit
appended to `design/FOLLOWUPS.md` under the next-patch tier.

### Phase 0 — API audit + test classification

No code change. Two outputs, both pinned to the PR:

1. **API audit:** grep every `md_codec::` import in `src/bin/md/`; verify each
   resolves to a `pub` item. Promote anything missing in this phase's commit
   (zero items expected). If the audit finds promotion candidates, document
   them.
2. **Test classification:** read each of the 21 files in `md-codec/tests/`;
   classify as "moves to md-cli" / "stays in md-codec" / "split" with a
   one-line reason per file. `template_roundtrip.rs` and `smoke.rs` get
   special attention.

### Phase 1 — Scaffold `md-cli`

- Add `crates/md-cli` to workspace `members`.
- Create `crates/md-cli/Cargo.toml` exactly as specified above — full
  dependencies, full features, full metadata. Phase 1's `main.rs` will not use
  most of those deps; that's fine. Cargo will warn about unused deps only at
  doc-build time, which Phase 1 doesn't run.
- Create `crates/md-cli/src/main.rs` with `fn main() { unimplemented!() }` and
  the `#![allow(missing_docs)]` suppression.
- Create `crates/md-cli/tests/smoke.rs` with one assertion: `md encode
  wpkh(@0/<0;1>/*)` exits 0 and outputs the known-good phrase string.
- **Smoke test fails** (binary panics at `unimplemented!()`). TDD invariant
  established.

### Phase 2 — Atomic source-move + manifest swap

Single commit. Touches both crates' manifests and moves all source files in
one go to avoid a broken intermediate build.

- `git mv` the source tree per "Source-tree changes" above.
- Replace `cmd/vectors.rs`'s `#[path]` with the portable `include!` form.
- `md-codec/Cargo.toml`: drop `[[bin]]`, drop the `[features]` block, drop the
  CLI optional deps (`clap`, `anyhow`, `miniscript`, `regex`, `serde`,
  `serde_json`), drop CLI-only dev-deps (`assert_cmd`, `predicates`,
  `tempfile`).
- `md-cli/Cargo.toml`: replace the manifest stub with the full one above.
- Verify by:
  - `cargo build --workspace` succeeds.
  - `cargo build --workspace --all-features` succeeds (exercises
    `cli-compiler`).
  - `cargo check -p md-cli --all-targets` returns zero warnings.
  - The Phase-1 smoke test passes.

### Phase 3 — Move CLI tests + snapshot fixtures

- `git mv` the test files identified in Phase 0.
- `git mv crates/md-codec/tests/snapshots/ crates/md-cli/tests/snapshots/`.
- No source edits expected; `Command::cargo_bin("md")` resolves uniquely
  because md-codec no longer defines the `md` binary (Phase 2).
- Verify by `cargo test --workspace` passing on the same set of tests as
  pre-PR.

### Phase 4 — Versions + CHANGELOG

- `crates/md-codec/Cargo.toml`: `version = "0.16.0"`.
- `crates/md-cli/Cargo.toml`: `version = "0.1.0"` (already set in Phase 1; no
  change).
- `CHANGELOG.md` (repo root) — add a per-crate section for each release. The
  repo already uses a single root `CHANGELOG.md` (no per-crate CHANGELOG
  files), so md-cli does not get its own CHANGELOG file.
  - md-codec entry: `## md-codec [0.16.0]` block stating "Library-only
    release. The `md` binary and `cli`/`cli-compiler`/`json` features have
    been extracted to a new `md-cli` crate. Breaking change: `cargo install
    md-codec` no longer ships an `md` binary — install `md-cli` instead. No
    wire-format change; no library API removal."
  - md-cli entry: `## md-cli [0.1.0]` initial-release block. Note that the
    `json` and `cli-compiler` feature flags carry over from md-codec (`json`
    in default features; `cli-compiler` opt-in). Link forward to the
    md-codec 0.16.0 entry as the source of the moved code.

## Risks & mitigation

- **Build broken between phases.** Mitigated by Phase 2's atomicity. Phases 1,
  3, 4 are independently buildable.
- **Hidden private-internals reach.** Phase 0 catches before any move. Existing
  evidence: zero items expected.
- **Behavior drift in the moved binary.** Phase 1's smoke test + Phase 3's
  moved CLI test suite pin behavior. Acceptance: full test suite passes pre-PR
  and post-PR with identical output.
- **Workspace lint regression.** `#![allow(missing_docs)]` carry verified by
  `cargo check -p md-cli --all-targets`.
- **Cross-tree `include!` portability.** `concat!(env!("CARGO_MANIFEST_DIR"),
  "/...")` is portable; `/` resolves correctly on Windows in Rust path literals.

## Rollback

Single feature branch. Rollback is `git checkout main` + delete the branch.
Pure refactor — zero behavior change, zero wire-format change. v0.16.0 is
released *after* merge. If the merged-but-unreleased state proves wrong, the
version bump reverts and we patch forward; md-codec v0.15.2 is the last
shipped artifact and is unaffected.

## Deferred to FOLLOWUPS

These are confirmed low/nit items pulled from architect review; they will be
appended to `design/FOLLOWUPS.md` under the next-patch-release tier, not
addressed in this PR:

- `crates/md-codec/Cargo.toml` `description` field still says "with `md` CLI"
  — update to library-only phrasing.
- `crates/md-codec/Cargo.toml` `categories` includes
  `"command-line-utilities"` — should be md-cli-only.
- `cmd/vectors.rs` runtime default output dir
  (`"crates/md-codec/tests/vectors"`) is a CWD-relative assumption; pre-existing,
  but worth FOLLOWUPS to make configurable or document explicitly.
- **C-state precondition:** before transplanting `md-cli` to a third sibling
  repo (or publishing it to crates.io), `md-cli/Cargo.toml`'s `md-codec` dep
  must gain a `version` field — Cargo rejects path-only deps at `cargo
  publish` time. Form: `md-codec = { path = "../md-codec", version = "0.16.0"
  }`. Path-only is fine for in-repo development now; surface this at the C-
  state cycle's brainstorm.

## Acceptance criteria

The PR is mergeable when all of the following hold:

1. `cargo build --workspace` succeeds.
2. `cargo build --workspace --all-features` succeeds.
3. `cargo test --workspace` passes the same number of tests as on `main`
   pre-PR (modulo any tests deliberately split during Phase 0).
4. `cargo install --path crates/md-cli` produces an `md` binary whose
   subcommand list, `--help` output structure, exit codes, and golden
   snapshots match the pre-PR `md` binary built from `crates/md-codec`.
   `md --version` output **differs by design** (now reports `md-cli`'s
   version `0.1.0`, not `md-codec`'s; clap derives `version` from the
   crate-level `CARGO_PKG_VERSION`).
5. `cargo check -p md-cli --all-targets` and `cargo check -p md-codec
   --all-targets` both return zero warnings.
6. `crates/md-codec/Cargo.toml` has no `[[bin]]`, no `[features]` block, and
   no CLI-only deps.
7. CHANGELOG entries land for both crates.
8. Per-phase agent-review reports are persisted under `design/agent-reports/`.
9. FOLLOWUPS entries are filed for the deferred items.
