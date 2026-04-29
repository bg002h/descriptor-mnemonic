# v0.7.0 Implementation Plan — test rebaseline + defensive corpus + md-signer-compat + policy compiler

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans`. Steps use checkbox (`- [ ]`) syntax for tracking. Per-phase Opus reviews dispatched per the user's autonomous workflow; reports persisted to `design/agent-reports/`.

**Goal:** Ship v0.7.0 — first post-strip-Layer-3 release. Repair 38 byte-literal tests broken by v0.6 wire-format reorganization; add defensive hand-AST tests for typing-awkward operators + hash byte-order pin; ship `md-signer-compat` workspace crate with named subsets; expose policy compiler wrapper API.

**Architecture:** Single coordinated release on `feature/v0.7.0-development` branch. Wire format byte-identical to v0.6.x; family-stable SHA reset rolls token only. New workspace member crate at `crates/md-signer-compat/`. Optional `compiler` cargo feature on md-codec.

**Test discipline:** Phase 1 is regen-driven (rebaseline existing tests). Phases 2/3/4/5 are TDD where structurally appropriate (new tests written first). Phase 6 is release plumbing.

**Spec reference:** `design/SPEC_v0_7_0.md` (round-1 review folded; see `design/agent-reports/v0-7-0-spec-review-1.md`).

---

## File structure

| File | Responsibility | Phase |
|---|---|---|
| `crates/md-codec/src/bytecode/{encode,decode}.rs` | Test rebaseline target (test modules) | 1 |
| `crates/md-codec/src/bytecode/path/...` | Test rebaseline target (test modules) | 1 |
| `crates/md-codec/src/policy.rs` | Test rebaseline target (test modules) | 1 |
| `crates/md-codec/src/vectors.rs` | Test rebaseline target (test module) | 1 |
| `crates/md-codec/tests/taproot.rs` (or new `tests/hand_ast_coverage.rs`) | Hand-AST tests + byte-order pin + per-arm decoder tests | 2 |
| `crates/md-codec/src/bytecode/encode.rs` | `validate_tap_leaf_subset_with_allowlist` refactor; `validate_tap_leaf_subset` shim | 3 |
| `crates/md-codec/src/lib.rs` | New pub re-exports if any | 3, 5 |
| `crates/md-signer-compat/Cargo.toml` (NEW) | Workspace member crate manifest | 4 |
| `crates/md-signer-compat/src/{lib,coldcard,ledger,tests}.rs` (NEW) | SignerSubset + named subsets + tests | 4 |
| `crates/md-signer-compat/README.md` (NEW) | Crate readme | 4 |
| `Cargo.toml` (workspace) | Add `crates/md-signer-compat` to members | 4 |
| `crates/md-codec/Cargo.toml` | `compiler` + `cli-compiler` feature definitions | 5 |
| `crates/md-codec/src/{policy_compiler.rs,lib.rs}` | `policy_to_bytecode` wrapper + `ScriptContext` enum | 5 |
| `crates/md-codec/src/bin/md/main.rs` | `--from-policy` CLI mode | 5 |
| `CHANGELOG.md` | `[0.7.0]` section | 6 |
| `MIGRATION.md` | `v0.6.x → v0.7.0` (no-breaking-changes summary) | 6 |
| `crates/md-codec/Cargo.toml` | Version 0.6.0 → 0.7.0 | 6 |
| `crates/md-codec/tests/vectors/{v0.1,v0.2}.json` | Regenerated with `"md-codec 0.7"` family token | 6 |
| `crates/md-codec/tests/vectors_schema.rs` | SHA pin updated | 6 |

---

## Phase 1 — Test rebaseline (Track A)

**Goal:** Repair 38 unit tests that pin v0.5 byte literals; replace literals with symbolic `Tag::Foo.as_byte()` references where practical.

**Strategy:** systematic walk through failing tests. For each:
1. Run the failing test in isolation.
2. Identify the byte-literal sites (typically `vec![...]` initializers).
3. Replace literals with symbolic refs where the byte represents a specific Tag.
4. Update inline-comment annotations that named v0.5 byte values.
5. Verify the test passes.

**Per-phase agent report**: `design/agent-reports/v0-7-0-phase-1-review.md` documents each test fix with file:line + old-bytes → new-bytes (per spec §8 acceptance criterion #3).

### Task 1.1 — Inventory failing tests

- [ ] **Step 1.1.1: Capture baseline**

Run: `cargo test -p md-codec 2>&1 | grep "^test " | grep "FAILED" > /tmp/v0-7-baseline-failures.txt && wc -l /tmp/v0-7-baseline-failures.txt`

Expected: ~38 lines.

- [ ] **Step 1.1.2: Categorize**

Group failing tests by file. Expected breakdown per spec §2.1:
- `bytecode::decode::tests::*_known_vector` (~10)
- `bytecode::decode::tests::*_rejects_*` (~8)
- `bytecode::encode::tests::encode_terminal_*` (~10)
- `bytecode::path::tests::*` (~6)
- `policy::tests::*` + `vectors::tests::*` (~5)

### Task 1.2 — Rebaseline `bytecode::decode::tests`

- [ ] **Step 1.2.1: For each failing test in decode.rs's tests module**, walk the test body, identify byte-literal sites, replace with symbolic refs.

Mapping table (per spec §2.2):

| Operator | v0.5 byte | v0.6 byte (current) |
|---|---|---|
| `Tag::TapTree` | 0x08 | 0x07 |
| `Tag::Multi` | 0x19 | 0x08 |
| `Tag::MultiA` | 0x1A | 0x0A |
| `Tag::Alt` | 0x0A | 0x0C |
| `Tag::Swap` | 0x0B | 0x0D |
| `Tag::Check` | 0x0C | 0x0E |
| `Tag::DupIf` | 0x0D | 0x0F |
| `Tag::Verify` | 0x0E | 0x10 |
| `Tag::NonZero` | 0x0F | 0x11 |
| `Tag::ZeroNotEqual` | 0x10 | 0x12 |
| `Tag::AndV` | 0x11 | 0x13 |
| `Tag::AndB` | 0x12 | 0x14 |
| `Tag::AndOr` | 0x13 | 0x15 |
| `Tag::OrB` | 0x14 | 0x16 |
| `Tag::OrC` | 0x15 | 0x17 |
| `Tag::OrD` | 0x16 | 0x18 |
| `Tag::OrI` | 0x17 | 0x19 |
| `Tag::Thresh` | 0x18 | 0x1A |
| `Tag::Placeholder` | 0x32 | 0x33 |
| `Tag::SharedPath` | 0x33 | 0x34 |

Per-test pattern:

```rust
// BEFORE
let bytes = vec![0x05, 0x16, 0x00, 0x01];
// = [Wsh, OrD, False, True]

