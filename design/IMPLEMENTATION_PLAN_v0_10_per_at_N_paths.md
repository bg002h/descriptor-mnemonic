# md-codec v0.10.0 — per-`@N` Origin Path Declaration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Per-phase opus reviewer reports persist to `design/agent-reports/v0-10-phase-N-review.md`.

**Goal:** Ship the v0.10.0 wire-format break that admits per-`@N` origin paths via new `Tag::OriginPaths = 0x36` block and reclaims header bit 3 as the OriginPaths flag. Closes the v0.x ≤ 0.9 silent path-divergence drop bug.

**Architecture:** Wire-format-additive at decoder for SharedPath-only encodings (byte-identical regen); wire-format-breaking for divergent-path policies (which were lossy in v0.9 and now encode correctly). New `WalletPolicy.decoded_origin_paths` field for round-trip stability. Encoder auto-detects path divergence per Q9-A.

**Tech Stack:** Same as v0.9.1 — Rust 2024, miniscript fork pinned to `f7f1689b…` (SHA bump deferred), codex32-derived BCH layer.

**Source of truth:** `design/SPEC_v0_10_per_at_N_paths.md` (commit `017401d`, 3 opus review passes — clean verdict). All 13 brainstorm decisions LOCKED at `design/BRAINSTORM_v0_10_per_at_N_paths.md`.

---

## Scope

### In-scope (closes FOLLOWUPS at ship)

- `md-per-at-N-path-tag-allocation` — the headline.

### Out-of-scope (deferred)

- BIP 393 recovery hints (`Tag::RecoveryHints` at `0x37`) — v1+ work.
- `WalletInstanceId::to_words()` rendering parity — `walletinstanceid-rendering-parity` FOLLOWUPS, v1+.
- mc-codex32 third-crate factoring — D-13, gated on both formats reaching v1.0.
- Reproducible-builds phase 2 (hermetic Nix/Docker) — v1.0 milestone.

### File structure

| File | Action | Notes |
|---|---|---|
| `crates/md-codec/Cargo.toml` | Modify | version `0.9.1` → `0.10.0` |
| `crates/md-codec/src/bytecode/header.rs` | Modify | `RESERVED_MASK 0x0B → 0x03`; add `origin_paths: bool`; `new_v0(bool)` → `new_v0(bool, bool)`; new `ORIGIN_PATHS_BIT = 0x08` constant; `origin_paths()` getter |
| `crates/md-codec/src/bytecode/tag.rs` | Modify | Add `OriginPaths = 0x36` variant; `from_byte` arm; `as_byte` (auto via repr) |
| `crates/md-codec/src/bytecode/path.rs` | Modify | New `MAX_PATH_COMPONENTS: usize = 10` const; cap enforcement in `encode_path` + `decode_path`; new `encode_origin_paths` / `decode_origin_paths` helpers |
| `crates/md-codec/src/error.rs` | Modify | Add `BytecodeErrorKind::OriginPathsCountTooLarge { count, max }`; add `Error::OriginPathsCountMismatch { expected, got }`; add `Error::PathComponentCountExceeded { got, max }` |
| `crates/md-codec/src/policy.rs` | Modify | Add `decoded_origin_paths: Option<Vec<DerivationPath>>` field; new `placeholder_paths_in_index_order` method; `to_bytecode` dispatch updates; `from_bytecode` populate-decoded field |
| `crates/md-codec/src/policy_id.rs` | Modify | Add `PolicyId::fingerprint() -> [u8; 4]` method |
| `crates/md-codec/src/options.rs` | Modify | Add `EncodeOptions::origin_paths: Option<Vec<DerivationPath>>` field (Tier 0 override); `with_origin_paths` builder method |
| `crates/md-codec/src/vectors.rs` | Modify | Add `o1`, `o2`, `o3` positive vectors; add `n_orig_*` negative vector generators |
| `crates/md-codec/src/bytecode/hand_ast_coverage.rs` | Modify | Add ~5 new tests for OriginPaths byte-position pinning, header round-trip, encoder dispatch, MAX_PATH_COMPONENTS boundary |
| `crates/md-codec/tests/vectors_schema.rs` | Modify | Bump corpus count assertion 44→45+ (depends on how many `o*` ship); update `V0_2_SHA256` pin |
| `crates/md-codec/tests/vectors/v0.1.json` | Regen | Family token `"md-codec 0.10"` |
| `crates/md-codec/tests/vectors/v0.2.json` | Regen | Same; plus new positive + negative vectors |
| `crates/md-codec/tests/conformance.rs` | Modify | New `rejects_origin_paths_count_too_large`, `rejects_path_component_count_exceeded`, `rejects_origin_paths_count_mismatch` tests |
| `bip/bip-mnemonic-descriptor.mediawiki` | Modify | New §"Per-`@N` path declaration"; new §"Authority precedence with MK"; new §"PolicyId types"; soften 12-word phrase engraving language |
| `README.md` | Modify | Update scope summary if it currently says "shared path only" |
| `MIGRATION.md` | Modify | New `## v0.9.x → v0.10.0` section with `BytecodeHeader::new_v0` signature update + sed snippet |
| `CHANGELOG.md` | Modify | New `[0.10.0]` section with "Why a wire-format break?" callout |
| `design/POLICY_BACKUP.md` | Modify | Update `Tag::RecoveryHints` slot from `0x36` → `0x37` |
| `design/FOLLOWUPS.md` | Modify | Mark `md-per-at-N-path-tag-allocation` resolved |
| `CLAUDE.md` | Modify | Drop the resolved entry from "Currently open" list |

---

## Pre-Phase-0 — Branch setup and dependency snapshot

- [ ] **Step 1: Confirm spec is at clean state**

```bash
cd /scratch/code/shibboleth/descriptor-mnemonic
git log --oneline -5 main
# Expect: most recent commits include 017401d (pass-3 review),
#         e09791f (pass-2 fixes), 82f763e (pass-1 fixes), 81745e6 (initial spec)
```

- [ ] **Step 2: Cut feature branch**

```bash
git checkout -b feature/v0.10-per-at-n-paths main
git status   # clean working tree
```

- [ ] **Step 3: Confirm v0.9.1 baseline tests pass**

```bash
cargo test --workspace --all-features 2>&1 | grep '^test result' | awk '{ok+=$4; failed+=$6} END {print "Total: ok="ok" failed="failed}'
# Expect: ok=678 failed=0 (verified against main commit 2a9c969 on 2026-04-29; v0.9.1 baseline). Pin this number in Phase-end commit messages so post-v0.10 phase totals are diff-able.
PATH="$HOME/.cargo/bin:$PATH" cargo +stable clippy --workspace --all-features --all-targets -- -D warnings 2>&1 | tail -3
# Expect: clean
PATH="$HOME/.cargo/bin:$PATH" cargo +stable fmt --all -- --check 2>&1 | head -3
# Expect: clean
```

If anything fails, stop and investigate before proceeding.

- [ ] **Step 4 (NEW per F15): Bump version to 0.10.0 at the foundation of the feature branch**

Per F15: `GENERATOR_FAMILY` = `"md-codec ${MAJOR}.${MINOR}"` is computed from `Cargo.toml` at compile time. Vectors regenerated mid-plan must produce `"md-codec 0.10"` strings, not `"md-codec 0.9"`. Bumping the version at Pre-Phase-0 ensures all subsequent regens use the correct family token; final Phase 7 release commit doesn't re-bump.

```toml
# crates/md-codec/Cargo.toml
version = "0.10.0"
```

```bash
cargo build --workspace --all-features 2>&1 | tail -3   # confirm clean compile
git add -A && git commit -m "chore(v0.10): bump version 0.9.1 → 0.10.0 (foundation commit)

Pre-Phase-0 version bump per IMPLEMENTATION_PLAN_v0_10 F15: ensures
GENERATOR_FAMILY = \"md-codec 0.10\" for all subsequent test-vector
regenerations during the v0.10 implementation phases.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
"
```

This means Phase 7 Step 7.2 becomes a verification-only step ("confirm version is 0.10.0") rather than a fresh bump.

---

## Phase 1 — Header bit 3 reclaim + Tag::OriginPaths variant (TDD foundation)

**Goal:** Land the lowest-level type changes with thorough tests, no policy-layer integration yet. After Phase 1, `cargo test` passes and the new types are usable in isolation.

### Files
- `crates/md-codec/src/bytecode/header.rs`
- `crates/md-codec/src/bytecode/tag.rs`
- `crates/md-codec/src/error.rs`

### Steps

- [ ] **Step 1.1: Write failing header tests for bit 3 reclaim**

In `bytecode/header.rs` test module, add:

```rust
#[test]
fn header_byte_0x08_decodes_with_origin_paths_flag_set() {
    let h = BytecodeHeader::from_byte(0x08).expect("0x08 must be valid v0.10");
    assert_eq!(h.version(), 0);
    assert!(h.origin_paths());
    assert!(!h.fingerprints());
}

#[test]
fn header_byte_0x0c_decodes_with_both_flags_set() {
    let h = BytecodeHeader::from_byte(0x0C).expect("0x0C must be valid v0.10");
    assert!(h.origin_paths());
    assert!(h.fingerprints());
}

#[test]
fn header_byte_0x02_rejects_with_reserved_bit_1() {
    let err = BytecodeHeader::from_byte(0x02).unwrap_err();
    assert!(matches!(err, Error::InvalidBytecode {
        kind: BytecodeErrorKind::ReservedBitsSet { byte: 0x02, mask: 0x03 },
        ..
    }));
}

#[test]
fn header_byte_0x01_rejects_with_reserved_bit_0() {
    let err = BytecodeHeader::from_byte(0x01).unwrap_err();
    assert!(matches!(err, Error::InvalidBytecode {
        kind: BytecodeErrorKind::ReservedBitsSet { byte: 0x01, mask: 0x03 },
        ..
    }));
}

#[test]
fn new_v0_signature_takes_origin_paths_bool() {
    let h = BytecodeHeader::new_v0(false, false);
    assert_eq!(h.as_byte(), 0x00);
    let h = BytecodeHeader::new_v0(true, false);
    assert_eq!(h.as_byte(), 0x04);
    let h = BytecodeHeader::new_v0(false, true);
    assert_eq!(h.as_byte(), 0x08);
    let h = BytecodeHeader::new_v0(true, true);
    assert_eq!(h.as_byte(), 0x0C);
}
```

