# wdm-codec v0.1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the v0.1 reference implementation of the Wallet Descriptor Mnemonic (WDM) format in Rust, producing concrete BIP test vectors.

**Architecture:** Single Rust crate at `crates/wdm-codec/` with library + 2 binary targets (`wdm` CLI, `gen_vectors`). Bytecode forks `descriptor-codec` (CC0) preserving tag values 0x00–0x31 verbatim and adding 0x32 (placeholder), 0x33 (shared-path). Encoding layer is codex32-derived (BIP 93 BCH polynomials with HRP `"wdm"`). Multi-string chunking with 4-byte cross-chunk SHA-256 hash. v0.1 scope is `wsh()`-only with no taproot, no fingerprints, no guided recovery.

**Tech Stack:** Rust 1.85, `bitcoin 0.32`, `miniscript 12`, `bech32 0.11`, `clap 4`, `serde`, `thiserror 2`, `indexmap 2`, `strum 0.26` (dev), `rand 0.8` (dev).

---

## Conventions

**TDD discipline:** red (failing test) → green (minimal impl) → commit. Every task that adds behavior follows this loop.

**Commit messages:** Conventional Commits. Format: `<type>(<scope>): <subject>` where `type ∈ {feat, fix, refactor, test, docs, chore, ci}` and `scope` is the module name (`bytecode`, `encoding`, `chunking`, `policy`, `cli`, `gen_vectors`, etc.).

**Branch strategy:** Work on `main` directly (single-developer v0.1; no PR review yet). Push after each task completes. CI must stay green at `main` HEAD.

**Working directory:** All file paths are relative to `crates/wdm-codec/` unless otherwise noted. `cd` into that directory after Phase 0 completes.

**Tooling commands** (run from `crates/wdm-codec/` unless stated):
- Test single function: `cargo test --lib tests::module::test_name -- --exact`
- Test single integration file: `cargo test --test corpus`
- All tests: `cargo test`
- Lint: `cargo clippy -- -D warnings`
- Format: `cargo fmt`
- Doc check: `cargo doc --no-deps`

---

## File Structure

```
descriptor-mnemonic/                       (existing repo root)
├── bip/                                    (existing)
├── design/                                 (existing)
├── crates/
│   └── wdm-codec/                          ← all new
│       ├── Cargo.toml
│       ├── README.md
│       ├── src/
│       │   ├── lib.rs                     ← top-level API + re-exports
│       │   ├── policy.rs                  ← WalletPolicy adapter
│       │   ├── error.rs                   ← Error enum + BytecodeErrorKind
│       │   ├── encoding.rs                ← bech32 + BCH polynomials
│       │   ├── chunking.rs                ← ChunkHeader + reassembly
│       │   ├── wallet_id.rs               ← WalletId + ChunkWalletId
│       │   ├── vectors.rs                 ← TestVectorFile schema
│       │   ├── bytecode/
│       │   │   ├── mod.rs                 ← top-level encode/decode
│       │   │   ├── tag.rs                 ← Tag enum
│       │   │   ├── key.rs                 ← WdmKey enum
│       │   │   ├── path.rs                ← path dictionary
│       │   │   ├── varint.rs              ← LEB128
│       │   │   ├── encode.rs              ← AST → bytes
│       │   │   └── decode.rs              ← bytes → AST
│       │   └── bin/
│       │       ├── wdm.rs                 ← CLI binary
│       │       └── gen_vectors.rs         ← test vector generator
│       └── tests/
│           ├── common/mod.rs              ← shared helpers
│           ├── corpus.rs                  ← 9 entries + Coldcard
│           ├── upstream_shapes.rs         ← 9 descriptor-codec shapes
│           ├── chunking.rs                ← 4 named hash tests
│           ├── ecc.rs                     ← BCH stress
│           ├── conformance.rs             ← rejects_* macro tests
│           ├── error_coverage.rs          ← strum exhaustiveness
│           ├── vectors_schema.rs          ← JSON schema check (P8)
│           └── vectors/
│               └── v0.1.json              ← committed at P8
└── Cargo.toml                              (existing — add workspace)
```

---

## Phase 0 — Workspace Setup (~0.5 d)

### Task 0.1: Add workspace to repo root Cargo.toml

**Files:**
- Modify: `Cargo.toml` (root)

- [ ] **Step 1: Verify the file does not already exist**

```bash
ls Cargo.toml 2>/dev/null && echo "exists" || echo "missing"
```

Expected: `missing`

- [ ] **Step 2: Create root Cargo.toml**

```toml
[workspace]
resolver = "2"
members = ["crates/wdm-codec"]

[workspace.package]
edition = "2024"
rust-version = "1.85"
license = "CC0-1.0"
repository = "https://github.com/bg002h/descriptor-mnemonic"

[workspace.lints.rust]
missing_docs = "warn"

[workspace.lints.clippy]
all = "warn"
```

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "chore(workspace): add Cargo workspace root for crates/wdm-codec"
git push
```

### Task 0.2: Verify MSRV compatibility before scaffolding

**Files:**
- None (verification only)

- [ ] **Step 1: Create scratch crate to test MSRV**

```bash
mkdir -p /tmp/wdm-msrv-check && cd /tmp/wdm-msrv-check
cargo init --name msrv-check --lib
```

- [ ] **Step 2: Add bitcoin and miniscript pinned versions**

Edit `/tmp/wdm-msrv-check/Cargo.toml`:

```toml
[package]
name = "msrv-check"
version = "0.1.0"
edition = "2024"
rust-version = "1.85"

[dependencies]
bitcoin = "0.32"
miniscript = "12"
bech32 = "0.11"
```

- [ ] **Step 3: Verify it builds on MSRV**

```bash
cd /tmp/wdm-msrv-check && cargo +1.85 build 2>&1 | tail -10
```

Expected: `Compiling msrv-check ...` and successful build with no `error[E0...]` lines.

- [ ] **Step 4: If build fails, downgrade MSRV pin**

If you see `error[E0658]: ... requires Rust X.Y` or similar, update both `Cargo.toml` files (root and crate) to match the highest MSRV among the three deps. Document the actual MSRV in the project's README.

- [ ] **Step 5: Clean up scratch and return to project**

```bash
rm -rf /tmp/wdm-msrv-check
cd /scratch/code/shibboleth/descriptor-mnemonic
```

No commit needed — this was a verification gate.

### Task 0.3: Create wdm-codec crate skeleton

**Files:**
- Create: `crates/wdm-codec/Cargo.toml`
- Create: `crates/wdm-codec/src/lib.rs`
- Create: `crates/wdm-codec/README.md`

- [ ] **Step 1: Create the crate package**

```bash
mkdir -p crates/wdm-codec/src
```

- [ ] **Step 2: Write Cargo.toml**

Create `crates/wdm-codec/Cargo.toml`:

```toml
[package]
name = "wdm-codec"
version = "0.1.0-dev"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "Reference implementation of the Wallet Descriptor Mnemonic (WDM) format for engravable BIP 388 wallet policy backups"
readme = "README.md"

[lints]
workspace = true

[lib]
name = "wdm_codec"

[[bin]]
name = "wdm"
path = "src/bin/wdm.rs"

[[bin]]
name = "gen_vectors"
path = "src/bin/gen_vectors.rs"

[dependencies]
bitcoin = "0.32"
miniscript = "12"
bech32 = "0.11"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
indexmap = "2.0"
clap = { version = "4.5", features = ["derive"] }

[dev-dependencies]
rand = "0.8"
strum = { version = "0.26", features = ["derive"] }
hex = "0.4"
```

- [ ] **Step 3: Write minimal lib.rs**

Create `crates/wdm-codec/src/lib.rs`:

```rust
//! Wallet Descriptor Mnemonic (WDM) — engravable backup format for BIP 388 wallet policies.
//!
//! See the BIP draft at `bip/bip-wallet-descriptor-mnemonic.mediawiki` for the format specification.

#![cfg_attr(not(test), deny(missing_docs))]
```

- [ ] **Step 4: Write README**

Create `crates/wdm-codec/README.md`:

```markdown
# wdm-codec

Reference implementation of the Wallet Descriptor Mnemonic (WDM) format.

**Status:** Pre-Draft, AI only, not yet human reviewed.

See the parent repository for the BIP draft and design documents.
```

- [ ] **Step 5: Verify it builds**

```bash
cd crates/wdm-codec
cargo build --lib
```

Expected: `Finished dev profile` with no errors.

- [ ] **Step 6: Commit**

```bash
cd /scratch/code/shibboleth/descriptor-mnemonic
git add crates/wdm-codec/
git commit -m "feat(wdm-codec): scaffold crate package with lib + 2 bin targets"
git push
```

### Task 0.4: Create empty module skeletons

**Files:**
- Create: `crates/wdm-codec/src/policy.rs`
- Create: `crates/wdm-codec/src/error.rs`
- Create: `crates/wdm-codec/src/encoding.rs`
- Create: `crates/wdm-codec/src/chunking.rs`
- Create: `crates/wdm-codec/src/wallet_id.rs`
- Create: `crates/wdm-codec/src/vectors.rs`
- Create: `crates/wdm-codec/src/bytecode/mod.rs`
- Create: `crates/wdm-codec/src/bytecode/tag.rs`
- Create: `crates/wdm-codec/src/bytecode/key.rs`
- Create: `crates/wdm-codec/src/bytecode/path.rs`
- Create: `crates/wdm-codec/src/bytecode/varint.rs`
- Create: `crates/wdm-codec/src/bytecode/encode.rs`
- Create: `crates/wdm-codec/src/bytecode/decode.rs`
- Create: `crates/wdm-codec/src/bin/wdm.rs`
- Create: `crates/wdm-codec/src/bin/gen_vectors.rs`
- Modify: `crates/wdm-codec/src/lib.rs`

- [ ] **Step 1: Create the bytecode and bin directories**

```bash
mkdir -p crates/wdm-codec/src/bytecode crates/wdm-codec/src/bin
```

- [ ] **Step 2: Create stub files with `// stub` markers**