// AFTER (symbolic refs preferred per user direction)
let bytes = vec![Tag::Wsh.as_byte(), Tag::OrD.as_byte(), Tag::False.as_byte(), Tag::True.as_byte()];
```

For multi-byte payloads (e.g., placeholder index `0x00`, varint values, Threshold k/n bytes), keep numeric literals — those are payload bytes, not Tag references.

- [ ] **Step 1.2.2: Run tests after each file's fixes**

Run: `cargo test -p md-codec --lib bytecode::decode::tests 2>&1 | grep "test result"`

Expected: all decode tests passing after the rebaseline.

### Task 1.3 — Rebaseline `bytecode::encode::tests`

Same pattern as Task 1.2, applied to encode.rs's tests module.

- [ ] **Step 1.3.1**: Walk each failing encode test; replace byte literals with symbolic refs.
- [ ] **Step 1.3.2**: Run tests; expect all encode tests passing.

### Task 1.4 — Rebaseline `bytecode::path::tests`

- [ ] **Step 1.4.1**: SharedPath byte (0x33→0x34) is the primary shift here; symbolic refs avoid future re-breaks.
- [ ] **Step 1.4.2**: Run tests.

### Task 1.5 — Rebaseline `policy::tests` and `vectors::tests`

- [ ] **Step 1.5.1**: Placeholder byte (0x32→0x33) primary shift.
- [ ] **Step 1.5.2**: Run tests.

### Task 1.6 — Full suite + commit

- [ ] **Step 1.6.1: Full suite pass**

Run: `cargo test -p md-codec 2>&1 | tail -5`

Expected: 432 of 432 pass; 0 failures.

- [ ] **Step 1.6.2: Commit**

```bash
git add -A
git commit -m "test(v0.7 phase 1): rebaseline 38 byte-literal tests

[full per-test fix log in commit body or agent report]

Closes FOLLOWUPS v06-test-byte-literal-rebaseline.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

### Task 1.7 — Phase 1 review

- [ ] **Step 1.7.1: Dispatch Opus reviewer**

Brief:
- Files: every test file modified in Phase 1.
- Verify: every fix replaces v0.5 byte literals with symbolic Tag refs (or v0.6 byte literals with rationale); no test SEMANTICS changed; no `#[ignore]` markers introduced; full suite green.
- Output: `design/agent-reports/v0-7-0-phase-1-review.md`.

- [ ] **Step 1.7.2: Address findings inline**

---

## Phase 2 — Defensive corpus growth (Track B)

**Goal:** Hand-AST tests for typing-awkward operators (or_c, d:, j:, n: wrappers); hash byte-order defensive pin; per-arm decoder unit tests.

**File:** `crates/md-codec/tests/hand_ast_coverage.rs` (NEW) — keeps defensive tests separate from corpus-driven tests in `taproot.rs`.

### Task 2.0 — Investigate decoder round-trip behavior for V-type leaves

- [ ] **Step 2.0.1: Test unwrapped or_c decode**

Synthesize bytecode for `or_c(pk_k(a), v:pk_k(b))` directly (skipping the encoder; build the byte sequence by hand). Feed to `decode_tap_terminal`. Observe: does it round-trip, or does it hit the `Miniscript::from_ast` typecheck-failed error?

- [ ] **Step 2.0.2: Document outcome**

If decoder round-trips: §3.1 of the spec applies as-is (use unwrapped or_c in hand-AST tests).

If decoder rejects: §3.1 spec fold-in O-4 applies (wrap in `t:or_c` = `and_v(or_c, true)` to produce B-type at top). Note the outcome in `design/agent-reports/v0-7-0-phase-2-review.md`.

### Task 2.1 — Hand-AST tests for typing-awkward operators

- [ ] **Step 2.1.1: Create `tests/hand_ast_coverage.rs`**

Skeleton:

```rust
//! Hand-AST coverage for tap-leaf operators that BIP 388 source-form
//! parsers reject due to top-level B-type requirement. These tests
//! bypass the parser via `Miniscript::from_ast` and assert the wire-byte
//! form of the encoded AST directly.
//!
//! Per spec §3.1 of `design/SPEC_v0_7_0.md`.

use std::str::FromStr;
use std::sync::Arc;
use std::collections::HashMap;

use bitcoin::hashes::Hash;
use miniscript::{DescriptorPublicKey, Miniscript, Tap, Terminal};
use md_codec::bytecode::{encode::EncodeTemplate, Tag};

fn dummy_key_a() -> DescriptorPublicKey {
    DescriptorPublicKey::from_str(
        "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5",
    )
    .unwrap()
}

fn dummy_key_b() -> DescriptorPublicKey {
    DescriptorPublicKey::from_str(
        "03a34b99f22c790c4e36b2b3c2c35a36db06226e41c692fc82b8b56ac1c540c5bd",
    )
    .unwrap()
}
```

- [ ] **Step 2.1.2: or_c hand-AST test**

Per spec §3.1 (with O-4 fallback if Phase 2.0 found decoder rejects unwrapped):

