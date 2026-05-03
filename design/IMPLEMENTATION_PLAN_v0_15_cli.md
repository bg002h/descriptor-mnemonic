# md-codec v0.15.0 — `md` CLI Restoration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Per-phase reviewer reports persist to `design/agent-reports/v0-15-phase-N-review.md`.

**Goal:** Restore command-line functionality stripped in v0.12.0. Ship a single `md` binary in v0.15.0 with seven subcommands (`encode`, `decode`, `verify`, `inspect`, `bytecode`, `vectors`, `compile`), JSON output on every read/write subcommand, and a help-text drift harness.

**Architecture:** Library API additive (no breaking changes). CLI lives in `crates/md-codec/src/bin/md/`. Template parsing is a two-pass pipeline: a regex tokenizer (Pass A) extracts per-`@i` multipath/origin-path tuples from the raw template before synthetic-key substitution; rust-miniscript parses the substituted template (Pass B) and the resulting AST is walked to populate `Descriptor`. CliError lives in the binary, wrapping `md_codec::Error`, so the library `Error` enum remains unchanged.

**Tech Stack:** Rust 2024, `bitcoin = "0.32"`, `miniscript = "13.0.0"` (crates.io, no `compiler` feature by default), `clap`, `anyhow`, `serde`, `serde_json`. Test deps: `assert_cmd`, `predicates`, `insta`, `tempfile`.

**Source of truth:** `design/SPEC_v0_15_cli.md` (commit `aa9b0ae`, two-pass architect review — clean verdict).

---

## Scope

### In-scope

- `md` binary with seven subcommands.
- Three Cargo features: `cli` (default), `json` (default), `cli-compiler` (opt-in).
- Template→Descriptor bridge (Pass A lexer + Pass B AST walker).
- Xpub validator (mainnet only, BIP 388 account-level depth checks).
- JSON shadow types in the binary (library remains serde-free).
- Six test harnesses: help-example drift, JSON snapshots, template round-trip, vector corpus diff, compiler determinism, exit codes.
- Docs: README CLI section, MIGRATION.md, CHANGELOG.md, docs/json-schema-v1.md.

### Out-of-scope (per spec §"Out of scope")

- HSM / hardware-wallet integration.
- Network access.
- Address derivation in the CLI.
- Separate `gen_vectors` binary.
- Library serde support.
- `--seed` flag (chunk-set-id override).
- Testnet/regtest xpubs in `--key`.
- Adding `#[non_exhaustive]` to library `Error`.

### File structure

| File | Action | Notes |
|---|---|---|
| `Cargo.toml` (workspace) | Modify | Add `miniscript = "13.0.0"` as workspace dep; bump categories |
| `crates/md-codec/Cargo.toml` | Modify | Bump `0.14.0` → `0.15.0`; add features `cli`/`json`/`cli-compiler`; add deps `clap`/`anyhow`/`miniscript`/`serde`/`serde_json` (gated); add `[[bin]] name = "md"`; add dev-deps `assert_cmd`/`predicates`/`insta`/`tempfile`; restore `command-line-utilities` category |
| `crates/md-codec/src/bin/md/main.rs` | Create | clap dispatch, exit-code translation |
| `crates/md-codec/src/bin/md/error.rs` | Create | `CliError` enum, `From<md_codec::Error>` |
| `crates/md-codec/src/bin/md/parse/mod.rs` | Create | re-export submodules |
| `crates/md-codec/src/bin/md/parse/keys.rs` | Create | `--key` xpub validator + `--fingerprint` parser |
| `crates/md-codec/src/bin/md/parse/path.rs` | Create | `--path` arg parser (name/hex/literal) |
| `crates/md-codec/src/bin/md/parse/template.rs` | Create | Two-pass template → Descriptor |
| `crates/md-codec/src/bin/md/format/mod.rs` | Create | re-export submodules |
| `crates/md-codec/src/bin/md/format/text.rs` | Create | Human-readable formatter |
| `crates/md-codec/src/bin/md/format/json.rs` | Create | Serde shadow types (cfg `json`) |
| `crates/md-codec/src/bin/md/cmd/mod.rs` | Create | re-export subcommand modules |
| `crates/md-codec/src/bin/md/cmd/encode.rs` | Create | `md encode` |
| `crates/md-codec/src/bin/md/cmd/decode.rs` | Create | `md decode` |
| `crates/md-codec/src/bin/md/cmd/verify.rs` | Create | `md verify` |
| `crates/md-codec/src/bin/md/cmd/inspect.rs` | Create | `md inspect` |
| `crates/md-codec/src/bin/md/cmd/bytecode.rs` | Create | `md bytecode` |
| `crates/md-codec/src/bin/md/cmd/vectors.rs` | Create | `md vectors` |
| `crates/md-codec/src/bin/md/cmd/compile.rs` | Create | `md compile` (cfg `cli-compiler`) |
| `crates/md-codec/src/bin/md/compile.rs` | Create | `compile_policy_to_template` (cfg `cli-compiler`) |
| `crates/md-codec/tests/vectors/manifest.rs` | Create | Vectors corpus source-of-truth |
| `crates/md-codec/tests/vectors/` | Create | Generated vector files (committed) |
| `crates/md-codec/tests/help_examples.rs` | Create | Drift harness |
| `crates/md-codec/tests/json_snapshots.rs` | Create | insta snapshots |
| `crates/md-codec/tests/template_roundtrip.rs` | Create | Round-trip from manifest |
| `crates/md-codec/tests/vector_corpus.rs` | Create | `md vectors` matches committed corpus |
| `crates/md-codec/tests/compile.rs` | Create | Compiler determinism (cfg `cli-compiler`) |
| `crates/md-codec/tests/exit_codes.rs` | Create | Per-subcommand exit code |
| `crates/md-codec/README.md` | Modify | Add `## CLI` section |
| `MIGRATION.md` | Modify | Add `## v0.14.x → v0.15.0` section |
| `CHANGELOG.md` | Modify | Add `[0.15.0]` entry |
| `docs/json-schema-v1.md` | Create | JSON schema field-by-field |

---

## Conventions used in this plan

- **TDD ordering:** Each task writes the failing test first, runs it to confirm failure, writes minimal impl, runs again to confirm pass, commits. The repo has a standing TDD convention — do not skip steps.
- **Run-test command (default):** `cargo test --workspace --features cli,json` unless a step says otherwise. Compiler-feature tests use `cargo test --workspace --features cli,json,cli-compiler`.
- **Commit message prefix:** `feat(v0.15/phase-N): <subject>` for code; `test(v0.15/phase-N): ...` for test-only commits; `docs(v0.15): ...` for docs.
- **Per-phase commit:** Each task ends in a commit. Each phase ends with a tag commit `chore(v0.15/phase-N): ship`.
- **Don't `git add -A`:** stage paths explicitly (root has untracked local helpers).
- **Co-author trailer:** every commit ends with the standard trailer.

---

## Pre-Phase-0 — Branch setup and baseline

- [ ] **Step 1: Confirm spec is at clean state**

```bash
cd /scratch/code/shibboleth/descriptor-mnemonic
git log --oneline -3 main
# Expect: most recent commit is aa9b0ae "design: SPEC v0.15 — md CLI restoration"
```

- [ ] **Step 2: Cut feature branch**

```bash
git checkout -b feat/v0.15-cli main
git status   # clean working tree
```

- [ ] **Step 3: Confirm v0.14 baseline tests pass**

```bash
cargo test --workspace 2>&1 | grep -E '^test result' | awk '{ok+=$4} END {print "Total ok:", ok}'
# Expect: Total ok: 253
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -3
# Expect: clean (no warnings)
```

- [ ] **Step 4: Confirm rustc version**

```bash
rustc --version
# Expect: 1.85+ (workspace MSRV)
```

---

## Phase 0 — Cargo manifest, binary scaffold, CliError

Phase goal: `cargo build` produces an `md` binary that responds to `--help` with usage text. CliError type compiles. No subcommand logic yet.

### Task 0.1: Workspace miniscript pin

**Files:**
- Modify: `Cargo.toml` (workspace root)

- [ ] **Step 1: Add miniscript as a workspace dep**

Add to `Cargo.toml` workspace root, at the end of the file:

```toml
[workspace.dependencies]
miniscript = { version = "13.0.0", default-features = false, features = ["std"] }
```

(If a `[workspace.dependencies]` table already exists, append the entry inside it.)

- [ ] **Step 2: Verify the workspace still resolves**

Run: `cargo metadata --format-version 1 > /dev/null && echo OK`
Expected: `OK`

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "$(cat <<'EOF'
feat(v0.15/phase-0): pin miniscript 13.0.0 in workspace

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 0.2: md-codec Cargo.toml — features, deps, bin entry

**Files:**
- Modify: `crates/md-codec/Cargo.toml`

- [ ] **Step 1: Bump version, add categories, features, bin, deps**

Replace the entire `crates/md-codec/Cargo.toml` with:

```toml
[package]
name = "md-codec"
version = "0.15.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "Reference implementation of the Mnemonic Descriptor (MD) format for engravable BIP 388 wallet policy backups, with `md` CLI"
readme = "README.md"
homepage = "https://github.com/bg002h/descriptor-mnemonic"
documentation = "https://docs.rs/md-codec"
keywords = ["bitcoin", "bip388", "wallet", "descriptor", "bech32"]
categories = ["cryptography::cryptocurrencies", "encoding", "command-line-utilities"]

[lints]
workspace = true

[lib]
name = "md_codec"

[[bin]]
name = "md"
path = "src/bin/md/main.rs"
required-features = ["cli"]

[features]
default = ["cli", "json"]
cli = ["dep:clap", "dep:anyhow", "dep:miniscript", "dep:regex"]
json = ["dep:serde", "dep:serde_json"]
cli-compiler = ["cli", "miniscript/compiler"]

[dependencies]
bitcoin = "0.32"
thiserror = "2.0"
bip39 = "2.2.2"
clap = { version = "4.5", features = ["derive"], optional = true }
anyhow = { version = "1.0", optional = true }
miniscript = { workspace = true, optional = true }
regex = { version = "1.10", optional = true }
serde = { version = "1.0", features = ["derive"], optional = true }
serde_json = { version = "1.0", optional = true }

[dev-dependencies]
assert_cmd = "2.0"
predicates = "3.1"
insta = { version = "1.40", features = ["json"] }
tempfile = "3.13"
```

- [ ] **Step 2: Verify cargo can resolve and start building**

Run: `cargo check --workspace --features cli,json 2>&1 | tail -5`
Expected: errors about missing `src/bin/md/main.rs` (we haven't created it yet) — that's fine.

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/Cargo.toml
git commit -m "$(cat <<'EOF'
feat(v0.15/phase-0): bump md-codec 0.15.0; add cli/json/cli-compiler features

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 0.3: CliError module

**Files:**
- Create: `crates/md-codec/src/bin/md/error.rs`

- [ ] **Step 1: Write the failing test**

Create `crates/md-codec/src/bin/md/error.rs` with:

```rust
use std::fmt;

#[derive(Debug)]
pub enum CliError {
    Codec(md_codec::Error),
    TemplateParse(String),
    BadXpub { i: u8, why: String },
    BadFingerprint { i: u8, why: String },
    Compile(String),
    Mismatch(String),
    BadArg(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::Codec(e) => write!(f, "codec error: {e}"),
            CliError::TemplateParse(m) => write!(f, "template parse error: {m}"),
            CliError::BadXpub { i, why } => write!(f, "--key @{i}: {why}"),
            CliError::BadFingerprint { i, why } => write!(f, "--fingerprint @{i}: {why}"),
            CliError::Compile(m) => write!(f, "compile error: {m}"),
            CliError::Mismatch(m) => write!(f, "MISMATCH: {m}"),
            CliError::BadArg(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for CliError {}

impl From<md_codec::Error> for CliError {
    fn from(e: md_codec::Error) -> Self { CliError::Codec(e) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_bad_xpub() {
        let e = CliError::BadXpub { i: 2, why: "checksum failed".into() };
        assert_eq!(format!("{e}"), "--key @2: checksum failed");
    }

    #[test]
    fn display_mismatch() {
        let e = CliError::Mismatch("policy id differs".into());
        assert_eq!(format!("{e}"), "MISMATCH: policy id differs");
    }

    #[test]
    fn from_codec_wraps() {
        let codec_err = md_codec::Error::ChunkSetIdOutOfRange { id: 0xFFFFFF };
        let cli_err: CliError = codec_err.into();
        assert!(matches!(cli_err, CliError::Codec(_)));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --features cli --bin md error::tests 2>&1 | tail -10`
Expected: error — `main.rs` doesn't exist, can't build the bin target.

- [ ] **Step 3: Create minimal main.rs so the bin target builds**

Create `crates/md-codec/src/bin/md/main.rs`:

```rust
mod error;

fn main() {}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --features cli --bin md error::tests 2>&1 | tail -5`
Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/md-codec/src/bin/md/error.rs crates/md-codec/src/bin/md/main.rs
git commit -m "$(cat <<'EOF'
feat(v0.15/phase-0): CliError module + minimal binary scaffold

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 0.4: clap dispatch scaffold (all 7 subcommands as `unimplemented!()`)

**Files:**
- Modify: `crates/md-codec/src/bin/md/main.rs`

- [ ] **Step 1: Write the failing test**

Create `crates/md-codec/tests/scaffold.rs`:

```rust
use assert_cmd::Command;

#[test]
fn md_help_runs() {
    let mut cmd = Command::cargo_bin("md").unwrap();
    cmd.arg("--help").assert().success();
}

#[test]
fn md_encode_help_runs() {
    let mut cmd = Command::cargo_bin("md").unwrap();
    cmd.args(["encode", "--help"]).assert().success();
}

#[test]
fn md_no_args_fails_with_usage_error() {
    let mut cmd = Command::cargo_bin("md").unwrap();
    cmd.assert().code(2);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --features cli --test scaffold 2>&1 | tail -5`
Expected: failures — the `md` binary's main is empty so `--help` produces nothing.

- [ ] **Step 3: Replace main.rs with clap dispatch**

Replace `crates/md-codec/src/bin/md/main.rs`:

```rust
mod error;

use clap::{Parser, Subcommand};
use std::process::ExitCode;

use error::CliError;

#[derive(Debug, Parser)]
#[command(name = "md", version, about = "Mnemonic Descriptor (MD) — engravable BIP 388 wallet policy backups", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Encode a wallet policy into MD backup string(s).
    Encode {
        /// BIP 388 template, e.g. `wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))`.
        template: Option<String>,
        /// Compile a sub-Miniscript-Policy expression into a template (cli-compiler).
        #[arg(long = "from-policy", value_name = "EXPR", conflicts_with = "template")]
        from_policy: Option<String>,
        /// Script context for `--from-policy`.
        #[arg(long, value_name = "CTX", value_parser = ["tap", "segwitv0"])]
        context: Option<String>,
        /// Override the inferred shared derivation path.
        #[arg(long, value_name = "PATH")]
        path: Option<String>,
        /// Concrete xpub for placeholder `@i`. Repeatable.
        #[arg(long = "key", value_name = "@i=XPUB")]
        keys: Vec<String>,
        /// Master-key fingerprint for placeholder `@i`. Repeatable.
        #[arg(long = "fingerprint", value_name = "@i=HEX")]
        fingerprints: Vec<String>,
        /// Force chunked encoding even for short policies.
        #[arg(long)]
        force_chunked: bool,
        /// Force the long BCH code even when the regular code suffices.
        #[arg(long)]
        force_long_code: bool,
        /// Print the freshly-computed PolicyId fingerprint after the phrase.
        #[arg(long)]
        policy_id_fingerprint: bool,
        /// Emit JSON output.
        #[arg(long)]
        json: bool,
    },
    /// Decode one or more MD backup strings into a wallet policy template.
    Decode {
        #[arg(required = true, num_args = 1..)]
        strings: Vec<String>,
        #[arg(long)]
        json: bool,
    },
    /// Verify backup strings re-encode to a given template.
    Verify {
        #[arg(required = true, num_args = 1..)]
        strings: Vec<String>,
        #[arg(long, required = true)]
        template: String,
        #[arg(long = "key", value_name = "@i=XPUB")]
        keys: Vec<String>,
        #[arg(long = "fingerprint", value_name = "@i=HEX")]
        fingerprints: Vec<String>,
    },
    /// Decode + pretty-print everything the codec sees.
    Inspect {
        #[arg(required = true, num_args = 1..)]
        strings: Vec<String>,
        #[arg(long)]
        json: bool,
    },
    /// Dump the raw payload bits in an annotated layout.
    Bytecode {
        #[arg(required = true, num_args = 1..)]
        strings: Vec<String>,
        #[arg(long)]
        json: bool,
    },
    /// Regenerate the project's test-vector corpus (maintainer tool).
    Vectors {
        #[arg(long, value_name = "DIR")]
        out: Option<String>,
    },
    /// Compile a sub-Miniscript-Policy expression into a BIP 388 template.
    Compile {
        expr: String,
        #[arg(long, value_name = "CTX", value_parser = ["tap", "segwitv0"], required = true)]
        context: String,
        #[arg(long)]
        json: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match dispatch(cli.command) {
        Ok(()) => ExitCode::from(0),
        Err(CliError::BadArg(m)) => {
            eprintln!("md: {m}");
            ExitCode::from(2)
        }
        Err(e) => {
            eprintln!("md: {e}");
            ExitCode::from(1)
        }
    }
}

fn dispatch(cmd: Command) -> Result<(), CliError> {
    match cmd {
        Command::Encode { .. } => unimplemented!("encode"),
        Command::Decode { .. } => unimplemented!("decode"),
        Command::Verify { .. } => unimplemented!("verify"),
        Command::Inspect { .. } => unimplemented!("inspect"),
        Command::Bytecode { .. } => unimplemented!("bytecode"),
        Command::Vectors { .. } => unimplemented!("vectors"),
        Command::Compile { .. } => unimplemented!("compile"),
    }
}
```

- [ ] **Step 4: Run scaffold tests**

Run: `cargo test --features cli --test scaffold 2>&1 | tail -5`
Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/md-codec/src/bin/md/main.rs crates/md-codec/tests/scaffold.rs
git commit -m "$(cat <<'EOF'
feat(v0.15/phase-0): clap dispatch scaffold for all 7 subcommands

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 0.5: Phase 0 ship tag