Each file gets a one-line stub. Run from `crates/wdm-codec/src/`:

```bash
for f in policy error encoding chunking wallet_id vectors; do
  echo "//! $f module — stub for v0.1." > "$f.rs"
done
for f in mod tag key path varint encode decode; do
  echo "//! bytecode::$f module — stub for v0.1." > "bytecode/$f.rs"
done
cat > bin/wdm.rs <<'EOF'
//! WDM CLI binary — stub.
fn main() {
    eprintln!("wdm CLI not yet implemented");
    std::process::exit(1);
}
EOF
cat > bin/gen_vectors.rs <<'EOF'
//! Test vector generator — stub.
fn main() {
    eprintln!("gen_vectors not yet implemented");
    std::process::exit(1);
}
EOF
```

- [ ] **Step 3: Update lib.rs to declare the modules**

Replace `crates/wdm-codec/src/lib.rs`:

```rust
//! Wallet Descriptor Mnemonic (WDM) — engravable backup format for BIP 388 wallet policies.

#![cfg_attr(not(test), deny(missing_docs))]

pub mod bytecode;
pub mod chunking;
pub mod encoding;
pub mod error;
pub mod policy;
pub mod vectors;
pub mod wallet_id;
```

- [ ] **Step 4: Verify it builds**

```bash
cd crates/wdm-codec && cargo build --lib && cargo build --bins
```

Expected: builds clean with only `unused` warnings on stub modules.

- [ ] **Step 5: Commit**

```bash
cd /scratch/code/shibboleth/descriptor-mnemonic
git add crates/wdm-codec/src/
git commit -m "feat(wdm-codec): create empty module skeletons for all v0.1 modules"
git push
```

### Task 0.5: Scaffold Error enum and BytecodeErrorKind sub-enum

**Files:**
- Modify: `crates/wdm-codec/src/error.rs`
- Modify: `crates/wdm-codec/src/lib.rs`

- [ ] **Step 1: Write the test for Error::Display**

Replace `crates/wdm-codec/src/error.rs`:

```rust
//! Error types for wdm-codec.

use thiserror::Error;

/// Forward declaration; defined fully in chunking.rs once available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkWalletId(pub(crate) u32);

/// All errors that wdm-codec can return.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum Error {
    /// HRP did not match the expected `"wdm"`.
    #[error("invalid HRP: expected 'wdm', got '{0}'")]
    InvalidHrp(String),

    /// Bech32 string contained mixed-case characters.
    #[error("invalid case: bech32 strings must be all-lowercase or all-uppercase")]
    MixedCase,

    /// Total string length is invalid (e.g., the reserved 94 or 95 char range).
    #[error("invalid string length: {0}")]
    InvalidStringLength(usize),

    /// BCH error correction failed (more than 4 substitutions).
    #[error("BCH decode failed: too many errors to correct")]
    BchUncorrectable,

    /// Bytecode parse failed at a specific offset.
    #[error("invalid bytecode at offset {offset}: {kind}")]
    InvalidBytecode {
        /// Byte offset within the canonical bytecode where the parse failed.
        offset: usize,
        /// Specific kind of bytecode error.
        kind: BytecodeErrorKind,
    },

    /// Format version is not supported by this implementation.
    #[error("unsupported format version: {0}")]
    UnsupportedVersion(u8),

    /// Card type is not supported.
    #[error("unsupported card type: {0}")]
    UnsupportedCardType(u8),

    /// Chunk index is out of range for the declared total.
    #[error("chunk index {index} out of range (total chunks: {total})")]
    ChunkIndexOutOfRange {
        /// The reported chunk index.
        index: u8,
        /// The declared total chunk count.
        total: u8,
    },

    /// A chunk index appears more than once during reassembly.
    #[error("duplicate chunk index: {0}")]
    DuplicateChunkIndex(u8),

    /// Two chunks reported different wallet identifiers.
    #[error("wallet identifier mismatch across chunks: expected {expected:?}, got {got:?}")]
    WalletIdMismatch {
        /// The expected (first-seen) chunk wallet identifier.
        expected: ChunkWalletId,
        /// The mismatched value seen on a later chunk.
        got: ChunkWalletId,
    },

    /// Two chunks reported different total chunk counts.
    #[error("total-chunks mismatch across chunks: expected {expected}, got {got}")]
    TotalChunksMismatch {
        /// The expected (first-seen) total.
        expected: u8,
        /// The mismatched value seen on a later chunk.
        got: u8,
    },

    /// Policy violates the v0.1 implementation scope.
    #[error("policy violates v0.1 scope: {0}")]
    PolicyScopeViolation(String),

    /// Cross-chunk integrity hash did not match the reassembled bytecode.
    #[error("cross-chunk hash mismatch")]
    CrossChunkHashMismatch,

    /// Policy parse error from the BIP 388 string form.
    #[error("policy parse error: {0}")]
    PolicyParse(String),

    /// Wraps a miniscript error as a string to insulate from upstream churn.
    #[error("miniscript: {0}")]
    Miniscript(String),
}

impl From<miniscript::Error> for Error {
    fn from(e: miniscript::Error) -> Self {
        Error::Miniscript(e.to_string())
    }
}

/// Kind of bytecode parse error, used inside [`Error::InvalidBytecode`].
#[non_exhaustive]
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum BytecodeErrorKind {
    /// Tag byte does not correspond to any defined operator.
    #[error("unknown tag {0:#04x}")]
    UnknownTag(u8),

    /// A length prefix declared more bytes than the buffer contains.
    #[error("truncated input")]
    Truncated,

    /// LEB128 varint exceeded its expected width.
    #[error("varint overflow")]
    VarintOverflow,

    /// Operator expected more children than were present.
    #[error("missing children: expected {expected}, got {got}")]
    MissingChildren {
        /// Number of children expected by the operator's arity.
        expected: usize,
        /// Number of children actually parsed.
        got: usize,
    },

    /// Cursor ran off the end of the buffer mid-parse.
    #[error("unexpected end of buffer")]
    UnexpectedEnd,

    /// Buffer had bytes remaining after the operator tree was fully consumed.
    #[error("trailing bytes after canonical bytecode")]
    TrailingBytes,
}

/// Result type used throughout wdm-codec.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_matches_thiserror_format() {
        let e = Error::InvalidHrp("btc".to_string());
        assert_eq!(e.to_string(), "invalid HRP: expected 'wdm', got 'btc'");
    }

    #[test]
    fn miniscript_error_is_wrapped_as_string() {
        // A real miniscript error will be wrapped as String; here we just
        // confirm the conversion compiles and produces our variant.
        let _e: Error = Error::Miniscript("test".to_string());
    }

    #[test]
    fn bytecode_error_kind_display() {
        let k = BytecodeErrorKind::UnknownTag(0xFF);
        assert_eq!(k.to_string(), "unknown tag 0xff");
    }
}
```

- [ ] **Step 2: Run the tests to verify they pass**

```bash
cd crates/wdm-codec && cargo test --lib error::tests
```

Expected: 3 tests pass.

- [ ] **Step 3: Re-export Error and Result from lib.rs**

Replace `crates/wdm-codec/src/lib.rs`:

```rust
//! Wallet Descriptor Mnemonic (WDM) — engravable backup format for BIP 388 wallet policies.

#![cfg_attr(not(test), deny(missing_docs))]

pub mod bytecode;
pub mod chunking;
pub mod encoding;
pub mod error;
pub mod policy;
pub mod vectors;
pub mod wallet_id;

pub use error::{BytecodeErrorKind, ChunkWalletId, Error, Result};
```

- [ ] **Step 4: Verify all tests still pass**

```bash
cargo test --lib
```

Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
cd /scratch/code/shibboleth/descriptor-mnemonic
git add crates/wdm-codec/src/error.rs crates/wdm-codec/src/lib.rs
git commit -m "feat(error): scaffold Error enum and BytecodeErrorKind sub-enum with tests"
git push
```

### Task 0.6: Add CI workflow stub (cargo check only)

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Create the workflow directory if missing**

```bash
mkdir -p .github/workflows
```

- [ ] **Step 2: Write the CI yml**

Create `.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: "-D warnings"

jobs:
  check:
    name: cargo check (linux)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.85.0
      - uses: Swatinem/rust-cache@v2
      - run: cargo check --workspace --all-targets

  fmt:
    name: cargo fmt
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt
      - run: cargo fmt --all --check

  # Tests, clippy, doc, llvm-cov, gen_vectors --verify, Windows/macOS sanity
  # are added at P5 once we have testable code. CI is intentionally minimal
  # at P0 to fail-fast on broken Cargo.toml changes only.
```

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add cargo-check stub workflow for P0; full CI added at P5"
git push
```

### Task 0.7: Propagate SHA-256 hash decision back to POLICY_BACKUP.md

**Files:**
- Modify: `design/POLICY_BACKUP.md`

- [ ] **Step 1: Find the DECIDE marker for the hash function**

```bash
grep -n "DECIDE.*[Hh]ash function" design/POLICY_BACKUP.md
```

Expected: a single line citing "Hash function for Wallet ID derivation (SHA-256 truncated, BLAKE3, or HMAC variant)" or similar.

- [ ] **Step 2: Replace DECIDE with RESOLVED**

Open `design/POLICY_BACKUP.md` and find the Hash-function line. Change it from:

```
- **DECIDE:** Hash function for Wallet ID derivation (SHA-256 truncated, BLAKE3, or HMAC variant).
```

To:

```
- **RESOLVED (2026-04-26):** Hash function for Wallet ID derivation is **SHA-256 truncated to 16 bytes**. Decision matches `bitcoin::hashes::sha256` (already a transitive dependency); avoids adding BLAKE3 or HMAC.
```

- [ ] **Step 3: Commit**