```rust
#[test]
fn or_c_tap_leaf_byte_form() {
    let key_a = dummy_key_a();
    let key_b = dummy_key_b();

    let pk_a = Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::PkK(key_a.clone())).unwrap();
    let pk_b = Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::PkK(key_b.clone())).unwrap();
    let v_pk_b = Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::Verify(Arc::new(pk_b))).unwrap();
    let or_c_term: Terminal<DescriptorPublicKey, Tap> =
        Terminal::OrC(Arc::new(pk_a), Arc::new(v_pk_b));

    let mut map = HashMap::new();
    map.insert(key_a, 0u8);
    map.insert(key_b, 1u8);

    let mut out = Vec::new();
    or_c_term.encode_template(&mut out, &map).unwrap();

    // Expected wire bytes: [Tag::OrC, Tag::PkK, Tag::Placeholder, 0,
    //                      Tag::Verify, Tag::PkK, Tag::Placeholder, 1]
    assert_eq!(out, vec![
        Tag::OrC.as_byte(),
        Tag::PkK.as_byte(), Tag::Placeholder.as_byte(), 0,
        Tag::Verify.as_byte(), Tag::PkK.as_byte(), Tag::Placeholder.as_byte(), 1,
    ]);

    // Round-trip via decoder (if Phase 2.0 found decoder rejects unwrapped or_c,
    // wrap in t: per O-4 fallback).
    // [body refined per Phase 2.0 outcome]
}
```

- [ ] **Step 2.1.3: d: wrapper hand-AST test**

`d:v:older(144)` form:

```rust
#[test]
fn d_wrapper_tap_leaf_byte_form() {
    use miniscript::RelLockTime;
    let older_term: Terminal<DescriptorPublicKey, Tap> =
        Terminal::Older(RelLockTime::from_consensus(144).unwrap());
    let older_ms = Miniscript::from_ast(older_term).unwrap();
    let v_older = Terminal::Verify(Arc::new(older_ms));
    let v_older_ms = Miniscript::from_ast(v_older).unwrap();
    let d_v_older: Terminal<DescriptorPublicKey, Tap> =
        Terminal::DupIf(Arc::new(v_older_ms));

    let mut out = Vec::new();
    d_v_older.encode_template(&mut out, &HashMap::new()).unwrap();

    // Expected: [Tag::DupIf, Tag::Verify, Tag::Older, varint(144) = 0x90 0x01]
    assert_eq!(out, vec![
        Tag::DupIf.as_byte(),
        Tag::Verify.as_byte(),
        Tag::Older.as_byte(),
        0x90, 0x01,
    ]);
}
```

- [ ] **Step 2.1.4: j: and n: wrapper hand-AST tests**

Similar pattern. j: requires Bn-type child (e.g., `j:pk_k`); n: requires B-type child.

```rust
#[test]
fn j_wrapper_tap_leaf_byte_form() {
    let key_a = dummy_key_a();
    let pk_a = Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::PkK(key_a.clone())).unwrap();
    let j_pk: Terminal<DescriptorPublicKey, Tap> = Terminal::NonZero(Arc::new(pk_a));

    let mut map = HashMap::new();
    map.insert(key_a, 0u8);
    let mut out = Vec::new();
    j_pk.encode_template(&mut out, &map).unwrap();

    assert_eq!(out, vec![
        Tag::NonZero.as_byte(),
        Tag::PkK.as_byte(), Tag::Placeholder.as_byte(), 0,
    ]);
}

#[test]
fn n_wrapper_tap_leaf_byte_form() {
    let key_a = dummy_key_a();
    // n: requires B-type child. c:pk_k is B-type.
    let pk_a = Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::PkK(key_a.clone())).unwrap();
    let c_pk = Miniscript::from_ast(Terminal::Check(Arc::new(pk_a))).unwrap();
    let n_c_pk: Terminal<DescriptorPublicKey, Tap> = Terminal::ZeroNotEqual(Arc::new(c_pk));

    let mut map = HashMap::new();
    map.insert(key_a, 0u8);
    let mut out = Vec::new();
    n_c_pk.encode_template(&mut out, &map).unwrap();

    assert_eq!(out, vec![
        Tag::ZeroNotEqual.as_byte(),
        Tag::Check.as_byte(),
        Tag::PkK.as_byte(), Tag::Placeholder.as_byte(), 0,
    ]);
}
```

### Task 2.2 — Hash byte-order defensive pin test

Per spec §3.2:

- [ ] **Step 2.2.1: Add the test**

```rust
#[test]
fn hash_terminals_encode_internal_byte_order_not_display_order() {
    use bitcoin::hashes::{hash160, ripemd160, sha256};
    use miniscript::hash256;

    let known_32 = [0xAAu8; 32];
    let known_20 = [0xBBu8; 20];
    let map: HashMap<DescriptorPublicKey, u8> = HashMap::new();

    // Sha256
    let term: Terminal<DescriptorPublicKey, Tap> =
        Terminal::Sha256(sha256::Hash::from_byte_array(known_32));
    let mut out = Vec::new();
    term.encode_template(&mut out, &map).unwrap();
    assert_eq!(out[0], Tag::Sha256.as_byte());
    assert_eq!(&out[1..33], &known_32[..]);

    // Hash256 — same byte order discipline (NOT reversed-display-order)
    let term: Terminal<DescriptorPublicKey, Tap> =
        Terminal::Hash256(hash256::Hash::from_byte_array(known_32));
    let mut out = Vec::new();
    term.encode_template(&mut out, &map).unwrap();
    assert_eq!(out[0], Tag::Hash256.as_byte());
    assert_eq!(&out[1..33], &known_32[..]);

    // Ripemd160
    let term: Terminal<DescriptorPublicKey, Tap> =
        Terminal::Ripemd160(ripemd160::Hash::from_byte_array(known_20));
    let mut out = Vec::new();
    term.encode_template(&mut out, &map).unwrap();
    assert_eq!(out[0], Tag::Ripemd160.as_byte());
    assert_eq!(&out[1..21], &known_20[..]);

    // Hash160
    let term: Terminal<DescriptorPublicKey, Tap> =
        Terminal::Hash160(hash160::Hash::from_byte_array(known_20));
    let mut out = Vec::new();
    term.encode_template(&mut out, &map).unwrap();
    assert_eq!(out[0], Tag::Hash160.as_byte());
    assert_eq!(&out[1..21], &known_20[..]);
}
```