- [ ] **Step 1: Confirm baseline + scaffold tests pass**

```bash
cargo test --workspace --features cli,json 2>&1 | grep -E '^test result' | awk '{ok+=$4} END {print "Total ok:", ok}'
# Expect: Total ok: 256 (253 baseline + 3 scaffold)
```

- [ ] **Step 2: Empty ship-tag commit**

```bash
git commit --allow-empty -m "$(cat <<'EOF'
chore(v0.15/phase-0): ship — Cargo manifest, scaffold, CliError

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 1 — Argument parsers

Phase goal: `parse/keys.rs`, `parse/path.rs`, and the Pass A lexer in `parse/template.rs` are testable in isolation. No subcommand uses them yet.

### Task 1.1: `parse/keys.rs` — xpub validator

**Files:**
- Create: `crates/md-codec/src/bin/md/parse/mod.rs`
- Create: `crates/md-codec/src/bin/md/parse/keys.rs`
- Modify: `crates/md-codec/src/bin/md/main.rs` (add `mod parse;`)

- [ ] **Step 1: Write the failing test**

Create `crates/md-codec/src/bin/md/parse/keys.rs`:

```rust
use crate::error::CliError;
use bitcoin::base58;

const XPUB_LEN: usize = 78;
const MAINNET_XPUB_VERSION: [u8; 4] = [0x04, 0x88, 0xB2, 0x1E];

/// Script-context expectation for depth validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptCtx {
    /// Single-sig: depth 3 expected (e.g. wpkh, pkh).
    SingleSig,
    /// Multisig / taproot: depth 4 expected (e.g. wsh, sh-wsh, tr).
    MultiSig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedKey {
    pub i: u8,
    /// chain code (32) ‖ compressed pubkey (33).
    pub payload: [u8; 65],
}

pub fn parse_key(arg: &str, ctx: ScriptCtx) -> Result<ParsedKey, CliError> {
    let (i_str, xpub_str) = arg.split_once('=').ok_or_else(|| CliError::BadArg(
        format!("--key expects @i=XPUB, got: {arg}")
    ))?;
    let i = parse_index(i_str)?;
    let bytes = base58::decode_check(xpub_str)
        .map_err(|e| CliError::BadXpub { i, why: format!("base58check decode: {e}") })?;
    if bytes.len() != XPUB_LEN {
        return Err(CliError::BadXpub { i, why: format!("expected 78 bytes, got {}", bytes.len()) });
    }
    if bytes[0..4] != MAINNET_XPUB_VERSION {
        return Err(CliError::BadXpub { i, why: format!(
            "expected mainnet xpub version 0488B21E, got {:02X}{:02X}{:02X}{:02X}",
            bytes[0], bytes[1], bytes[2], bytes[3]
        )});
    }
    let depth = bytes[4];
    let expected_depth = match ctx { ScriptCtx::SingleSig => 3, ScriptCtx::MultiSig => 4 };
    if depth != expected_depth {
        return Err(CliError::BadXpub { i, why: format!(
            "expected depth {expected_depth} for this script context, got {depth}"
        )});
    }
    let mut payload = [0u8; 65];
    payload.copy_from_slice(&bytes[13..78]);
    Ok(ParsedKey { i, payload })
}

fn parse_index(s: &str) -> Result<u8, CliError> {
    let stripped = s.strip_prefix('@').unwrap_or(s);
    stripped.parse::<u8>().map_err(|_| CliError::BadArg(
        format!("--key index must be 0..255, got: {s}")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real xpub at depth 4 (m/48'/0'/0'/2'), mainnet — known-good test fixture.
    const XPUB_DEPTH4: &str = "xpub6E8v9bU1iAcW3WdkSCMQ8YnTQR1NDZRgRfDRm6jjvRMzcKNxd5z4eGn3xsk5SpadQ56iESwx1tUtv9wkZGZRtthcULPQAdwgK2VfWVpYpQc";

    #[test]
    fn rejects_no_equals() {
        let err = parse_key("@0xpub6...", ScriptCtx::MultiSig).unwrap_err();
        assert!(matches!(err, CliError::BadArg(_)));
    }

    #[test]
    fn rejects_bad_index() {
        let err = parse_key(format!("@notnum={XPUB_DEPTH4}").as_str(), ScriptCtx::MultiSig).unwrap_err();
        assert!(matches!(err, CliError::BadArg(_)));
    }

    #[test]
    fn rejects_bad_checksum() {
        let err = parse_key("@0=xpubBADCHECKSUMxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", ScriptCtx::MultiSig).unwrap_err();
        assert!(matches!(err, CliError::BadXpub { i: 0, .. }), "got: {err:?}");
    }

    #[test]
    fn accepts_valid_depth4_xpub() {
        let parsed = parse_key(format!("@2={XPUB_DEPTH4}").as_str(), ScriptCtx::MultiSig).unwrap();
        assert_eq!(parsed.i, 2);
        assert_eq!(parsed.payload.len(), 65);
    }

    #[test]
    fn rejects_depth4_xpub_in_singlesig_context() {
        let err = parse_key(format!("@0={XPUB_DEPTH4}").as_str(), ScriptCtx::SingleSig).unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("depth 3"), "got: {msg}");
    }

    #[test]
    fn strips_optional_at_prefix() {
        // Both forms accepted.
        let a = parse_key(format!("@1={XPUB_DEPTH4}").as_str(), ScriptCtx::MultiSig).unwrap();
        let b = parse_key(format!("1={XPUB_DEPTH4}").as_str(), ScriptCtx::MultiSig).unwrap();
        assert_eq!(a.i, b.i);
        assert_eq!(a.payload, b.payload);
    }
}
```

Create `crates/md-codec/src/bin/md/parse/mod.rs`:

```rust
pub mod keys;
```

- [ ] **Step 2: Add `mod parse;` to main.rs**

Edit `crates/md-codec/src/bin/md/main.rs`, change the top of the file:

```rust
mod error;
mod parse;

use clap::{Parser, Subcommand};
// ... rest unchanged
```

- [ ] **Step 3: Run test to verify it fails or compiles**

Run: `cargo test --features cli --bin md parse::keys::tests 2>&1 | tail -5`
Expected: passes (the impl is included in step 1).

- [ ] **Step 4: Commit**

```bash
git add crates/md-codec/src/bin/md/parse/mod.rs crates/md-codec/src/bin/md/parse/keys.rs crates/md-codec/src/bin/md/main.rs
git commit -m "$(cat <<'EOF'
feat(v0.15/phase-1): xpub validator with depth + mainnet checks

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 1.2: `parse/keys.rs` — fingerprint parser

**Files:**
- Modify: `crates/md-codec/src/bin/md/parse/keys.rs`

- [ ] **Step 1: Add tests + impl for fingerprint parsing**

Append to `crates/md-codec/src/bin/md/parse/keys.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedFingerprint {
    pub i: u8,
    pub fp: [u8; 4],
}

pub fn parse_fingerprint(arg: &str) -> Result<ParsedFingerprint, CliError> {
    let (i_str, hex_str) = arg.split_once('=').ok_or_else(|| CliError::BadArg(
        format!("--fingerprint expects @i=HEX, got: {arg}")
    ))?;
    let i = parse_index(i_str)?;
    let hex = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    if hex.len() != 8 {
        return Err(CliError::BadFingerprint { i, why: format!(
            "expected 8 hex chars (4 bytes), got {}", hex.len()
        )});
    }
    let mut fp = [0u8; 4];
    for (n, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let s = std::str::from_utf8(chunk).map_err(|_| CliError::BadFingerprint {
            i, why: "non-utf8 hex".into()
        })?;
        fp[n] = u8::from_str_radix(s, 16).map_err(|_| CliError::BadFingerprint {
            i, why: format!("invalid hex byte: {s}")
        })?;
    }
    Ok(ParsedFingerprint { i, fp })
}

#[cfg(test)]
mod fp_tests {
    use super::*;

    #[test]
    fn accepts_8_hex_chars() {
        let p = parse_fingerprint("@0=deadbeef").unwrap();
        assert_eq!(p.i, 0);
        assert_eq!(p.fp, [0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn accepts_0x_prefix() {
        let p = parse_fingerprint("@1=0xCAFEBABE").unwrap();
        assert_eq!(p.fp, [0xCA, 0xFE, 0xBA, 0xBE]);
    }

    #[test]
    fn rejects_wrong_length() {
        let err = parse_fingerprint("@0=dead").unwrap_err();
        assert!(matches!(err, CliError::BadFingerprint { i: 0, .. }));
    }

    #[test]
    fn rejects_non_hex() {
        let err = parse_fingerprint("@0=zzzzzzzz").unwrap_err();
        assert!(matches!(err, CliError::BadFingerprint { i: 0, .. }));
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --features cli --bin md parse::keys::fp_tests 2>&1 | tail -5`
Expected: 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/src/bin/md/parse/keys.rs
git commit -m "$(cat <<'EOF'
feat(v0.15/phase-1): fingerprint parser with @i=HEX format

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 1.3: `parse/path.rs` — path arg parser

**Files:**
- Create: `crates/md-codec/src/bin/md/parse/path.rs`
- Modify: `crates/md-codec/src/bin/md/parse/mod.rs`

- [ ] **Step 1: Write the parser + tests**

Create `crates/md-codec/src/bin/md/parse/path.rs`:

```rust
use crate::error::CliError;
use bitcoin::bip32::DerivationPath;
use std::str::FromStr;

/// Parse a `--path <PATH>` argument: a name, a hex indicator, or a literal path.
pub fn parse_path(arg: &str) -> Result<DerivationPath, CliError> {
    if let Some(p) = parse_path_name(arg) {
        return Ok(p);
    }
    if let Some(p) = parse_path_hex(arg)? {
        return Ok(p);
    }
    DerivationPath::from_str(arg).map_err(|e| CliError::BadArg(
        format!("--path could not parse `{arg}` as name, hex, or literal path: {e}")
    ))
}

fn parse_path_name(s: &str) -> Option<DerivationPath> {
    match s {
        "bip44" => Some(DerivationPath::from_str("m/44'/0'/0'").unwrap()),
        "bip49" => Some(DerivationPath::from_str("m/49'/0'/0'").unwrap()),
        "bip84" => Some(DerivationPath::from_str("m/84'/0'/0'").unwrap()),
        "bip86" => Some(DerivationPath::from_str("m/86'/0'/0'").unwrap()),
        "bip48" => Some(DerivationPath::from_str("m/48'/0'/0'/2'").unwrap()),
        _ => None,
    }
}

fn parse_path_hex(s: &str) -> Result<Option<DerivationPath>, CliError> {
    let Some(rest) = s.strip_prefix("0x") else { return Ok(None) };
    let n = u32::from_str_radix(rest, 16).map_err(|_| CliError::BadArg(
        format!("--path hex value invalid: {s}")
    ))?;
    // Hex indicator selects a single hardened account-level path m/n'.
    let path = DerivationPath::from_str(&format!("m/{n}'")).map_err(|e| CliError::BadArg(
        format!("--path hex {s} → m/{n}': {e}")
    ))?;
    Ok(Some(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_name_bip48() {
        let p = parse_path("bip48").unwrap();
        assert_eq!(p.to_string(), "48'/0'/0'/2'");
    }

    #[test]
    fn parses_hex() {
        let p = parse_path("0x05").unwrap();
        assert_eq!(p.to_string(), "5'");
    }

    #[test]
    fn parses_literal() {
        let p = parse_path("m/48'/0'/0'/2'").unwrap();
        assert_eq!(p.to_string(), "48'/0'/0'/2'");
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_path("not-a-path").is_err());
    }
}
```

Edit `crates/md-codec/src/bin/md/parse/mod.rs` to:

```rust
pub mod keys;
pub mod path;
```

- [ ] **Step 2: Run tests**

Run: `cargo test --features cli --bin md parse::path::tests 2>&1 | tail -5`
Expected: 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/src/bin/md/parse/path.rs crates/md-codec/src/bin/md/parse/mod.rs
git commit -m "$(cat <<'EOF'
feat(v0.15/phase-1): --path arg parser (name|hex|literal)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 1.4: `parse/template.rs` — Pass A placeholder lexer

**Files:**
- Create: `crates/md-codec/src/bin/md/parse/template.rs`
- Modify: `crates/md-codec/src/bin/md/parse/mod.rs`

- [ ] **Step 1: Write the lexer + edge-case tests**

Create `crates/md-codec/src/bin/md/parse/template.rs`:

```rust
use crate::error::CliError;
use bitcoin::bip32::DerivationPath;
use regex::Regex;
use std::str::FromStr;
use std::sync::OnceLock;

/// One occurrence of a `@i/...` placeholder in the raw template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceholderOccurrence {
    pub i: u8,
    pub origin_path: Option<DerivationPath>,
    pub multipath_alts: Vec<u32>,
    pub wildcard_hardened: bool,
}

/// Pass A: extract every `@i/...` placeholder from the raw template string.
pub fn lex_placeholders(template: &str) -> Result<Vec<PlaceholderOccurrence>, CliError> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        // Captures:
        //   1: @i index digits
        //   2: optional origin path (e.g. "/48'/0'/0'/2'")
        //   3: optional multipath body (e.g. "0;1")
        //   4: wildcard with optional hardening (e.g. "*", "*'", "*h")
        Regex::new(
            r"@(\d+)((?:/\d+'?)*)(?:/<([0-9;]+)>)?(/\*(?:'|h)?)?"
        ).expect("static regex compiles")
    });
    let mut out = Vec::new();
    for caps in re.captures_iter(template) {
        let i: u8 = caps[1].parse().map_err(|_| CliError::TemplateParse(
            format!("@i index out of range: @{}", &caps[1])
        ))?;
        let origin_path = if let Some(m) = caps.get(2) {
            let s = m.as_str();
            if s.is_empty() { None } else {
                Some(DerivationPath::from_str(s.trim_start_matches('/'))
                    .map_err(|e| CliError::TemplateParse(format!("@{i} origin path `{s}`: {e}")))?)
            }
        } else { None };
        let multipath_alts = if let Some(m) = caps.get(3) {
            m.as_str().split(';').map(|n| n.parse::<u32>()
                .map_err(|_| CliError::TemplateParse(format!("@{i} multipath alt `{n}` not u32"))))
                .collect::<Result<Vec<_>, _>>()?
        } else { Vec::new() };
        let wildcard_hardened = caps.get(4).map(|m| m.as_str().ends_with('\'') || m.as_str().ends_with('h')).unwrap_or(false);
        out.push(PlaceholderOccurrence { i, origin_path, multipath_alts, wildcard_hardened });
    }
    if out.is_empty() {
        return Err(CliError::TemplateParse("template contains no @i placeholders".into()));
    }
    Ok(out)
}

#[cfg(test)]
mod lex_tests {
    use super::*;

    #[test]
    fn single_at0_no_multipath() {
        let v = lex_placeholders("wpkh(@0/*)").unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].i, 0);
        assert_eq!(v[0].multipath_alts, Vec::<u32>::new());
        assert!(!v[0].wildcard_hardened);
    }

    #[test]
    fn at0_hardened_wildcard() {
        let v = lex_placeholders("wpkh(@0/*')").unwrap();
        assert!(v[0].wildcard_hardened);
    }

    #[test]
    fn at0_hardened_wildcard_h_form() {
        let v = lex_placeholders("wpkh(@0/*h)").unwrap();
        assert!(v[0].wildcard_hardened);
    }

    #[test]
    fn multipath_arity_2() {
        let v = lex_placeholders("wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))").unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].multipath_alts, vec![0, 1]);
        assert_eq!(v[1].multipath_alts, vec![0, 1]);
    }

    #[test]
    fn multipath_arity_3() {
        let v = lex_placeholders("wpkh(@0/<0;1;2>/*)").unwrap();
        assert_eq!(v[0].multipath_alts, vec![0, 1, 2]);
    }

    #[test]
    fn origin_path_extracted() {
        let v = lex_placeholders("wpkh(@0/48'/0'/0'/2'/<0;1>/*)").unwrap();
        assert_eq!(v[0].origin_path.as_ref().unwrap().to_string(), "48'/0'/0'/2'");
    }

    #[test]
    fn multiple_at_i_collected() {
        let v = lex_placeholders("wsh(multi(3,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))").unwrap();
        assert_eq!(v.len(), 3);
        assert_eq!(v.iter().map(|p| p.i).collect::<Vec<_>>(), vec![0, 1, 2]);
    }

    #[test]
    fn rejects_template_with_no_placeholders() {
        assert!(lex_placeholders("wpkh(xpubAAAAA)").is_err());
    }
}
```

Edit `crates/md-codec/src/bin/md/parse/mod.rs`:

```rust
pub mod keys;
pub mod path;
pub mod template;
```

- [ ] **Step 2: Run tests**

Run: `cargo test --features cli --bin md parse::template::lex_tests 2>&1 | tail -5`
Expected: 7 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/src/bin/md/parse/template.rs crates/md-codec/src/bin/md/parse/mod.rs
git commit -m "$(cat <<'EOF'
feat(v0.15/phase-1): Pass A placeholder lexer with edge-case coverage

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 1.5: Pass A consistency rules — divergent vs shared

**Files:**
- Modify: `crates/md-codec/src/bin/md/parse/template.rs`

- [ ] **Step 1: Add consistency-resolution layer + tests**

Append to `crates/md-codec/src/bin/md/parse/template.rs`:

```rust
use md_codec::origin_path::PathDecl;
use md_codec::origin_path::PathDeclPaths;
use md_codec::use_site_path::UseSitePath;

/// Resolved per-`@i` view after consistency checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPlaceholders {
    pub n: u8,
    pub path_decl: PathDecl,
    pub use_site_path: UseSitePath,
    pub use_site_path_overrides: Vec<(u8, UseSitePath)>,
}

pub fn resolve_placeholders(occs: &[PlaceholderOccurrence]) -> Result<ResolvedPlaceholders, CliError> {
    // Collapse same-@i occurrences; reject if conflicting.
    let mut by_i: std::collections::BTreeMap<u8, &PlaceholderOccurrence> = std::collections::BTreeMap::new();
    for occ in occs {
        if let Some(prev) = by_i.get(&occ.i) {
            if prev.multipath_alts != occ.multipath_alts
                || prev.wildcard_hardened != occ.wildcard_hardened
                || prev.origin_path != occ.origin_path
            {
                return Err(CliError::TemplateParse(format!(
                    "@{} appears with inconsistent path/multipath/hardening", occ.i
                )));
            }
        } else {
            by_i.insert(occ.i, occ);
        }
    }
    let n = (by_i.keys().max().copied().ok_or_else(|| CliError::TemplateParse("no placeholders".into()))? as usize + 1) as u8;
    // Verify dense 0..n.
    for i in 0..n {
        if !by_i.contains_key(&i) {
            return Err(CliError::TemplateParse(format!("@{i} not present; placeholders must be dense 0..n")));
        }
    }
    let at0 = by_i[&0];
    // Build use_site_path from @0.
    let use_site_path = make_use_site_path(at0)?;
    let mut use_site_path_overrides = Vec::new();
    for i in 1..n {
        let occ = by_i[&i];
        let usp_i = make_use_site_path(occ)?;
        if usp_i != use_site_path {
            use_site_path_overrides.push((i, usp_i));
        }
    }
    let path_decl = make_path_decl(&by_i, n, at0)?;
    Ok(ResolvedPlaceholders { n, path_decl, use_site_path, use_site_path_overrides })
}

fn make_use_site_path(occ: &PlaceholderOccurrence) -> Result<UseSitePath, CliError> {
    use md_codec::use_site_path::Alternative;
    let alts: Vec<Alternative> = occ.multipath_alts.iter()
        .map(|v| Alternative { hardened: false, value: *v })
        .collect();
    Ok(UseSitePath {
        multipath: if alts.is_empty() { None } else { Some(alts) },
        wildcard_hardened: occ.wildcard_hardened,
    })
}

fn make_path_decl(
    by_i: &std::collections::BTreeMap<u8, &PlaceholderOccurrence>,
    n: u8,
    at0: &PlaceholderOccurrence,
) -> Result<PathDecl, CliError> {
    let all_same = (0..n).all(|i| by_i[&i].origin_path == at0.origin_path);
    let paths = if all_same {
        PathDeclPaths::Single(at0.origin_path.clone().unwrap_or_else(|| DerivationPath::master()))
    } else {
        let v: Vec<DerivationPath> = (0..n).map(|i| by_i[&i].origin_path.clone().unwrap_or_else(|| DerivationPath::master())).collect();
        PathDeclPaths::Divergent(v)
    };
    Ok(PathDecl { paths })
}

#[cfg(test)]
mod resolve_tests {
    use super::*;

    #[test]
    fn shared_use_site_path_when_all_match() {
        let occs = lex_placeholders("wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))").unwrap();
        let r = resolve_placeholders(&occs).unwrap();
        assert_eq!(r.n, 2);
        assert!(r.use_site_path_overrides.is_empty());
    }

    #[test]
    fn divergent_use_site_path_when_at1_differs() {
        let occs = lex_placeholders("wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))").unwrap();
        let r = resolve_placeholders(&occs).unwrap();
        assert_eq!(r.n, 2);
        assert_eq!(r.use_site_path_overrides.len(), 1);
        assert_eq!(r.use_site_path_overrides[0].0, 1);
    }

    #[test]
    fn rejects_nondense_placeholders() {
        let occs = lex_placeholders("wsh(multi(2,@0/<0;1>/*,@2/<0;1>/*))").unwrap();
        let err = resolve_placeholders(&occs).unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("dense"), "got: {msg}");
    }

    #[test]
    fn rejects_same_at_i_conflicting() {
        // Synthesize directly, lexer would also accept these as separate occurrences.
        let occs = vec![
            PlaceholderOccurrence { i: 0, origin_path: None, multipath_alts: vec![0,1], wildcard_hardened: false },
            PlaceholderOccurrence { i: 0, origin_path: None, multipath_alts: vec![2,3], wildcard_hardened: false },
        ];
        let err = resolve_placeholders(&occs).unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("inconsistent"), "got: {msg}");
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --features cli --bin md parse::template::resolve_tests 2>&1 | tail -5`
Expected: 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/src/bin/md/parse/template.rs
git commit -m "$(cat <<'EOF'
feat(v0.15/phase-1): Pass A consistency rules — shared vs divergent use_site_path

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 1.6: Phase 1 ship tag

- [ ] **Step 1: Run full test suite**

```bash
cargo test --workspace --features cli,json 2>&1 | grep -E '^test result' | awk '{ok+=$4} END {print "Total ok:", ok}'
# Expect: Total ok: 275 (256 + 19 from Phase 1)
```

- [ ] **Step 2: Empty ship-tag commit**

```bash
git commit --allow-empty -m "$(cat <<'EOF'
chore(v0.15/phase-1): ship — keys, path, template Pass A parsers

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 2 — Template→Descriptor bridge (Pass B + tag mapping)

Phase goal: `parse_template(s, &keys, &fingerprints)` returns a `Descriptor` for any template the lexer accepts. All `Tag` variants used by BIP 388 are covered by tag-mapping arms.

### Task 2.1: Pass B scaffold — synthetic substitution + miniscript dispatch

**Files:**
- Modify: `crates/md-codec/src/bin/md/parse/template.rs`

- [ ] **Step 1: Add substitute_synthetic + dispatch**

Append to `crates/md-codec/src/bin/md/parse/template.rs`:

```rust
/// A synthetic xpub keyed by placeholder index `i`. Deterministic.
/// Used only inside the parser; never emitted to wire.
fn synthetic_xpub_for(i: u8) -> String {
    // Use a fixed mainnet xpub at depth 4, varying byte 13 (start of chain code).
    // The base58check string is regenerated from raw bytes each call (cheap; only at parse time).
    use bitcoin::base58;
    let mut bytes = [0u8; 78];
    bytes[0..4].copy_from_slice(&MAINNET_XPUB_VERSION);
    bytes[4] = 4;                         // depth
    bytes[5..9].copy_from_slice(&[0;4]);  // parent fp (zeros)
    bytes[9..13].copy_from_slice(&[0;4]); // child number (zeros)
    bytes[13] = i;                        // first chain-code byte = i (uniqueness)
    bytes[45] = 0x02;                     // compressed pubkey prefix (even)
    bytes[46..78].copy_from_slice(&[i; 32]); // pubkey body = 0x{ii} * 32
    base58::encode_check(&bytes)
}

/// Substitute each `@i/<M;N>/*` (or `@i/*` etc.) with a synthetic key reference
/// suitable for miniscript parsing. Returns substituted template + key map.
fn substitute_synthetic(template: &str) -> Result<(String, std::collections::BTreeMap<String, u8>), CliError> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(
        r"@(\d+)((?:/\d+'?)*)(?:/<[0-9;]+>)?(?:/\*(?:'|h)?)?"
    ).expect("static regex compiles"));
    let mut key_map = std::collections::BTreeMap::new();
    let mut keys_seen = std::collections::HashSet::new();
    let out = re.replace_all(template, |caps: &regex::Captures| {
        let i: u8 = caps[1].parse().unwrap_or(0);
        let xpub = synthetic_xpub_for(i);
        if keys_seen.insert(i) {
            key_map.insert(xpub.clone(), i);
        }
        xpub
    }).into_owned();
    Ok((out, key_map))
}