```bash
git add design/POLICY_BACKUP.md
git commit -m "docs(design): resolve hash-function DECIDE in POLICY_BACKUP (SHA-256)"
git push
```

---

## Phase 1 — Encoding Layer (~1.5 d, parallel with P2)

### Task 1.1: BchCode enum

**Files:**
- Modify: `crates/wdm-codec/src/encoding.rs`

- [ ] **Step 1: Write the failing test**

Replace `crates/wdm-codec/src/encoding.rs`:

```rust
//! Encoding layer: bech32 alphabet conversion and BCH error correction.
//!
//! Implements the codex32-derived (BIP 93) encoding with HRP `"wdm"`.

/// Which BCH code variant a string uses.
///
/// Determined by the total data-part length: regular for ≤93 chars,
/// long for 96–108 chars. Lengths 94–95 are reserved-invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BchCode {
    /// Regular code: BCH(93,80,8). 13-char checksum.
    Regular,
    /// Long code: BCH(108,93,8). 15-char checksum.
    Long,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bch_code_equality() {
        assert_eq!(BchCode::Regular, BchCode::Regular);
        assert_ne!(BchCode::Regular, BchCode::Long);
    }

    #[test]
    fn bch_code_can_be_hashed() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(BchCode::Regular);
        set.insert(BchCode::Long);
        set.insert(BchCode::Regular);
        assert_eq!(set.len(), 2);
    }
}
```

- [ ] **Step 2: Run the tests**

```bash
cd crates/wdm-codec && cargo test --lib encoding::tests
```

Expected: 2 tests pass.

- [ ] **Step 3: Re-export from lib.rs**

Add to `crates/wdm-codec/src/lib.rs` after existing `pub use`:

```rust
pub use encoding::BchCode;
```

- [ ] **Step 4: Verify build**

```bash
cargo build --lib
```

- [ ] **Step 5: Commit**

```bash
cd /scratch/code/shibboleth/descriptor-mnemonic
git add crates/wdm-codec/src/encoding.rs crates/wdm-codec/src/lib.rs
git commit -m "feat(encoding): add BchCode enum (Regular | Long)"
git push
```

### Task 1.2: Bech32 alphabet conversion (8 ↔ 5 bit pack/unpack)

**Files:**
- Modify: `crates/wdm-codec/src/encoding.rs`

- [ ] **Step 1: Write failing tests for `bytes_to_chars` and `chars_to_bytes`**

Append to `crates/wdm-codec/src/encoding.rs` (before the existing `#[cfg(test)]` module):

```rust
/// The bech32 32-character alphabet, in 5-bit-value order.
///
/// `q=0, p=1, z=2, r=3, y=4, 9=5, x=6, 8=7, g=8, f=9, 2=10, t=11, v=12,
///  d=13, w=14, 0=15, s=16, 3=17, j=18, n=19, 5=20, 4=21, k=22, h=23,
///  c=24, e=25, 6=26, m=27, u=28, a=29, 7=30, l=31`.
pub const ALPHABET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// Inverse lookup: char (lowercase ASCII) -> 5-bit value, or 0xFF if not in alphabet.
const ALPHABET_INV: [u8; 128] = build_alphabet_inv();

const fn build_alphabet_inv() -> [u8; 128] {
    let mut inv = [0xFFu8; 128];
    let mut i = 0;
    while i < 32 {
        inv[ALPHABET[i] as usize] = i as u8;
        i += 1;
    }
    inv
}

/// Convert a sequence of 8-bit bytes to a sequence of 5-bit values
/// (padded with zero bits at the end if the bit count is not a multiple of 5).
pub fn bytes_to_5bit(bytes: &[u8]) -> Vec<u8> {
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    let mut out = Vec::with_capacity((bytes.len() * 8 + 4) / 5);
    for &b in bytes {
        acc = (acc << 8) | b as u32;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(((acc >> bits) & 0x1F) as u8);
        }
    }
    if bits > 0 {
        out.push(((acc << (5 - bits)) & 0x1F) as u8);
    }
    out
}

/// Convert a sequence of 5-bit values back to 8-bit bytes.
/// Returns None if the bit padding is non-zero (i.e., trailing bits are nonzero).
pub fn five_bit_to_bytes(values: &[u8]) -> Option<Vec<u8>> {
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    let mut out = Vec::with_capacity(values.len() * 5 / 8);
    for &v in values {
        if v >= 32 {
            return None;
        }
        acc = (acc << 5) | v as u32;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push(((acc >> bits) & 0xFF) as u8);
        }
    }
    // Any remaining bits must be zero (padding).
    if bits >= 5 {
        return None;
    }
    if (acc & ((1 << bits) - 1)) != 0 {
        return None;
    }
    Some(out)
}
```

Add tests inside the existing `#[cfg(test)] mod tests` block:

```rust
    #[test]
    fn alphabet_is_32_unique_chars() {
        let mut seen = std::collections::HashSet::new();
        for &c in ALPHABET {
            assert!(seen.insert(c), "duplicate char in alphabet: {}", c as char);
        }
        assert_eq!(seen.len(), 32);
    }

    #[test]
    fn bytes_to_5bit_round_trip_zero() {
        let bytes = vec![0x00];
        let fives = bytes_to_5bit(&bytes);
        assert_eq!(fives, vec![0, 0]);
        let back = five_bit_to_bytes(&fives).unwrap();
        assert_eq!(back, bytes);
    }

    #[test]
    fn bytes_to_5bit_round_trip_known_value() {
        // 0xFF = binary 11111111. Splits as 11111 (=31) and 111 (padded with 00 to 11100=28).
        let bytes = vec![0xFF];
        let fives = bytes_to_5bit(&bytes);
        assert_eq!(fives, vec![31, 28]);
    }

    #[test]
    fn five_bit_to_bytes_rejects_nonzero_padding() {
        // Two 5-bit values = 10 bits, of which 8 form a byte and 2 are padding.
        // If padding bits are nonzero, decode must fail.
        // 31 = 11111, 1 = 00001. Last 2 bits (= 01) are nonzero padding.
        assert!(five_bit_to_bytes(&[31, 1]).is_none());
    }

    #[test]
    fn five_bit_to_bytes_rejects_value_out_of_range() {
        assert!(five_bit_to_bytes(&[32]).is_none());
    }
```

- [ ] **Step 2: Run tests**

```bash
cd crates/wdm-codec && cargo test --lib encoding::tests
```

Expected: all 7 tests pass (2 prior + 5 new).

- [ ] **Step 3: Commit**

```bash
cd /scratch/code/shibboleth/descriptor-mnemonic
git add crates/wdm-codec/src/encoding.rs
git commit -m "feat(encoding): add bech32 alphabet + 8/5-bit pack and unpack"
git push
```

### Task 1.3: HRP, separator, and length validation

**Files:**
- Modify: `crates/wdm-codec/src/encoding.rs`

- [ ] **Step 1: Write failing tests**

Append to `crates/wdm-codec/src/encoding.rs` (before the test module):

```rust
/// The fixed human-readable part for WDM strings.
pub const HRP: &str = "wdm";

/// The bech32 separator character.
pub const SEPARATOR: char = '1';

/// Determine the BchCode variant from a total data-part length.
///
/// Returns `None` for invalid lengths (94 and 95 are reserved-invalid;
/// lengths > 108 or < 14 are also rejected).
pub fn bch_code_for_length(data_part_len: usize) -> Option<BchCode> {
    match data_part_len {
        14..=93 => Some(BchCode::Regular),
        94..=95 => None,
        96..=108 => Some(BchCode::Long),
        _ => None,
    }
}

/// Check whether a string is all-lowercase, all-uppercase, or mixed.
pub fn case_check(s: &str) -> CaseStatus {
    let mut has_lower = false;
    let mut has_upper = false;
    for c in s.chars() {
        if c.is_ascii_lowercase() {
            has_lower = true;
        } else if c.is_ascii_uppercase() {
            has_upper = true;
        }
    }
    match (has_lower, has_upper) {
        (true, true) => CaseStatus::Mixed,
        (true, false) => CaseStatus::Lower,
        (false, true) => CaseStatus::Upper,
        (false, false) => CaseStatus::Lower, // empty / no letters; treat as lower
    }
}

/// Result of a case check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseStatus {
    /// All-lowercase or no letters.
    Lower,
    /// All-uppercase.
    Upper,
    /// Both lowercase and uppercase letters present (invalid).
    Mixed,
}
```

Add to the test module:

```rust
    #[test]
    fn bch_code_for_length_regular() {
        assert_eq!(bch_code_for_length(14), Some(BchCode::Regular));
        assert_eq!(bch_code_for_length(93), Some(BchCode::Regular));
    }

    #[test]
    fn bch_code_for_length_long() {
        assert_eq!(bch_code_for_length(96), Some(BchCode::Long));
        assert_eq!(bch_code_for_length(108), Some(BchCode::Long));
    }

    #[test]
    fn bch_code_for_length_rejects_94_and_95() {
        assert_eq!(bch_code_for_length(94), None);
        assert_eq!(bch_code_for_length(95), None);
    }

    #[test]
    fn bch_code_for_length_rejects_extremes() {
        assert_eq!(bch_code_for_length(0), None);
        assert_eq!(bch_code_for_length(13), None);
        assert_eq!(bch_code_for_length(109), None);
        assert_eq!(bch_code_for_length(1000), None);
    }

    #[test]
    fn case_check_lowercase() {
        assert_eq!(case_check("wdm1qq"), CaseStatus::Lower);
    }

    #[test]
    fn case_check_uppercase() {
        assert_eq!(case_check("WDM1QQ"), CaseStatus::Upper);
    }

    #[test]
    fn case_check_mixed() {
        assert_eq!(case_check("wDm1qq"), CaseStatus::Mixed);
    }
```

- [ ] **Step 2: Run tests**

```bash
cargo test --lib encoding::tests
```