### Task 2.3 — Per-arm decoder unit tests

Per spec §3.3:

- [ ] **Step 2.3.1: Add 5-7 targeted decoder tests**

```rust
#[test]
fn decoder_arm_sortedmulti_a_consumes_correct_bytes() {
    use md_codec::bytecode::decode::tag_to_bip388_name;
    // [k=2, n=3, key_0_placeholder, key_1_placeholder, key_2_placeholder]
    // followed by trailing byte to detect over-read.
    let mut bytecode = vec![
        Tag::Wsh.as_byte(),  // outer descriptor
        // ... — actual implementation refined during Phase 2 execution.
    ];
    // Decode and assert:
    // 1. Resulting Terminal matches expected variant + threshold params.
    // 2. Cursor position confirms exact byte consumption.
}
```

(5 more such tests for: AndOr 3-child consumption; Thresh k/n + children; After varint; Hash256 32-byte payload; SortedMultiA 2-of-3.)

### Task 2.4 — Run + commit

- [ ] **Step 2.4.1: Run tests**

```bash
cargo test -p md-codec --test hand_ast_coverage 2>&1 | tail -10
```

Expected: 8-10 new tests, all passing.

- [ ] **Step 2.4.2: Commit**

```bash
git add -A
git commit -m "test(v0.7 phase 2): defensive hand-AST + byte-order + per-arm decoder tests

Closes FOLLOWUPS:
- v06-corpus-or-c-coverage
- v06-corpus-d-wrapper-coverage
- v06-corpus-j-n-wrapper-coverage
- v06-corpus-byte-order-defensive-test
- v06-plan-targeted-decoder-arm-tests

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

### Task 2.5 — Phase 2 review

- [ ] **Step 2.5.1: Dispatch Opus reviewer**

Brief:
- File: `crates/md-codec/tests/hand_ast_coverage.rs`.
- Verify: hand-AST tests pin actual wire-byte forms (not just smoke-test the encoder); decoder round-trip behavior documented per §3.1 fold-in; byte-order test exercises all 4 hash terminals + uses `as_byte_array()` not `to_byte_array()` reversed-display-order; per-arm decoder tests verify byte consumption + AST equality.
- Output: `design/agent-reports/v0-7-0-phase-2-review.md`.

---

## Phase 3 — `validate_tap_leaf_subset_with_allowlist` refactor

**Goal:** Refactor `validate_tap_leaf_terminal` to accept a caller-supplied allowlist; expose new `pub fn validate_tap_leaf_subset_with_allowlist`; preserve `validate_tap_leaf_subset` signature as back-compat shim.

**File:** `crates/md-codec/src/bytecode/encode.rs` (refactor in place).

### Task 3.1 — Refactor `validate_tap_leaf_terminal`

- [ ] **Step 3.1.1**: Change `validate_tap_leaf_terminal` signature to take `&[&str]` allowlist; replace the hardcoded match arms with allowlist-membership-check via `tag_to_bip388_name`.

```rust
fn validate_tap_leaf_terminal_with_allowlist(
    term: &Terminal<DescriptorPublicKey, Tap>,
    allowlist: &[&str],
) -> Result<(), Error> {
    // Walk children first (allows recursive admission of compound operators).
    match term {
        Terminal::AndV(a, b) | Terminal::AndB(a, b) /* ... */ => {
            validate_tap_leaf_terminal_with_allowlist(&a.node, allowlist)?;
            validate_tap_leaf_terminal_with_allowlist(&b.node, allowlist)?;
        }
        // ... — exhaustive child-recursion shape preserved.
        _ => {}  // leaf operators don't recurse
    }
    // Check this operator is in the allowlist.
    let op_name = tap_terminal_name(term);
    if !allowlist.contains(&op_name) {
        return Err(Error::SubsetViolation {
            operator: op_name.to_string(),
            leaf_index: None,
        });
    }
    Ok(())
}
```

The recursive structure mirrors the current implementation; only the leaf-allow-or-reject check changes from match-arm to allowlist-lookup. ~30-50 line diff per spec O-2.

### Task 3.2 — Add `pub fn validate_tap_leaf_subset_with_allowlist`

- [ ] **Step 3.2.1**: New pub function:

```rust
/// Validate a tap-leaf miniscript against a caller-supplied operator allowlist.
///
/// Operator names follow rust-miniscript desugared AST node naming (matching
/// `tag_to_bip388_name`'s output). See `md_signer_compat::SignerSubset` for
/// the canonical caller pattern.
pub fn validate_tap_leaf_subset_with_allowlist(
    ms: &Miniscript<DescriptorPublicKey, Tap>,
    allowlist: &[&str],
    leaf_index: Option<usize>,
) -> Result<(), Error> {
    validate_tap_leaf_terminal_with_allowlist(&ms.node, allowlist).map_err(|e| match e {
        Error::SubsetViolation { operator, .. } => Error::SubsetViolation {
            operator,
            leaf_index,
        },
        other => other,
    })
}
```

### Task 3.3 — Preserve `validate_tap_leaf_subset` as back-compat shim

- [ ] **Step 3.3.1**: Existing `pub fn validate_tap_leaf_subset` becomes a thin shim:

```rust
/// Historical Coldcard tap-leaf subset constants.
const HISTORICAL_COLDCARD_TAP_OPERATORS: &[&str] = &[
    "pk_k", "pk_h", "multi_a", "or_d", "and_v", "older",
    "c:", "v:",
];