#[cfg(test)]
mod sub_tests {
    use super::*;

    #[test]
    fn synthetic_for_0_and_1_differ() {
        assert_ne!(synthetic_xpub_for(0), synthetic_xpub_for(1));
    }

    #[test]
    fn synthetic_for_0_is_stable() {
        assert_eq!(synthetic_xpub_for(0), synthetic_xpub_for(0));
    }

    #[test]
    fn substitution_strips_at_i_suffix() {
        let (s, _) = substitute_synthetic("wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))").unwrap();
        assert!(!s.contains('@'));
        assert!(!s.contains('<'));
        assert!(!s.contains('*'));
    }

    #[test]
    fn substitution_emits_consistent_keys_per_index() {
        let (s, km) = substitute_synthetic("wsh(or_d(pk(@0/<0;1>/*),pk(@0/<0;1>/*)))").unwrap();
        assert_eq!(km.len(), 1);
        // Same key string appears twice in the output.
        let key = synthetic_xpub_for(0);
        assert_eq!(s.matches(&key).count(), 2);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --features cli --bin md parse::template::sub_tests 2>&1 | tail -5`
Expected: 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/src/bin/md/parse/template.rs
git commit -m "$(cat <<'EOF'
feat(v0.15/phase-2): Pass B synthetic substitution + key map

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 2.2: Tag mapping — script context arms (wpkh / pkh / wsh / sh / tr)

**Files:**
- Modify: `crates/md-codec/src/bin/md/parse/template.rs`

- [ ] **Step 1: Add script-context dispatch + tests**

Append to `crates/md-codec/src/bin/md/parse/template.rs`:

```rust
use md_codec::tag::Tag;
use md_codec::tree::{Body, Node};
use miniscript::{Descriptor as MsDescriptor, DescriptorPublicKey};

/// Walk the miniscript Descriptor's outermost wrapper and emit the root `Tag`.
fn walk_root(desc: &MsDescriptor<DescriptorPublicKey>, key_map: &std::collections::BTreeMap<String, u8>)
    -> Result<Node, CliError>
{
    use miniscript::Descriptor::*;
    match desc {
        Wpkh(w) => Ok(Node {
            tag: Tag::Wpkh,
            body: Body::SingleKey { idx: lookup_key(&w.as_inner().to_string(), key_map)? },
        }),
        Pkh(p) => Ok(Node {
            tag: Tag::Pkh,
            body: Body::SingleKey { idx: lookup_key(&p.as_inner().to_string(), key_map)? },
        }),
        Wsh(w) => walk_wsh(w, key_map),
        Sh(s) => walk_sh(s, key_map),
        Tr(t) => walk_tr(t, key_map),
        _ => Err(CliError::TemplateParse(format!("unsupported descriptor wrapper: {desc}"))),
    }
}

fn lookup_key(key_str: &str, key_map: &std::collections::BTreeMap<String, u8>) -> Result<u8, CliError> {
    // miniscript may render the key with derivation suffix; strip suffix for lookup.
    let base = key_str.split('/').next().unwrap_or(key_str);
    key_map.get(base).copied().ok_or_else(|| CliError::TemplateParse(
        format!("internal: synthetic key {base} not found in key map (rendered: {key_str})")
    ))
}

// Stubs filled in next tasks.
fn walk_wsh(_w: &miniscript::descriptor::Wsh<DescriptorPublicKey>, _km: &std::collections::BTreeMap<String, u8>) -> Result<Node, CliError> { unimplemented!("wsh in Task 2.3") }
fn walk_sh(_s: &miniscript::descriptor::Sh<DescriptorPublicKey>, _km: &std::collections::BTreeMap<String, u8>) -> Result<Node, CliError> { unimplemented!("sh in Task 2.3") }
fn walk_tr(_t: &miniscript::descriptor::Tr<DescriptorPublicKey>, _km: &std::collections::BTreeMap<String, u8>) -> Result<Node, CliError> { unimplemented!("tr in Task 2.4") }

#[cfg(test)]
mod root_tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn wpkh_root() {
        let (s, km) = substitute_synthetic("wpkh(@0/<0;1>/*)").unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        assert_eq!(root.tag, Tag::Wpkh);
    }

    #[test]
    fn pkh_root() {
        let (s, km) = substitute_synthetic("pkh(@0/<0;1>/*)").unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        assert_eq!(root.tag, Tag::Pkh);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --features cli --bin md parse::template::root_tests 2>&1 | tail -5`
Expected: 2 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/src/bin/md/parse/template.rs
git commit -m "$(cat <<'EOF'
feat(v0.15/phase-2): root tag dispatch — wpkh, pkh

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 2.3: Tag mapping — wsh inner expressions (multi, sortedmulti, miniscript)

**Files:**
- Modify: `crates/md-codec/src/bin/md/parse/template.rs`

- [ ] **Step 1: Implement walk_wsh + walk_sh + tests**

Replace the stub `walk_wsh` and `walk_sh` in `crates/md-codec/src/bin/md/parse/template.rs` with:

```rust
fn walk_wsh(w: &miniscript::descriptor::Wsh<DescriptorPublicKey>, km: &std::collections::BTreeMap<String, u8>) -> Result<Node, CliError> {
    let body = walk_wsh_inner(w, km)?;
    Ok(Node { tag: Tag::Wsh, body })
}

fn walk_sh(s: &miniscript::descriptor::Sh<DescriptorPublicKey>, km: &std::collections::BTreeMap<String, u8>) -> Result<Node, CliError> {
    use miniscript::descriptor::ShInner;
    match s.as_inner() {
        ShInner::Wsh(w) => {
            let body = walk_wsh_inner(w, km)?;
            Ok(Node { tag: Tag::ShWsh, body })
        }
        ShInner::Wpkh(wp) => Ok(Node {
            tag: Tag::ShWpkh,
            body: Body::SingleKey { idx: lookup_key(&wp.as_inner().to_string(), km)? },
        }),
        ShInner::Ms(ms) => {
            let body = walk_miniscript_node(ms, km)?;
            Ok(Node { tag: Tag::Sh, body })
        }
        ShInner::SortedMulti(sm) => Ok(Node {
            tag: Tag::Sh,
            body: build_multi_body(true, sm.k(), sm.pks().iter().collect::<Vec<_>>(), km)?,
        }),
    }
}

fn walk_wsh_inner(w: &miniscript::descriptor::Wsh<DescriptorPublicKey>, km: &std::collections::BTreeMap<String, u8>) -> Result<Body, CliError> {
    use miniscript::descriptor::WshInner;
    match w.as_inner() {
        WshInner::Ms(ms) => walk_miniscript_node(ms, km).map(|body| body),
        WshInner::SortedMulti(sm) => build_multi_body(true, sm.k(), sm.pks().iter().collect::<Vec<_>>(), km),
    }
}

fn build_multi_body(sorted: bool, k: usize, keys: Vec<&DescriptorPublicKey>, km: &std::collections::BTreeMap<String, u8>) -> Result<Body, CliError> {
    let indices = keys.iter().map(|kk| lookup_key(&kk.to_string(), km)).collect::<Result<Vec<u8>, _>>()?;
    Ok(Body::Multi {
        k: k as u8,
        sorted,
        indices,
    })
}

fn walk_miniscript_node<C: miniscript::ScriptContext>(
    ms: &miniscript::Miniscript<DescriptorPublicKey, C>,
    km: &std::collections::BTreeMap<String, u8>,
) -> Result<Body, CliError> {
    use miniscript::miniscript::decode::Terminal;
    match &ms.node {
        Terminal::PkK(k) | Terminal::PkH(k) => Ok(Body::SingleKey { idx: lookup_key(&k.to_string(), km)? }),
        Terminal::Multi(threshold, keys) | Terminal::MultiA(threshold, keys) => {
            build_multi_body(false, threshold.k(), keys.iter().collect(), km)
        }
        // Other miniscript fragments — left as TemplateParse error until BIP 388 templates need them.
        _ => Err(CliError::TemplateParse(format!("unsupported miniscript fragment: {ms}"))),
    }
}

#[cfg(test)]
mod wsh_tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn wsh_multi_2of2() {
        let (s, km) = substitute_synthetic("wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))").unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        assert_eq!(root.tag, Tag::Wsh);
        match root.body {
            Body::Multi { k, sorted, indices } => {
                assert_eq!(k, 2);
                assert!(!sorted);
                assert_eq!(indices, vec![0, 1]);
            }
            _ => panic!("expected Body::Multi"),
        }
    }

    #[test]
    fn wsh_sortedmulti_2of3() {
        let (s, km) = substitute_synthetic("wsh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))").unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        assert_eq!(root.tag, Tag::Wsh);
        match root.body {
            Body::Multi { k, sorted, indices } => {
                assert_eq!(k, 2);
                assert!(sorted);
                assert_eq!(indices, vec![0, 1, 2]);
            }
            _ => panic!("expected Body::Multi"),
        }
    }

    #[test]
    fn sh_wpkh() {
        let (s, km) = substitute_synthetic("sh(wpkh(@0/<0;1>/*))").unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        assert_eq!(root.tag, Tag::ShWpkh);
    }

    #[test]
    fn sh_wsh_multi() {
        let (s, km) = substitute_synthetic("sh(wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*)))").unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        assert_eq!(root.tag, Tag::ShWsh);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --features cli --bin md parse::template::wsh_tests 2>&1 | tail -5`
Expected: 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/src/bin/md/parse/template.rs
git commit -m "$(cat <<'EOF'
feat(v0.15/phase-2): wsh + sh dispatch (multi, sortedmulti, sh-wpkh, sh-wsh)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 2.4: Tag mapping — taproot (tr key-only and tr with tap-tree)

**Files:**
- Modify: `crates/md-codec/src/bin/md/parse/template.rs`

- [ ] **Step 1: Implement walk_tr + tests**

Replace the stub `walk_tr` in `crates/md-codec/src/bin/md/parse/template.rs` with:

```rust
fn walk_tr(t: &miniscript::descriptor::Tr<DescriptorPublicKey>, km: &std::collections::BTreeMap<String, u8>) -> Result<Node, CliError> {
    use miniscript::descriptor::TapTree;
    let internal_idx = lookup_key(&t.internal_key().to_string(), km)?;
    let tree = if let Some(taptree) = t.tap_tree() {
        Some(walk_tap_tree(taptree, km)?)
    } else { None };
    Ok(Node {
        tag: Tag::Tr,
        body: Body::Tr { internal_idx, tree },
    })
}

fn walk_tap_tree(tt: &miniscript::descriptor::TapTree<DescriptorPublicKey>, km: &std::collections::BTreeMap<String, u8>) -> Result<md_codec::tree::TapTreeNode, CliError> {
    use miniscript::descriptor::TapTree;
    use md_codec::tree::TapTreeNode;
    match tt {
        TapTree::Leaf(ms) => {
            let body = walk_miniscript_node(ms, km)?;
            Ok(TapTreeNode::Leaf(Node { tag: Tag::TapLeaf, body }))
        }
        TapTree::Tree { left, right, .. } => Ok(TapTreeNode::Branch {
            left: Box::new(walk_tap_tree(left, km)?),
            right: Box::new(walk_tap_tree(right, km)?),
        }),
    }
}

#[cfg(test)]
mod tr_tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn tr_key_only() {
        let (s, km) = substitute_synthetic("tr(@0/<0;1>/*)").unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        assert_eq!(root.tag, Tag::Tr);
        match root.body {
            Body::Tr { internal_idx, tree } => {
                assert_eq!(internal_idx, 0);
                assert!(tree.is_none());
            }
            _ => panic!("expected Body::Tr"),
        }
    }

    #[test]
    fn tr_with_one_leaf() {
        let (s, km) = substitute_synthetic("tr(@0/<0;1>/*,pk(@1/<0;1>/*))").unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        assert_eq!(root.tag, Tag::Tr);
        match root.body {
            Body::Tr { internal_idx, tree } => {
                assert_eq!(internal_idx, 0);
                assert!(tree.is_some());
            }
            _ => panic!("expected Body::Tr"),
        }
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --features cli --bin md parse::template::tr_tests 2>&1 | tail -5`
Expected: 2 tests pass. (If `md_codec::tree::TapTreeNode` has a different variant shape, adjust the construction; the plan task is "make these two tests pass against the actual `tree.rs` API." Read `crates/md-codec/src/tree.rs` once and align.)

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/src/bin/md/parse/template.rs
git commit -m "$(cat <<'EOF'
feat(v0.15/phase-2): tr dispatch with tap-tree walker

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 2.5: `parse_template` entry point — combine Pass A + Pass B + key/fp substitution

**Files:**
- Modify: `crates/md-codec/src/bin/md/parse/template.rs`

- [ ] **Step 1: Add the public entry point + tests**

Append to `crates/md-codec/src/bin/md/parse/template.rs`:

```rust
use crate::parse::keys::{ParsedKey, ParsedFingerprint, ScriptCtx};
use md_codec::encode::Descriptor;
use md_codec::tlv::TlvSection;

pub fn parse_template(
    template: &str,
    keys: &[ParsedKey],
    fingerprints: &[ParsedFingerprint],
) -> Result<Descriptor, CliError> {
    let occs = lex_placeholders(template)?;
    let resolved = resolve_placeholders(&occs)?;

    let (substituted, key_map) = substitute_synthetic(template)?;
    let ms_desc = MsDescriptor::<DescriptorPublicKey>::from_str(&substituted)
        .map_err(|e| CliError::TemplateParse(format!("miniscript parse failed: {e}")))?;
    let tree = walk_root(&ms_desc, &key_map)?;

    let pubkeys = if keys.is_empty() { None } else {
        Some(keys.iter().map(|k| (k.i, k.payload)).collect())
    };
    let fp_vec = if fingerprints.is_empty() { None } else {
        Some(fingerprints.iter().map(|f| (f.i, f.fp)).collect())
    };
    let use_site_path_overrides = if resolved.use_site_path_overrides.is_empty() { None } else {
        Some(resolved.use_site_path_overrides)
    };

    Ok(Descriptor {
        n: resolved.n,
        path_decl: resolved.path_decl,
        use_site_path: resolved.use_site_path,
        tree,
        tlv: TlvSection {
            use_site_path_overrides,
            fingerprints: fp_vec,
            pubkeys,
            // any other TLV fields default to None per the actual TlvSection definition.
            ..Default::default()
        },
    })
}

/// Convenience: derive script-context expectation from the template's outer wrapper.
pub fn ctx_for_template(template: &str) -> ScriptCtx {
    let head = template.trim_start();
    if head.starts_with("wpkh(") || head.starts_with("pkh(") || head.starts_with("sh(wpkh(") {
        ScriptCtx::SingleSig
    } else {
        ScriptCtx::MultiSig
    }
}

#[cfg(test)]
mod entry_tests {
    use super::*;

    #[test]
    fn end_to_end_wsh_multi_template_only() {
        let d = parse_template(
            "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))",
            &[],
            &[],
        ).unwrap();
        assert_eq!(d.n, 2);
        assert_eq!(d.tree.tag, Tag::Wsh);
        assert!(d.tlv.pubkeys.is_none());
    }

    #[test]
    fn end_to_end_with_fingerprints() {
        let fps = vec![
            ParsedFingerprint { i: 0, fp: [0xDE, 0xAD, 0xBE, 0xEF] },
            ParsedFingerprint { i: 1, fp: [0xCA, 0xFE, 0xBA, 0xBE] },
        ];
        let d = parse_template("wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))", &[], &fps).unwrap();
        let v = d.tlv.fingerprints.unwrap();
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn ctx_for_wpkh_is_singlesig() {
        assert_eq!(ctx_for_template("wpkh(@0/<0;1>/*)"), ScriptCtx::SingleSig);
    }

    #[test]
    fn ctx_for_wsh_is_multisig() {
        assert_eq!(ctx_for_template("wsh(multi(2,...))"), ScriptCtx::MultiSig);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --features cli --bin md parse::template::entry_tests 2>&1 | tail -5`
Expected: 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/src/bin/md/parse/template.rs
git commit -m "$(cat <<'EOF'
feat(v0.15/phase-2): parse_template entry point + script-context inference

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 2.6: Phase 2 ship tag

- [ ] **Step 1: Run full suite**

```bash
cargo test --workspace --features cli,json 2>&1 | grep -E '^test result' | awk '{ok+=$4} END {print "Total ok:", ok}'
# Expect: ~291 (275 + 16 from Phase 2)
```

- [ ] **Step 2: Empty ship-tag commit**

```bash
git commit --allow-empty -m "$(cat <<'EOF'
chore(v0.15/phase-2): ship — template→Descriptor bridge complete

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 3 — Text formatter

Phase goal: `format::text` can render a `Descriptor` back to its template string and print identity hashes / chunk metadata in a stable layout.

### Task 3.1: `format/text.rs` — Descriptor → template string

**Files:**
- Create: `crates/md-codec/src/bin/md/format/mod.rs`
- Create: `crates/md-codec/src/bin/md/format/text.rs`
- Modify: `crates/md-codec/src/bin/md/main.rs` (add `mod format;`)

- [ ] **Step 1: Write the renderer + tests**

Create `crates/md-codec/src/bin/md/format/text.rs`:

```rust
use crate::error::CliError;
use md_codec::encode::Descriptor;
use md_codec::tag::Tag;
use md_codec::tree::{Body, Node, TapTreeNode};
use md_codec::use_site_path::UseSitePath;
use std::fmt::Write as _;

/// Render a `Descriptor` back to a BIP 388 template string with `@i` placeholders.
pub fn descriptor_to_template(d: &Descriptor) -> Result<String, CliError> {
    let mut out = String::new();
    render_node(&d.tree, &d.use_site_path, &d.tlv.use_site_path_overrides.as_deref(), &mut out)?;
    Ok(out)
}

fn render_node(
    node: &Node,
    default_usp: &UseSitePath,
    overrides: &Option<&[(u8, UseSitePath)]>,
    out: &mut String,
) -> Result<(), CliError> {
    match node.tag {
        Tag::Wpkh => {
            out.push_str("wpkh(");
            render_body(&node.body, default_usp, overrides, out)?;
            out.push(')');
        }
        Tag::Pkh => {
            out.push_str("pkh(");
            render_body(&node.body, default_usp, overrides, out)?;
            out.push(')');
        }
        Tag::Wsh => {
            out.push_str("wsh(");
            render_body(&node.body, default_usp, overrides, out)?;
            out.push(')');
        }
        Tag::Sh => {
            out.push_str("sh(");
            render_body(&node.body, default_usp, overrides, out)?;
            out.push(')');
        }
        Tag::ShWpkh => {
            out.push_str("sh(wpkh(");
            render_body(&node.body, default_usp, overrides, out)?;
            out.push_str("))");
        }
        Tag::ShWsh => {
            out.push_str("sh(wsh(");
            render_body(&node.body, default_usp, overrides, out)?;
            out.push_str("))");
        }
        Tag::Tr => {
            out.push_str("tr(");
            render_body(&node.body, default_usp, overrides, out)?;
            out.push(')');
        }
        other => return Err(CliError::TemplateParse(format!("unsupported tag in render: {other:?}"))),
    }
    Ok(())
}

fn render_body(
    body: &Body,
    default_usp: &UseSitePath,
    overrides: &Option<&[(u8, UseSitePath)]>,
    out: &mut String,
) -> Result<(), CliError> {
    match body {
        Body::SingleKey { idx } => render_key(*idx, default_usp, overrides, out),
        Body::Multi { k, sorted, indices } => {
            if *sorted { write!(out, "sortedmulti({k}").unwrap(); } else { write!(out, "multi({k}").unwrap(); }
            for &idx in indices {
                out.push(',');
                render_key(idx, default_usp, overrides, out)?;
            }
            out.push(')');
            Ok(())
        }
        Body::Tr { internal_idx, tree } => {
            render_key(*internal_idx, default_usp, overrides, out)?;
            if let Some(t) = tree {
                out.push(',');
                render_taptree(t, default_usp, overrides, out)?;
            }
            Ok(())
        }
    }
}

fn render_taptree(
    tt: &TapTreeNode,
    default_usp: &UseSitePath,
    overrides: &Option<&[(u8, UseSitePath)]>,
    out: &mut String,
) -> Result<(), CliError> {
    match tt {
        TapTreeNode::Leaf(node) => {
            // Render leaf body without a wrapper (it's already inside a tap-leaf position).
            render_body(&node.body, default_usp, overrides, out)
        }
        TapTreeNode::Branch { left, right } => {
            out.push('{');
            render_taptree(left, default_usp, overrides, out)?;
            out.push(',');
            render_taptree(right, default_usp, overrides, out)?;
            out.push('}');
            Ok(())
        }
    }
}

fn render_key(idx: u8, default_usp: &UseSitePath, overrides: &Option<&[(u8, UseSitePath)]>, out: &mut String) -> Result<(), CliError> {
    let usp = overrides.and_then(|v| v.iter().find(|(i, _)| *i == idx).map(|(_, u)| u)).unwrap_or(default_usp);
    write!(out, "@{idx}").unwrap();
    if let Some(alts) = &usp.multipath {
        out.push_str("/<");
        for (n, alt) in alts.iter().enumerate() {
            if n > 0 { out.push(';'); }
            write!(out, "{}", alt.value).unwrap();
            if alt.hardened { out.push('\''); }
        }
        out.push_str(">/*");
    } else {
        out.push_str("/*");
    }
    if usp.wildcard_hardened { out.push('\''); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::template::parse_template;

    #[test]
    fn roundtrip_wpkh_singlepath() {
        let t = "wpkh(@0/*)";
        let d = parse_template(t, &[], &[]).unwrap();
        assert_eq!(descriptor_to_template(&d).unwrap(), t);
    }

    #[test]
    fn roundtrip_wsh_multi_2of2() {
        let t = "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))";
        let d = parse_template(t, &[], &[]).unwrap();
        assert_eq!(descriptor_to_template(&d).unwrap(), t);
    }

    #[test]
    fn roundtrip_sh_wpkh() {
        let t = "sh(wpkh(@0/<0;1>/*))";
        let d = parse_template(t, &[], &[]).unwrap();
        assert_eq!(descriptor_to_template(&d).unwrap(), t);
    }

    #[test]
    fn roundtrip_tr_keyonly() {
        let t = "tr(@0/<0;1>/*)";
        let d = parse_template(t, &[], &[]).unwrap();
        assert_eq!(descriptor_to_template(&d).unwrap(), t);
    }
}
```

Create `crates/md-codec/src/bin/md/format/mod.rs`:

```rust
pub mod text;
```

Edit `crates/md-codec/src/bin/md/main.rs`, add `mod format;` after `mod parse;`.

- [ ] **Step 2: Run tests**

Run: `cargo test --features cli --bin md format::text::tests 2>&1 | tail -5`
Expected: 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/src/bin/md/format/mod.rs crates/md-codec/src/bin/md/format/text.rs crates/md-codec/src/bin/md/main.rs
git commit -m "$(cat <<'EOF'
feat(v0.15/phase-3): Descriptor → template string renderer

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 3.2: `format/text.rs` — identity hashes and chunk metadata display

**Files:**
- Modify: `crates/md-codec/src/bin/md/format/text.rs`

- [ ] **Step 1: Add display helpers + tests**

Append to `crates/md-codec/src/bin/md/format/text.rs`:

```rust
use md_codec::identity::{Md1EncodingId, WalletDescriptorTemplateId, WalletPolicyId};
use md_codec::chunk::ChunkHeader;

pub fn fmt_md1_id(id: &Md1EncodingId) -> String {
    let bytes = id.as_bytes();
    let mut s = String::with_capacity(64);
    for b in bytes { write!(s, "{b:02x}").unwrap(); }
    s
}
pub fn fmt_template_id(id: &WalletDescriptorTemplateId) -> String {
    let bytes = id.as_bytes();
    let mut s = String::with_capacity(64);
    for b in bytes { write!(s, "{b:02x}").unwrap(); }
    s
}
pub fn fmt_policy_id(id: &WalletPolicyId) -> String {
    let bytes = id.as_bytes();
    let mut s = String::with_capacity(32);
    for b in bytes { write!(s, "{b:02x}").unwrap(); }
    s
}
pub fn fmt_policy_id_fingerprint(id: &WalletPolicyId) -> String {
    let fp = id.fingerprint();
    format!("0x{:02x}{:02x}{:02x}{:02x}", fp[0], fp[1], fp[2], fp[3])
}
pub fn fmt_chunk_header(h: &ChunkHeader) -> String {
    format!("chunk-set-id=0x{:05x}, count={}, index={}", h.chunk_set_id, h.count, h.index)
}

#[cfg(test)]
mod hash_tests {
    use super::*;

    #[test]
    fn policy_id_fingerprint_format() {
        let bytes = [0x9E, 0x1D, 0x72, 0xB6, 0x00, 0,0,0, 0,0,0,0, 0,0,0,0];
        let id = WalletPolicyId::from_bytes(bytes);
        assert_eq!(fmt_policy_id_fingerprint(&id), "0x9e1d72b6");
    }
}
```

(If `WalletPolicyId::from_bytes` is `pub(crate)` or absent, the test should construct via `compute_wallet_policy_id` against a fixed `Descriptor`. Read `identity.rs` to find the right constructor; the test stays.)

- [ ] **Step 2: Run tests**

Run: `cargo test --features cli --bin md format::text::hash_tests 2>&1 | tail -5`
Expected: 1 test passes.

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/src/bin/md/format/text.rs
git commit -m "$(cat <<'EOF'
feat(v0.15/phase-3): identity-hash and chunk-header display helpers

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 3.3: Phase 3 ship tag

- [ ] **Step 1: Run full suite**

```bash
cargo test --workspace --features cli,json 2>&1 | grep -E '^test result' | awk '{ok+=$4} END {print "Total ok:", ok}'
# Expect: ~296 (291 + 5 from Phase 3)
```

- [ ] **Step 2: Empty ship-tag commit**

```bash
git commit --allow-empty -m "$(cat <<'EOF'
chore(v0.15/phase-3): ship — text formatter

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 4 — Read/write subcommands (text mode only; JSON in Phase 5)

Phase goal: `md encode`, `md decode`, `md verify`, `md inspect`, `md bytecode` all work in text mode end-to-end. `md vectors` and `md compile` are still stubs.

### Task 4.1: `cmd/encode.rs`

**Files:**
- Create: `crates/md-codec/src/bin/md/cmd/mod.rs`
- Create: `crates/md-codec/src/bin/md/cmd/encode.rs`
- Modify: `crates/md-codec/src/bin/md/main.rs` (add `mod cmd;`, wire encode dispatch)

- [ ] **Step 1: Write the encode command + integration test**

Create `crates/md-codec/src/bin/md/cmd/encode.rs`:

```rust
use crate::error::CliError;
use crate::format::text;
use crate::parse::keys::{parse_fingerprint, parse_key, ScriptCtx};
use crate::parse::template::{ctx_for_template, parse_template};

use md_codec::encode::encode_md1_string;
use md_codec::chunk::{derive_chunk_set_id, split};
use md_codec::identity::compute_wallet_policy_id;

pub struct EncodeArgs<'a> {
    pub template: &'a str,
    pub keys: &'a [String],
    pub fingerprints: &'a [String],
    pub force_chunked: bool,
    pub force_long_code: bool,
    pub policy_id_fingerprint: bool,
}

pub fn run(args: EncodeArgs<'_>) -> Result<(), CliError> {
    let ctx = match ctx_for_template(args.template) {
        ScriptCtx::SingleSig => ScriptCtx::SingleSig,
        ScriptCtx::MultiSig => ScriptCtx::MultiSig,
    };
    let parsed_keys = args.keys.iter().map(|k| parse_key(k, ctx)).collect::<Result<Vec<_>, _>>()?;
    let parsed_fps = args.fingerprints.iter().map(|s| parse_fingerprint(s)).collect::<Result<Vec<_>, _>>()?;
    let descriptor = parse_template(args.template, &parsed_keys, &parsed_fps)?;

    let phrase = encode_md1_string(&descriptor)?;
    println!("{phrase}");

    if args.force_chunked {
        // Split into chunks and emit one per line; print chunk-set-id header.
        let chunks = split(&descriptor, /*max_chunk_bytes=*/ 32)?;
        let csid = chunks.first().map(|c| c.header.chunk_set_id).unwrap_or(0);
        println!("chunk-set-id: 0x{csid:05x}");
        for c in &chunks {
            println!("{}", c.phrase);
        }
    }

    if args.policy_id_fingerprint {
        let id = compute_wallet_policy_id(&descriptor)?;
        println!("policy-id-fingerprint: {}", text::fmt_policy_id_fingerprint(&id));
    }

    Ok(())
}
```

Create `crates/md-codec/src/bin/md/cmd/mod.rs`:

```rust
pub mod encode;
```

Edit `crates/md-codec/src/bin/md/main.rs`:
- Add `mod cmd;` after `mod format;`.
- In `dispatch()`, replace the `Command::Encode { .. } => unimplemented!()` arm with:

```rust
Command::Encode {
    template, from_policy: _, context: _, path: _,
    keys, fingerprints, force_chunked, force_long_code,
    policy_id_fingerprint, json: _,
} => {
    let template = template.ok_or_else(|| CliError::BadArg(
        "encode: TEMPLATE required (or use --from-policy with cli-compiler)".into()
    ))?;
    cmd::encode::run(cmd::encode::EncodeArgs {
        template: &template,
        keys: &keys,
        fingerprints: &fingerprints,
        force_chunked,
        force_long_code,
        policy_id_fingerprint,
    })
}
```

Create `crates/md-codec/tests/cmd_encode.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn encode_template_only_emits_a_phrase() {
    let mut cmd = Command::cargo_bin("md").unwrap();
    cmd.args(["encode", "wpkh(@0/<0;1>/*)"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("md "));
}

#[test]
fn encode_with_policy_id_fingerprint_prints_two_lines() {
    let mut cmd = Command::cargo_bin("md").unwrap();
    cmd.args(["encode", "wpkh(@0/<0;1>/*)", "--policy-id-fingerprint"])
        .assert()
        .success()
        .stdout(predicate::str::contains("policy-id-fingerprint: 0x"));
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --features cli --test cmd_encode 2>&1 | tail -10`
Expected: 2 tests pass. (If miniscript can't parse the synthetic xpub at `pkh` depth 3, adjust `synthetic_xpub_for` to vary depth by context — likely needs a SingleSig path. Address as a fix-forward in this same task.)

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/src/bin/md/cmd/mod.rs crates/md-codec/src/bin/md/cmd/encode.rs crates/md-codec/src/bin/md/main.rs crates/md-codec/tests/cmd_encode.rs
git commit -m "$(cat <<'EOF'
feat(v0.15/phase-4): md encode (text mode)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 4.2: `cmd/decode.rs`

**Files:**
- Create: `crates/md-codec/src/bin/md/cmd/decode.rs`
- Modify: `crates/md-codec/src/bin/md/cmd/mod.rs`
- Modify: `crates/md-codec/src/bin/md/main.rs`

- [ ] **Step 1: Write the decode command + tests**

Create `crates/md-codec/src/bin/md/cmd/decode.rs`:

```rust
use crate::error::CliError;
use crate::format::text;
use md_codec::decode::decode_md1_string;
use md_codec::chunk::reassemble;

pub fn run(strings: &[String]) -> Result<(), CliError> {
    let descriptor = if strings.len() == 1 {
        decode_md1_string(&strings[0])?
    } else {
        reassemble(strings.iter().map(String::as_str))?
    };
    let template = text::descriptor_to_template(&descriptor)?;
    println!("{template}");
    Ok(())
}
```

Edit `crates/md-codec/src/bin/md/cmd/mod.rs` to:

```rust
pub mod decode;
pub mod encode;
```

Edit `crates/md-codec/src/bin/md/main.rs`, replace `Command::Decode { .. } => unimplemented!()`:

```rust
Command::Decode { strings, json: _ } => cmd::decode::run(&strings),
```

Create `crates/md-codec/tests/cmd_decode.rs`:

```rust
use assert_cmd::Command;
use std::process::Command as StdCommand;

fn encode(template: &str) -> String {
    let out = StdCommand::new(assert_cmd::cargo::cargo_bin("md"))
        .args(["encode", template])
        .output()
        .unwrap();
    let s = String::from_utf8(out.stdout).unwrap();
    s.lines().next().unwrap().to_string()
}

#[test]
fn decode_round_trips_to_template() {
    let template = "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))";
    let phrase = encode(template);
    let mut cmd = Command::cargo_bin("md").unwrap();
    cmd.args(["decode", &phrase]).assert().success().stdout(predicates::str::contains(template));
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --features cli --test cmd_decode 2>&1 | tail -5`
Expected: 1 test passes.

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/src/bin/md/cmd/decode.rs crates/md-codec/src/bin/md/cmd/mod.rs crates/md-codec/src/bin/md/main.rs crates/md-codec/tests/cmd_decode.rs
git commit -m "$(cat <<'EOF'
feat(v0.15/phase-4): md decode (text mode)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 4.3: `cmd/verify.rs`

**Files:**
- Create: `crates/md-codec/src/bin/md/cmd/verify.rs`
- Modify: `crates/md-codec/src/bin/md/cmd/mod.rs`
- Modify: `crates/md-codec/src/bin/md/main.rs`

- [ ] **Step 1: Implement verify + tests**

Create `crates/md-codec/src/bin/md/cmd/verify.rs`:

```rust
use crate::error::CliError;
use crate::parse::keys::{parse_fingerprint, parse_key};
use crate::parse::template::{ctx_for_template, parse_template};
use md_codec::decode::decode_md1_string;
use md_codec::chunk::reassemble;
use md_codec::encode::encode_payload;

pub struct VerifyArgs<'a> {
    pub strings: &'a [String],
    pub template: &'a str,
    pub keys: &'a [String],
    pub fingerprints: &'a [String],
}

pub fn run(args: VerifyArgs<'_>) -> Result<(), CliError> {
    let decoded = if args.strings.len() == 1 {
        decode_md1_string(&args.strings[0])?
    } else {
        reassemble(args.strings.iter().map(String::as_str))?
    };
    let ctx = ctx_for_template(args.template);
    let parsed_keys = args.keys.iter().map(|k| parse_key(k, ctx)).collect::<Result<Vec<_>, _>>()?;
    let parsed_fps = args.fingerprints.iter().map(|s| parse_fingerprint(s)).collect::<Result<Vec<_>, _>>()?;
    let expected = parse_template(args.template, &parsed_keys, &parsed_fps)?;
    let (decoded_bytes, decoded_bits) = encode_payload(&decoded)?;
    let (expected_bytes, expected_bits) = encode_payload(&expected)?;
    if decoded_bytes != expected_bytes || decoded_bits != expected_bits {
        return Err(CliError::Mismatch(format!(
            "expected {expected_bits}-bit payload, got {decoded_bits}-bit ({} vs {} bytes)",
            expected_bytes.len(), decoded_bytes.len()
        )));
    }
    println!("OK");
    Ok(())
}
```

Edit `crates/md-codec/src/bin/md/cmd/mod.rs`:

```rust
pub mod decode;
pub mod encode;
pub mod verify;
```

Edit `crates/md-codec/src/bin/md/main.rs`, replace verify arm:

```rust
Command::Verify { strings, template, keys, fingerprints } => cmd::verify::run(cmd::verify::VerifyArgs {
    strings: &strings,
    template: &template,
    keys: &keys,
    fingerprints: &fingerprints,
}),
```

Create `crates/md-codec/tests/cmd_verify.rs`:

```rust
use assert_cmd::Command;
use std::process::Command as StdCommand;

fn encode(template: &str) -> String {
    let out = StdCommand::new(assert_cmd::cargo::cargo_bin("md"))
        .args(["encode", template])
        .output()
        .unwrap();
    String::from_utf8(out.stdout).unwrap().lines().next().unwrap().to_string()
}

#[test]
fn verify_match_returns_0() {
    let template = "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))";
    let phrase = encode(template);
    Command::cargo_bin("md").unwrap()
        .args(["verify", &phrase, "--template", template])
        .assert().code(0).stdout(predicates::str::contains("OK"));
}

#[test]
fn verify_mismatch_returns_1() {
    let template = "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))";
    let phrase = encode(template);
    let wrong = "wpkh(@0/<0;1>/*)";
    Command::cargo_bin("md").unwrap()
        .args(["verify", &phrase, "--template", wrong])
        .assert().code(1).stderr(predicates::str::contains("MISMATCH"));
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --features cli --test cmd_verify 2>&1 | tail -5`
Expected: 2 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/src/bin/md/cmd/verify.rs crates/md-codec/src/bin/md/cmd/mod.rs crates/md-codec/src/bin/md/main.rs crates/md-codec/tests/cmd_verify.rs
git commit -m "$(cat <<'EOF'
feat(v0.15/phase-4): md verify with OK/MISMATCH and exit codes

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 4.4: `cmd/inspect.rs`

**Files:**
- Create: `crates/md-codec/src/bin/md/cmd/inspect.rs`
- Modify: `crates/md-codec/src/bin/md/cmd/mod.rs`
- Modify: `crates/md-codec/src/bin/md/main.rs`

- [ ] **Step 1: Implement inspect + tests**

Create `crates/md-codec/src/bin/md/cmd/inspect.rs`:

```rust
use crate::error::CliError;
use crate::format::text;
use md_codec::decode::decode_md1_string;
use md_codec::chunk::reassemble;
use md_codec::identity::{compute_md1_encoding_id, compute_wallet_descriptor_template_id, compute_wallet_policy_id};

pub fn run(strings: &[String]) -> Result<(), CliError> {
    let descriptor = if strings.len() == 1 {
        decode_md1_string(&strings[0])?
    } else {
        reassemble(strings.iter().map(String::as_str))?
    };

    println!("template: {}", text::descriptor_to_template(&descriptor)?);
    println!("n: {}", descriptor.n);
    println!("wallet-policy-mode: {}", descriptor.is_wallet_policy());

    let md1 = compute_md1_encoding_id(&descriptor)?;
    println!("md1-encoding-id: {}", text::fmt_md1_id(&md1));

    let tpl = compute_wallet_descriptor_template_id(&descriptor)?;
    println!("wallet-descriptor-template-id: {}", text::fmt_template_id(&tpl));

    let pid = compute_wallet_policy_id(&descriptor)?;
    println!("wallet-policy-id: {}", text::fmt_policy_id(&pid));
    println!("wallet-policy-id-fingerprint: {}", text::fmt_policy_id_fingerprint(&pid));

    Ok(())
}
```

Edit `crates/md-codec/src/bin/md/cmd/mod.rs`:

```rust
pub mod decode;
pub mod encode;
pub mod inspect;
pub mod verify;
```

Edit `crates/md-codec/src/bin/md/main.rs`, replace inspect arm:

```rust
Command::Inspect { strings, json: _ } => cmd::inspect::run(&strings),
```

Create `crates/md-codec/tests/cmd_inspect.rs`:

```rust
use assert_cmd::Command;
use std::process::Command as StdCommand;

fn encode(template: &str) -> String {
    let out = StdCommand::new(assert_cmd::cargo::cargo_bin("md"))
        .args(["encode", template])
        .output().unwrap();
    String::from_utf8(out.stdout).unwrap().lines().next().unwrap().to_string()
}

#[test]
fn inspect_prints_all_fields() {
    let phrase = encode("wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))");
    Command::cargo_bin("md").unwrap()
        .args(["inspect", &phrase])
        .assert().success()
        .stdout(predicates::str::contains("template:"))
        .stdout(predicates::str::contains("md1-encoding-id:"))
        .stdout(predicates::str::contains("wallet-policy-id-fingerprint: 0x"));
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --features cli --test cmd_inspect 2>&1 | tail -5`
Expected: 1 test passes.

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/src/bin/md/cmd/inspect.rs crates/md-codec/src/bin/md/cmd/mod.rs crates/md-codec/src/bin/md/main.rs crates/md-codec/tests/cmd_inspect.rs
git commit -m "$(cat <<'EOF'
feat(v0.15/phase-4): md inspect — full Descriptor + identity hashes

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 4.5: `cmd/bytecode.rs`

**Files:**
- Create: `crates/md-codec/src/bin/md/cmd/bytecode.rs`
- Modify: `crates/md-codec/src/bin/md/cmd/mod.rs`
- Modify: `crates/md-codec/src/bin/md/main.rs`

- [ ] **Step 1: Implement bytecode + tests**

Create `crates/md-codec/src/bin/md/cmd/bytecode.rs`:

```rust
use crate::error::CliError;
use md_codec::decode::decode_md1_string;
use md_codec::chunk::reassemble;
use md_codec::encode::encode_payload;

pub fn run(strings: &[String]) -> Result<(), CliError> {
    let descriptor = if strings.len() == 1 {
        decode_md1_string(&strings[0])?
    } else {
        reassemble(strings.iter().map(String::as_str))?
    };
    let (bytes, bit_len) = encode_payload(&descriptor)?;
    println!("payload-bits: {bit_len}");
    println!("payload-bytes: {}", bytes.len());
    print!("hex: ");
    for b in &bytes { print!("{b:02x}"); }
    println!();
    Ok(())
}
```

Edit `crates/md-codec/src/bin/md/cmd/mod.rs`:

```rust
pub mod bytecode;
pub mod decode;
pub mod encode;
pub mod inspect;
pub mod verify;
```

Edit `crates/md-codec/src/bin/md/main.rs`, replace bytecode arm:

```rust
Command::Bytecode { strings, json: _ } => cmd::bytecode::run(&strings),
```

Create `crates/md-codec/tests/cmd_bytecode.rs`:

```rust
use assert_cmd::Command;
use std::process::Command as StdCommand;

fn encode(template: &str) -> String {
    let out = StdCommand::new(assert_cmd::cargo::cargo_bin("md"))
        .args(["encode", template])
        .output().unwrap();
    String::from_utf8(out.stdout).unwrap().lines().next().unwrap().to_string()
}

#[test]
fn bytecode_prints_hex_and_lengths() {
    let phrase = encode("wpkh(@0/<0;1>/*)");
    Command::cargo_bin("md").unwrap()
        .args(["bytecode", &phrase])
        .assert().success()
        .stdout(predicates::str::contains("payload-bits:"))
        .stdout(predicates::str::contains("payload-bytes:"))
        .stdout(predicates::str::contains("hex:"));
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --features cli --test cmd_bytecode 2>&1 | tail -5`
Expected: 1 test passes.

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/src/bin/md/cmd/bytecode.rs crates/md-codec/src/bin/md/cmd/mod.rs crates/md-codec/src/bin/md/main.rs crates/md-codec/tests/cmd_bytecode.rs
git commit -m "$(cat <<'EOF'
feat(v0.15/phase-4): md bytecode — annotated payload-byte dump

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 4.6: Phase 4 ship tag

- [ ] **Step 1: Run full suite**

```bash
cargo test --workspace --features cli,json 2>&1 | grep -E '^test result' | awk '{ok+=$4} END {print "Total ok:", ok}'
# Expect: ~303 (296 + 7 from Phase 4)
```

- [ ] **Step 2: Empty ship-tag commit**

```bash
git commit --allow-empty -m "$(cat <<'EOF'
chore(v0.15/phase-4): ship — encode/decode/verify/inspect/bytecode in text mode

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 5 — JSON shadow types and `--json` flag wiring

Phase goal: every read/write subcommand respects `--json` and emits a versioned schema. All shadow types live in `format/json.rs`; library remains serde-free.

### Task 5.1: Schema constant and base shadow types

**Files:**
- Create: `crates/md-codec/src/bin/md/format/json.rs`
- Modify: `crates/md-codec/src/bin/md/format/mod.rs`

- [ ] **Step 1: Write base shadows + tests**

Create `crates/md-codec/src/bin/md/format/json.rs`:

```rust
use serde::Serialize;
use md_codec::header::Header;
use md_codec::chunk::ChunkHeader;
use md_codec::identity::{Md1EncodingId, WalletDescriptorTemplateId, WalletPolicyId};

pub const SCHEMA: &str = "md-cli/1";

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes { use std::fmt::Write as _; write!(s, "{b:02x}").unwrap(); }
    s
}

#[derive(Serialize)]
pub struct JsonHeader {
    pub version: u8,
    pub divergent_paths: bool,
}
impl From<&Header> for JsonHeader {
    fn from(h: &Header) -> Self {
        Self { version: h.version, divergent_paths: h.divergent_paths }
    }
}

#[derive(Serialize)]
pub struct JsonChunkHeader {
    pub version: u8,
    pub chunk_set_id: String,
    pub count: u8,
    pub index: u8,
}
impl From<&ChunkHeader> for JsonChunkHeader {
    fn from(h: &ChunkHeader) -> Self {
        Self {
            version: h.version,
            chunk_set_id: format!("0x{:05x}", h.chunk_set_id),
            count: h.count,
            index: h.index,
        }
    }
}

#[derive(Serialize)]
pub struct JsonHash {
    pub hex: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
}
impl From<&Md1EncodingId> for JsonHash {
    fn from(id: &Md1EncodingId) -> Self {
        Self { hex: hex(id.as_bytes()), fingerprint: None }
    }
}
impl From<&WalletDescriptorTemplateId> for JsonHash {
    fn from(id: &WalletDescriptorTemplateId) -> Self {
        Self { hex: hex(id.as_bytes()), fingerprint: None }
    }
}
impl From<&WalletPolicyId> for JsonHash {
    fn from(id: &WalletPolicyId) -> Self {
        let fp = id.fingerprint();
        Self {
            hex: hex(id.as_bytes()),
            fingerprint: Some(format!("0x{:02x}{:02x}{:02x}{:02x}", fp[0], fp[1], fp[2], fp[3])),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_constant() {
        assert_eq!(SCHEMA, "md-cli/1");
    }

    #[test]
    fn header_serializes() {
        let h = Header { version: 0, divergent_paths: false };
        let v = serde_json::to_value(JsonHeader::from(&h)).unwrap();
        assert_eq!(v["version"], 0);
        assert_eq!(v["divergent_paths"], false);
    }

    #[test]
    fn chunk_header_csid_formatted() {
        let h = ChunkHeader { version: 0, chunk_set_id: 0xABCDE, count: 3, index: 1 };
        let v = serde_json::to_value(JsonChunkHeader::from(&h)).unwrap();
        assert_eq!(v["chunk_set_id"], "0xabcde");
    }
}
```

Edit `crates/md-codec/src/bin/md/format/mod.rs`:

```rust
#[cfg(feature = "json")]
pub mod json;
pub mod text;
```

- [ ] **Step 2: Run tests**

Run: `cargo test --features cli,json --bin md format::json::tests 2>&1 | tail -5`
Expected: 3 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/src/bin/md/format/json.rs crates/md-codec/src/bin/md/format/mod.rs
git commit -m "$(cat <<'EOF'
feat(v0.15/phase-5): JSON schema constant + base shadow types

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 5.2: Descriptor + Tree + TLV shadows (adjacent-tagged enums)

**Files:**
- Modify: `crates/md-codec/src/bin/md/format/json.rs`

- [ ] **Step 1: Add Descriptor / Tree / TLV shadows + tests**

Append to `crates/md-codec/src/bin/md/format/json.rs`:

```rust
use md_codec::encode::Descriptor;
use md_codec::tag::Tag;
use md_codec::tree::{Body, Node, TapTreeNode};
use md_codec::tlv::TlvSection;
use md_codec::origin_path::{PathDecl, PathDeclPaths};
use md_codec::use_site_path::UseSitePath;

#[derive(Serialize)]
pub struct JsonDescriptor {
    pub n: u8,
    pub path_decl: JsonPathDecl,
    pub use_site_path: JsonUseSitePath,
    pub tree: JsonNode,
    pub tlv: JsonTlv,
}
impl From<&Descriptor> for JsonDescriptor {
    fn from(d: &Descriptor) -> Self {
        Self {
            n: d.n,
            path_decl: (&d.path_decl).into(),
            use_site_path: (&d.use_site_path).into(),
            tree: (&d.tree).into(),
            tlv: (&d.tlv).into(),
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "tag", content = "data")]
pub enum JsonPathDecl {
    Single(String),
    Divergent(Vec<String>),
}
impl From<&PathDecl> for JsonPathDecl {
    fn from(p: &PathDecl) -> Self {
        match &p.paths {
            PathDeclPaths::Single(d) => JsonPathDecl::Single(d.to_string()),
            PathDeclPaths::Divergent(v) => JsonPathDecl::Divergent(v.iter().map(|d| d.to_string()).collect()),
        }
    }
}

#[derive(Serialize)]
pub struct JsonUseSitePath {
    pub multipath: Option<Vec<JsonAlt>>,
    pub wildcard_hardened: bool,
}
#[derive(Serialize)]
pub struct JsonAlt { pub hardened: bool, pub value: u32 }
impl From<&UseSitePath> for JsonUseSitePath {
    fn from(u: &UseSitePath) -> Self {
        Self {
            multipath: u.multipath.as_ref().map(|alts| alts.iter().map(|a| JsonAlt { hardened: a.hardened, value: a.value }).collect()),
            wildcard_hardened: u.wildcard_hardened,
        }
    }
}

#[derive(Serialize)]
pub struct JsonNode {
    pub tag: String,
    pub body: JsonBody,
}
impl From<&Node> for JsonNode {
    fn from(n: &Node) -> Self {
        Self { tag: format!("{:?}", n.tag), body: (&n.body).into() }
    }
}

#[derive(Serialize)]
#[serde(tag = "kind", content = "data")]
pub enum JsonBody {
    SingleKey { idx: u8 },
    Multi { k: u8, sorted: bool, indices: Vec<u8> },
    Tr { internal_idx: u8, tree: Option<JsonTapTree> },
}
impl From<&Body> for JsonBody {
    fn from(b: &Body) -> Self {
        match b {
            Body::SingleKey { idx } => JsonBody::SingleKey { idx: *idx },
            Body::Multi { k, sorted, indices } => JsonBody::Multi { k: *k, sorted: *sorted, indices: indices.clone() },
            Body::Tr { internal_idx, tree } => JsonBody::Tr {
                internal_idx: *internal_idx,
                tree: tree.as_ref().map(|t| t.into()),
            },
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "kind", content = "data")]
pub enum JsonTapTree {
    Leaf(JsonNode),
    Branch { left: Box<JsonTapTree>, right: Box<JsonTapTree> },
}
impl From<&TapTreeNode> for JsonTapTree {
    fn from(t: &TapTreeNode) -> Self {
        match t {
            TapTreeNode::Leaf(node) => JsonTapTree::Leaf(node.into()),
            TapTreeNode::Branch { left, right } => JsonTapTree::Branch {
                left: Box::new(left.as_ref().into()),
                right: Box::new(right.as_ref().into()),
            },
        }
    }
}

#[derive(Serialize, Default)]
pub struct JsonTlv {
    pub use_site_path_overrides: Option<Vec<(u8, JsonUseSitePath)>>,
    pub fingerprints: Option<Vec<(u8, String)>>,
    pub pubkeys: Option<Vec<(u8, String)>>,
    pub origin_path_overrides: Option<Vec<(u8, String)>>,
}
impl From<&TlvSection> for JsonTlv {
    fn from(t: &TlvSection) -> Self {
        Self {
            use_site_path_overrides: t.use_site_path_overrides.as_ref().map(|v| v.iter().map(|(i, u)| (*i, u.into())).collect()),
            fingerprints: t.fingerprints.as_ref().map(|v| v.iter().map(|(i, fp)| (*i, hex(fp))).collect()),
            pubkeys: t.pubkeys.as_ref().map(|v| v.iter().map(|(i, p)| (*i, hex(p))).collect()),
            origin_path_overrides: t.origin_path_overrides.as_ref().map(|v| v.iter().map(|(i, d)| (*i, d.to_string())).collect()),
        }
    }
}

#[cfg(test)]
mod descriptor_json_tests {
    use super::*;
    use crate::parse::template::parse_template;

    #[test]
    fn wsh_multi_serializes() {
        let d = parse_template("wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))", &[], &[]).unwrap();
        let j = serde_json::to_value(JsonDescriptor::from(&d)).unwrap();
        assert_eq!(j["n"], 2);
        assert_eq!(j["tree"]["tag"], "Wsh");
        assert_eq!(j["tree"]["body"]["kind"], "Multi");
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --features cli,json --bin md format::json::descriptor_json_tests 2>&1 | tail -5`
Expected: 1 test passes.

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/src/bin/md/format/json.rs
git commit -m "$(cat <<'EOF'
feat(v0.15/phase-5): JsonDescriptor / JsonNode / JsonBody / JsonTlv shadows

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 5.3: Wire `--json` through encode

**Files:**
- Modify: `crates/md-codec/src/bin/md/cmd/encode.rs`
- Modify: `crates/md-codec/src/bin/md/main.rs`
- Modify: `crates/md-codec/tests/cmd_encode.rs`

- [ ] **Step 1: Update encode to accept and emit JSON**

Edit `crates/md-codec/src/bin/md/cmd/encode.rs`. Add field `json: bool` to `EncodeArgs` and add a JSON-emit branch:

```rust
pub struct EncodeArgs<'a> {
    pub template: &'a str,
    pub keys: &'a [String],
    pub fingerprints: &'a [String],
    pub force_chunked: bool,
    pub force_long_code: bool,
    pub policy_id_fingerprint: bool,
    pub json: bool,
}

pub fn run(args: EncodeArgs<'_>) -> Result<(), CliError> {
    // ... existing parsing ...
    let parsed_keys = args.keys.iter().map(|k| parse_key(k, ctx)).collect::<Result<Vec<_>, _>>()?;
    let parsed_fps = args.fingerprints.iter().map(|s| parse_fingerprint(s)).collect::<Result<Vec<_>, _>>()?;
    let descriptor = parse_template(args.template, &parsed_keys, &parsed_fps)?;
    let phrase = encode_md1_string(&descriptor)?;

    #[cfg(feature = "json")]
    if args.json {
        use crate::format::json::SCHEMA;
        let id = compute_wallet_policy_id(&descriptor)?;
        let mut obj = serde_json::Map::new();
        obj.insert("schema".into(), SCHEMA.into());
        obj.insert("phrase".into(), phrase.into());
        if args.policy_id_fingerprint {
            obj.insert("policy_id_fingerprint".into(),
                crate::format::text::fmt_policy_id_fingerprint(&id).into());
        }
        println!("{}", serde_json::to_string_pretty(&obj).unwrap());
        return Ok(());
    }

    println!("{phrase}");
    if args.policy_id_fingerprint {
        let id = compute_wallet_policy_id(&descriptor)?;
        println!("policy-id-fingerprint: {}", text::fmt_policy_id_fingerprint(&id));
    }
    Ok(())
}
```

Edit `crates/md-codec/src/bin/md/main.rs`, add `json` to the encode dispatch:

```rust
Command::Encode { template, from_policy: _, context: _, path: _, keys, fingerprints,
                  force_chunked, force_long_code, policy_id_fingerprint, json } => {
    let template = template.ok_or_else(|| CliError::BadArg(
        "encode: TEMPLATE required (or use --from-policy with cli-compiler)".into()))?;
    cmd::encode::run(cmd::encode::EncodeArgs {
        template: &template, keys: &keys, fingerprints: &fingerprints,
        force_chunked, force_long_code, policy_id_fingerprint, json,
    })
}
```

Append to `crates/md-codec/tests/cmd_encode.rs`:

```rust
#[test]
fn encode_json_has_schema_and_phrase() {
    Command::cargo_bin("md").unwrap()
        .args(["encode", "wpkh(@0/<0;1>/*)", "--json"])
        .assert().success()
        .stdout(predicate::str::contains("\"schema\": \"md-cli/1\""))
        .stdout(predicate::str::contains("\"phrase\":"));
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --features cli,json --test cmd_encode 2>&1 | tail -5`
Expected: 3 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/src/bin/md/cmd/encode.rs crates/md-codec/src/bin/md/main.rs crates/md-codec/tests/cmd_encode.rs
git commit -m "$(cat <<'EOF'
feat(v0.15/phase-5): wire --json through md encode

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 5.4: Wire `--json` through decode

**Files:**
- Modify: `crates/md-codec/src/bin/md/cmd/decode.rs`
- Modify: `crates/md-codec/src/bin/md/main.rs`
- Modify: `crates/md-codec/tests/cmd_decode.rs`

- [ ] **Step 1: Add JSON branch to decode**

Replace `crates/md-codec/src/bin/md/cmd/decode.rs`:

```rust
use crate::error::CliError;
use crate::format::text;
use md_codec::decode::decode_md1_string;
use md_codec::chunk::reassemble;

pub fn run(strings: &[String], json: bool) -> Result<(), CliError> {
    let descriptor = if strings.len() == 1 {
        decode_md1_string(&strings[0])?
    } else {
        reassemble(strings.iter().map(String::as_str))?
    };

    #[cfg(feature = "json")]
    if json {
        use crate::format::json::{JsonDescriptor, SCHEMA};
        let mut obj = serde_json::Map::new();
        obj.insert("schema".into(), SCHEMA.into());
        obj.insert("descriptor".into(), serde_json::to_value(JsonDescriptor::from(&descriptor)).unwrap());
        println!("{}", serde_json::to_string_pretty(&obj).unwrap());
        return Ok(());
    }
    let _ = json;

    let template = text::descriptor_to_template(&descriptor)?;
    println!("{template}");
    Ok(())
}
```

Edit `crates/md-codec/src/bin/md/main.rs`, replace decode arm:

```rust
Command::Decode { strings, json } => cmd::decode::run(&strings, json),
```

Append to `crates/md-codec/tests/cmd_decode.rs`:

```rust
#[test]
fn decode_json_emits_schema_and_descriptor() {
    let phrase = encode("wpkh(@0/<0;1>/*)");
    Command::cargo_bin("md").unwrap()
        .args(["decode", &phrase, "--json"])
        .assert().success()
        .stdout(predicates::str::contains("\"schema\": \"md-cli/1\""))
        .stdout(predicates::str::contains("\"descriptor\":"));
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --features cli,json --test cmd_decode 2>&1 | tail -5`
Expected: 2 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/src/bin/md/cmd/decode.rs crates/md-codec/src/bin/md/main.rs crates/md-codec/tests/cmd_decode.rs
git commit -m "$(cat <<'EOF'
feat(v0.15/phase-5): wire --json through md decode

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 5.5: Wire `--json` through inspect and bytecode

**Files:**
- Modify: `crates/md-codec/src/bin/md/cmd/inspect.rs`
- Modify: `crates/md-codec/src/bin/md/cmd/bytecode.rs`
- Modify: `crates/md-codec/src/bin/md/main.rs`
- Modify: `crates/md-codec/tests/cmd_inspect.rs`
- Modify: `crates/md-codec/tests/cmd_bytecode.rs`

- [ ] **Step 1: Add `json: bool` to inspect and bytecode signatures + JSON branches**

Replace the body of `crates/md-codec/src/bin/md/cmd/inspect.rs`:

```rust
use crate::error::CliError;
use crate::format::text;
use md_codec::decode::decode_md1_string;
use md_codec::chunk::reassemble;
use md_codec::identity::{compute_md1_encoding_id, compute_wallet_descriptor_template_id, compute_wallet_policy_id};

pub fn run(strings: &[String], json: bool) -> Result<(), CliError> {
    let descriptor = if strings.len() == 1 {
        decode_md1_string(&strings[0])?
    } else {
        reassemble(strings.iter().map(String::as_str))?
    };
    let md1 = compute_md1_encoding_id(&descriptor)?;
    let tpl = compute_wallet_descriptor_template_id(&descriptor)?;
    let pid = compute_wallet_policy_id(&descriptor)?;

    #[cfg(feature = "json")]
    if json {
        use crate::format::json::{JsonDescriptor, JsonHash, SCHEMA};
        let mut obj = serde_json::Map::new();
        obj.insert("schema".into(), SCHEMA.into());
        obj.insert("descriptor".into(), serde_json::to_value(JsonDescriptor::from(&descriptor)).unwrap());
        obj.insert("md1_encoding_id".into(), serde_json::to_value(JsonHash::from(&md1)).unwrap());
        obj.insert("wallet_descriptor_template_id".into(), serde_json::to_value(JsonHash::from(&tpl)).unwrap());
        obj.insert("wallet_policy_id".into(), serde_json::to_value(JsonHash::from(&pid)).unwrap());
        println!("{}", serde_json::to_string_pretty(&obj).unwrap());
        return Ok(());
    }
    let _ = json;

    println!("template: {}", text::descriptor_to_template(&descriptor)?);
    println!("n: {}", descriptor.n);
    println!("wallet-policy-mode: {}", descriptor.is_wallet_policy());
    println!("md1-encoding-id: {}", text::fmt_md1_id(&md1));
    println!("wallet-descriptor-template-id: {}", text::fmt_template_id(&tpl));
    println!("wallet-policy-id: {}", text::fmt_policy_id(&pid));
    println!("wallet-policy-id-fingerprint: {}", text::fmt_policy_id_fingerprint(&pid));
    Ok(())
}
```

Replace the body of `crates/md-codec/src/bin/md/cmd/bytecode.rs`:

```rust
use crate::error::CliError;
use md_codec::decode::decode_md1_string;
use md_codec::chunk::reassemble;
use md_codec::encode::encode_payload;

pub fn run(strings: &[String], json: bool) -> Result<(), CliError> {
    let descriptor = if strings.len() == 1 {
        decode_md1_string(&strings[0])?
    } else {
        reassemble(strings.iter().map(String::as_str))?
    };
    let (bytes, bit_len) = encode_payload(&descriptor)?;

    #[cfg(feature = "json")]
    if json {
        use crate::format::json::SCHEMA;
        let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        let v = serde_json::json!({
            "schema": SCHEMA,
            "payload_bits": bit_len,
            "payload_bytes": bytes.len(),
            "hex": hex,
        });
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
        return Ok(());
    }
    let _ = json;

    println!("payload-bits: {bit_len}");
    println!("payload-bytes: {}", bytes.len());
    print!("hex: ");
    for b in &bytes { print!("{b:02x}"); }
    println!();
    Ok(())
}
```

Edit `crates/md-codec/src/bin/md/main.rs`:

```rust
Command::Inspect { strings, json } => cmd::inspect::run(&strings, json),
Command::Bytecode { strings, json } => cmd::bytecode::run(&strings, json),
```

Append to `crates/md-codec/tests/cmd_inspect.rs`:

```rust
#[test]
fn inspect_json_has_schema_and_descriptor() {
    let phrase = encode("wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))");
    Command::cargo_bin("md").unwrap()
        .args(["inspect", &phrase, "--json"])
        .assert().success()
        .stdout(predicates::str::contains("\"schema\": \"md-cli/1\""))
        .stdout(predicates::str::contains("\"wallet_policy_id\":"));
}
```

Append to `crates/md-codec/tests/cmd_bytecode.rs`:

```rust
#[test]
fn bytecode_json_has_payload_fields() {
    let phrase = encode("wpkh(@0/<0;1>/*)");
    Command::cargo_bin("md").unwrap()
        .args(["bytecode", &phrase, "--json"])
        .assert().success()
        .stdout(predicates::str::contains("\"schema\": \"md-cli/1\""))
        .stdout(predicates::str::contains("\"payload_bytes\":"));
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --features cli,json --test cmd_inspect --test cmd_bytecode 2>&1 | tail -10`
Expected: 4 tests pass total (2 each).

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/src/bin/md/cmd/inspect.rs crates/md-codec/src/bin/md/cmd/bytecode.rs crates/md-codec/src/bin/md/main.rs crates/md-codec/tests/cmd_inspect.rs crates/md-codec/tests/cmd_bytecode.rs
git commit -m "$(cat <<'EOF'
feat(v0.15/phase-5): wire --json through inspect and bytecode

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 5.6: Phase 5 ship tag

- [ ] **Step 1: Run full suite**

```bash
cargo test --workspace --features cli,json 2>&1 | grep -E '^test result' | awk '{ok+=$4} END {print "Total ok:", ok}'
# Expect: ~314 (303 + 11 from Phase 5)
```

- [ ] **Step 2: Empty ship-tag commit**

```bash
git commit --allow-empty -m "$(cat <<'EOF'
chore(v0.15/phase-5): ship — JSON shadow types and --json on all read/write subcommands

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 6 — `vectors` subcommand

Phase goal: `md vectors --out tests/vectors/` produces a deterministic corpus of 12 entries spanning the spec's coverage table; the corpus is committed and CI checks for drift.

### Task 6.1: Manifest scaffold

**Files:**
- Create: `crates/md-codec/tests/vectors/manifest.rs`
- Create: `crates/md-codec/tests/vectors/.gitkeep`

- [ ] **Step 1: Manifest source**

Create `crates/md-codec/tests/vectors/manifest.rs`:

```rust
//! Vectors corpus source-of-truth. Used both by `md vectors` and by
//! `tests/template_roundtrip.rs`.

pub struct Vector {
    pub name: &'static str,
    pub template: &'static str,
    pub keys: &'static [(u8, &'static str)],
    pub fingerprints: &'static [(u8, [u8; 4])],
    pub force_chunked: bool,
}

pub const MANIFEST: &[Vector] = &[
    Vector { name: "wpkh_basic",         template: "wpkh(@0/<0;1>/*)",                                   keys: &[], fingerprints: &[], force_chunked: false },
    Vector { name: "pkh_basic",          template: "pkh(@0/<0;1>/*)",                                    keys: &[], fingerprints: &[], force_chunked: false },
    Vector { name: "wsh_multi_2of2",     template: "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))",                keys: &[], fingerprints: &[], force_chunked: false },
    Vector { name: "wsh_multi_2of3",     template: "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))",     keys: &[], fingerprints: &[], force_chunked: false },
    Vector { name: "wsh_sortedmulti",    template: "wsh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))", keys: &[], fingerprints: &[], force_chunked: false },
    Vector { name: "tr_keyonly",         template: "tr(@0/<0;1>/*)",                                     keys: &[], fingerprints: &[], force_chunked: false },
    Vector { name: "tr_with_leaf",       template: "tr(@0/<0;1>/*,pk(@1/<0;1>/*))",                      keys: &[], fingerprints: &[], force_chunked: false },
    Vector { name: "sh_wpkh",            template: "sh(wpkh(@0/<0;1>/*))",                               keys: &[], fingerprints: &[], force_chunked: false },
    Vector { name: "sh_wsh_multi",       template: "sh(wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*)))",            keys: &[], fingerprints: &[], force_chunked: false },
    Vector { name: "wsh_divergent_paths", template: "wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))",               keys: &[], fingerprints: &[], force_chunked: false },
    Vector { name: "wsh_with_fingerprints", template: "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))",
        keys: &[],
        fingerprints: &[(0, [0xDE,0xAD,0xBE,0xEF]), (1, [0xCA,0xFE,0xBA,0xBE])],
        force_chunked: false },
    Vector { name: "wsh_multi_chunked",  template: "wsh(multi(3,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))",     keys: &[], fingerprints: &[], force_chunked: true },
];
```

Create `crates/md-codec/tests/vectors/.gitkeep` (empty file) so the directory exists in git.

- [ ] **Step 2: Verify manifest compiles**

Run: `cargo check --workspace --features cli,json 2>&1 | tail -3`
Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/tests/vectors/manifest.rs crates/md-codec/tests/vectors/.gitkeep
git commit -m "$(cat <<'EOF'
feat(v0.15/phase-6): vectors manifest with 12 coverage entries

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 6.2: `cmd/vectors.rs` — generator

**Files:**
- Create: `crates/md-codec/src/bin/md/cmd/vectors.rs`
- Modify: `crates/md-codec/src/bin/md/cmd/mod.rs`
- Modify: `crates/md-codec/src/bin/md/main.rs`

- [ ] **Step 1: Implement vectors generator**

Create `crates/md-codec/src/bin/md/cmd/vectors.rs`:

```rust
use crate::error::CliError;
use crate::parse::keys::ParsedFingerprint;
use crate::parse::template::{ctx_for_template, parse_template};
use std::path::PathBuf;
use std::fs;

#[path = "../../../../tests/vectors/manifest.rs"]
mod manifest;
use manifest::MANIFEST;

pub fn run(out: Option<String>) -> Result<(), CliError> {
    let out_dir = PathBuf::from(out.unwrap_or_else(|| "crates/md-codec/tests/vectors".into()));
    fs::create_dir_all(&out_dir).map_err(|e| CliError::BadArg(format!("mkdir {out_dir:?}: {e}")))?;

    let mut entries: Vec<&manifest::Vector> = MANIFEST.iter().collect();
    entries.sort_by_key(|v| v.name);

    for v in entries {
        let _ctx = ctx_for_template(v.template);
        let fps: Vec<ParsedFingerprint> = v.fingerprints.iter().map(|(i, fp)| ParsedFingerprint { i: *i, fp: *fp }).collect();
        let descriptor = parse_template(v.template, &[], &fps)?;
        let (bytes, _bits) = md_codec::encode::encode_payload(&descriptor)?;
        let phrase = md_codec::encode::encode_md1_string(&descriptor)?;

        write_lf(&out_dir.join(format!("{}.template", v.name)), v.template)?;
        write_lf(&out_dir.join(format!("{}.bytes.hex", v.name)),
            &bytes.iter().map(|b| format!("{b:02x}")).collect::<String>())?;
        write_lf(&out_dir.join(format!("{}.phrase.txt", v.name)), &phrase)?;

        #[cfg(feature = "json")]
        {
            use crate::format::json::JsonDescriptor;
            let json = serde_json::to_string_pretty(&JsonDescriptor::from(&descriptor)).unwrap();
            write_lf(&out_dir.join(format!("{}.descriptor.json", v.name)), &json)?;
        }
    }
    Ok(())
}

fn write_lf(path: &std::path::Path, contents: &str) -> Result<(), CliError> {
    let mut s = contents.replace("\r\n", "\n");
    if !s.ends_with('\n') { s.push('\n'); }
    fs::write(path, s.as_bytes()).map_err(|e| CliError::BadArg(format!("write {path:?}: {e}")))
}
```

Edit `crates/md-codec/src/bin/md/cmd/mod.rs`:

```rust
pub mod bytecode;
pub mod decode;
pub mod encode;
pub mod inspect;
pub mod vectors;
pub mod verify;
```

Edit `crates/md-codec/src/bin/md/main.rs`, replace vectors arm:

```rust
Command::Vectors { out } => cmd::vectors::run(out),
```

- [ ] **Step 2: Generate corpus**

Run: `cargo run --features cli,json --bin md -- vectors`
Expected: command exits 0; `crates/md-codec/tests/vectors/` now contains ~48 files (12 entries × 4 files each).

- [ ] **Step 3: Commit generator + initial corpus**

```bash
git add crates/md-codec/src/bin/md/cmd/vectors.rs crates/md-codec/src/bin/md/cmd/mod.rs crates/md-codec/src/bin/md/main.rs crates/md-codec/tests/vectors/
git commit -m "$(cat <<'EOF'
feat(v0.15/phase-6): md vectors generator + initial 12-entry corpus

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 6.3: Phase 6 ship tag

- [ ] **Step 1: Run full suite**

```bash
cargo test --workspace --features cli,json 2>&1 | grep -E '^test result' | awk '{ok+=$4} END {print "Total ok:", ok}'
# Expect: still ~314 (no new tests; Phase 8 adds the corpus diff test)
```

- [ ] **Step 2: Empty ship-tag commit**

```bash
git commit --allow-empty -m "$(cat <<'EOF'
chore(v0.15/phase-6): ship — vectors subcommand + corpus

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 7 — `compile` subcommand (cli-compiler)

Phase goal: `md compile <EXPR> --context tap` and `md encode --from-policy <EXPR> --context tap` work behind the opt-in `cli-compiler` feature.

### Task 7.1: `compile.rs` — `compile_policy_to_template` + CompileError

**Files:**
- Create: `crates/md-codec/src/bin/md/compile.rs`
- Modify: `crates/md-codec/src/bin/md/main.rs` (add `#[cfg] mod compile;`)

- [ ] **Step 1: Implement compile_policy_to_template + tests**

Create `crates/md-codec/src/bin/md/compile.rs`:

```rust
use crate::error::CliError;
use std::str::FromStr;

#[derive(Debug)]
pub enum CompileError {
    Parse(String),
    Compile(String),
    BadContext(String),
}
impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::Parse(m) => write!(f, "parse: {m}"),
            CompileError::Compile(m) => write!(f, "compile: {m}"),
            CompileError::BadContext(m) => write!(f, "bad-context: {m}"),
        }
    }
}
impl From<CompileError> for CliError {
    fn from(e: CompileError) -> Self { CliError::Compile(e.to_string()) }
}

#[derive(Debug, Clone, Copy)]
pub enum ScriptContext { Tap, SegwitV0 }
impl FromStr for ScriptContext {
    type Err = CompileError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s { "tap" => Ok(Self::Tap), "segwitv0" => Ok(Self::SegwitV0),
                  other => Err(CompileError::BadContext(other.into())) }
    }
}

pub fn compile_policy_to_template(expr: &str, ctx: ScriptContext) -> Result<String, CompileError> {
    use miniscript::policy::concrete::Policy;
    let policy: Policy<String> = expr.parse().map_err(|e| CompileError::Parse(format!("{e}")))?;
    match ctx {
        ScriptContext::SegwitV0 => {
            let ms = policy.compile::<miniscript::Segwitv0>().map_err(|e| CompileError::Compile(format!("{e}")))?;
            Ok(format!("wsh({ms})"))
        }
        ScriptContext::Tap => {
            let ms = policy.compile::<miniscript::Tap>().map_err(|e| CompileError::Compile(format!("{e}")))?;
            Ok(format!("tr({ms})"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_segwitv0_pk() {
        let s = compile_policy_to_template("pk(@0)", ScriptContext::SegwitV0).unwrap();
        assert!(s.starts_with("wsh("));
        assert!(s.contains("@0"));
    }

    #[test]
    fn bad_context() {
        assert!("xpub".parse::<ScriptContext>().is_err());
    }
}
```

Edit `crates/md-codec/src/bin/md/main.rs`, add at the top after the other `mod` lines:

```rust
#[cfg(feature = "cli-compiler")]
mod compile;
```

- [ ] **Step 2: Run tests**

Run: `cargo test --features cli,json,cli-compiler --bin md compile::tests 2>&1 | tail -5`
Expected: 2 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/src/bin/md/compile.rs crates/md-codec/src/bin/md/main.rs
git commit -m "$(cat <<'EOF'
feat(v0.15/phase-7): compile_policy_to_template + CompileError

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 7.2: `cmd/compile.rs` subcommand

**Files:**
- Create: `crates/md-codec/src/bin/md/cmd/compile.rs`
- Modify: `crates/md-codec/src/bin/md/cmd/mod.rs`
- Modify: `crates/md-codec/src/bin/md/main.rs`

- [ ] **Step 1: Wire the compile subcommand**

Create `crates/md-codec/src/bin/md/cmd/compile.rs`:

```rust
use crate::error::CliError;
use crate::compile::{compile_policy_to_template, ScriptContext};

pub fn run(expr: &str, ctx_str: &str, json: bool) -> Result<(), CliError> {
    let ctx: ScriptContext = ctx_str.parse().map_err(|e: crate::compile::CompileError| {
        CliError::Compile(e.to_string())
    })?;
    let template = compile_policy_to_template(expr, ctx).map_err(CliError::from)?;

    #[cfg(feature = "json")]
    if json {
        use crate::format::json::SCHEMA;
        let v = serde_json::json!({ "schema": SCHEMA, "template": template, "context": ctx_str });
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
        return Ok(());
    }
    let _ = json;

    println!("{template}");
    Ok(())
}
```

Edit `crates/md-codec/src/bin/md/cmd/mod.rs`:

```rust
pub mod bytecode;
#[cfg(feature = "cli-compiler")]
pub mod compile;
pub mod decode;
pub mod encode;
pub mod inspect;
pub mod vectors;
pub mod verify;
```

Edit `crates/md-codec/src/bin/md/main.rs`, replace compile arm:

```rust
Command::Compile { expr, context, json } => {
    #[cfg(feature = "cli-compiler")]
    { cmd::compile::run(&expr, &context, json) }
    #[cfg(not(feature = "cli-compiler"))]
    { let _ = (expr, context, json); Err(CliError::BadArg(
        "compile requires the cli-compiler feature; rebuild with --features cli-compiler".into())) }
},
```

Create `crates/md-codec/tests/cmd_compile.rs`:

```rust
#![cfg(feature = "cli-compiler")]
use assert_cmd::Command;

#[test]
fn compile_pk_segwitv0() {
    Command::cargo_bin("md").unwrap()
        .args(["compile", "pk(@0)", "--context", "segwitv0"])
        .assert().success()
        .stdout(predicates::str::starts_with("wsh("));
}

#[test]
fn compile_json() {
    Command::cargo_bin("md").unwrap()
        .args(["compile", "pk(@0)", "--context", "segwitv0", "--json"])
        .assert().success()
        .stdout(predicates::str::contains("\"schema\": \"md-cli/1\""))
        .stdout(predicates::str::contains("\"template\":"));
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --features cli,json,cli-compiler --test cmd_compile 2>&1 | tail -5`
Expected: 2 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/src/bin/md/cmd/compile.rs crates/md-codec/src/bin/md/cmd/mod.rs crates/md-codec/src/bin/md/main.rs crates/md-codec/tests/cmd_compile.rs
git commit -m "$(cat <<'EOF'
feat(v0.15/phase-7): md compile subcommand (cli-compiler)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 7.3: `encode --from-policy` integration

**Files:**
- Modify: `crates/md-codec/src/bin/md/main.rs`
- Modify: `crates/md-codec/tests/cmd_encode.rs`

- [ ] **Step 1: Wire from-policy through encode dispatch**

Edit `crates/md-codec/src/bin/md/main.rs`, replace the encode arm to handle `from_policy`:

```rust
Command::Encode { template, from_policy, context, path: _, keys, fingerprints,
                  force_chunked, force_long_code, policy_id_fingerprint, json } => {
    let template_str: String = if let Some(expr) = from_policy {
        #[cfg(feature = "cli-compiler")]
        {
            let ctx: compile::ScriptContext = context
                .ok_or_else(|| CliError::BadArg("--from-policy requires --context tap|segwitv0".into()))?
                .parse().map_err(|e: compile::CompileError| CliError::Compile(e.to_string()))?;
            compile::compile_policy_to_template(&expr, ctx).map_err(CliError::from)?
        }
        #[cfg(not(feature = "cli-compiler"))]
        { let _ = (expr, context); return Err(CliError::BadArg(
            "--from-policy requires the cli-compiler feature".into())); }
    } else {
        template.ok_or_else(|| CliError::BadArg(
            "encode: TEMPLATE required (or use --from-policy with cli-compiler)".into()))?
    };
    cmd::encode::run(cmd::encode::EncodeArgs {
        template: &template_str, keys: &keys, fingerprints: &fingerprints,
        force_chunked, force_long_code, policy_id_fingerprint, json,
    })
}
```

Append to `crates/md-codec/tests/cmd_encode.rs`:

```rust
#[cfg(feature = "cli-compiler")]
#[test]
fn encode_from_policy_segwitv0() {
    Command::cargo_bin("md").unwrap()
        .args(["encode", "--from-policy", "pk(@0)", "--context", "segwitv0"])
        .assert().success()
        .stdout(predicate::str::starts_with("md "));
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --features cli,json,cli-compiler --test cmd_encode 2>&1 | tail -5`
Expected: 4 tests pass (3 existing + 1 new).

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/src/bin/md/main.rs crates/md-codec/tests/cmd_encode.rs
git commit -m "$(cat <<'EOF'
feat(v0.15/phase-7): encode --from-policy compile-and-encode shortcut

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 7.4: Phase 7 ship tag

- [ ] **Step 1: Run full suite**

```bash
cargo test --workspace --features cli,json,cli-compiler 2>&1 | grep -E '^test result' | awk '{ok+=$4} END {print "Total ok:", ok}'
# Expect: ~319 (314 + 5 from Phase 7)
```

- [ ] **Step 2: Empty ship-tag commit**

```bash
git commit --allow-empty -m "$(cat <<'EOF'
chore(v0.15/phase-7): ship — compile + encode --from-policy

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 8 — Test harnesses

Phase goal: drift-detection harnesses for help text, JSON schemas, round-trip, vector corpus, compiler determinism, and exit codes are all in place.

### Task 8.1: Help-example drift harness

**Files:**
- Modify: `crates/md-codec/src/bin/md/main.rs` (add `after_long_help` blocks)
- Create: `crates/md-codec/tests/help_examples.rs`

- [ ] **Step 1: Add `after_long_help` to each subcommand**

Edit `crates/md-codec/src/bin/md/main.rs`. For each subcommand variant, add `after_long_help` to the `clap::Subcommand` derive. Example pattern (apply analogously to each subcommand):

```rust
#[derive(Debug, Subcommand)]
enum Command {
    /// Encode a wallet policy into MD backup string(s).
    #[command(after_long_help = "EXAMPLES:\n  $ md encode 'wpkh(@0/<0;1>/*)'\n  md heir afford coffee chase canvas neck ozone broken below trick clutch")]
    Encode { /* ...as before... */ },

    #[command(after_long_help = "EXAMPLES:\n  $ md decode 'md heir afford coffee chase canvas neck ozone broken below trick clutch'\n  wpkh(@0/<0;1>/*)")]
    Decode { /* ... */ },

    // and so on for verify/inspect/bytecode/vectors/compile.
}
```

Note: the literal "EXAMPLES" output strings will need to be regenerated against actual `md encode` output once Phase 4 is done; the harness in Step 2 fails until they match.

Create `crates/md-codec/tests/help_examples.rs`:

```rust
use assert_cmd::Command;

fn long_help(sub: &str) -> String {
    let out = Command::cargo_bin("md").unwrap().args([sub, "--help"]).output().unwrap();
    String::from_utf8(out.stdout).unwrap()
}

/// Parse the EXAMPLES block from `<sub> --help`. Returns (cmdline, expected_stdout).
fn parse_example(help: &str) -> Option<(String, String)> {
    let block = help.split("EXAMPLES:").nth(1)?;
    let lines: Vec<&str> = block.lines().filter(|l| !l.trim().is_empty()).collect();
    let cmd_line = lines.first()?.trim().strip_prefix("$ ")?.to_string();
    let expected = lines.iter().skip(1).map(|s| s.trim_start()).collect::<Vec<_>>().join("\n");
    Some((cmd_line, expected))
}

#[test]
fn encode_example_matches_actual_output() {
    let help = long_help("encode");
    let (cmdline, expected) = parse_example(&help).expect("encode --help has EXAMPLES block");
    let parts: Vec<&str> = cmdline.split_whitespace().collect();
    assert_eq!(parts[0], "md");
    let out = Command::cargo_bin("md").unwrap().args(&parts[1..]).output().unwrap();
    let actual = String::from_utf8(out.stdout).unwrap();
    assert_eq!(actual.trim_end(), expected.trim_end(),
        "drift between encode --help EXAMPLE and actual stdout");
}
```

(Add identical test functions for each subcommand: `decode_example_matches_actual_output`, etc. The harness itself is generic — copy-paste with renamed function and changed `long_help` argument.)

- [ ] **Step 2: Run harness, regenerate EXAMPLES until it passes**

Run: `cargo run --features cli,json --bin md -- encode 'wpkh(@0/<0;1>/*)'` to capture the actual output.

Edit the `after_long_help` string in `main.rs` to match the captured output verbatim. Repeat for each subcommand.

Run: `cargo test --features cli,json --test help_examples 2>&1 | tail -10`
Expected: all per-subcommand tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/src/bin/md/main.rs crates/md-codec/tests/help_examples.rs
git commit -m "$(cat <<'EOF'
test(v0.15/phase-8): help-example drift harness with EXAMPLES per subcommand

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 8.2: JSON snapshot tests

**Files:**
- Create: `crates/md-codec/tests/json_snapshots.rs`

- [ ] **Step 1: insta snapshots for read-side --json output**

Create `crates/md-codec/tests/json_snapshots.rs`:

```rust
#![cfg(feature = "json")]
use assert_cmd::Command;

mod manifest {
    include!("vectors/manifest.rs");
}

fn encode(template: &str) -> String {
    let out = Command::cargo_bin("md").unwrap().args(["encode", template]).output().unwrap();
    String::from_utf8(out.stdout).unwrap().lines().next().unwrap().to_string()
}

#[test]
fn decode_json_snapshots() {
    for v in manifest::MANIFEST {
        if v.force_chunked { continue; }
        let phrase = encode(v.template);
        let out = Command::cargo_bin("md").unwrap().args(["decode", &phrase, "--json"]).output().unwrap();
        let body = String::from_utf8(out.stdout).unwrap();
        insta::with_settings!({ snapshot_suffix => v.name }, {
            insta::assert_snapshot!("decode", body);
        });
    }
}

#[test]
fn inspect_json_snapshots() {
    for v in manifest::MANIFEST {
        if v.force_chunked { continue; }
        let phrase = encode(v.template);
        let out = Command::cargo_bin("md").unwrap().args(["inspect", &phrase, "--json"]).output().unwrap();
        let body = String::from_utf8(out.stdout).unwrap();
        insta::with_settings!({ snapshot_suffix => v.name }, {
            insta::assert_snapshot!("inspect", body);
        });
    }
}
```

- [ ] **Step 2: Generate snapshots**

Run: `cargo insta test --features cli,json --test json_snapshots --review`
Expected: insta launches review UI; accept all snapshots after eyeballing them.

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/tests/json_snapshots.rs crates/md-codec/tests/snapshots/
git commit -m "$(cat <<'EOF'
test(v0.15/phase-8): JSON snapshot tests for decode/inspect across manifest

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 8.3: Template round-trip tests

**Files:**
- Create: `crates/md-codec/tests/template_roundtrip.rs`

- [ ] **Step 1: Write the round-trip table-driven test**

Create `crates/md-codec/tests/template_roundtrip.rs`:

```rust
mod manifest {
    include!("vectors/manifest.rs");
}

use assert_cmd::Command;

fn encode(template: &str) -> String {
    let out = Command::cargo_bin("md").unwrap().args(["encode", template]).output().unwrap();
    String::from_utf8(out.stdout).unwrap().lines().next().unwrap().to_string()
}

fn decode(phrase: &str) -> String {
    let out = Command::cargo_bin("md").unwrap().args(["decode", phrase]).output().unwrap();
    String::from_utf8(out.stdout).unwrap().lines().next().unwrap().to_string()
}

#[test]
fn round_trip_each_manifest_entry() {
    for v in manifest::MANIFEST {
        if v.force_chunked { continue; }   // multi-chunk handled separately
        let phrase = encode(v.template);
        let back = decode(&phrase);
        assert_eq!(back, v.template, "round-trip mismatch for {}: got {} want {}", v.name, back, v.template);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --features cli,json --test template_roundtrip 2>&1 | tail -5`
Expected: 1 test passes (covers ~11 entries internally).

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/tests/template_roundtrip.rs
git commit -m "$(cat <<'EOF'
test(v0.15/phase-8): template round-trip across full manifest

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 8.4: Vector corpus diff test

**Files:**
- Create: `crates/md-codec/tests/vector_corpus.rs`

- [ ] **Step 1: Write the diff test**

Create `crates/md-codec/tests/vector_corpus.rs`:

```rust
use assert_cmd::Command;
use std::process::Command as StdCommand;
use tempfile::tempdir;

#[test]
fn vectors_output_matches_committed_corpus() {
    let tmp = tempdir().unwrap();
    Command::cargo_bin("md").unwrap()
        .args(["vectors", "--out", tmp.path().to_str().unwrap()])
        .assert().success();
    let status = StdCommand::new("diff")
        .args(["-r", tmp.path().to_str().unwrap(), "crates/md-codec/tests/vectors"])
        .status().unwrap();
    assert!(status.success(), "vectors corpus drift detected");
}
```

- [ ] **Step 2: Run test**

Run: `cargo test --features cli,json --test vector_corpus 2>&1 | tail -5`
Expected: 1 test passes.

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/tests/vector_corpus.rs
git commit -m "$(cat <<'EOF'
test(v0.15/phase-8): vector corpus drift detector

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 8.5: Compiler determinism table

**Files:**
- Create: `crates/md-codec/tests/compile.rs`

- [ ] **Step 1: Write golden table + tests**

Create `crates/md-codec/tests/compile.rs`:

```rust
#![cfg(feature = "cli-compiler")]
use assert_cmd::Command;

const GOLDEN: &[(&str, &str, &str)] = &[
    // (policy, context, expected_template_starts_with)
    ("pk(@0)",                                  "segwitv0", "wsh(pk(@0))"),
    ("thresh(2,pk(@0),pk(@1),pk(@2))",          "segwitv0", "wsh(multi(2,@0,@1,@2))"),
    ("pk(@0)",                                  "tap",       "tr(@0)"),
];

#[test]
fn compiler_golden_table() {
    for (expr, ctx, expected_prefix) in GOLDEN {
        let out = Command::cargo_bin("md").unwrap()
            .args(["compile", expr, "--context", ctx])
            .output().unwrap();
        let actual = String::from_utf8(out.stdout).unwrap();
        let actual_first = actual.lines().next().unwrap();
        assert!(
            actual_first.starts_with(expected_prefix),
            "compile({expr}, {ctx}) → {actual_first}, expected prefix {expected_prefix}"
        );
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --features cli,json,cli-compiler --test compile 2>&1 | tail -5`
Expected: 1 test passes.

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/tests/compile.rs
git commit -m "$(cat <<'EOF'
test(v0.15/phase-8): compiler determinism golden table

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 8.6: Exit-code spot-checks

**Files:**
- Create: `crates/md-codec/tests/exit_codes.rs`

- [ ] **Step 1: Write per-subcommand exit-code assertions**

Create `crates/md-codec/tests/exit_codes.rs`:

```rust
use assert_cmd::Command;

#[test]
fn no_args_returns_2() {
    Command::cargo_bin("md").unwrap().assert().code(2);
}

#[test]
fn unknown_subcommand_returns_2() {
    Command::cargo_bin("md").unwrap().arg("bogus").assert().code(2);
}

#[test]
fn encode_bad_template_returns_1() {
    Command::cargo_bin("md").unwrap()
        .args(["encode", "this is not a template"])
        .assert().code(1);
}

#[test]
fn decode_bad_string_returns_1() {
    Command::cargo_bin("md").unwrap()
        .args(["decode", "not-a-valid-md-string"])
        .assert().code(1);
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --features cli,json --test exit_codes 2>&1 | tail -5`
Expected: 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/tests/exit_codes.rs
git commit -m "$(cat <<'EOF'
test(v0.15/phase-8): per-subcommand exit-code spot checks

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 8.7: Phase 8 ship tag

- [ ] **Step 1: Run full suite**

```bash
cargo test --workspace --features cli,json,cli-compiler 2>&1 | grep -E '^test result' | awk '{ok+=$4} END {print "Total ok:", ok}'
# Expect: ~340 (319 + ~21 across the harnesses; help/snapshots count by subcommand)
```

- [ ] **Step 2: Empty ship-tag commit**

```bash
git commit --allow-empty -m "$(cat <<'EOF'
chore(v0.15/phase-8): ship — six test harnesses

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 9 — Docs and release

Phase goal: README/MIGRATION/CHANGELOG/json-schema all reflect v0.15.0; crate ready to tag.

### Task 9.1: README CLI section

**Files:**
- Modify: `crates/md-codec/README.md`

- [ ] **Step 1: Add `## CLI` section near the top**

Open `crates/md-codec/README.md`. After the crate's one-paragraph intro, insert:

```markdown
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

Every read/write subcommand accepts `--json` for structured output (schema versioned as `md-cli/1`). Each subcommand's `--help` shows a worked example.

To build without the CLI: `cargo build --no-default-features`.
```

- [ ] **Step 2: Commit**

```bash
git add crates/md-codec/README.md
git commit -m "$(cat <<'EOF'
docs(v0.15): README CLI section

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 9.2: docs/json-schema-v1.md

**Files:**
- Create: `docs/json-schema-v1.md`

- [ ] **Step 1: Write field-level schema**

Create `docs/json-schema-v1.md`:

```markdown
# md-cli JSON schema v1

Every JSON output carries `"schema": "md-cli/1"`. Schema version bumps with breaking changes.

## Hex encoding
- `[u8; N]` and `Vec<u8>` → lowercase hex, no `0x` prefix.
- Identity-hash fingerprints → `"0x" + 8 hex chars`.

## Top-level wrappers per subcommand

### `encode --json`
| Field | Type | Always present? |
|---|---|---|
| `schema` | string | yes |
| `phrase` | string | yes |
| `chunk_set_id` | string `0xXXXXX` | iff `--force-chunked` |
| `policy_id_fingerprint` | string `0xXXXXXXXX` | iff `--policy-id-fingerprint` |

### `decode --json`
| Field | Type |
|---|---|
| `schema` | string |
| `descriptor` | `JsonDescriptor` (see below) |

### `inspect --json`
| Field | Type |
|---|---|
| `schema` | string |
| `descriptor` | `JsonDescriptor` |
| `md1_encoding_id` | `JsonHash` |
| `wallet_descriptor_template_id` | `JsonHash` |
| `wallet_policy_id` | `JsonHash` (with `fingerprint`) |

### `bytecode --json`
| Field | Type |
|---|---|
| `schema` | string |
| `payload_bits` | u32 |
| `payload_bytes` | u32 |
| `hex` | string |

### `compile --json`
| Field | Type |
|---|---|
| `schema` | string |
| `template` | string |
| `context` | `"tap"` or `"segwitv0"` |

## Shadow types

### `JsonDescriptor`
| Field | Type |
|---|---|
| `n` | u8 |
| `path_decl` | `JsonPathDecl` |
| `use_site_path` | `JsonUseSitePath` |
| `tree` | `JsonNode` |
| `tlv` | `JsonTlv` |

### `JsonPathDecl` (adjacent-tagged)
- `{"tag": "Single", "data": "m/48'/0'/0'/2'"}`
- `{"tag": "Divergent", "data": ["m/...", "m/..."]}`

### `JsonUseSitePath`
| Field | Type |
|---|---|
| `multipath` | `[{"hardened": bool, "value": u32}, ...]` or `null` |
| `wildcard_hardened` | bool |

### `JsonNode`
| Field | Type |
|---|---|
| `tag` | string (Tag variant name) |
| `body` | `JsonBody` |

### `JsonBody` (adjacent-tagged on `kind`)
- `{"kind": "SingleKey", "data": {"idx": u8}}`
- `{"kind": "Multi", "data": {"k": u8, "sorted": bool, "indices": [u8, ...]}}`
- `{"kind": "Tr", "data": {"internal_idx": u8, "tree": JsonTapTree | null}}`

### `JsonTapTree` (adjacent-tagged on `kind`)
- `{"kind": "Leaf", "data": JsonNode}`
- `{"kind": "Branch", "data": {"left": JsonTapTree, "right": JsonTapTree}}`

### `JsonTlv`
| Field | Type |
|---|---|
| `use_site_path_overrides` | `[(u8, JsonUseSitePath), ...]` or `null` |
| `fingerprints` | `[(u8, hex8), ...]` or `null` |
| `pubkeys` | `[(u8, hex130), ...]` or `null` |
| `origin_path_overrides` | `[(u8, "m/..."), ...]` or `null` |

### `JsonHash`
| Field | Type |
|---|---|
| `hex` | string |
| `fingerprint` | string `0xXXXXXXXX`, only on `WalletPolicyId` |
```

- [ ] **Step 2: Commit**

```bash
git add docs/json-schema-v1.md
git commit -m "$(cat <<'EOF'
docs(v0.15): JSON schema v1 reference

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 9.3: MIGRATION.md

**Files:**
- Modify: `MIGRATION.md`

- [ ] **Step 1: Add v0.14 → v0.15 section**

Open `MIGRATION.md`. At the top of the existing migration entries (after the file header), insert:

```markdown
## v0.14.x → v0.15.0

v0.15.0 reintroduces the `md` CLI binary stripped in v0.12.0. **Library API is
unchanged** — no source changes required for downstream library consumers.

### What's new

- New `md` binary: `cargo install md-codec` produces it.
- Default features `cli` and `json` are on. Library-only consumers:

  ```toml
  md-codec = { version = "0.15", default-features = false }
  ```

- New opt-in `cli-compiler` feature pulls `miniscript/compiler` for the
  `compile` subcommand and `encode --from-policy`.

### What didn't change

- Wire format (v0.13/v0.14 unchanged).
- Library `Error` enum (CLI-specific errors live in the binary's own
  `CliError`).
- Public exports of `md_codec::*`.

### What's not coming back

- `--seed` flag for chunk-set-id override (v0.11 had it). The
  `derive_chunk_set_id` function is fully deterministic from the payload;
  if you need a known id for a test corpus, use `md vectors`.
- Separate `gen_vectors` binary — folded into `md vectors`.
- Testnet/regtest xpubs for `--key` — mainnet only in v0.15.0.
```

- [ ] **Step 2: Commit**

```bash
git add MIGRATION.md
git commit -m "$(cat <<'EOF'
docs(v0.15): MIGRATION entry for v0.14 → v0.15

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 9.4: CHANGELOG.md

**Files:**
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Add 0.15.0 entry**

Open `CHANGELOG.md`. At the top of the entries, insert:

```markdown
## [0.15.0] — 2026-05-XX

### Added

- `md` CLI binary with seven subcommands: `encode`, `decode`, `verify`,
  `inspect`, `bytecode`, `vectors`, `compile`.
- `--json` output on every read/write subcommand (schema `md-cli/1`).
- Help-text drift harness (`tests/help_examples.rs`) — every subcommand's
  worked example is asserted byte-equal against actual stdout in CI.
- Vectors corpus generator — `md vectors` regenerates 12 deterministic
  test fixtures; CI fails on drift.
- New Cargo features: `cli` (default), `json` (default), `cli-compiler`
  (opt-in).
- New deps: `clap`, `anyhow`, `regex`, `miniscript = "13.0.0"` (gated on
  `cli`); `serde`, `serde_json` (gated on `json`).

### Unchanged

- Wire format. Library `Error` enum. Public `md_codec::*` exports.
```

- [ ] **Step 2: Commit**

```bash
git add CHANGELOG.md
git commit -m "$(cat <<'EOF'
docs(v0.15): CHANGELOG entry

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 9.5: Release tag

- [ ] **Step 1: Run the full test matrix one more time**

```bash
cargo test --workspace --features cli,json 2>&1 | grep -E '^test result' | awk '{ok+=$4} END {print "no-compiler total ok:", ok}'
cargo test --workspace --features cli,json,cli-compiler 2>&1 | grep -E '^test result' | awk '{ok+=$4} END {print "with-compiler total ok:", ok}'
cargo clippy --workspace --features cli,json,cli-compiler --all-targets -- -D warnings 2>&1 | tail -3
# Expect: clean clippy, both totals ≥ 340.
```

- [ ] **Step 2: Verify packaging**

```bash
cargo package --no-verify -p md-codec
# Expect: exits 0, lists the bin paths in the output.
```

- [ ] **Step 3: Final ship commit**

```bash
git commit --allow-empty -m "$(cat <<'EOF'
release: md-codec v0.15.0 — md CLI restoration

Restores command-line functionality stripped in v0.12.0. Single `md`
binary with seven subcommands: encode, decode, verify, inspect,
bytecode, vectors, compile. JSON output on every read/write subcommand
with versioned schema. Help-text drift harness, vectors-corpus drift
detector, template round-trip suite, compiler-determinism table, and
exit-code spot-checks.

Library API additive — no source changes required for downstream
library consumers. Wire format unchanged from v0.13/v0.14.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Verification (end-to-end)

Once all phases are complete, validate the full feature works as a user would experience it:

- [ ] **Build the binary**

```bash
cargo build --release --features cli,json,cli-compiler
ls -la target/release/md
# Expect: executable file present
```

- [ ] **Smoke-test top-level help**

```bash
./target/release/md --help
# Expect: usage line + all 7 subcommands listed
```

- [ ] **End-to-end encode → decode**

```bash
PHRASE=$(./target/release/md encode 'wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))' | head -1)
./target/release/md decode "$PHRASE"
# Expect: wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))
```

- [ ] **End-to-end verify match**

```bash
./target/release/md verify "$PHRASE" --template 'wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))'
echo "exit=$?"
# Expect: OK \n exit=0
```

- [ ] **End-to-end verify mismatch**

```bash
./target/release/md verify "$PHRASE" --template 'wpkh(@0/<0;1>/*)' || echo "exit=$?"
# Expect: MISMATCH ... \n exit=1
```

- [ ] **JSON output**

```bash
./target/release/md decode "$PHRASE" --json | jq '.schema'
# Expect: "md-cli/1"
```

- [ ] **Compile + encode in one shot**

```bash
./target/release/md encode --from-policy 'pk(@0)' --context segwitv0 | head -1
# Expect: an `md ...` 12-word phrase
```

- [ ] **Vectors regeneration is deterministic**

```bash
mkdir -p /tmp/md-vectors-check
./target/release/md vectors --out /tmp/md-vectors-check
diff -r /tmp/md-vectors-check crates/md-codec/tests/vectors
# Expect: silent (no output, no diff)
```

- [ ] **`cargo install` from local crate**

```bash
cargo install --path crates/md-codec --force
which md && md --version
# Expect: ~/.cargo/bin/md and version string `md 0.15.0`
```

If every command above produces the expected output, the v0.15.0 release is functionally complete.

---

## Out of scope for this plan

- Pushing the branch / opening a PR. Author decides when to push.
- Crates.io publish (`cargo publish`) — separate release-process step.
- Adding `--network` for testnet/regtest xpubs — deferred to v0.16+.
- Address derivation as a CLI subcommand — library has it; deferred until demand emerges.