Expected: all tests pass (≥14 tests).

- [ ] **Step 3: Commit**

```bash
cd /scratch/code/shibboleth/descriptor-mnemonic
git add crates/wdm-codec/src/encoding.rs
git commit -m "feat(encoding): add HRP, separator, length validation, case check"
git push
```

### Task 1.4: BCH polynomial constants from BIP 93

**Files:**
- Modify: `crates/wdm-codec/src/encoding.rs`

- [ ] **Step 1: Locate the polynomial coefficients in BIP 93**

Open https://bips.dev/93/ in a browser. Find the section "Generator polynomial". Extract the two arrays of coefficients (one for the regular checksum, one for the long checksum).

The values from BIP 93 Python reference:

```python
GEN = [0xe0e0e0e0e_0_0_0_0_0_0_0,  # actually arrays of 32-bit values per polynomial term
       ...]
```

For implementation purposes, use the values from the BIP 93 reference Python implementation: `https://github.com/bitcoin/bips/blob/master/bip-0093/codex32_secret_share.py` (or whichever location BIP 93 cites; verify the values match the BIP text).

- [ ] **Step 2: Write a test that asserts polynomial array sizes**

Append to `crates/wdm-codec/src/encoding.rs` (before tests):

```rust
/// BCH generator polynomial coefficients for the regular code (BCH(93,80,8)).
///
/// Source: BIP 93 §"Generator polynomial". 13 coefficients in total, one for
/// each checksum character. Each coefficient is a 5-bit value packed into
/// the low bits of a u32, with a 32-bit feedback shift used during encoding.
///
/// **TODO during P1 implementation**: paste the exact 13 values from BIP 93's
/// reference implementation (https://github.com/bitcoin/bips/blob/master/bip-0093/codex32_secret_share.py)
/// after verifying they match the BIP text.
pub const GEN_REGULAR: [u32; 13] = [
    // Placeholder — replace with verified values during implementation.
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

/// BCH generator polynomial coefficients for the long code (BCH(108,93,8)).
/// 15 coefficients. Source: BIP 93 §"Generator polynomial".
pub const GEN_LONG: [u32; 15] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];
```

Add tests:

```rust
    #[test]
    fn gen_regular_has_13_entries() {
        assert_eq!(GEN_REGULAR.len(), 13);
    }

    #[test]
    fn gen_long_has_15_entries() {
        assert_eq!(GEN_LONG.len(), 15);
    }
```

- [ ] **Step 3: Run tests**

```bash
cargo test --lib encoding::tests
```

Expected: all tests pass; the `// TODO` placeholder values still build.

- [ ] **Step 4: Replace placeholder values with verified BIP 93 constants**

Fetch BIP 93 reference impl. The current values from the canonical BIP 93 Python reference (as of mid-2024):

```rust
pub const GEN_REGULAR: [u32; 13] = [
    0xe0e_0e0_e0_e_0_0_0_0_0_0_0,  // ← REPLACE with actual values from BIP 93 reference
    // ...
];
```

**CRITICAL:** This file's accuracy is load-bearing. Cross-check every coefficient byte-for-byte against the BIP 93 reference Python implementation. Use Python to print the values directly:

```python
# From https://github.com/bitcoin/bips/blob/master/bip-0093/codex32_secret_share.py
from codex32_secret_share import GEN
print(','.join(f'0x{c:x}' for c in GEN))
```

Paste the output values (formatted as Rust u32 hex literals) into `GEN_REGULAR` and `GEN_LONG`. The exact array element type and byte ordering must match the reference; consult the BIP 93 spec for whether values are `u32`, `u64`, or symbolic in GF(32).

- [ ] **Step 5: Add a known-vector test against BIP 93 reference**

Add to test module:

```rust
    #[test]
    fn gen_regular_matches_bip93_first_value() {
        // First coefficient from BIP 93 reference (verify against:
        // https://github.com/bitcoin/bips/blob/master/bip-0093/codex32_secret_share.py)
        // Update this test once the actual value is pasted.
        assert_ne!(GEN_REGULAR[0], 0, "GEN_REGULAR placeholder value still in place");
    }
```

- [ ] **Step 6: Commit**

```bash
cd /scratch/code/shibboleth/descriptor-mnemonic
git add crates/wdm-codec/src/encoding.rs
git commit -m "feat(encoding): add BCH generator polynomial constants from BIP 93"
git push
```

### Task 1.5: BCH polynomial multiplication (GF arithmetic)

**Files:**
- Modify: `crates/wdm-codec/src/encoding.rs`

- [ ] **Step 1: Write failing tests**

Append before tests:

```rust
/// Polynomial multiplication in the BCH field, used during checksum
/// computation. Multiplies the running `accum` by `x` (modulo the generator
/// polynomial) and XORs in `value`.
///
/// This is a port of BIP 93's `polymod` step. See the codex32 reference
/// implementation for the canonical form.
fn polymod_step(accum: u32, value: u32, gen: &[u32]) -> u32 {
    let top = accum >> 25;
    let mut new_accum = ((accum & 0x1FFFFFF) << 5) ^ value;
    for (i, &g) in gen.iter().enumerate() {
        if (top >> i) & 1 != 0 {
            new_accum ^= g;
        }
    }
    new_accum
}
```

Add tests:

```rust
    #[test]
    fn polymod_step_zero_input() {
        // Multiplying zero accum by x and XORing zero gives zero.
        assert_eq!(polymod_step(0, 0, &GEN_REGULAR), 0);
    }
```

(Comprehensive validation comes via the round-trip test in Task 1.7.)

- [ ] **Step 2: Run tests**

```bash
cargo test --lib encoding::tests
```

- [ ] **Step 3: Commit**

```bash
cd /scratch/code/shibboleth/descriptor-mnemonic
git add crates/wdm-codec/src/encoding.rs
git commit -m "feat(encoding): add BCH polymod_step (GF arithmetic core)"
git push
```

### Task 1.6: BCH checksum encode + verify (regular code)

**Files:**
- Modify: `crates/wdm-codec/src/encoding.rs`

- [ ] **Step 1: Write failing tests**

Append before existing tests:

```rust
/// Compute the 13-character BCH checksum for the regular code over the
/// given HRP and data part (5-bit values, no checksum).
///
/// Returns the 13-element array of 5-bit checksum values to append.
pub fn bch_checksum_regular(hrp: &str, data: &[u8]) -> [u8; 13] {
    let mut accum: u32 = 1;
    // Mix HRP characters as 5-bit values: high nibble (bits 5-7), then 0, then low nibble (bits 0-4).
    for &c in hrp.as_bytes() {
        accum = polymod_step(accum, (c >> 5) as u32, &GEN_REGULAR);
    }
    accum = polymod_step(accum, 0, &GEN_REGULAR);
    for &c in hrp.as_bytes() {
        accum = polymod_step(accum, (c & 0x1F) as u32, &GEN_REGULAR);
    }
    for &v in data {
        accum = polymod_step(accum, v as u32, &GEN_REGULAR);
    }
    // Append 13 zero values to pull the checksum out.
    for _ in 0..13 {
        accum = polymod_step(accum, 0, &GEN_REGULAR);
    }
    // Final XOR with the BIP 93 "target residue" (defined by the spec; placeholder 1 here).
    accum ^= 1;

    let mut out = [0u8; 13];
    for i in 0..13 {
        out[i] = ((accum >> (5 * (12 - i))) & 0x1F) as u8;
    }
    out
}

/// Verify a regular-code BCH-checksummed string. Takes the HRP and the
/// full data part (data + checksum). Returns true if the checksum matches.
pub fn bch_verify_regular(hrp: &str, data_with_checksum: &[u8]) -> bool {
    if data_with_checksum.len() < 13 {
        return false;
    }
    let mut accum: u32 = 1;
    for &c in hrp.as_bytes() {
        accum = polymod_step(accum, (c >> 5) as u32, &GEN_REGULAR);
    }
    accum = polymod_step(accum, 0, &GEN_REGULAR);
    for &c in hrp.as_bytes() {
        accum = polymod_step(accum, (c & 0x1F) as u32, &GEN_REGULAR);
    }
    for &v in data_with_checksum {
        accum = polymod_step(accum, v as u32, &GEN_REGULAR);
    }
    accum == 1 // target residue from BIP 93
}
```

Add tests:

```rust
    #[test]
    fn bch_checksum_then_verify_regular_round_trip() {
        let hrp = "wdm";
        let data: Vec<u8> = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let checksum = bch_checksum_regular(hrp, &data);
        assert_eq!(checksum.len(), 13);

        let mut data_with_checksum = data.clone();
        data_with_checksum.extend_from_slice(&checksum);
        assert!(bch_verify_regular(hrp, &data_with_checksum));
    }

    #[test]
    fn bch_verify_rejects_corrupted() {
        let hrp = "wdm";
        let data: Vec<u8> = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let checksum = bch_checksum_regular(hrp, &data);
        let mut data_with_checksum = data.clone();
        data_with_checksum.extend_from_slice(&checksum);
        // Flip one bit
        data_with_checksum[5] ^= 0x01;
        assert!(!bch_verify_regular(hrp, &data_with_checksum));
    }
```

- [ ] **Step 2: Run tests** — they will likely FAIL because `GEN_REGULAR` still has placeholder zeros from Task 1.4

Expected: round-trip test fails until Task 1.4's polynomials are filled in with verified BIP 93 values.

- [ ] **Step 3: Verify the round-trip works once Task 1.4 polynomials are correct**

After Task 1.4's `GEN_REGULAR` has the real BIP 93 values:

```bash
cargo test --lib encoding::tests::bch_checksum_then_verify_regular_round_trip
```

Expected: PASS.

- [ ] **Step 4: Cross-check first 3 vectors against BIP 93 reference**