/// Validate a tap-leaf miniscript against the historical Coldcard subset.
///
/// **v0.7 note:** retained as a back-compat shim around
/// `validate_tap_leaf_subset_with_allowlist(...)` with the historical
/// hardcoded Coldcard operator list. New callers should use
/// `validate_tap_leaf_subset_with_allowlist` with their own allowlist
/// (or the named subsets from the `md-signer-compat` crate).
pub fn validate_tap_leaf_subset(
    ms: &Miniscript<DescriptorPublicKey, Tap>,
    leaf_index: Option<usize>,
) -> Result<(), Error> {
    validate_tap_leaf_subset_with_allowlist(ms, HISTORICAL_COLDCARD_TAP_OPERATORS, leaf_index)
}
```

### Task 3.4 — Compile + test + commit

- [ ] **Step 3.4.1: cargo check + test**

Run: `cargo check -p md-codec && cargo test -p md-codec`

Expected: clean. Existing callers of `validate_tap_leaf_subset` continue to pass (back-compat shim).

- [ ] **Step 3.4.2: Commit**

```bash
git add -A
git commit -m "refactor(v0.7 phase 3): validate_tap_leaf_subset accepts allowlist

[body]

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

### Task 3.5 — Phase 3 review

- [ ] **Step 3.5.1: Dispatch Opus reviewer**

Brief:
- File: `crates/md-codec/src/bytecode/encode.rs`.
- Verify: `validate_tap_leaf_terminal_with_allowlist` recursion mirrors the original (no logic regression); `pub fn validate_tap_leaf_subset_with_allowlist` rustdoc explains operator-naming convention; `validate_tap_leaf_subset` back-compat shim preserved with HISTORICAL_COLDCARD_TAP_OPERATORS constant.
- Output: `design/agent-reports/v0-7-0-phase-3-review.md`.

---

## Phase 4 — `md-signer-compat` workspace crate (Track C)

**Goal:** New crate at `crates/md-signer-compat/` with `SignerSubset`, `COLDCARD_TAP`, `LEDGER_TAP`, `validate()` function, and unit tests.

### Task 4.1 — Create crate skeleton

- [ ] **Step 4.1.1: Create directory structure**

```bash
mkdir -p crates/md-signer-compat/src
```

- [ ] **Step 4.1.2: Cargo.toml**

```toml
[package]
name = "md-signer-compat"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "Named signer-subset validators for Mnemonic Descriptor (MD) wallet policies"

[lints]
workspace = true

[dependencies]
md-codec = { path = "../md-codec" }
miniscript = { git = "https://github.com/apoelstra/rust-miniscript", rev = "f7f1689ba92a17b09b03ea12f7048c15d134583e" }

[dev-dependencies]
bitcoin = "0.32"
```

- [ ] **Step 4.1.3: Add to workspace members**

In root `Cargo.toml`:

```toml
[workspace]
members = ["crates/md-codec", "crates/md-signer-compat"]
```

### Task 4.2 — `lib.rs` — SignerSubset + validate

- [ ] **Step 4.2.1: Write `crates/md-signer-compat/src/lib.rs`**

```rust
//! Named signer-subset validators for Mnemonic Descriptor (MD) wallet policies.
//!
//! This crate provides opt-in caller-driven validation of MD-encoded BIP 388
//! wallet policies against named hardware-signer operator subsets. md-codec
//! itself is signer-neutral — per `design/MD_SCOPE_DECISION_2026-04-28.md`,
//! v0.6 removed the encoder/decoder default validator gate. md-signer-compat
//! is the layered checker callers can use for explicit pre-encode validation.
//!
//! # Example
//!
//! ```no_run
//! use md_signer_compat::{COLDCARD_TAP, validate};
//! use std::str::FromStr;
//!
//! let policy: md_codec::WalletPolicy =
//!     "tr(@0/**, sortedmulti_a(2, @1/**, @2/**))".parse().unwrap();
//! // Walk the policy's tap leaves; for each, validate against COLDCARD_TAP.
//! // (Exact API for "iterate tap leaves" refined during implementation.)
//! ```
//!
//! # Vendor citation discipline
//!
//! Each named subset (`COLDCARD_TAP`, `LEDGER_TAP`, ...) carries an inline
//! comment with the source URL, source repo's commit SHA, and last-checked
//! date. Vendor doc revisions → subset bump → crate patch release.

mod coldcard;
mod ledger;

pub use coldcard::COLDCARD_TAP;
pub use ledger::LEDGER_TAP;

/// A named subset of miniscript operators a hardware signer is documented
/// to admit. Operator names follow rust-miniscript desugared AST node
/// naming (matching md-codec's `tag_to_bip388_name` adapter output).
///
/// See module-level documentation for vendor-citation discipline.
#[derive(Debug, Clone)]
pub struct SignerSubset {
    /// Human-readable name (e.g., "Coldcard tap-leaf").
    pub name: &'static str,
    /// Operator names (rust-miniscript desugared AST node names) the signer admits.
    pub allowed_operators: &'static [&'static str],
}

/// Validate a tap-context miniscript leaf against a named signer subset.
///
/// Returns `Ok(())` if every operator in the leaf AST appears in
/// `subset.allowed_operators`. Returns
/// [`md_codec::Error::SubsetViolation`] with the offending operator
/// name and `leaf_index` on the first out-of-subset operator.
pub fn validate(
    subset: &SignerSubset,
    ms: &miniscript::Miniscript<miniscript::DescriptorPublicKey, miniscript::Tap>,
    leaf_index: Option<usize>,
) -> Result<(), md_codec::Error> {
    md_codec::bytecode::encode::validate_tap_leaf_subset_with_allowlist(
        ms,
        subset.allowed_operators,
        leaf_index,
    )
}

#[cfg(test)]
mod tests;
```

### Task 4.3 — `coldcard.rs`

- [ ] **Step 4.3.1: Write `crates/md-signer-compat/src/coldcard.rs`**

```rust
//! Coldcard tap-leaf miniscript subset.

use crate::SignerSubset;