- [ ] **Step 1.2: Run tests; verify they fail**

```bash
cargo test --package md-codec bytecode::header 2>&1 | tail -20
```

Expected: 5 new tests fail (signature mismatch on `new_v0`; reserved-bit rejections wrong byte values; `origin_paths()` method missing).

Additionally, per F1: two existing tests will fail after Step 1.3 lands the new `RESERVED_MASK = 0x03`:
- `reserved_bit_3_set` (line 194) — asserts `from_byte(0x08) → Err`. Under v0.10, `0x08` is valid (OriginPaths flag). **Action: delete this test in Step 1.3** (it's the inversion of the new behavior).
- `all_reserved_bits_set_no_fingerprints` (line 209) — tests `0x0B` rejection with comment "bits 3, 1, 0 all set." Under the new mask `0x03`, bit 3 is no longer reserved. The test would coincidentally pass (bits 1+0 still reserved) but with stale prose. **Action: rewrite in Step 1.3** as `all_reserved_bits_set_in_v0_10` testing `0x03` (bits 1+0 set, no flags).

Both edits land alongside the `RESERVED_MASK` change in Step 1.3. The TDD-discipline framing in Step 1.2 captures all 7 failures (5 new tests + 2 existing-test renames/deletions) so Step 1.3's pass-bar is unambiguous.

- [ ] **Step 1.3: Update `BytecodeHeader` struct + impl, plus migrate the two existing tests**

Edit `bytecode/header.rs`:

```rust
const RESERVED_MASK: u8 = 0x03;            // was 0x0B
const FINGERPRINTS_BIT: u8 = 0x04;
const ORIGIN_PATHS_BIT: u8 = 0x08;         // NEW

#[non_exhaustive]
pub struct BytecodeHeader {
    version: u8,
    fingerprints: bool,
    origin_paths: bool,                    // NEW
}

impl BytecodeHeader {
    pub fn from_byte(b: u8) -> Result<BytecodeHeader, Error> {
        let version = b >> 4;
        if version != 0 {
            return Err(Error::UnsupportedVersion(version));
        }
        let reserved = b & RESERVED_MASK;
        if reserved != 0 {
            return Err(Error::InvalidBytecode {
                offset: 0,
                kind: BytecodeErrorKind::ReservedBitsSet { byte: b, mask: RESERVED_MASK },
            });
        }
        Ok(BytecodeHeader {
            version,
            fingerprints: (b & FINGERPRINTS_BIT) != 0,
            origin_paths: (b & ORIGIN_PATHS_BIT) != 0,
        })
    }

    pub const fn new_v0(fingerprints: bool, origin_paths: bool) -> Self {
        Self { version: 0, fingerprints, origin_paths }
    }

    pub const fn as_byte(self) -> u8 {
        let mut b = self.version << 4;
        if self.fingerprints { b |= FINGERPRINTS_BIT; }
        if self.origin_paths { b |= ORIGIN_PATHS_BIT; }
        b
    }

    pub const fn fingerprints(&self) -> bool { self.fingerprints }
    pub const fn origin_paths(&self) -> bool { self.origin_paths }
    pub const fn version(&self) -> u8 { self.version }
}
```

- [ ] **Step 1.4: Run tests; verify the 5 new tests pass**

```bash
cargo test --package md-codec bytecode::header 2>&1 | tail -10
# Expect: ok all 5 new tests + existing tests
```

- [ ] **Step 1.5: Find + update existing `new_v0(bool)` call sites**

```bash
rg -n 'new_v0\(' crates/md-codec/src/ crates/md-codec/tests/
```

Update each caller from `new_v0(fingerprints)` to `new_v0(fingerprints, false)` (origin_paths = false default; Phase 4 will update `to_bytecode` to dispatch on real path-divergence detection).

```bash
cargo build --workspace --all-features 2>&1 | tail -5
# Expect: clean compile after all call sites updated
```

- [ ] **Step 1.6: Add `Tag::OriginPaths = 0x36` variant**

Edit `bytecode/tag.rs`:

```rust
pub enum Tag {
    // ... existing variants ...
    Placeholder = 0x33,
    SharedPath = 0x34,
    Fingerprints = 0x35,
    OriginPaths = 0x36,    // NEW
}

impl Tag {
    pub fn from_byte(b: u8) -> Option<Self> {
        match b {
            // ... existing arms ...
            0x33 => Some(Tag::Placeholder),
            0x34 => Some(Tag::SharedPath),
            0x35 => Some(Tag::Fingerprints),
            0x36 => Some(Tag::OriginPaths),    // NEW
            _ => None,
        }
    }
}
```

Per F4: there are THREE existing tag-table-coverage tests, all needing update:

- **`tag_v0_6_high_bytes_unallocated`** (line ~294): loop bound becomes `0x37..=0xFF`. Rename to `tag_v0_10_high_bytes_unallocated` per project convention (test names track the version that pinned the byte range).
- **`tag_rejects_unknown_bytes`** (line ~317, second loop in same fn): same loop-bound update `0x37..=0xFF`.
- **`tag_round_trip_all_defined`** (line ~305): the `v0_6_allocated` Vec needs `0x36` added to the chain — `(0x00..=0x23).chain(0x33..=0x36)`. Rename `v0_6_allocated` → `v0_10_allocated`.

All three updates land in this step alongside the new `Tag::OriginPaths` variant.

- [ ] **Step 1.7: Add Tag::OriginPaths byte-position test**

```rust
#[test]
fn tag_origin_paths_byte_position() {
    assert_eq!(Tag::OriginPaths.as_byte(), 0x36);
    assert_eq!(Tag::from_byte(0x36), Some(Tag::OriginPaths));
}

#[test]
fn tag_v0_10_unallocated_starts_at_0x37() {
    for b in 0x37..=0xFF_u8 {
        assert!(Tag::from_byte(b).is_none(), "byte {:#04x} should be unallocated", b);
    }
}
```

```bash
cargo test --package md-codec bytecode::tag 2>&1 | tail -5
```

- [ ] **Step 1.8: Add new error variants + extend `ErrorVariantName` mirror enum (F2)**

Edit `error.rs`:

```rust
// In BytecodeErrorKind:
pub enum BytecodeErrorKind {
    // ... existing variants ...
    /// The OriginPaths count byte is structurally invalid (zero or exceeds
    /// the BIP 388 placeholder cap of 32).
    OriginPathsCountTooLarge { count: u8, max: u8 },
}

// In Error:
pub enum Error {
    // ... existing variants ...
    /// The OriginPaths bytecode count doesn't match the tree's actual
    /// placeholder count after parse.
    #[error("OriginPaths count mismatch: tree has {expected} placeholders, OriginPaths declares {got}")]
    OriginPathsCountMismatch { expected: usize, got: usize },

    /// An explicit-form path declaration exceeded `MAX_PATH_COMPONENTS = 10`.
    /// Applies to both `Tag::SharedPath` and `Tag::OriginPaths`.
    #[error("path component count {got} exceeds maximum {max}")]
    PathComponentCountExceeded { got: usize, max: usize },
}
```

Also: `BytecodeErrorKind::OriginPathsCountTooLarge`'s `Display` is needed; add to whatever `match` site renders bytecode-error-kinds (e.g., `bytecode_error_kind_display_message` if such a helper exists). Quick rg:

```bash
rg -n 'BytecodeErrorKind::ReservedBitsSet' crates/md-codec/src/error.rs
```

**Per F2 (blocker):** the conformance gate uses a hand-mirrored `ErrorVariantName` enum at `crates/md-codec/tests/error_coverage.rs` lines ~37-65. Without extending it for the two new top-level `Error` variants, the `every_error_variant_has_a_rejects_test_in_conformance` gate will pass spuriously (visible-to-conformance variants don't include `OriginPathsCountMismatch` / `PathComponentCountExceeded`).

Add to `tests/error_coverage.rs::ErrorVariantName`:

```rust
pub enum ErrorVariantName {
    // ... existing variants ...
    OriginPathsCountMismatch,
    PathComponentCountExceeded,
}
```

Per the file header comment: the enum is hand-mirrored intentionally. Adding a new top-level `Error` variant requires extending this enum. `BytecodeErrorKind` sub-variants (e.g., `OriginPathsCountTooLarge`) do NOT need entries — they're covered by the wrapping `InvalidBytecode` variant via the `INVALID_BYTECODE_PREFIX` machinery.

After this addition, Step 1.10's "Phase 4 will add: rejects_*" framing is correct: the gate genuinely fails until those rejection tests land in P4.

- [ ] **Step 1.9: Verify all of Phase 1 builds + new tests pass**

```bash
cargo build --workspace --all-features 2>&1 | tail -3
cargo test --package md-codec bytecode 2>&1 | grep '^test result' | head -3
```

- [ ] **Step 1.10: Run conformance gate**

```bash
cargo test --package md-codec --test error_coverage 2>&1 | tail -10
```

`every_error_variant_has_a_rejects_test_in_conformance` will fail because we added 3 new variants without conformance test coverage. That's expected; Phase 5 adds the conformance tests. For now, mark as a known-failing test or use `#[ignore]` temporarily — but cleaner: just record it as "Phase 5 will add: `rejects_origin_paths_count_too_large`, `rejects_origin_paths_count_mismatch`, `rejects_path_component_count_exceeded`."

- [ ] **Step 1.11: Commit Phase 1**

```bash
git add -A
git commit -m "$(cat <<'EOF'
refactor(v0.10-p1): reclaim header bit 3 + add Tag::OriginPaths variant

Foundation phase for v0.10 per-@N origin paths:

- BytecodeHeader: RESERVED_MASK 0x0B → 0x03; new origin_paths field;
  new_v0 signature gains second bool argument; ORIGIN_PATHS_BIT = 0x08.
  Valid header bytes now: 0x00, 0x04, 0x08, 0x0C.
- Tag enum: new Tag::OriginPaths = 0x36 variant.
- Error: new BytecodeErrorKind::OriginPathsCountTooLarge (structural,
  bytecode layer); new Error::OriginPathsCountMismatch (semantic, policy
  layer); new Error::PathComponentCountExceeded (applies to both path
  tags). Per spec §1 / §3 / §4 structural-vs-semantic split.

All Phase 1 tests pass. Conformance test for the new variants will
fail until Phase 5 lands the rejection vectors — known-pending.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 1.12: Opus reviewer pass on Phase 1**

Dispatch `Agent` with `model: opus`, `description: "v0.10 P1 header + tag + error review"`. Persist final report to `design/agent-reports/v0-10-phase-1-review.md`. Address any blockers/strong findings before P2.

---

## Phase 2 — Path component cap + OriginPaths bytecode encoder/decoder

**Goal:** Enforce `MAX_PATH_COMPONENTS = 10` in `encode_path` and `decode_path` (covers SharedPath uniformly per Q8). Add `encode_origin_paths` and `decode_origin_paths` helpers as standalone functions, with thorough tests, before any policy-layer integration.

### Files
- `crates/md-codec/src/bytecode/path.rs`

### Steps

- [ ] **Step 2.1: Write failing tests for MAX_PATH_COMPONENTS cap**

In `bytecode/path.rs` test module, add:

```rust
#[test]
fn encode_path_rejects_11_components() {
    let p = DerivationPath::from_str("m/0'/0'/0'/0'/0'/0'/0'/0'/0'/0'/0'").unwrap();
    let err = encode_path(&p).unwrap_err();
    assert!(matches!(err, Error::PathComponentCountExceeded { got: 11, max: 10 }));
}

#[test]
fn encode_path_accepts_10_components() {
    let p = DerivationPath::from_str("m/0'/0'/0'/0'/0'/0'/0'/0'/0'/0'").unwrap();
    encode_path(&p).expect("10 components must round-trip");
}

#[test]
fn decode_path_rejects_11_components_in_explicit_form() {
    // Synthesize a 0xFE explicit-form path with 11 components.
    let mut bytes = vec![0xFE, 0x0B];        // 0xFE indicator + count=11
    for _ in 0..11 { bytes.push(0x01); }      // 11 components, all m/0'
    let mut cur = Cursor::new(&bytes);
    let err = decode_path(&mut cur).unwrap_err();
    assert!(matches!(err, Error::PathComponentCountExceeded { got: 11, max: 10 }));
}

#[test]
fn decode_path_cap_check_fires_before_component_decode() {
    // Per F5: pin error-priority ordering — cap check must fire BEFORE
    // attempting to decode components. Synthesize count=11 with NO component
    // bytes after; if the cap check is correctly placed, we get
    // PathComponentCountExceeded. If the cap check is moved after component
    // decoding (a future refactor risk), we'd get UnexpectedEnd instead.
    let bytes = vec![0xFE, 0x0B];   // 0xFE indicator + count=11, no components
    let mut cur = Cursor::new(&bytes);
    let err = decode_path(&mut cur).unwrap_err();
    assert!(matches!(err, Error::PathComponentCountExceeded { got: 11, max: 10 }),
            "cap check must fire before component decode — got {:?}", err);
}
```

- [ ] **Step 2.2: Run tests; verify they fail**

```bash
cargo test --package md-codec bytecode::path encode_path_rejects 2>&1 | tail -10
```

- [ ] **Step 2.3: Add `MAX_PATH_COMPONENTS` cap enforcement**

In `bytecode/path.rs`:

```rust
/// Maximum derivation-path component count. Aligns with mk1 SPEC §3.5.
/// No real-world BIP-32 path approaches 10 components (BIP 48 + change/index = 6).
pub const MAX_PATH_COMPONENTS: usize = 10;

pub fn encode_path(path: &DerivationPath) -> Result<Vec<u8>, Error> {
    let len = path.len();
    if len > MAX_PATH_COMPONENTS {
        return Err(Error::PathComponentCountExceeded { got: len, max: MAX_PATH_COMPONENTS });
    }
    // ... existing encoding logic ...
}

pub fn decode_path(cursor: &mut Cursor) -> Result<DerivationPath, Error> {
    // ... existing dictionary/explicit dispatch ...
    // Inside the explicit-form arm, after decoding count:
    if count > MAX_PATH_COMPONENTS as u64 {
        return Err(Error::PathComponentCountExceeded { got: count as usize, max: MAX_PATH_COMPONENTS });
    }
    // ... continue decoding components ...
}
```

**Per F6 (strong): `encode_path` is currently infallible (`pub fn encode_path(path: &DerivationPath) -> Vec<u8>` at `path.rs:65`).** This step changes it to `Result<Vec<u8>, Error>` to surface the cap rejection — a public-API break on a `pub` function.

Decision: take the break (option B in F6). Reasoning: symmetric with `decode_path`'s already-fallible signature; cleaner invariants; fewer hidden footguns. Mechanical fix at every call site: add `?` propagation, or `.expect("validated upstream")` where the caller has already validated component count.

Call sites to update (rg-able):

```bash
rg -n '\bencode_path\(' crates/md-codec/src/ crates/md-codec/tests/
```

Expected: ~6+ sites in `path.rs` test module + `bytecode/encode.rs` callers + `policy.rs::encode_declaration`. Update each with `?` (or `.expect()` if upstream guarantees).

This API break MUST be added to MIGRATION.md (Phase 6 Step 6.9): "`encode_path(&DerivationPath) -> Vec<u8>` becomes `encode_path(&DerivationPath) -> Result<Vec<u8>, Error>`. Consumer updates: append `?` to call sites or `.expect()` if the path is known short."

- [ ] **Step 2.4: Run cap tests; verify they pass**

```bash
cargo test --package md-codec bytecode::path 2>&1 | grep '^test result' | head -3
```

- [ ] **Step 2.5: Address `decode_path_round_trip_multi_byte_component_count` test**

This existing test (line ~686) exercises 128-component paths to validate multi-byte LEB128. Under v0.10's cap of 10, this fails. Per F15: rewrite to exercise multi-byte LEB128 in the **child-index dimension** rather than **component-count dimension**:

```rust
#[test]
fn decode_path_round_trip_multi_byte_child_index() {
    // m/16384 — 16384 = 2*8192 requires 2-byte LEB128 in the child-index field.
    let path = DerivationPath::from_str("m/16384").unwrap();
    let bytes = encode_path(&path).expect("multi-byte LEB128 child-index round-trip");
    let mut cur = Cursor::new(&bytes);
    let recovered = decode_path(&mut cur).expect("decode round-trip");
    assert_eq!(path, recovered);
}
```

Drop the old 128-component test or rename to `_legacy_disabled` with explanation.

- [ ] **Step 2.6: Write failing tests for `encode_origin_paths` / `decode_origin_paths`**

```rust
#[test]
fn encode_origin_paths_round_trip_three_paths() {
    let paths = vec![
        DerivationPath::from_str("m/48'/0'/0'/2'").unwrap(),
        DerivationPath::from_str("m/48'/0'/0'/2'").unwrap(),
        DerivationPath::from_str("m/48'/0'/0'/100'").unwrap(),
    ];
    let bytes = encode_origin_paths(&paths).unwrap();
    // Per spec §2 Example B: 36 03 05 05 FE 04 61 01 01 C9 01
    assert_eq!(bytes[0], 0x36);   // Tag::OriginPaths
    assert_eq!(bytes[1], 0x03);   // count
    let mut cur = Cursor::new(&bytes[2..]);
    let recovered = decode_origin_paths(&mut cur).unwrap();
    assert_eq!(recovered.len(), 3);
    assert_eq!(recovered, paths);
}

#[test]
fn decode_origin_paths_rejects_count_zero() {
    let bytes = vec![0x00];   // count = 0 (Tag::OriginPaths byte already consumed)
    let mut cur = Cursor::new(&bytes);
    let err = decode_origin_paths(&mut cur).unwrap_err();
    assert!(matches!(err, Error::InvalidBytecode {
        kind: BytecodeErrorKind::OriginPathsCountTooLarge { count: 0, max: 32 },
        ..
    }));
}

#[test]
fn decode_origin_paths_rejects_count_33() {
    let bytes = vec![33];   // count = 33 (one over BIP 388 cap)
    let mut cur = Cursor::new(&bytes);
    let err = decode_origin_paths(&mut cur).unwrap_err();
    assert!(matches!(err, Error::InvalidBytecode {
        kind: BytecodeErrorKind::OriginPathsCountTooLarge { count: 33, max: 32 },
        ..
    }));
}

#[test]
fn decode_origin_paths_truncated_mid_list() {
    // count = 3, but only 2 path-decls follow.
    let bytes = vec![0x03, 0x05, 0x05];   // count + 2 dictionary indicators
    let mut cur = Cursor::new(&bytes);
    let err = decode_origin_paths(&mut cur).unwrap_err();
    assert!(matches!(err, Error::InvalidBytecode {
        kind: BytecodeErrorKind::UnexpectedEnd,
        ..
    }));
}
```

- [ ] **Step 2.7: Implement `encode_origin_paths` + `decode_origin_paths`**

```rust
pub fn encode_origin_paths(paths: &[DerivationPath]) -> Result<Vec<u8>, Error> {
    if paths.len() > 32 {
        // Should never happen — BIP 388 cap is upstream — but defense in depth.
        return Err(Error::OriginPathsCountMismatch { expected: 32, got: paths.len() });
    }
    let mut out = Vec::new();
    out.push(Tag::OriginPaths.as_byte());
    out.push(paths.len() as u8);
    for path in paths {
        out.extend_from_slice(&encode_path(path)?);
    }
    Ok(out)
}

pub fn decode_origin_paths(cursor: &mut Cursor) -> Result<Vec<DerivationPath>, Error> {
    // Tag::OriginPaths byte already consumed by caller.
    let count = cursor.read_u8()?;
    if count == 0 || count > 32 {
        return Err(Error::InvalidBytecode {
            offset: cursor.offset() - 1,
            kind: BytecodeErrorKind::OriginPathsCountTooLarge { count, max: 32 },
        });
    }
    let mut paths = Vec::with_capacity(count as usize);
    for _ in 0..count {
        paths.push(decode_path(cursor)?);
    }
    Ok(paths)
}
```

- [ ] **Step 2.8: Run all path-module tests; verify pass**

```bash
cargo test --package md-codec bytecode::path 2>&1 | grep '^test result'
```

- [ ] **Step 2.9: Add cursor-sentinel + asymmetric-byte-fill defensive tests**

Per `v07-decoder-arm-cursor-sentinel-pattern` lesson:

```rust
#[test]
fn decode_origin_paths_consumes_exact_bytes() {
    let paths = vec![
        DerivationPath::from_str("m/48'/0'/0'/2'").unwrap(),
        DerivationPath::from_str("m/48'/0'/0'/2'").unwrap(),
    ];
    let mut bytes = encode_origin_paths(&paths).unwrap();
    bytes.push(0xFF);   // trailing sentinel byte
    let mut cur = Cursor::new(&bytes[1..]);   // skip Tag byte
    decode_origin_paths(&mut cur).unwrap();
    // Cursor should be positioned at the sentinel.
    assert_eq!(cur.read_u8().unwrap(), 0xFF, "decoder consumed too many bytes");
}
```

- [ ] **Step 2.10: Commit Phase 2**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat(v0.10-p2): MAX_PATH_COMPONENTS cap + encode/decode_origin_paths

- bytecode/path.rs: new MAX_PATH_COMPONENTS = 10 const; cap enforced in
  encode_path and decode_path (uniformly applies to Tag::SharedPath and
  Tag::OriginPaths per Q8). New Error::PathComponentCountExceeded.
- New encode_origin_paths / decode_origin_paths helpers with thorough
  test coverage: round-trip, count=0 rejection, count>32 rejection,
  cursor exhaustion mid-list, and cursor-sentinel exact-consumption test.
- Existing decode_path_round_trip_multi_byte_component_count rewrites
  to exercise multi-byte LEB128 in the child-index dimension instead
  of component-count dimension (per F15).

Standalone helpers — Phase 4 wires them into to_bytecode / from_bytecode.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 2.11: Opus reviewer pass on Phase 2**

Persist to `design/agent-reports/v0-10-phase-2-review.md`. Address findings before P3.

---

## Phase 3 — Policy-layer integration (decoded_origin_paths field, to_bytecode dispatch)

**Goal:** Wire the new bytecode helpers into `WalletPolicy::to_bytecode` and `from_bytecode`. Add the `decoded_origin_paths` field. Auto-detect path divergence and emit OriginPaths or SharedPath accordingly. Implement the 4-tier precedence chain (Tier 0 opts override; Tier 1 decoded; Tier 2 KIV walk; Tier 3 shared fallback).

### Files
- `crates/md-codec/src/policy.rs`
- `crates/md-codec/src/options.rs`

### Steps

- [ ] **Step 3.1: Add `decoded_origin_paths` field + invariant**

In `policy.rs`:

```rust
pub struct WalletPolicy {
    // ... existing fields ...
    decoded_shared_path: Option<DerivationPath>,
    /// Per-`@N` origin paths populated by `from_bytecode` when the source
    /// bytecode used `Tag::OriginPaths`. Tier 1 source for the encoder's
    /// per-`@N`-path precedence chain.
    ///
    /// Invariant: at most one of `decoded_shared_path` and `decoded_origin_paths`
    /// is `Some`; the two are wire-level mutually exclusive (Q3-A).
    decoded_origin_paths: Option<Vec<DerivationPath>>,    // NEW
}
```

- [ ] **Step 3.2: Add `EncodeOptions::origin_paths` Tier 0 override**

In `options.rs`:

```rust
pub struct EncodeOptions {
    // ... existing fields ...
    /// Optional per-`@N` origin path override for deterministic encoding.
    /// Tier 0 in the encoder's per-`@N`-path precedence chain. Used by
    /// test-vector generation; production callers leave `None`.
    pub origin_paths: Option<Vec<DerivationPath>>,         // NEW
}

impl EncodeOptions {
    pub fn with_origin_paths(mut self, paths: Vec<DerivationPath>) -> Self {
        self.origin_paths = Some(paths);
        self
    }
}
```

- [ ] **Step 3.3: Implement `placeholder_paths_in_index_order`**

In `policy.rs`:

```rust
impl WalletPolicy {
    /// Return the per-`@N` origin path in placeholder-index order.
    /// Per spec §4 4-tier precedence chain.
    fn placeholder_paths_in_index_order(&self, opts: &EncodeOptions) -> Result<Vec<DerivationPath>, Error> {
        // Tier 0: opts override
        if let Some(ref paths) = opts.origin_paths {
            return Ok(paths.clone());
        }
        // Tier 1: decoded_origin_paths (round-trip stability)
        if let Some(ref paths) = self.decoded_origin_paths {
            return Ok(paths.clone());
        }
        // Tier 2: walk key-information-vector (concrete-key descriptor case)
        if let Some(paths) = self.try_extract_paths_from_kiv()? {
            return Ok(paths);
        }
        // Tier 3: fall through to shared-path tier chain (existing v0.x logic).
        let shared = self.shared_path_for_encoding(opts)?;
        let count = self.key_count();
        Ok(vec![shared; count])
    }
}
```

`try_extract_paths_from_kiv` is a new helper that walks `WalletPolicy.key_info_vector` (or whatever the concrete-key field is named) and extracts per-key origin paths. Returns `None` if no KIV present (template-only policy without decoded info).

- [ ] **Step 3.4: Update `to_bytecode` dispatch**

```rust
pub fn to_bytecode(&self, opts: &EncodeOptions) -> Result<Vec<u8>, Error> {
    // ... existing fingerprints validation ...

    // NEW: determine per-`@N` paths and detect divergence.
    let paths = self.placeholder_paths_in_index_order(opts)?;
    let all_share = paths.windows(2).all(|w| w[0] == w[1]);

    let header = BytecodeHeader::new_v0(opts.fingerprints.is_some(), !all_share);
    let mut out = Vec::new();
    out.push(header.as_byte());

    if all_share {
        out.extend_from_slice(&encode_declaration(&paths[0])?);
    } else {
        out.extend_from_slice(&encode_origin_paths(&paths)?);
    }

    if let Some(fps) = &opts.fingerprints {
        // existing fingerprints emission, unchanged
    }

    out.extend_from_slice(&tree_bytes);
    Ok(out)
}
```

- [ ] **Step 3.5: Update `from_bytecode` to populate `decoded_origin_paths`**

```rust
pub fn from_bytecode(bytes: &[u8]) -> Result<WalletPolicy, Error> {
    let mut cur = Cursor::new(bytes);
    let header = BytecodeHeader::from_byte(cur.read_u8()?)?;

    let (decoded_shared_path, decoded_origin_paths) = if header.origin_paths() {
        let tag = cur.read_u8()?;
        if tag != Tag::OriginPaths.as_byte() {
            return Err(Error::InvalidBytecode {
                offset: cur.offset() - 1,
                kind: BytecodeErrorKind::UnexpectedTag { expected: 0x36, got: tag },
            });
        }
        let paths = decode_origin_paths(&mut cur)?;
        (None, Some(paths))
    } else {
        let path = decode_declaration(&mut cur)?;
        (Some(path), None)
    };

    // ... existing fingerprints + tree-walk logic ...

    // After tree-walk, validate count consistency:
    if let Some(ref paths) = decoded_origin_paths {
        let placeholder_count = max_placeholder_index + 1;
        if paths.len() != placeholder_count {
            return Err(Error::OriginPathsCountMismatch {
                expected: placeholder_count,
                got: paths.len(),
            });
        }
    }

    Ok(WalletPolicy {
        // ... existing fields ...
        decoded_shared_path,
        decoded_origin_paths,
    })
}
```

- [ ] **Step 3.6: Round-trip tests**

```rust
#[test]
fn round_trip_shared_path_byte_identical_to_v0_9() {
    let p: WalletPolicy = "wsh(sortedmulti(2, @0/**, @1/**, @2/**))".parse().unwrap();
    let bytes = p.to_bytecode(&EncodeOptions::default()).unwrap();
    // First byte should be 0x00 (header bits 0+1+2+3 all clear)
    assert_eq!(bytes[0], 0x00, "shared-path policy should emit header 0x00");
    // Second byte should be 0x34 (Tag::SharedPath)
    assert_eq!(bytes[1], 0x34);
    let recovered = WalletPolicy::from_bytecode(&bytes).unwrap();
    assert!(recovered.decoded_origin_paths.is_none());
    assert!(recovered.decoded_shared_path.is_some());
}

#[test]
fn round_trip_divergent_paths_via_origin_paths_override() {
    let p: WalletPolicy = "wsh(sortedmulti(2, @0/**, @1/**, @2/**))".parse().unwrap();
    let opts = EncodeOptions::default().with_origin_paths(vec![
        DerivationPath::from_str("m/48'/0'/0'/2'").unwrap(),
        DerivationPath::from_str("m/48'/0'/0'/2'").unwrap(),
        DerivationPath::from_str("m/48'/0'/0'/100'").unwrap(),
    ]);
    let bytes = p.to_bytecode(&opts).unwrap();
    assert_eq!(bytes[0], 0x08, "divergent paths should set header bit 3");
    assert_eq!(bytes[1], 0x36, "Tag::OriginPaths at path-decl slot");

    // First-pass round-trip: decode and re-encode, expect byte-identical.
    let recovered = WalletPolicy::from_bytecode(&bytes).unwrap();
    assert!(recovered.decoded_origin_paths.is_some());
    let bytes2 = recovered.to_bytecode(&EncodeOptions::default()).unwrap();
    assert_eq!(bytes, bytes2, "decode→encode must be byte-identical");
}
```

- [ ] **Step 3.6.5 (NEW per F8 + F9): Tier-precedence and round-trip stability tests**

Tier-collision coverage (Tier 0 > Tier 1 > Tier 2 > Tier 3):

```rust
#[test]
fn tier_0_origin_paths_override_wins_over_tier_1() {
    // Decode an OriginPaths-bearing bytecode (populates Tier 1 = decoded_origin_paths),
    // then re-encode with EncodeOptions::with_origin_paths overriding to different paths.
    // Expect the wire reflects the override, not the decoded.
    let bytes_a = vec![0x08, 0x36, 0x02, 0x05, 0x06, /* tree */];
    let p = WalletPolicy::from_bytecode(&bytes_a).unwrap();
    let override_paths = vec![
        DerivationPath::from_str("m/87'/0'/0'").unwrap(),
        DerivationPath::from_str("m/87'/0'/0'").unwrap(),
    ];
    let opts = EncodeOptions::default().with_origin_paths(override_paths);
    let bytes_b = p.to_bytecode(&opts).unwrap();
    // Override paths agree → encoder emits SharedPath, NOT OriginPaths.
    assert_eq!(bytes_b[0], 0x00, "all-shared override must clear bit 3");
    assert_eq!(bytes_b[1], 0x34, "expected SharedPath after override");
}

#[test]
fn tier_1_decoded_wins_over_tier_2_kiv_walk() {
    // After from_bytecode, decoded_origin_paths is Tier 1. If the policy also has
    // KIV data (Tier 2), Tier 1 must win on re-encode.
    // (Implementation: synthesize a WalletPolicy with both fields populated;
    // if the construction is impossible without unsafe access, document why
    // and verify by inspection.)
    // ...
}

#[test]
fn tier_3_shared_fallback_for_template_only_policy() {
    // A policy parsed from a bare BIP 388 template (no concrete keys, no
    // decoded_origin_paths) falls through to Tier 3: shared-path fallback.
    let p: WalletPolicy = "wsh(sortedmulti(2, @0/**, @1/**, @2/**))".parse().unwrap();
    let bytes = p.to_bytecode(&EncodeOptions::default()).unwrap();
    assert_eq!(bytes[0], 0x00, "Tier 3 shared-path fallback");
    assert_eq!(bytes[1], 0x34, "Tier 3 emits SharedPath");
}
```

Round-trip stability:

```rust
#[test]
fn double_round_trip_origin_paths_byte_identical() {
    // encode → decode → encode → decode → encode is byte-stable; first round-trip
    // is the typical happy path, but second round-trip catches Tier 1 ↔ Tier 0
    // priority asymmetries.
    let p: WalletPolicy = "wsh(sortedmulti(2, @0/**, @1/**, @2/**))".parse().unwrap();
    let opts = EncodeOptions::default().with_origin_paths(vec![
        DerivationPath::from_str("m/48'/0'/0'/2'").unwrap(),
        DerivationPath::from_str("m/48'/0'/0'/2'").unwrap(),
        DerivationPath::from_str("m/48'/0'/0'/100'").unwrap(),
    ]);
    let bytes1 = p.to_bytecode(&opts).unwrap();
    let p1 = WalletPolicy::from_bytecode(&bytes1).unwrap();
    let bytes2 = p1.to_bytecode(&EncodeOptions::default()).unwrap();
    let p2 = WalletPolicy::from_bytecode(&bytes2).unwrap();
    let bytes3 = p2.to_bytecode(&EncodeOptions::default()).unwrap();
    assert_eq!(bytes1, bytes2, "first round-trip");
    assert_eq!(bytes2, bytes3, "second round-trip — Tier 1 stability");
}

#[test]
fn decoded_shared_path_and_decoded_origin_paths_mutually_exclusive_after_decode() {
    // After from_bytecode, exactly one of the two decoded-path fields is Some.
    let shared_bytes = vec![0x00, 0x34, 0x05, /* tree */];
    let p_shared = WalletPolicy::from_bytecode(&shared_bytes).unwrap();
    assert!(p_shared.decoded_shared_path.is_some());
    assert!(p_shared.decoded_origin_paths.is_none());

    let origin_bytes = vec![0x08, 0x36, 0x02, 0x05, 0x06, /* tree */];
    let p_origin = WalletPolicy::from_bytecode(&origin_bytes).unwrap();
    assert!(p_origin.decoded_shared_path.is_none());
    assert!(p_origin.decoded_origin_paths.is_some());
}
```

(Synthetic-byte test inputs above are sketches — actual byte sequences need real tree bytes after the path-decl slot.)

- [ ] **Step 3.7: Test conflicting-path-decl rejection**

```rust
#[test]
fn from_bytecode_rejects_header_bit_3_set_with_shared_path_tag() {
    // Synthesize: header 0x08 (origin_paths flag) but Tag::SharedPath at offset 1.
    let bytes = vec![0x08, 0x34, 0x05];
    let err = WalletPolicy::from_bytecode(&bytes).unwrap_err();
    assert!(matches!(err, Error::InvalidBytecode {
        offset: 1,
        kind: BytecodeErrorKind::UnexpectedTag { expected: 0x36, got: 0x34 },
    }));
}

#[test]
fn from_bytecode_rejects_header_bit_3_clear_with_origin_paths_tag() {
    let bytes = vec![0x00, 0x36, 0x01, 0x05];   // header clear + Tag::OriginPaths
    let err = WalletPolicy::from_bytecode(&bytes).unwrap_err();
    assert!(matches!(err, Error::InvalidBytecode {
        offset: 1,
        kind: BytecodeErrorKind::UnexpectedTag { expected: 0x34, got: 0x36 },
    }));
}
```

- [ ] **Step 3.8: Cargo build + test gate**

```bash
cargo build --workspace --all-features 2>&1 | tail -3
cargo test --package md-codec policy 2>&1 | grep '^test result' | head -3
```

- [ ] **Step 3.9: Commit Phase 3**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat(v0.10-p3): policy-layer integration for OriginPaths

- WalletPolicy: new decoded_origin_paths: Option<Vec<DerivationPath>>
  field for round-trip stability. Invariant documented: at most one
  of decoded_shared_path / decoded_origin_paths is Some.
- New placeholder_paths_in_index_order method implementing the spec §4
  4-tier precedence chain (opts override → decoded → KIV walk → shared
  fallback).
- to_bytecode dispatch: auto-detect path divergence; emit Tag::SharedPath
  if all paths agree, Tag::OriginPaths otherwise. Header bit 3 set
  accordingly. Per Q9-A.
- from_bytecode: dispatch on header bit 3; validate header-bit-vs-tag
  consistency via BytecodeErrorKind::UnexpectedTag; populate either
  decoded_shared_path or decoded_origin_paths. Validate count consistency
  with tree placeholder count post-walk.
- EncodeOptions: new origin_paths Tier 0 override for deterministic
  test-vector generation. New with_origin_paths builder method.
- Round-trip tests: shared-path byte-identical to v0.9; divergent-path
  with origin_paths override emits header 0x08 + Tag::OriginPaths;
  decode→encode is byte-identical.
- Conflicting-path-decl rejection tests.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 3.10: Opus reviewer pass on Phase 3**

Persist to `design/agent-reports/v0-10-phase-3-review.md`. Address findings.

---

## Phase 4 — Test corpus + conformance + hand-AST coverage

**Goal:** Add positive vectors (`o1`, `o2`, `o3`), negative vectors (`n_orig_*`), conformance tests for the new error variants, and hand-AST coverage.

### Files
- `crates/md-codec/src/vectors.rs`
- `crates/md-codec/src/bytecode/hand_ast_coverage.rs`
- `crates/md-codec/tests/conformance.rs`
- `crates/md-codec/tests/vectors_schema.rs`

### Steps

- [ ] **Step 4.1: Add positive vectors o1, o2, o3**

In `vectors.rs`, parallel to `build_v0_9_testnet_p2sh_p2wsh_vector`:

```rust
// Per F3: vectors o1 and o2 mirror SPEC §2 Example C (header 0x08, no fps)
// and Example B (header 0x0C, fps {deadbeef, cafebabe, d00df00d}) respectively.
// The OriginPaths block bytes `36 03 05 05 FE 04 61 01 01 C9 01` are pinned
// in the spec; corpus regen MUST produce these same bytes. If the spec
// example values change, both spec and corpus update in lockstep.
fn build_v0_10_origin_paths_vectors() -> Vec<Vector> {
    use bitcoin::bip32::DerivationPath;
    use std::str::FromStr;

    let mainnet = DerivationPath::from_str("m/48'/0'/0'/2'").unwrap();
    let custom = DerivationPath::from_str("m/48'/0'/0'/100'").unwrap();

    let o1 = build_origin_paths_vector(
        "o1_sortedmulti_2of3_divergent_paths",
        "O1 — wsh(sortedmulti(2,...)) 2-of-3 with two cosigners on m/48'/0'/0'/2' and one on m/48'/0'/0'/100' (divergent path indicator triggers Tag::OriginPaths)",
        "wsh(sortedmulti(2,@0/**,@1/**,@2/**))",
        vec![mainnet.clone(), mainnet.clone(), custom.clone()],
        None,   // no fingerprints
    );

    let o2 = build_origin_paths_vector(
        "o2_sortedmulti_2of3_divergent_paths_with_fingerprints",
        "O2 — same template as O1, with all 3 master-key fingerprints (header 0x0C exercises both flags)",
        "wsh(sortedmulti(2,@0/**,@1/**,@2/**))",
        vec![mainnet.clone(), mainnet.clone(), custom.clone()],
        Some(vec![[0xde, 0xad, 0xbe, 0xef], [0xca, 0xfe, 0xba, 0xbe], [0xd0, 0x0d, 0xf0, 0x0d]]),
    );

    let o3 = build_origin_paths_vector(
        "o3_wsh_sortedmulti_2of4_divergent_paths",
        "O3 — wsh(sortedmulti(2,...)) 2-of-4 exercising count=4 boundary with multiple distinct dictionary indicators",
        "wsh(sortedmulti(2,@0/**,@1/**,@2/**,@3/**))",
        vec![
            mainnet.clone(),
            mainnet.clone(),
            DerivationPath::from_str("m/48'/0'/0'/1'").unwrap(),
            DerivationPath::from_str("m/87'/0'/0'").unwrap(),
        ],
        None,
    );

    vec![o1, o2, o3]
}

fn build_origin_paths_vector(
    id: &str,
    description: &str,
    policy_str: &str,
    paths: Vec<DerivationPath>,
    fps: Option<Vec<[u8; 4]>>,
) -> Vector {
    // ... build via to_bytecode with EncodeOptions::with_origin_paths + optional with_fingerprints ...
}
```

Wire into `build_positive_vectors_v2`:

```rust
out.extend(build_v0_10_origin_paths_vectors());   // adds 3
```

Add an inline test that pins the spec-corpus mutual validation:

```rust
#[test]
fn o2_vector_origin_paths_block_matches_spec_example_b() {
    // SPEC §2 Example B's OriginPaths block bytes (header + path-decl slot only).
    // If this assertion ever fires, either the spec example or the corpus drifted.
    const EXPECTED_ORIGIN_PATHS_BYTES: &str = "36030505fe046101 01c901".replace(' ', "");
    let vectors = build_v0_10_origin_paths_vectors();
    let o2 = vectors.iter().find(|v| v.id == "o2_sortedmulti_2of3_divergent_paths_with_fingerprints").unwrap();
    assert!(o2.expected_bytecode_hex.contains(&EXPECTED_ORIGIN_PATHS_BYTES),
            "o2 expected_bytecode_hex must contain SPEC §2 Example B's OriginPaths bytes");
}
```

- [ ] **Step 4.2: Add negative vectors**

Add `n16` through `n19` (or whichever next IDs):

- `n_orig_paths_count_zero` — header 0x08, Tag::OriginPaths byte, count=0.
- `n_orig_paths_count_too_large` — header 0x08, Tag::OriginPaths, count=33.
- `n_orig_paths_truncated` — header 0x08, Tag::OriginPaths, count=3, only 2 path-decls follow before tree.
- `n_orig_paths_count_mismatch` — header 0x08, Tag::OriginPaths count=4, but tree has only 3 placeholders.
- `n_path_components_too_long` — `Tag::SharedPath` (or OriginPaths) with 11-component explicit path.
- `n_conflicting_path_declarations_bit_set_tag_shared` — header 0x08 with `Tag::SharedPath` at offset 1.

Add generators per existing pattern (`generate_n08_*` etc.).

- [ ] **Step 4.3: Update vectors_schema.rs corpus count assertion**

```rust
assert_eq!(
    v2.vectors.len(),
    47,     // was 44 in v0.9; add 3 for o1/o2/o3
    "expected exactly 47 positive corpus vectors in schema-2 (v0.10 added o1/o2/o3 for OriginPaths)"
);
```

- [ ] **Step 4.4: Add conformance tests**

In `tests/conformance.rs`:

```rust
#[test]
fn rejects_origin_paths_count_too_large() {
    // Synthesize: header 0x08 + Tag::OriginPaths + count=33 + 33 path-decls
    let mut bytes = vec![0x08, 0x36, 33];
    for _ in 0..33 { bytes.push(0x05); }   // 33 dictionary indicators
    bytes.extend_from_slice(&[/* tree bytes */]);
    let err = WalletPolicy::from_bytecode(&bytes).unwrap_err();
    assert!(matches!(err, Error::InvalidBytecode {
        kind: BytecodeErrorKind::OriginPathsCountTooLarge { count: 33, max: 32 },
        ..
    }));
}

#[test]
fn rejects_origin_paths_count_mismatch() {
    // count=2 but tree has 3 placeholders
    // (build via byte synthesis or use a real policy with deliberately-wrong opts)
    // ...
}

#[test]
fn rejects_path_component_count_exceeded() {
    // SharedPath with 11-component explicit path
    // ...
}
```

- [ ] **Step 4.5: Add hand-AST coverage tests**

In `bytecode/hand_ast_coverage.rs`:

```rust
#[test]
fn header_origin_paths_flag_round_trip() {
    let h = BytecodeHeader::new_v0(false, true);
    let b = h.as_byte();
    assert_eq!(b, 0x08);
    let h2 = BytecodeHeader::from_byte(b).unwrap();
    assert_eq!(h, h2);
}

#[test]
fn encoder_emits_shared_path_when_all_paths_agree() {
    let p: WalletPolicy = "wsh(sortedmulti(2, @0/**, @1/**, @2/**))".parse().unwrap();
    let bytes = p.to_bytecode(&EncodeOptions::default()).unwrap();
    assert_ne!(bytes[1], 0x36, "encoder must NOT emit OriginPaths for shared-path policy");
    assert_eq!(bytes[1], 0x34);
}

#[test]
fn encoder_emits_origin_paths_when_paths_diverge() {
    let p: WalletPolicy = "wsh(sortedmulti(2, @0/**, @1/**, @2/**))".parse().unwrap();
    let opts = EncodeOptions::default().with_origin_paths(vec![
        DerivationPath::from_str("m/48'/0'/0'/2'").unwrap(),
        DerivationPath::from_str("m/48'/0'/0'/2'").unwrap(),
        DerivationPath::from_str("m/48'/0'/0'/100'").unwrap(),
    ]);
    let bytes = p.to_bytecode(&opts).unwrap();
    assert_eq!(bytes[1], 0x36);
    assert_eq!(bytes[0], 0x08, "header bit 3 must be set");
}

#[test]
fn max_path_components_boundary_10_passes_11_rejects() {
    let path_10 = DerivationPath::from_str("m/0'/0'/0'/0'/0'/0'/0'/0'/0'/0'").unwrap();
    let path_11 = DerivationPath::from_str("m/0'/0'/0'/0'/0'/0'/0'/0'/0'/0'/0'").unwrap();
    encode_path(&path_10).expect("10 components must encode");
    let err = encode_path(&path_11).unwrap_err();
    assert!(matches!(err, Error::PathComponentCountExceeded { got: 11, max: 10 }));
}
```

- [ ] **Step 4.6: Regenerate vectors and update SHA pin**

```bash
cargo run --features test-helpers --bin gen_vectors -- --output crates/md-codec/tests/vectors/v0.1.json --schema 1
cargo run --features test-helpers --bin gen_vectors -- --output crates/md-codec/tests/vectors/v0.2.json --schema 2
sha256sum crates/md-codec/tests/vectors/v0.2.json
```

Update `V0_2_SHA256` in `tests/vectors_schema.rs`.

- [ ] **Step 4.7: Full test gate**

```bash
cargo test --workspace --all-features 2>&1 | grep '^test result' | awk '{ok+=$4; failed+=$6} END {print "ok="ok" failed="failed}'
```

Expected: ~700+ passing, 0 failing.

- [ ] **Step 4.8: Commit Phase 4**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat(v0.10-p4): test corpus + conformance + hand-AST for OriginPaths

- vectors.rs: 3 new positive vectors o1/o2/o3 covering
  OriginPaths-with-no-fingerprints, OriginPaths-with-fingerprints
  (header 0x0C), and 4-placeholder count-boundary case.
- 6 new negative vectors: count=0, count=33, truncation,
  count-mismatch with tree, path-component-cap exceeded,
  conflicting path declarations.
- conformance.rs: 3 new rejects_* tests closing the
  every_error_variant_has_a_rejects_test gate from Phase 1.
- hand_ast_coverage.rs: 4 new tests pinning Tag::OriginPaths byte
  position, header bit-3 round trip, encoder dispatch determinism,
  and MAX_PATH_COMPONENTS = 10 boundary.
- vectors_schema.rs: corpus count 44 → 47; SHA pin updated.
- Vectors regenerate under family token "md-codec 0.10".

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 4.9: Opus reviewer pass on Phase 4**

Persist to `design/agent-reports/v0-10-phase-4-review.md`.

---

## Phase 5 — `PolicyId::fingerprint()` API

**Goal:** Land the small `PolicyId::fingerprint() -> [u8; 4]` API addition per Q13. Pure additive; no wire-format change.

### Files
- `crates/md-codec/src/policy_id.rs`
- `crates/md-codec/src/bin/md/main.rs` (optional CLI integration)

### Steps

- [ ] **Step 5.1: Add `fingerprint()` method**

```rust
impl PolicyId {
    /// Return the first 32 bits of this PolicyId as a 4-byte array, parallel
    /// to BIP 32 master-key fingerprints. Suitable as a short identifier in
    /// CLI output, log lines, or as a minimal-cost engraving anchor.
    pub fn fingerprint(&self) -> [u8; 4] {
        let mut fp = [0u8; 4];
        fp.copy_from_slice(&self.0[0..4]);
        fp
    }
}
```

- [ ] **Step 5.2: Test fingerprint stability + determinism**

```rust
#[test]
fn policy_id_fingerprint_is_first_4_bytes() {
    let id = PolicyId([0xa1, 0xb2, 0xc3, 0xd4, 0xe5, 0xf6, 0x07, 0x18, 0x29, 0x3a, 0x4b, 0x5c, 0x6d, 0x7e, 0x8f, 0x90]);
    assert_eq!(id.fingerprint(), [0xa1, 0xb2, 0xc3, 0xd4]);
}

#[test]
fn policy_id_fingerprint_deterministic_from_policy() {
    let p: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
    let id1 = compute_policy_id_for_policy(&p).unwrap();
    let id2 = compute_policy_id_for_policy(&p).unwrap();
    assert_eq!(id1.fingerprint(), id2.fingerprint());
}
```

- [ ] **Step 5.3: Commit Phase 5**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat(v0.10-p5): add PolicyId::fingerprint() short-identifier API

Per spec Q13: 4-byte (32-bit) short identifier extracted from the
top of PolicyId. Parallel to BIP 32 master-key fingerprint API.
Renders as 8 hex characters. Use cases: CLI output, log lines,
minimal-cost engraving anchor for users who don't want the full
12-word phrase.

Pure additive API; no wire-format change.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 5.4: Opus reviewer pass on Phase 5 (light)**

Persist to `design/agent-reports/v0-10-phase-5-review.md`.

---

## Phase 6 — BIP draft + docs updates

**Goal:** Bring the BIP draft, README, MIGRATION.md, and CHANGELOG up to v0.10 state. No code changes.

### Files
- `bip/bip-mnemonic-descriptor.mediawiki`
- `README.md`
- `MIGRATION.md`
- `CHANGELOG.md`
- `design/POLICY_BACKUP.md`

### Steps

- [ ] **Step 6.1: Update BIP draft — add §"Per-`@N` path declaration"**

After §"Path declaration" (or wherever the existing path-decl prose lives), insert a new §"Per-`@N` path declaration" subsection per spec §6 prose:

```mediawiki
=====Per-`@N` path declaration (v0.10+)=====

When the wallet policy's `@N` placeholders use *different* origin paths (e.g., a multisig where each cosigner derives from a distinct BIP 48 account), the encoder emits a `Tag::OriginPaths` block in place of `Tag::SharedPath`. The block is gated by header bit 3 (`0x08`).

[... full prose per spec §2-§3 ...]
```

- [ ] **Step 6.2: Add §"PolicyId types" teaching subsection**

Per spec §6 prose for Type 0 / Type 1 typology.

- [ ] **Step 6.3: Soften 12-word PolicyId engraving language**

Per spec §6 "PolicyId UX — engraving language softening" prose.

- [ ] **Step 6.4: Add §"Authority precedence with MK"**

Three-sentence subsection cross-referencing mk1 BIP §"Authority precedence" / SPEC §5.1.

- [ ] **Step 6.5: Update path dictionary BIP table**

Add wire-format-cap statement: "Explicit-form paths (`0xFE`) MUST NOT exceed 10 components. Decoders MUST reject longer paths with `Error::PathComponentCountExceeded`."

- [ ] **Step 6.6: Update `design/POLICY_BACKUP.md`**

Per Q1 lock: change the `Tag::RecoveryHints` slot mention from `0x36` → `0x37`.

- [ ] **Step 6.7: Update README.md**

If the README currently says "shared path only" or similar in the scope section, update to "shared paths and per-`@N` divergent paths (v0.10+)."

- [ ] **Step 6.8: Add CHANGELOG `[0.10.0]` section**

Lead with "Why a wire-format break?" callout per spec §6 prose. Sections: Why a wire-format break, Added, Changed, Wire format, FOLLOWUPS closed.

- [ ] **Step 6.9: Add MIGRATION `## v0.9.x → v0.10.0` section**

Lead with brief "Why a wire-format break?" framing. Sections: What renamed/added/changed, Mechanical sed, Hand-rename items (`BytecodeHeader::new_v0` signature), Wire format, Test rewrite for `MAX_PATH_COMPONENTS`.

- [ ] **Step 6.10: Commit Phase 6**

```bash
git add -A
git commit -m "$(cat <<'EOF'
docs(v0.10-p6): BIP draft + README + MIGRATION + CHANGELOG for v0.10

- BIP §"Per-`@N` path declaration" (new): full wire-format prose for
  Tag::OriginPaths block, header bit 3 dispatch, and divergent-path
  encoding semantics.
- BIP §"PolicyId types" (new): Type 0 / Type 1 teaching subsection
  per spec Q12.
- BIP §"Authority precedence with MK" (new): three-sentence
  cross-reference to mk1 BIP / SPEC §5.1 per Q5.
- BIP §"Path dictionary" path-component-cap statement (Q8).
- BIP 12-word phrase engraving language softens to MAY-engrave.
- design/POLICY_BACKUP.md: Tag::RecoveryHints slot 0x36 → 0x37.
- README scope section update.
- MIGRATION: new v0.9.x → v0.10.0 section with sed snippet.
- CHANGELOG: [0.10.0] with "Why a wire-format break?" callout.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 6.11: Opus reviewer pass on Phase 6**

Persist to `design/agent-reports/v0-10-phase-6-review.md`.

---

## Phase 7 — Release v0.10.0

**Goal:** Standard release dance: version bump, FOLLOWUPS closure, CLAUDE.md update, PR, tag, GitHub release.

### Files
- `crates/md-codec/Cargo.toml`
- `crates/md-signer-compat/Cargo.toml` (if any breakage)
- `design/FOLLOWUPS.md`
- `CLAUDE.md`

### Steps

- [ ] **Step 7.1: Audit `md-signer-compat` for breakage**

```bash
rg 'BytecodeHeader|Tag::|MAX_PATH_COMPONENTS|OriginPaths|policy_id_seed' crates/md-signer-compat/
```

If hits in public API surface: minor bump (e.g., `0.1.x → 0.2.0`). If only internal/test: patch bump (e.g., `0.1.1 → 0.1.2`). If zero hits: stays at current version.

- [ ] **Step 7.2: Confirm version is 0.10.0** (already bumped in Pre-Phase-0 Step 4 per F15)

```bash
grep '^version' crates/md-codec/Cargo.toml
# Expect: version = "0.10.0"
```

If the version is still `0.9.1` (e.g., the Pre-Phase-0 commit was missed), bump now:

```toml
# crates/md-codec/Cargo.toml
version = "0.10.0"
```

But note: this means mid-plan vector regenerations would have used the wrong family token. Phase 4 vectors must be regenerated again with the correct version. Easier path: ensure Pre-Phase-0 Step 4 lands first, as the plan instructs.

- [ ] **Step 7.3: Mark FOLLOWUPS resolved**

In `design/FOLLOWUPS.md`, flip status of `md-per-at-N-path-tag-allocation` from `open` to `resolved md-codec-v0.10.0` with a brief description matching the v0.9.0 pattern.

- [ ] **Step 7.4: Update CLAUDE.md**

Drop the resolved entry from the "Currently open mk1-surfaced items" list.

- [ ] **Step 7.5: Final test + lint + doc gate**

```bash
cargo build --workspace --all-features
cargo test --workspace --all-features
PATH="$HOME/.cargo/bin:$PATH" cargo +stable clippy --workspace --all-features --all-targets -- -D warnings
PATH="$HOME/.cargo/bin:$PATH" cargo +stable fmt --all -- --check
cargo doc --workspace --all-features --no-deps 2>&1 | grep -E '^(warning|error)' | head -3
```

Expected: all clean.

- [ ] **Step 7.6: Final release commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
release(v0.10.0): per-@N origin paths + header bit 3 reclaim

- New Tag::OriginPaths = 0x36 block with dense per-@N path encoding
- Header bit 3 = OriginPaths flag (was reserved-must-be-zero in v0.x)
- MAX_PATH_COMPONENTS = 10 enforced uniformly on Tag::SharedPath and
  Tag::OriginPaths
- Encoder auto-detects divergent-path policies per Q9-A; emits
  OriginPaths when needed, SharedPath otherwise
- New decoded_origin_paths field on WalletPolicy for round-trip
  byte-stability
- New PolicyId::fingerprint() → [u8; 4] short-identifier API
- BIP §"Per-@N path declaration" + §"PolicyId types" teaching
  subsection + §"Authority precedence with MK" cross-reference

Closes md-per-at-N-path-tag-allocation FOLLOWUPS.

Wire-format break: header bit 3 reclaimed; v0.x ≤ 0.9 decoders
reject v0.10 OriginPaths-using encodings via Error::ReservedBitsSet
(intended forward-compat behavior). Shared-path encodings remain
byte-identical to v0.9 (modulo family-token roll in vectors regen).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 7.7: Push, open PR**

```bash
git push -u origin feature/v0.10-per-at-n-paths
gh pr create --title "release(v0.10.0): per-@N origin paths + header bit 3 reclaim" --body "..."
```

- [ ] **Step 7.8: Wait for CI green, address fmt/clippy if surfaced**

```bash
gh pr checks <pr-number>
```

If CI fails, fix and re-push. Don't merge until clean.

- [ ] **Step 7.9: Merge PR, return to main, tag**

```bash
gh pr merge <pr-number> --merge
git checkout main && git pull --ff-only
git tag -a md-codec-v0.10.0 <merge-commit> -m "md-codec v0.10.0"
git push origin md-codec-v0.10.0
```

- [ ] **Step 7.10: Create GitHub release**

```bash
awk '/^## \[0\.10\.0\]/{flag=1} /^## \[0\.9\.1\]/{flag=0} flag' CHANGELOG.md > /tmp/v010-release-notes.md
gh release create md-codec-v0.10.0 --title "md-codec v0.10.0" --notes-file /tmp/v010-release-notes.md
```

- [ ] **Step 7.11: Cross-update sibling mnemonic-key (per F7 + RELEASE_PROCESS.md)**

Per `design/RELEASE_PROCESS.md` §"CLAUDE.md crosspointer maintenance":

```bash
cd /scratch/code/shibboleth/mnemonic-key
git fetch origin && git checkout main && git pull --ff-only
```

a. **Audit forward-reference text** that becomes resolvable post-v0.10:

```bash
git grep -i 'md-per-at-N\|0x36\|OriginPaths\|md1.*path-tag\|md1 currently\|will land\|in flight'
```

If hits exist (likely in `bip/bip-mnemonic-key.mediawiki`, `design/SPEC_mk_v0_1.md`, `design/DECISIONS.md`, `design/IMPLEMENTATION_PLAN_mk_v0_1.md`, `docs/superpowers/specs/2026-04-29-mk1-open-questions-closure-design.md`), update them to point at the v0.10.0 release-tag prose and shipped state.

b. **Update mk1's companion FOLLOWUPS entry**:

```bash
# In /scratch/code/shibboleth/mnemonic-key/design/FOLLOWUPS.md
# Find the `md-per-N-path-tag-allocation` entry; flip Status to:
#   Status: resolved by md-codec-v0.10.0 (commit <md1-merge-sha>)
```

c. **Audit mk1's BIP for post-brainstorm edits** that would break md1's §"Authority precedence with MK" cross-reference:

```bash
git log --oneline bip/bip-mnemonic-key.mediawiki | head -10
```

If mk1 BIP §"Authority precedence" prose has changed since brainstorm-time (2026-04-29), reconcile: either update md1's reference prose OR push back to user.

d. **Open a small mk1 PR** for these updates (separate sibling-repo branch):

```bash
git checkout -b feature/md-codec-v0.10-shipped-cross-update
# ... edit files ...
git commit -m "docs(post-md-v0.10.0): de-hedge md1 path-tag references"
git push -u origin feature/md-codec-v0.10-shipped-cross-update
gh pr create --title "docs(post-md-v0.10.0): de-hedge md1 path-tag references" \
  --body "Cross-update for md-codec v0.10.0 ship (md1 PR <md1-pr-#>). Resolves the md-per-N-path-tag-allocation companion FOLLOWUPS entry."
```

e. **Update CLAUDE.md** in the md1 repo to drop the resolved entry (already covered in Step 7.4 above, but verify the entry is gone).

If mk1's main branch has unrelated in-flight work (per the v0.9.0 hedge audit pattern), use a worktree off `origin/main` to avoid disturbing parallel sessions:

```bash
cd /scratch/code/shibboleth/mnemonic-key
git worktree add ../mk-v010-cross-update -b feature/md-codec-v0.10-shipped-cross-update origin/main
cd ../mk-v010-cross-update
# ... make edits + commit + push ...
```

Persist a hedge-audit report to `descriptor-mnemonic/design/agent-reports/v0-10-phase-7-mk1-hedge-audit.md` documenting what changed and pinning the md1 commit/tag — same pattern as v0.9's hedge audit at `v0-9-phase-0-mk1-hedge-audit.md`.

---

## Plan review pass-1 status (opus, persisted to `design/agent-reports/v0-10-plan-review-1.md`)

| F | Severity | Status |
|---|---|---|
| F1 | blocker | ✅ Step 1.2 + 1.3 explicitly handle existing `reserved_bit_3_set` deletion + `all_reserved_bits_set_no_fingerprints` rewrite. |
| F2 | blocker | ✅ Step 1.8 includes `ErrorVariantName` mirror-enum extension. |
| F3 | blocker | ✅ Step 4.1 includes spec-corpus pin doc comment + inline assertion test. |
| F4 | strong | ✅ Step 1.6 lists all 3 affected tag.rs tests with line numbers + rename guidance. |
| F5 | strong | ✅ Step 2.1 adds `decode_path_cap_check_fires_before_component_decode` defensive test. |
| F6 | strong | ✅ Step 2.3 owns the `encode_path` API break explicitly; Phase 6 Step 6.9 will list it in MIGRATION.md. |
| F7 | strong | ✅ Step 7.11 expanded with full sibling-repo coordination protocol (audit, FOLLOWUPS update, BIP review, hedge-audit-report persistence). |
| F8 | strong | ✅ New Step 3.6.5 adds Tier-precedence collision tests (Tier 0 vs 1, Tier 1 vs 2, Tier 3 fallback). |
| F9 | strong | ✅ New Step 3.6.5 adds double-round-trip + mutual-exclusion tests. |
| F10 | nice-to-have | folded into Phase 6 implementer guidance — copy spec prose verbatim from §6 prose blocks. |
| F11 | nice-to-have | not addressed inline; can fold into P5 by implementer if desired. |
| F12 | confirmation | ✅ md-signer-compat audit pre-confirmed; no version bump needed. |
| F13 | nice-to-have | folded into Step 4.2 — implementer maps each negative vector to its conformance test. |
| F14 | nice-to-have | ✅ Pre-Phase-0 Step 3 baseline pinned to 678 tests. |
| F15 | nice-to-have | ✅ Pre-Phase-0 Step 4 (NEW) bumps version to 0.10.0 before any vector regen. Phase 7 Step 7.2 becomes verification-only. |

Open implementer questions (1-5 in pass-1 report) carried as plan-time decisions during phased implementation.

## Self-review checklist (pre-opus-review)

- [x] All 13 brainstorm questions LOCKED and reflected in the plan.
- [x] Each phase has a clear scope, files, TDD-step sequence, and commit boundary.
- [x] Per-phase opus reviewer gates with persistent reports to `design/agent-reports/`.
- [x] Round-trip stability via `decoded_origin_paths` field per spec F2.
- [x] Structural-vs-semantic error split (`BytecodeErrorKind::OriginPathsCountTooLarge` vs `Error::OriginPathsCountMismatch`) per spec F4.
- [x] `Error::ConflictingPathDeclarations` not introduced; `BytecodeErrorKind::UnexpectedTag` reused per spec F5.
- [x] `MAX_PATH_COMPONENTS = 10` cap applied uniformly to `Tag::SharedPath` and `Tag::OriginPaths` per Q8.
- [x] Existing `decode_path_round_trip_multi_byte_component_count` test rewrite addressed per F15.
- [x] Test corpus additions (3 positive + 6 negative + 4 hand-AST + 3 conformance).
- [x] BIP draft updates (5 new/updated subsections).
- [x] MIGRATION.md and CHANGELOG framing per spec §6.
- [x] Release dance (version bump, FOLLOWUPS, CLAUDE.md, PR, tag, release notes).

## Open implementer questions for plan-time decisions

1. **`md-signer-compat` version bump magnitude?** Decided in P7.1 audit; default expected: stays at current version (no public-API surface uses `BytecodeHeader::new_v0` directly).

2. **CLI rendering format for `PolicyId::fingerprint()`?** `0x{:08x}` is the natural choice. Defer to P5 implementer.

3. **Order of `o1`/`o2`/`o3` in vector enumeration?** Place after the v0.9 T1 vector. Defer to P4 implementer.

4. **Should we reuse `gen_vectors`'s existing infrastructure for the new origin-paths vectors, or add a new `--with-origin-paths` flag?** Reuse — the existing `EncodeOptions::with_origin_paths` builder is sufficient. Defer to P4.