Use the BIP 93 Python reference to compute checksums for 3 known inputs (e.g., the BIP 93 spec's own test vectors). Compare byte-for-byte against `bch_checksum_regular`. Document the comparison in a comment.

- [ ] **Step 5: Commit**

```bash
cd /scratch/code/shibboleth/descriptor-mnemonic
git add crates/wdm-codec/src/encoding.rs
git commit -m "feat(encoding): add bch_checksum_regular + bch_verify_regular with round-trip test"
git push
```

### Task 1.7: BCH checksum encode + verify (long code)

**Files:**
- Modify: `crates/wdm-codec/src/encoding.rs`

- [ ] **Step 1: Write functions analogous to Task 1.6 but for long code**

Append:

```rust
/// Compute the 15-character BCH checksum for the long code.
pub fn bch_checksum_long(hrp: &str, data: &[u8]) -> [u8; 15] {
    let mut accum: u32 = 1;
    for &c in hrp.as_bytes() {
        accum = polymod_step(accum, (c >> 5) as u32, &GEN_LONG);
    }
    accum = polymod_step(accum, 0, &GEN_LONG);
    for &c in hrp.as_bytes() {
        accum = polymod_step(accum, (c & 0x1F) as u32, &GEN_LONG);
    }
    for &v in data {
        accum = polymod_step(accum, v as u32, &GEN_LONG);
    }
    for _ in 0..15 {
        accum = polymod_step(accum, 0, &GEN_LONG);
    }
    accum ^= 1;

    let mut out = [0u8; 15];
    for i in 0..15 {
        out[i] = ((accum >> (5 * (14 - i))) & 0x1F) as u8;
    }
    out
}

/// Verify a long-code BCH-checksummed string.
pub fn bch_verify_long(hrp: &str, data_with_checksum: &[u8]) -> bool {
    if data_with_checksum.len() < 15 {
        return false;
    }
    let mut accum: u32 = 1;
    for &c in hrp.as_bytes() {
        accum = polymod_step(accum, (c >> 5) as u32, &GEN_LONG);
    }
    accum = polymod_step(accum, 0, &GEN_LONG);
    for &c in hrp.as_bytes() {
        accum = polymod_step(accum, (c & 0x1F) as u32, &GEN_LONG);
    }
    for &v in data_with_checksum {
        accum = polymod_step(accum, v as u32, &GEN_LONG);
    }
    accum == 1
}
```

Add tests:

```rust
    #[test]
    fn bch_long_round_trip() {
        let hrp = "wdm";
        let data: Vec<u8> = (0..50).collect();
        let checksum = bch_checksum_long(hrp, &data);
        assert_eq!(checksum.len(), 15);

        let mut full = data.clone();
        full.extend_from_slice(&checksum);
        assert!(bch_verify_long(hrp, &full));
    }
```

- [ ] **Step 2: Run tests**

```bash
cargo test --lib encoding::tests
```

- [ ] **Step 3: Commit**

```bash
cd /scratch/code/shibboleth/descriptor-mnemonic
git add crates/wdm-codec/src/encoding.rs
git commit -m "feat(encoding): add bch_checksum_long + bch_verify_long with round-trip"
git push
```

### Task 1.8: BCH error correction (regular and long)

**Files:**
- Modify: `crates/wdm-codec/src/encoding.rs`

- [ ] **Step 1: Add the correction function skeleton**

Append:

```rust
/// Result of a BCH decode + correct attempt.
#[derive(Debug, Clone)]
pub struct CorrectionResult {
    /// Corrected data + checksum (input may have been modified).
    pub data: Vec<u8>,
    /// Number of substitutions corrected (0 means clean input).
    pub corrections_applied: usize,
    /// Positions (within the data part) of corrected characters.
    pub corrected_positions: Vec<usize>,
}

/// Attempt to correct up to 4 substitution errors in a BCH-checksummed
/// regular-code string. Returns Ok(result) on success, Err on uncorrectable.
pub fn bch_correct_regular(
    hrp: &str,
    data_with_checksum: &[u8],
) -> std::result::Result<CorrectionResult, ()> {
    if bch_verify_regular(hrp, data_with_checksum) {
        return Ok(CorrectionResult {
            data: data_with_checksum.to_vec(),
            corrections_applied: 0,
            corrected_positions: vec![],
        });
    }

    // Brute-force search: try flipping 1, 2, 3, 4 positions to each of the
    // other 31 alphabet values until verify succeeds. For up to 4 errors in
    // a string of ~80–110 chars this is at most C(110,4) * 31^4 ≈ 5×10^12,
    // far too slow. We replace this with the proper BIP 93 syndrome-based
    // correction in a follow-up task; here we provide a 1-error brute force
    // as a working baseline.
    let mut best: Option<CorrectionResult> = None;
    for i in 0..data_with_checksum.len() {
        for v in 0..32u8 {
            if v == data_with_checksum[i] {
                continue;
            }
            let mut trial = data_with_checksum.to_vec();
            trial[i] = v;
            if bch_verify_regular(hrp, &trial) {
                let r = CorrectionResult {
                    data: trial,
                    corrections_applied: 1,
                    corrected_positions: vec![i],
                };
                if best.is_none() {
                    best = Some(r);
                }
            }
        }
    }
    best.ok_or(())
}

/// Same as `bch_correct_regular` but for the long code.
pub fn bch_correct_long(
    hrp: &str,
    data_with_checksum: &[u8],
) -> std::result::Result<CorrectionResult, ()> {
    if bch_verify_long(hrp, data_with_checksum) {
        return Ok(CorrectionResult {
            data: data_with_checksum.to_vec(),
            corrections_applied: 0,
            corrected_positions: vec![],
        });
    }
    let mut best: Option<CorrectionResult> = None;
    for i in 0..data_with_checksum.len() {
        for v in 0..32u8 {
            if v == data_with_checksum[i] {
                continue;
            }
            let mut trial = data_with_checksum.to_vec();
            trial[i] = v;
            if bch_verify_long(hrp, &trial) {
                let r = CorrectionResult {
                    data: trial,
                    corrections_applied: 1,
                    corrected_positions: vec![i],
                };
                if best.is_none() {
                    best = Some(r);
                }
            }
        }
    }
    best.ok_or(())
}
```

Add tests:

```rust
    #[test]
    fn bch_correct_regular_clean_input() {
        let hrp = "wdm";
        let data: Vec<u8> = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let checksum = bch_checksum_regular(hrp, &data);
        let mut full = data.clone();
        full.extend_from_slice(&checksum);
        let r = bch_correct_regular(hrp, &full).unwrap();
        assert_eq!(r.corrections_applied, 0);
    }

    #[test]
    fn bch_correct_regular_one_error() {
        let hrp = "wdm";
        let data: Vec<u8> = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let checksum = bch_checksum_regular(hrp, &data);
        let mut full = data.clone();
        full.extend_from_slice(&checksum);
        let original = full.clone();
        full[3] = (full[3] + 1) & 0x1F;  // 1-char corruption
        let r = bch_correct_regular(hrp, &full).unwrap();
        assert_eq!(r.corrections_applied, 1);
        assert_eq!(r.data, original);
    }
```

- [ ] **Step 2: Run tests**

```bash
cargo test --lib encoding::tests
```

- [ ] **Step 3: Add a TODO note for full BCH syndrome decoding**

Add this comment at the top of the bch_correct_regular function:

```rust
// TODO(v0.2 spec): replace brute-force 1-error correction with proper
// syndrome-based BCH decoding (Berlekamp-Massey / Forney algorithms) to
// achieve the spec-promised 4-error correction in O(n^2) time. v0.1's
// brute-force is correct for ≤1 errors; tests using 2-4 errors will fail
// and are deferred to v0.2.
```

- [ ] **Step 4: Commit**

```bash
cd /scratch/code/shibboleth/descriptor-mnemonic
git add crates/wdm-codec/src/encoding.rs
git commit -m "feat(encoding): add brute-force 1-error BCH correction (regular + long)

Spec promises 4-error correction; v0.1 ships 1-error brute-force baseline.
Full syndrome-based correction deferred to v0.2."
git push
```

> **NOTE for implementer:** The full 4-error BCH correction is a substantial algorithmic task (Berlekamp-Massey / Chien search / Forney's algorithm over GF(2^5)). For v0.1's stated DoD ("BCH correction paths exercised by ≥1 named test"), the brute-force 1-error path satisfies the conformance requirement. If time permits in P5.5 spec reconciliation, replace with the full algorithm; otherwise document the limitation in the BIP and defer.

### Task 1.9: Encode/decode full WDM strings (HRP + data + checksum)

**Files:**
- Modify: `crates/wdm-codec/src/encoding.rs`

- [ ] **Step 1: Write failing tests**

Append:

```rust
/// Encode a payload (raw bytes) into a WDM string with the given header
/// bytes and code variant. Returns the full string including HRP and checksum.
pub fn encode_string(header: &[u8], payload: &[u8], code: BchCode) -> String {
    let mut all_bytes = Vec::with_capacity(header.len() + payload.len());
    all_bytes.extend_from_slice(header);
    all_bytes.extend_from_slice(payload);
    let data_5bit = bytes_to_5bit(&all_bytes);

    let checksum = match code {
        BchCode::Regular => bch_checksum_regular(HRP, &data_5bit).to_vec(),
        BchCode::Long => bch_checksum_long(HRP, &data_5bit).to_vec(),
    };

    let mut full = String::with_capacity(HRP.len() + 1 + data_5bit.len() + checksum.len());
    full.push_str(HRP);
    full.push(SEPARATOR);
    for &v in &data_5bit {
        full.push(ALPHABET[v as usize] as char);
    }
    for &v in &checksum {
        full.push(ALPHABET[v as usize] as char);
    }
    full
}

/// Decoded string: the data part (without HRP/separator/checksum), with
/// the BCH code variant detected from the length.
#[derive(Debug, Clone)]
pub struct DecodedString {
    /// Full data part as 5-bit values (header + payload, no checksum).
    pub data: Vec<u8>,
    /// Detected BCH code variant.
    pub code: BchCode,
    /// Number of substitution errors corrected (0 = clean input).
    pub corrections_applied: usize,
    /// Positions of corrected characters within the data part.
    pub corrected_positions: Vec<usize>,
}

/// Decode a WDM string, validating HRP, case, length, and checksum.
/// Performs error correction up to 1 substitution (v0.1 brute-force baseline).
pub fn decode_string(s: &str) -> std::result::Result<DecodedString, crate::error::Error> {
    use crate::error::Error;

    if matches!(case_check(s), CaseStatus::Mixed) {
        return Err(Error::MixedCase);
    }
    let s_lower = s.to_lowercase();

    let sep_pos = s_lower
        .rfind(SEPARATOR)
        .ok_or_else(|| Error::InvalidHrp(s_lower.clone()))?;
    let (hrp, rest) = s_lower.split_at(sep_pos);
    let data_part = &rest[1..]; // skip the '1' separator

    if hrp != HRP {
        return Err(Error::InvalidHrp(hrp.to_string()));
    }

    let code = bch_code_for_length(data_part.len()).ok_or(Error::InvalidStringLength(data_part.len()))?;

    // Convert characters to 5-bit values
    let mut values = Vec::with_capacity(data_part.len());
    for c in data_part.chars() {
        let v = ALPHABET_INV[c as usize];
        if v == 0xFF {
            return Err(Error::InvalidStringLength(data_part.len()));
        }
        values.push(v);
    }

    let correction = match code {
        BchCode::Regular => bch_correct_regular(hrp, &values),
        BchCode::Long => bch_correct_long(hrp, &values),
    };

    let result = correction.map_err(|_| Error::BchUncorrectable)?;

    let checksum_len = match code {
        BchCode::Regular => 13,
        BchCode::Long => 15,
    };
    let data_only = result.data[..result.data.len() - checksum_len].to_vec();

    Ok(DecodedString {
        data: data_only,
        code,
        corrections_applied: result.corrections_applied,
        corrected_positions: result.corrected_positions,
    })
}
```

Add tests:

```rust
    #[test]
    fn encode_decode_round_trip() {
        let header = vec![0u8; 2];
        let payload = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let s = encode_string(&header, &payload, BchCode::Regular);
        assert!(s.starts_with("wdm1"));

        let decoded = decode_string(&s).unwrap();
        let bytes = five_bit_to_bytes(&decoded.data).unwrap();
        let mut expected = header.clone();
        expected.extend_from_slice(&payload);
        assert_eq!(bytes, expected);
    }

    #[test]
    fn decode_rejects_invalid_hrp() {
        let payload = vec![0u8; 10];
        let s = encode_string(&[], &payload, BchCode::Regular);
        let bad = s.replace("wdm", "btc");
        assert!(matches!(
            decode_string(&bad),
            Err(crate::error::Error::InvalidHrp(_))
        ));
    }

    #[test]
    fn decode_rejects_mixed_case() {
        let payload = vec![0u8; 10];
        let s = encode_string(&[], &payload, BchCode::Regular);
        // Capitalize one letter in the middle of the data part
        let bad = s
            .chars()
            .enumerate()
            .map(|(i, c)| if i == 5 { c.to_ascii_uppercase() } else { c })
            .collect::<String>();
        assert!(matches!(
            decode_string(&bad),
            Err(crate::error::Error::MixedCase)
        ));
    }
```

- [ ] **Step 2: Run tests**

```bash
cargo test --lib encoding::tests
```

- [ ] **Step 3: Commit**

```bash
cd /scratch/code/shibboleth/descriptor-mnemonic
git add crates/wdm-codec/src/encoding.rs
git commit -m "feat(encoding): add encode_string + decode_string with HRP/case/length/BCH validation"
git push
```

---

## Phase 2 — Bytecode Foundation (~3.0 d, parallel with P1)

### Task 2.1: Tag enum (operators 0x00–0x33)

**Files:**
- Modify: `crates/wdm-codec/src/bytecode/tag.rs`

- [ ] **Step 1: Write failing test**

Replace `crates/wdm-codec/src/bytecode/tag.rs`:

```rust
//! Bytecode tag enum.

/// Single-byte tag identifying an operator in the canonical bytecode.
///
/// Values 0x00–0x31 are vendored verbatim from joshdoman/descriptor-codec
/// (CC0). Values 0x32–0x33 are WDM-specific extensions.
///
/// Tag 0x35 (fingerprints block) is reserved for v0.2 and is NOT in the
/// v0.1 enum.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tag {
    // Boolean
    False = 0x00,
    True = 0x01,
    // Top-level wrappers
    Pkh = 0x02,
    Sh = 0x03,
    Wpkh = 0x04,
    Wsh = 0x05,
    Tr = 0x06,
    Bare = 0x07,
    // Taproot
    TapTree = 0x08,
    // Multisig
    SortedMulti = 0x09,
    // Wrappers
    Alt = 0x0A,
    Swap = 0x0B,
    Check = 0x0C,
    DupIf = 0x0D,
    Verify = 0x0E,
    NonZero = 0x0F,
    ZeroNotEqual = 0x10,
    // Logical
    AndV = 0x11,
    AndB = 0x12,
    AndOr = 0x13,
    OrB = 0x14,
    OrC = 0x15,
    OrD = 0x16,
    OrI = 0x17,
    Thresh = 0x18,
    Multi = 0x19,
    MultiA = 0x1A,
    // Key scripts
    PkK = 0x1B,
    PkH = 0x1C,
    RawPkH = 0x1D,
    // Timelocks
    After = 0x1E,
    Older = 0x1F,
    // Hashes
    Sha256 = 0x20,
    Hash256 = 0x21,
    Ripemd160 = 0x22,
    Hash160 = 0x23,
    // 0x24–0x31 are reserved for descriptor-codec compatibility
    // (key origins, inline keys, wildcards) but not used in v0.1's
    // wallet-policy framing. Listed as Reserved for now.
    ReservedOrigin = 0x24,
    ReservedNoOrigin = 0x25,
    ReservedUncompressedFullKey = 0x26,
    ReservedCompressedFullKey = 0x27,
    ReservedXOnly = 0x28,
    ReservedXPub = 0x29,
    ReservedMultiXPub = 0x2A,
    ReservedUncompressedSinglePriv = 0x2B,
    ReservedCompressedSinglePriv = 0x2C,
    ReservedXPriv = 0x2D,
    ReservedMultiXPriv = 0x2E,
    ReservedNoWildcard = 0x2F,
    ReservedUnhardenedWildcard = 0x30,
    ReservedHardenedWildcard = 0x31,
    // WDM-specific extensions
    Placeholder = 0x32,
    SharedPath = 0x33,
}

impl Tag {
    /// Convert from a raw byte. Returns None for unknown values.
    pub fn from_byte(b: u8) -> Option<Self> {
        if b > 0x33 {
            return None;
        }
        // Safety: we just bounds-checked.
        Some(unsafe { std::mem::transmute(b) })
    }

    /// Convert to its byte value.
    pub fn as_byte(self) -> u8 {
        self as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_round_trip_all_defined() {
        for b in 0u8..=0x33 {
            let t = Tag::from_byte(b);
            assert!(t.is_some(), "byte {:#04x} should be a valid tag", b);
            assert_eq!(t.unwrap().as_byte(), b);
        }
    }

    #[test]
    fn tag_rejects_unknown_bytes() {
        for b in 0x34u8..=0xFF {
            assert!(Tag::from_byte(b).is_none(), "byte {:#04x} should be rejected", b);
        }
    }

    #[test]
    fn tag_specific_values() {
        assert_eq!(Tag::Wsh.as_byte(), 0x05);
        assert_eq!(Tag::PkK.as_byte(), 0x1B);
        assert_eq!(Tag::Sha256.as_byte(), 0x20);
        assert_eq!(Tag::Placeholder.as_byte(), 0x32);
        assert_eq!(Tag::SharedPath.as_byte(), 0x33);
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cd crates/wdm-codec && cargo test --lib bytecode::tag::tests
```

Expected: 3 tests pass.

- [ ] **Step 3: Update bytecode/mod.rs to expose tag**

Replace `crates/wdm-codec/src/bytecode/mod.rs`:

```rust
//! Bytecode encoding and decoding for canonical WDM bytecode.

pub mod decode;
pub mod encode;
pub mod key;
pub mod path;
pub mod tag;
pub mod varint;

pub use tag::Tag;
```

- [ ] **Step 4: Commit**

```bash
cd /scratch/code/shibboleth/descriptor-mnemonic
git add crates/wdm-codec/src/bytecode/
git commit -m "feat(bytecode): add Tag enum (0x00-0x33) with from_byte/as_byte"
git push
```

### Task 2.2: LEB128 varint encode

**Files:**
- Modify: `crates/wdm-codec/src/bytecode/varint.rs`

- [ ] **Step 1: Write failing tests**

Replace `crates/wdm-codec/src/bytecode/varint.rs`:

```rust
//! LEB128 unsigned variable-length integer encoding.

/// Encode an unsigned u64 as LEB128 bytes, appending to `out`.
pub fn encode_u64(mut value: u64, out: &mut Vec<u8>) {
    loop {
        let byte = (value & 0x7F) as u8;
        value >>= 7;
        if value == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

/// Decode an unsigned LEB128 value from a byte slice. Returns the value and
/// number of bytes consumed, or None if input is malformed (truncated or
/// overflow).
pub fn decode_u64(bytes: &[u8]) -> Option<(u64, usize)> {
    let mut value: u64 = 0;
    let mut shift: u32 = 0;
    for (i, &b) in bytes.iter().enumerate() {
        if shift >= 64 {
            return None; // overflow
        }
        let chunk = (b & 0x7F) as u64;
        value |= chunk
            .checked_shl(shift)
            .ok_or(())
            .ok()?;
        if b & 0x80 == 0 {
            return Some((value, i + 1));
        }
        shift += 7;
    }
    None // truncated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_zero() {
        let mut buf = Vec::new();
        encode_u64(0, &mut buf);
        assert_eq!(buf, vec![0]);
        assert_eq!(decode_u64(&buf), Some((0, 1)));
    }

    #[test]
    fn encode_decode_127() {
        let mut buf = Vec::new();
        encode_u64(127, &mut buf);
        assert_eq!(buf, vec![0x7F]);
        assert_eq!(decode_u64(&buf), Some((127, 1)));
    }

    #[test]
    fn encode_decode_128() {
        let mut buf = Vec::new();
        encode_u64(128, &mut buf);
        assert_eq!(buf, vec![0x80, 0x01]);
        assert_eq!(decode_u64(&buf), Some((128, 2)));
    }

    #[test]
    fn encode_decode_known_timelocks() {
        // 1200000 (block height): 3 bytes
        let mut buf = Vec::new();
        encode_u64(1200000, &mut buf);
        assert_eq!(buf.len(), 3);
        assert_eq!(decode_u64(&buf), Some((1200000, 3)));

        // 4032 (~28 days): 2 bytes
        let mut buf = Vec::new();
        encode_u64(4032, &mut buf);
        assert_eq!(buf.len(), 2);
        assert_eq!(decode_u64(&buf), Some((4032, 2)));

        // 52560 (1 year): 3 bytes
        let mut buf = Vec::new();
        encode_u64(52560, &mut buf);
        assert_eq!(buf.len(), 3);
        assert_eq!(decode_u64(&buf), Some((52560, 3)));
    }

    #[test]
    fn decode_rejects_truncated() {
        // 0x80 with no continuation
        assert_eq!(decode_u64(&[0x80]), None);
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test --lib bytecode::varint::tests
```

Expected: 5 tests pass.

- [ ] **Step 3: Commit**

```bash
cd /scratch/code/shibboleth/descriptor-mnemonic
git add crates/wdm-codec/src/bytecode/varint.rs
git commit -m "feat(varint): add LEB128 encode/decode for u64"
git push
```

### Task 2.3: WdmKey enum

**Files:**
- Modify: `crates/wdm-codec/src/bytecode/key.rs`

- [ ] **Step 1: Write failing test**

Replace `crates/wdm-codec/src/bytecode/key.rs`:

```rust
//! WdmKey: the v0.1 wrapper around descriptor-codec key handling.

use miniscript::descriptor::DescriptorPublicKey;

/// A key reference in the canonical bytecode.
///
/// In v0.1, every key position in a WDM-encoded wallet policy is a
/// `Placeholder(u8)` referencing the BIP 388 key information vector at
/// that index. The `Key(DescriptorPublicKey)` variant exists for v1+
/// foreign-xpub support and is unused in v0.1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WdmKey {
    /// BIP 388 `@i` placeholder.
    Placeholder(u8),
    /// Inline xpub. Reserved for v1+; v0.1 must reject.
    Key(DescriptorPublicKey),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_construction() {
        let k = WdmKey::Placeholder(0);
        assert_eq!(k, WdmKey::Placeholder(0));
        assert_ne!(k, WdmKey::Placeholder(1));
    }
}
```

- [ ] **Step 2: Run test**

```bash
cargo test --lib bytecode::key::tests
```

- [ ] **Step 3: Commit**

```bash
cd /scratch/code/shibboleth/descriptor-mnemonic
git add crates/wdm-codec/src/bytecode/key.rs
git commit -m "feat(bytecode): add WdmKey enum with Placeholder + Key variants"
git push
```

### Task 2.4–2.20: Encoder/Decoder for each operator

> **Note for implementer:** Tasks 2.4 through 2.20 implement encoder and decoder logic for each operator group. Each follows the same pattern: write a unit test that constructs a known-value AST node, encodes it, asserts the bytecode matches expected hex; then decode, assert it round-trips.
>
> Rather than enumerate ~15 individual tasks at this level of detail (which would balloon this plan to 1000s of lines), the implementer should structure the work as:
>
> - Task 2.4: Encoder skeleton — empty function `encode_miniscript` that walks a miniscript AST and emits tags
> - Task 2.5: Add encoder arms for `wsh`, `pk_k`, `pk_h` (3 operators) — write 3 unit tests, encode each, commit
> - Task 2.6: Add encoder arms for `multi`, `sortedmulti`, `multi_a` — same pattern
> - Task 2.7: Add encoder arms for `and_v`, `and_b`, `andor`, `or_b`, `or_c`, `or_d`, `or_i` — same
> - Task 2.8: Add encoder arms for `thresh`, `after`, `older`
> - Task 2.9: Add encoder arms for hash literals `sha256`, `hash256`, `ripemd160`, `hash160`
> - Task 2.10: Add encoder arms for wrappers `v:`, `c:`, `s:`, `a:`, `d:`, `n:`, `j:`, `t:`, `l:`, `u:`
> - Task 2.11: Add encoder arms for `tr` and `TapTree` (taproot) — **DEFER to v0.2**
>
> Tasks 2.12–2.20 mirror these for the decoder (each operator's decode arm).
>
> Each task:
> 1. Write failing unit test with concrete miniscript fragment + expected bytecode hex
> 2. Run test, see fail
> 3. Add the encoder/decoder arms in encode.rs and decode.rs
> 4. Run test, see pass
> 5. Commit with message `feat(bytecode): encode/decode <operator-group>`
>
> **Reference for expected bytecode hex:** Use the descriptor-codec source code directly (it implements the same encoding for these operators) plus the `design/CORPUS.md` byte-level breakdowns for entries C1–C5.
>
> **Time budget:** Tasks 2.4–2.20 collectively are 16 of P2's 24 work-hours (3 days × 8 hours). About 1 hour per operator-group task on average. Expect P2 overrun of 0.5–1 day if any operator's encoding diverges from descriptor-codec in subtle ways.

---

## Phases 3–10 — Outline (full task expansion deferred)

The remaining phases follow the same TDD pattern. Below is a high-level task outline; the implementer should expand each into the same 5-step structure during execution. Each item below is one task unit (~30 min – 2 hours).

### Phase 3 — WDM Extensions (~1.0 d)

- Task 3.1: Bytecode header byte (version + flags); `from_byte`/`as_byte` round-trip test
- Task 3.2: Path dictionary const table (BIP 44/49/84/86/48/87 mainnet + testnet)
- Task 3.3: Path encoding function — `encode_path(path, &dict) -> Vec<u8>` with explicit-LEB128 fallback
- Task 3.4: Path decoding function (inverse)
- Task 3.5: `IndexMap<DerivationPath, u8>` for path emission ordering
- Task 3.6: Encoder arm for `Tag::Placeholder` (reads `@i` index, emits 2 bytes)
- Task 3.7: Encoder arm for `Tag::SharedPath` (emits path declaration)
- Task 3.8: Decoder arms for both
- Task 3.9: End-to-end test: encode `wsh(pk(@0/**))` → bytecode → `WalletPolicy` → identical canonical string

### Phase 4 — Chunking (~1.0 d)

- Task 4.1: `ChunkHeader` struct in `chunking.rs` with explicit field layout
- Task 4.2: `ChunkHeader::to_bytes` + `from_bytes` round-trip test
- Task 4.3: `WalletId` type in `wallet_id.rs` with hex Display + LowerHex + AsRef + From
- Task 4.4: `WalletIdWords` type with Display impl (space-joined) + IntoIterator
- Task 4.5: `ChunkWalletId(u32)` newtype with MAX const + `new()` panicking constructor
- Task 4.6: `WalletId::truncate() -> ChunkWalletId` + `WalletId::to_words() -> WalletIdWords`
- Task 4.7: `compute_wallet_id(policy)` from canonical bytecode SHA-256
- Task 4.8: `chunking_decision(bytecode_len, force) -> ChunkingPlan` enum
- Task 4.9: `chunk_bytes(bytecode, plan, wallet_id) -> Vec<Chunk>` (no encoding yet)
- Task 4.10: `reassemble_chunks(parsed_chunks) -> Result<Vec<u8>, Error>` with all validations

### Phase 5 — Top-Level API Wiring (~0.5 d)

- Task 5.1: `WalletPolicy` struct with `inner: miniscript::descriptor::WalletPolicy` field
- Task 5.2: `impl FromStr for WalletPolicy` delegating to miniscript
- Task 5.3: `to_canonical_string` + `canonicalize` (whitespace, `/**` expansion, `'` for hardened)
- Task 5.4: `key_count`, `shared_path`, `inner` accessors
- Task 5.5: `to_bytecode` and `from_bytecode` methods on `WalletPolicy`
- Task 5.6: Free functions `encode_bytecode`, `decode_bytecode`
- Task 5.7: Top-level `encode(policy, &EncodeOptions) -> Result<WdmBackup>` wiring
- Task 5.8: Top-level `decode(strings, &DecodeOptions) -> Result<DecodeResult>` wiring
- Task 5.9: `EncodeOptions`, `DecodeOptions`, `DecodeReport`, `Verifications`, `Confidence`, `Correction`, `WalletIdSeed`, `WdmBackup`, `EncodedChunk`, `DecodeOutcome`, `DecodeResult` types in lib.rs / chunking.rs
- Task 5.10: Upgrade CI from `cargo check` to `cargo test`; add clippy + fmt --check + doc

### Phase 5.5 — Spec Reconciliation (~0.5 d)

- Task 5.5.1: Grep for `// TODO: spec` markers across all source files; record each
- Task 5.5.2: Resolve each marker by either: (a) updating the BIP draft, (b) updating POLICY_BACKUP.md, or (c) confirming the spec is correct and removing the marker
- Task 5.5.3: Commit BIP edits as a single focused commit before any test vectors generated
- Task 5.5.4: Update POLICY_BACKUP.md DECIDE markers (HRP "wdm", `'` for hardened, etc.) to RESOLVED with date

### Phase 6 — Test Corpus (~1.5 d)

- Task 6.1: `tests/common/mod.rs` with `round_trip_assert`, `corrupt_n`, `load_vector`, `assert_structural_eq`
- Task 6.2: `tests/corpus.rs` C1: `wsh(pk(@0/**))`
- Task 6.3: `tests/corpus.rs` C2: `wsh(sortedmulti(2,@0/**,@1/**,@2/**))`
- Task 6.4: `tests/corpus.rs` C3: `wsh(or_d(multi(2,@0/**,@1/**),and_v(v:older(52560),pk(@2/**))))`
- Task 6.5: `tests/corpus.rs` C4: 6-key inheritance miniscript (the user's example from earlier)
- Task 6.6: `tests/corpus.rs` C5: 5-of-9 + 2-key recovery (long-code boundary)
- Task 6.7: `tests/corpus.rs` E10: Liana Simple Inheritance
- Task 6.8: `tests/corpus.rs` E12: Liana Expanding Multisig
- Task 6.9: `tests/corpus.rs` E13: HTLC with sha256 inline hash
- Task 6.10: `tests/corpus.rs` E14: Decaying multisig 3-of-3 → 2-of-3 with 6 distinct keys
- Task 6.11: `tests/corpus.rs` Coldcard-exported BIP 388 policy (sourced from Coldcard docs / Stack Exchange)
- Task 6.12: `tests/corpus.rs` encode-decode-encode idempotency loop
- Task 6.13: `tests/corpus.rs` HRP-lowercase property check
- Task 6.14: `tests/upstream_shapes.rs` 9 descriptor-codec policy shapes rewritten in `@i` form
- Task 6.15: `tests/chunking.rs` `chunk_hash_mismatch_rejects`
- Task 6.16: `tests/chunking.rs` `chunk_hash_correct_reassembly`
- Task 6.17: `tests/chunking.rs` `chunk_out_of_order_reassembly`
- Task 6.18: `tests/chunking.rs` `natural_long_code_boundary`
- Task 6.19: `tests/ecc.rs` deterministic constructed BCH stress (single-substitution every position)
- Task 6.20: `tests/ecc.rs` `many_substitutions_always_rejected` fixed-seed loop (N=1000)
- Task 6.21: `tests/conformance.rs` macro definition + 18 named rejects_*
- Task 6.22: `tests/error_coverage.rs` strum-based exhaustiveness over Error variants

### Phase 7 — CLI Binary (~1.0 d)

- Task 7.1: `bin/wdm.rs` clap derive setup with subcommand enum
- Task 7.2: `wdm encode` subcommand (calls `encode`, prints JSON or human form)
- Task 7.3: `wdm decode` subcommand (multi-string input, prints DecodeReport)
- Task 7.4: `wdm verify` subcommand (decode + match against expected policy)
- Task 7.5: `wdm inspect` subcommand (parse + report without full decode)
- Task 7.6: `wdm bytecode` subcommand (hex dump of canonical bytecode)
- Task 7.7: `wdm vectors` subcommand (delegates to gen_vectors logic)
- Task 7.8: Path argument parser (tries name → hex byte → literal path)

### Phase 8 — Test Vector Generation (~0.5 d)

- Task 8.1: `src/vectors.rs` schema types (`TestVectorFile`, `Vector`, `NegativeVector`) with serde derive
- Task 8.2: `bin/gen_vectors.rs` clap setup + `--output`/`--verify` modes
- Task 8.3: `--output` mode: generate from corpus + conformance suite, write JSON
- Task 8.4: `--verify` mode: deserialize committed → regenerate in-memory → typed compare
- Task 8.5: Run `cargo run --bin gen_vectors -- --output tests/vectors/v0.1.json`
- Task 8.6: Inspect generated JSON for sanity; commit
- Task 8.7: `tests/vectors_schema.rs` deserializes committed JSON, asserts structural invariants
- Task 8.8: Update `bip/bip-wallet-descriptor-mnemonic.mediawiki` "Test Vectors" section to reference the JSON via permalink + content hash

### Phase 9 — Documentation (~0.5 d)

- Task 9.1: rustdoc on every public item (build clean under `#![deny(missing_docs)]`)
- Task 9.2: Crate-level rustdoc with usage example
- Task 9.3: Update `crates/wdm-codec/README.md` with quickstart
- Task 9.4: Update root `README.md` "Status" section to reflect ref-impl exists
- Task 9.5: Update `bip/README.md` to point to ref impl

### Phase 10 — Pre-Release Review + Tag (~0.5 d)

- Task 10.1: Run full CI locally (`cargo test`, `cargo clippy`, `cargo fmt --check`, `cargo doc`, `gen_vectors --verify`)
- Task 10.2: Verify CI green on Linux + Windows + macOS (sanity)
- Task 10.3: Run `cargo-llvm-cov --lcov` and confirm line coverage ≥ 85%
- Task 10.4: Self-review all public API against `design/IMPLEMENTATION_PLAN_v0.1.md` §3
- Task 10.5: Verify every Error variant produced by ≥1 negative test (run `tests/error_coverage.rs`)
- Task 10.6: Commit any final polish; tag `git tag wdm-codec-v0.1.0 && git push --tags`
- Task 10.7: Update root `README.md` status from "Pre-Draft, AI only" to "Pre-Draft, AI + ref impl, awaiting human review"
- Task 10.8: Update memory file at `~/.claude/projects/-scratch-code-shibboleth-descriptor-mnemonic/memory/project_shibboleth_wallet.md` to reflect v0.1 ref impl exists

---

## Spec Coverage Check

This plan covers every section of `design/IMPLEMENTATION_PLAN_v0.1.md`:

- §1 Scope — establishment in P0; v0.1 scope enforced by `tests/conformance.rs` rejection cases
- §2 Architecture — implemented across P0 (skeleton) + P1–P4 (modules)
- §3 Public API — implemented in P5; covered by every integration test
- §4 Data flow — implemented across P1–P5; covered by `tests/corpus.rs` round-trips
- §5 Test strategy — implemented in P6
- §6 Build order — this document IS the executable form of §6
- §7 Risk register — risks addressed in respective phases
- §8 Definition of done — gated at P10
- §9 v0.2 / v0.3 staging — out-of-scope for this plan
- §10 References — fully cited inline

## Plan Self-Review Notes

**Granularity disclosure (honest):** This plan provides full TDD-step detail (5 substeps with code blocks and commands) for **Phase 0 + Phase 1 + Phases 2.1–2.3** (the foundation and bedrock infrastructure — about 3 days of work). For **Tasks 2.4 onward through Phase 10** (the remaining ~7 days), the plan provides task-level outlines without expanded step-level code. The deferred expansion is a deliberate trade-off: each remaining task follows the same TDD pattern (red → green → commit), and the implementer has the spec (`design/IMPLEMENTATION_PLAN_v0.1.md` §3 for the complete API surface, §4 for data flow, §5 for test layout) plus the reference corpus (`design/CORPUS.md` with byte-level breakdowns) plus the BIP draft for authoritative semantics. Fully expanding ~75 tasks at 5-step granularity would balloon this document past 5,000 lines without adding much value beyond the spec the implementer already has open.

**Recommended execution mode given this granularity:** Subagent-Driven Development. Each task delegated to a fresh subagent has the full context window to fill in the TDD detail from the spec + this outline. The subagent receives "Implement Task 6.4 from `design/IMPLEMENTATION_TASKS_v0.1.md` per the TDD pattern" and produces the 5-step expansion in its own session.

**Placeholder scan:** Tasks P0–P2.3 contain no "TBD"/"TODO"/"implement later". Tasks 2.4 onward have task-level descriptions only (no per-step code). The implementer or subagent must produce the per-step TDD detail. This is documented above.

**Type consistency:** Type names are stable across tasks (`WdmKey`, `WalletId`, `ChunkWalletId`, `ChunkHeader`, `Tag`, `BchCode`, `WdmBackup`, `EncodedChunk`, `DecodeReport`, etc.) and match `design/IMPLEMENTATION_PLAN_v0.1.md` §3. Spec coverage is verified above.

**Risk acknowledgments:**
- Task 1.4: `GEN_REGULAR` and `GEN_LONG` arrays must be filled with verified BIP 93 reference values; placeholder zeros will fail subsequent BCH tests until corrected. The implementer should fetch from the BIP 93 reference Python implementation and cross-check first 3 values.
- Task 1.8: BCH brute-force 1-error correction is a v0.1 baseline; full 4-error correction (Berlekamp-Massey / Forney) is deferred to v0.2 with the limitation documented in the BIP.
- Tasks 2.4–2.20 are summarized at the operator-group level; per-operator detail is the implementer's (or subagent's) responsibility. Reference: descriptor-codec source for tag-emission patterns + `design/CORPUS.md` for expected byte counts.
- Tasks 3.1–10.8 follow the same outline-only treatment.

**What this plan does NOT replace:**
- The implementation spec (`design/IMPLEMENTATION_PLAN_v0.1.md`) — read first
- The BIP draft (`bip/bip-wallet-descriptor-mnemonic.mediawiki`) — authoritative semantics
- The corpus (`design/CORPUS.md`) — concrete test cases
- The prior-art survey (`design/PRIOR_ART.md`) — design rationale
- BIP 93 reference implementation (external) — BCH polynomial values

The implementer or subagent must read those documents before executing tasks.

---

## Execution Handoff

**Plan complete and saved to `design/IMPLEMENTATION_TASKS_v0.1.md`. Two execution options:**

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration. Reduces context bloat in the main session.

**2. Inline Execution** — Execute tasks in this session using `superpowers:executing-plans`, batch execution with checkpoints for review. More direct but consumes more context per task.

**Which approach?**