/// Coldcard tap-leaf miniscript subset.
///
/// **Source:** `Coldcard/firmware` repo, `edge` branch, `docs/taproot.md`
/// §"Allowed descriptors". Verified at edge HEAD on 2026-04-28.
///
/// Documented allowed shapes (per `docs/taproot.md`):
///   - `tr(key)` — single-sig keypath
///   - `tr(internal_key, sortedmulti_a(2, @0, @1))`
///   - `tr(internal_key, pk(@0))`
///   - `tr(internal_key, {sortedmulti_a(...), pk(@2)})`
///   - `tr(internal_key, {or_d(pk(@0), and_v(v:pkh(@1), older(1000))), pk(@2)})`
///
/// Operators extracted (desugared-AST naming):
///   - `pk_k` (from `pk(K)` desugaring + as `pk_k` directly)
///   - `pk_h` (from `pkh(K)` desugaring)
///   - `multi_a` (Coldcard documents both `multi_a` and `sortedmulti_a`)
///   - `sortedmulti_a` (NEW in v0.6; coldcard documented)
///   - `or_d`
///   - `and_v`
///   - `older`
///   - `c:` (required for `pk(K)` and `pkh(K)` desugaring)
///   - `v:` (required for `and_v(v:..., ...)` and `v:pkh(...)`)
pub const COLDCARD_TAP: SignerSubset = SignerSubset {
    name: "Coldcard tap-leaf (firmware/edge as of 2026-04-28)",
    allowed_operators: &[
        "pk_k", "pk_h", "multi_a", "sortedmulti_a",
        "or_d", "and_v", "older",
        "c:", "v:",
    ],
};
```

### Task 4.4 — `ledger.rs`

- [ ] **Step 4.4.1: Write `crates/md-signer-compat/src/ledger.rs`**

```rust
//! Ledger tap-leaf miniscript subset.

use crate::SignerSubset;

/// Ledger tap-leaf miniscript subset.
///
/// **Source:** `LedgerHQ/vanadium`, `apps/bitcoin/common/src/bip388/cleartext.rs`.
/// Verified on 2026-04-28.
///
/// Variants admitted (from the `cleartext.rs` enum):
///   - `Singlesig` (key-only `tr`)
///   - `SortedMultisig` (`sortedmulti_a`)
///   - `Multisig` (`multi_a`)
///   - `RelativeHeightlockMultiSig` (`and_v(v:multi_a, older(n<65536))`)
///   - `RelativeTimelockMultiSig` (`and_v(v:multi_a, older(time-encoding range))`)
///   - `AbsoluteHeightlockMultiSig` (`and_v(v:multi_a, after(n<500_000_000))`)
///   - `AbsoluteTimelockMultiSig` (`and_v(v:multi_a, after(n>=500_000_000))`)
///
/// Operators extracted (desugared-AST naming):
///   - `pk_k` (single-sig keypath)
///   - `pk_h`
///   - `multi_a`
///   - `sortedmulti_a`
///   - `and_v`
///   - `older`
///   - `after`
///   - `c:` (for sugar desugaring)
///   - `v:` (for `and_v(v:..., ...)`)
pub const LEDGER_TAP: SignerSubset = SignerSubset {
    name: "Ledger tap-leaf (LedgerHQ/vanadium as of 2026-04-28)",
    allowed_operators: &[
        "pk_k", "pk_h", "multi_a", "sortedmulti_a",
        "and_v", "older", "after",
        "c:", "v:",
    ],
};
```

### Task 4.5 — `tests.rs`

- [ ] **Step 4.5.1: Write 5 tests per spec §4.6**

```rust
//! Unit tests for md-signer-compat.

use std::str::FromStr;
use std::sync::Arc;
use std::collections::HashMap;

use bitcoin::hashes::Hash;
use miniscript::{DescriptorPublicKey, Miniscript, Tap, Terminal};

use crate::{COLDCARD_TAP, LEDGER_TAP, validate};

fn dummy_key() -> DescriptorPublicKey {
    DescriptorPublicKey::from_str(
        "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5",
    )
    .unwrap()
}

#[test]
fn coldcard_admits_documented_pk_shape() {
    let key = dummy_key();
    let pk_k = Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::PkK(key)).unwrap();
    // pk(K) desugars to c:pk_k(K); the AST has Check around PkK.
    let c_pk = Miniscript::from_ast(Terminal::Check(Arc::new(pk_k))).unwrap();
    assert!(validate(&COLDCARD_TAP, &c_pk, Some(0)).is_ok());
}

#[test]
fn coldcard_rejects_thresh_with_operator_name() {
    use miniscript::Threshold;
    let key = dummy_key();
    let pk_k = Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::PkK(key.clone())).unwrap();
    let c_pk = Arc::new(Miniscript::from_ast(Terminal::Check(Arc::new(pk_k))).unwrap());
    let thresh_term = Terminal::Thresh(Threshold::new(1, vec![c_pk]).unwrap());
    let thresh_ms = Miniscript::from_ast(thresh_term).unwrap();
    let err = validate(&COLDCARD_TAP, &thresh_ms, Some(2)).unwrap_err();
    match err {
        md_codec::Error::SubsetViolation { operator, leaf_index } => {
            assert_eq!(operator, "thresh");
            assert_eq!(leaf_index, Some(2));
        }
        other => panic!("expected SubsetViolation, got {other:?}"),
    }
}

#[test]
fn ledger_admits_relative_timelock_multisig_shape() {
    use miniscript::{RelLockTime, Threshold};
    let key_a = dummy_key();
    let key_b = DescriptorPublicKey::from_str(
        "03a34b99f22c790c4e36b2b3c2c35a36db06226e41c692fc82b8b56ac1c540c5bd",
    ).unwrap();
    let multi_a = Miniscript::<DescriptorPublicKey, Tap>::from_ast(
        Terminal::MultiA(Threshold::new(2, vec![key_a, key_b]).unwrap())
    ).unwrap();
    let v_multi_a = Miniscript::from_ast(Terminal::Verify(Arc::new(multi_a))).unwrap();
    let older = Miniscript::from_ast(Terminal::Older(RelLockTime::from_consensus(144).unwrap())).unwrap();
    let and_v = Terminal::AndV(Arc::new(v_multi_a), Arc::new(older));
    let and_v_ms = Miniscript::from_ast(and_v).unwrap();
    assert!(validate(&LEDGER_TAP, &and_v_ms, Some(0)).is_ok());
}

#[test]
fn ledger_rejects_sha256() {
    use bitcoin::hashes::sha256;
    let h = sha256::Hash::from_byte_array([0xAA; 32]);
    let sha = Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::Sha256(h)).unwrap();
    let err = validate(&LEDGER_TAP, &sha, Some(1)).unwrap_err();
    match err {
        md_codec::Error::SubsetViolation { operator, leaf_index } => {
            assert_eq!(operator, "sha256");
            assert_eq!(leaf_index, Some(1));
        }
        other => panic!("expected SubsetViolation, got {other:?}"),
    }
}

#[test]
fn allowlist_entries_are_recognized_by_naming_hook() {
    // Per spec §4.6 acceptance: every entry in COLDCARD_TAP and LEDGER_TAP
    // allowlists must be a name md-codec's operator-naming hook can produce.
    // Strategy: for each allowlist entry, construct a minimal Terminal that
    // produces that name, and verify validate_tap_leaf_subset_with_allowlist
    // accepts it under a single-entry allowlist containing only that name.
    //
    // Detailed implementation refined during Phase 4 execution.
}
```

### Task 4.6 — Build + test + commit

- [ ] **Step 4.6.1: Build + test**

```bash
cargo build -p md-signer-compat
cargo test -p md-signer-compat
```

Expected: clean build; all 5 tests pass.

- [ ] **Step 4.6.2: Commit**

```bash
git add -A
git commit -m "feat(v0.7 phase 4): md-signer-compat workspace crate

[body]

Closes md-signer-compat-checker-separate-library.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

### Task 4.7 — Phase 4 review

- [ ] **Step 4.7.1: Dispatch Opus reviewer**

Brief:
- Files: `crates/md-signer-compat/`.
- Verify: vendor citations include source URLs (no missing references); allowlist entries match desugared-AST names; `validate()` correctly delegates to md-codec; all 5 unit tests cover happy + rejection + coverage paths.
- Output: `design/agent-reports/v0-7-0-phase-4-review.md`.

---

## Phase 5 — Policy compiler wrapper API (Track D)

**Goal:** Add `compiler` cargo feature; `ScriptContext` enum + `policy_to_bytecode()` wrapper; CLI `--from-policy` mode.

### Task 5.1 — Cargo feature

- [ ] **Step 5.1.1**: Add to `crates/md-codec/Cargo.toml`:

```toml
[features]
default = ["cli"]
cli = ["dep:clap", "dep:anyhow"]
compiler = ["miniscript/compiler"]
cli-compiler = ["cli", "compiler"]
```

### Task 5.2 — Implement `ScriptContext` + `policy_to_bytecode`

- [ ] **Step 5.2.1**: Create `crates/md-codec/src/policy_compiler.rs`:

```rust
//! Policy compiler wrapper API.
//!
//! Available only when the `compiler` cargo feature is enabled.

#[cfg(feature = "compiler")]
use miniscript::policy::Concrete;

/// Script context for policy compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptContext {
    /// Segwit v0 (`wsh()` descriptors).
    Segwitv0,
    /// Tapscript (`tr()` descriptors).
    Tap,
}

/// Parse a high-level Concrete Policy expression, compile to optimal
/// miniscript, and encode the result as MD bytecode.
///
/// Available only when the `compiler` cargo feature is enabled.
///
/// # Errors
///
/// - [`Error::PolicyParse`] on Concrete-Policy parse failure.
/// - [`Error::Miniscript`] on compiler unsatisfiable.
/// - Any encode-side error.
pub fn policy_to_bytecode(
    policy: &str,
    options: &crate::EncodeOptions,
    script_context: ScriptContext,
) -> Result<Vec<u8>, crate::Error> {
    let concrete: Concrete<miniscript::DescriptorPublicKey> = policy.parse()
        .map_err(|e| crate::Error::PolicyParse(format!("{e}")))?;
    match script_context {
        ScriptContext::Segwitv0 => {
            let ms = concrete.compile::<miniscript::Segwitv0>()
                .map_err(|e| crate::Error::Miniscript(format!("{e}")))?;
            // Wrap in wsh() and call to_bytecode.
            // [exact assembly refined during Phase 5 execution]
            todo!("wrap in wsh + to_bytecode")
        }
        ScriptContext::Tap => {
            let ms = concrete.compile::<miniscript::Tap>()
                .map_err(|e| crate::Error::Miniscript(format!("{e}")))?;
            // Wrap in tr(KEY, leaf) — but compiler returns miniscript only;
            // need an internal key for tr(). [refine during execution.]
            todo!("wrap in tr + to_bytecode")
        }
    }
}
```

NOTE: This implementation has open questions (which internal key for tap context? does the API take an internal-key parameter, or generate an unspendable NUMS key?). Phase 5 execution decides; current sketch is illustrative.

- [ ] **Step 5.2.2**: Add module to `lib.rs`:

```rust
#[cfg(feature = "compiler")]
pub mod policy_compiler;

#[cfg(feature = "compiler")]
pub use policy_compiler::{policy_to_bytecode, ScriptContext};
```

### Task 5.3 — CLI `--from-policy` mode

- [ ] **Step 5.3.1**: Add the subcommand variant + `cli-compiler` gating per spec §5.3.

### Task 5.4 — Tests

- [ ] **Step 5.4.1**: Add 3-4 unit tests in `policy_compiler.rs` (happy-path Tap, happy-path Segwitv0, parse-error, compile-error).

- [ ] **Step 5.4.2**: Add CLI integration test in `tests/cli.rs` for `md encode --from-policy ... --context tap`.

### Task 5.5 — Build + test + commit

- [ ] **Step 5.5.1: Build with feature**

```bash
cargo build -p md-codec --features compiler
cargo build -p md-codec --features cli-compiler
cargo test -p md-codec --features cli-compiler
```

- [ ] **Step 5.5.2: Commit**

```bash
git add -A
git commit -m "feat(v0.7 phase 5): policy compiler wrapper API

[body]

Closes md-policy-compiler-feature.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

### Task 5.6 — Phase 5 review

- [ ] **Step 5.6.1: Dispatch Opus reviewer**

Brief:
- Files: `crates/md-codec/Cargo.toml`, `policy_compiler.rs`, `bin/md/main.rs`.
- Verify: feature gating correct; `policy_to_bytecode` handles both contexts; internal-key strategy for Tap context documented; CLI integration test exercises happy path.
- Output: `design/agent-reports/v0-7-0-phase-5-review.md`.

---

## Phase 6 — Release plumbing

**Goal:** Cargo bump 0.6.0 → 0.7.0; vector regen with `"md-codec 0.7"` family token; CHANGELOG; MIGRATION; tag.

### Task 6.0 — Pre-roll verification

- [ ] **Step 6.0.1: Verify v0.6.x corpus round-trips before rolling family token** (per spec O-1)

```bash
cargo run --bin gen_vectors -- --verify crates/md-codec/tests/vectors/v0.1.json
cargo run --bin gen_vectors -- --verify crates/md-codec/tests/vectors/v0.2.json
```

Expected: both succeed (confirms wire format byte-identical to v0.6.0).

### Task 6.1 — Version bump

- [ ] **Step 6.1.1**: `crates/md-codec/Cargo.toml`: `0.6.0` → `0.7.0`.

### Task 6.2 — Vector regen + SHA pin

- [ ] **Step 6.2.1: Regenerate**

```bash
cargo run --bin gen_vectors -- --output crates/md-codec/tests/vectors/v0.1.json --schema 1
cargo run --bin gen_vectors -- --output crates/md-codec/tests/vectors/v0.2.json
```

- [ ] **Step 6.2.2: Update SHA pin**

`crates/md-codec/tests/vectors_schema.rs::V0_2_SHA256` to new value from `sha256sum tests/vectors/v0.2.json`.

### Task 6.3 — CHANGELOG + MIGRATION

- [ ] **Step 6.3.1: CHANGELOG `[0.7.0]` section**

```markdown
## [0.7.0] — 2026-04-29

First post-strip-Layer-3 release. Bundles four tracks.

### Changed
- 38 unit tests rebaselined from v0.5 byte literals to symbolic
  `Tag::Foo.as_byte()` references (closes `v06-test-byte-literal-rebaseline`).

### Added
- New workspace crate `md-signer-compat` with `COLDCARD_TAP` and
  `LEDGER_TAP` named subsets for opt-in caller-driven validation
  (closes `md-signer-compat-checker-separate-library`).
- `md-codec` `compiler` cargo feature (default-off): exposes
  `ScriptContext` enum + `policy_to_bytecode(policy, options, ctx)`
  wrapper around rust-miniscript's policy compiler.
- `md-codec` `cli-compiler` cargo feature: `md encode --from-policy
  <expr> --context <tap|segwitv0>` mode.
- `pub fn validate_tap_leaf_subset_with_allowlist(ms, allowlist,
  leaf_index)` — caller-supplied operator allowlist; existing
  `validate_tap_leaf_subset` becomes a back-compat shim.
- 8-10 hand-AST defensive tests for typing-awkward operators
  (`or_c`, `d:`, `j:`, `n:` wrappers); hash byte-order pin test;
  per-arm decoder unit tests (closes 5 v06-corpus FOLLOWUPS entries).

### Notes
- Wire format byte-identical to v0.6.x.
- `GENERATOR_FAMILY` rolls `"md-codec 0.6"` → `"md-codec 0.7"`.
- v0.1.json + v0.2.json regenerate; SHA pins update once.

### Closes FOLLOWUPS
- v06-test-byte-literal-rebaseline
- v06-corpus-or-c-coverage / d-wrapper / j-n-wrapper / byte-order-defensive-test
- v06-plan-targeted-decoder-arm-tests
- md-signer-compat-checker-separate-library
- md-policy-compiler-feature

### NEW FOLLOWUPS
- v07-cli-validate-signer-subset (deferred CLI track)
```

- [ ] **Step 6.3.2: MIGRATION `v0.6.x → v0.7.0`**

```markdown
## v0.6.x → v0.7.0

v0.7.0 is purely additive. No breaking changes.

### What's new

1. NEW workspace crate `md-signer-compat`. Add as a dependency if
   you want named-subset validation (Coldcard, Ledger).
2. NEW pub function `md_codec::bytecode::encode::validate_tap_leaf_subset_with_allowlist`.
3. NEW cargo features on md-codec: `compiler` (default-off) and
   `cli-compiler`.
4. NEW types behind `compiler` feature: `ScriptContext`, `policy_to_bytecode`.

### What didn't change

- Wire format (byte-identical to v0.6.x).
- All existing public API (no removals; no renames; back-compat shims preserve v0.6 callers).
- MSRV: 1.85 (unchanged).

### How to upgrade

```bash
cargo update -p md-codec --precise 0.7.0
```
```

### Task 6.4 — Full test + commit + tag

- [ ] **Step 6.4.1: Full workspace test**

```bash
cargo test --workspace 2>&1 | tail -10
```

Expected: all tests pass.

- [ ] **Step 6.4.2: Commit**

- [ ] **Step 6.4.3: Tag**

```bash
git tag -a md-codec-v0.7.0 -m "md-codec v0.7.0 — test rebaseline + signer-compat + compiler"
git push origin md-codec-v0.7.0
```

### Task 6.5 — Phase 6 review

(Optional given Phase 6 is mechanical; skip if time-constrained.)

---

## Phase 7 — Final reconciliation

### Task 7.1 — Reconcile agent reports vs FOLLOWUPS

- [ ] **Step 7.1.1**: List all `design/agent-reports/v0-7-0-*` reports; for each "Follow-up items" section, verify FOLLOWUPS entry exists.

### Task 7.2 — Update PR + memory + GitHub Release

- [ ] **Step 7.2.1**: Update PR #3 description with completed checkbox states.
- [ ] **Step 7.2.2**: Add v0.7.0 entry to project memory.
- [ ] **Step 7.2.3**: Merge PR + create GitHub Release per v0.6.0 pattern.

---

(End of v0.7.0 implementation plan.)
