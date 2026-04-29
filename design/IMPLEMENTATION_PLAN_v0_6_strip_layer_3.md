# v0.6 Strip Layer 3 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Phase reviews dispatched to Opus reviewer subagents per the user's directive (`favor opus agents for non-mechanical tasks`); reports persisted to `design/agent-reports/`.

**Goal:** Strip MD's signer-compatibility curation layer (default `validate_tap_leaf_subset` invocation in encoder/decoder), reorganize the Tag enum, drop the `Reserved*` variant range, rename `Error::TapLeafSubsetViolation` → `Error::SubsetViolation`, expand corpus to lock newly-admitted shapes, rewrite BIP draft, and ship as v0.6.0.

**Architecture:** Single coordinated breaking release on `feature/v0.6-strip-layer-3`. The Tag rework changes byte values for many operators, so corpus regen is forced; the validator default-flip widens admit set to the full BIP 379 + BIP 388 surface. `validate_tap_leaf_subset` retained as `pub fn` for explicit-call use. Wire-format-additive (`Tag::SortedMultiA` allocated) and wire-format-subtractive (`Tag::Bare` + `Reserved*` dropped) in the same release.

**Test discipline:** Phase 1 follows TDD (failing tests first, then implementation). Phases 2/3 are *regen-and-verify* — the existing test suite (corpus round-trips, error_coverage gate, vectors_schema) catches regressions; adding per-arm unit tests to those phases would be defensive but is filed as a FOLLOWUPS nice-to-have rather than gating the v0.6 ship. Phase 4 is mechanical (sed substitution); Phase 5 introduces new corpus fixtures that *are* the test artifact; Phase 6+ are doc-only.

**Tech Stack:** Rust 1.85, miniscript pinned to `apoelstra/rust-miniscript` rev `f7f1689b...`, bitcoin 0.32, bech32 0.11, serde, indexmap.

**Spec reference:** `design/SPEC_v0_6_strip_layer_3.md` (round-1 review folded in; see `design/agent-reports/v0-6-spec-review-1.md`).

**Rationale doc:** `design/MD_SCOPE_DECISION_2026-04-28.md`.

---

## File structure

Files modified by this plan, with their post-strip responsibility:

| File | Responsibility | Phase |
|---|---|---|
| `crates/md-codec/src/bytecode/tag.rs` | Tag enum + `from_byte`/`as_byte` + tests; new layout per spec §2.2 | 1 |
| `crates/md-codec/src/bytecode/encode.rs` | Encoder; default validator call removed; `encode_tap_terminal` made exhaustive; `Tag::SortedMultiA` arm added; `validate_tap_leaf_subset` retained as `pub fn` | 2 |
| `crates/md-codec/src/bytecode/decode.rs` | Decoder; catch-all rejection removed; ~20 new tap-leaf arms added; `Tag::SortedMultiA` arm added; explicit `validate_tap_leaf_subset` calls removed at decode.rs:295 + decode.rs:802 | 3 |
| `crates/md-codec/src/error.rs` | `Error::TapLeafSubsetViolation` renamed to `Error::SubsetViolation` | 4 |
| `crates/md-codec/src/vectors.rs` | Corpus expansion: 18+ new positive vectors per spec §6.1 | 5 |
| `crates/md-codec/tests/error_coverage.rs` | EnumIter mirror table updated for `SubsetViolation` rename | 4 |
| `crates/md-codec/tests/{taproot,conformance,corpus}.rs` | Test fixtures audited; rejection-asserting tests flipped or removed where they assert now-admitted operators | 5 |
| `crates/md-codec/tests/vectors/v0.1.json`, `v0.2.json` | Regenerated via `gen_vectors --output` | 8 |
| `crates/md-codec/tests/vectors_schema.rs` | SHA pins updated to v0.6 values | 8 |
| `crates/md-codec/Cargo.toml` | Version 0.5.0 → 0.6.0 | 10 |
| `crates/md-codec/src/lib.rs` | Re-exports updated if any `Reserved*` were re-exported (none expected) | 4 |
| `crates/md-codec/src/bin/md/main.rs` | CLI help text updated per spec §8.3 | 7 |
| `bip/bip-mnemonic-descriptor.mediawiki` | BIP draft: §"Taproot tree" MUST→MAY, new §"Signer compatibility (informational)", Tag table updates per spec §7.3 | 6 |
| `README.md` + `crates/md-codec/README.md` | Recovery-responsibility framing per spec §8 | 7 |
| `CHANGELOG.md` | `[Unreleased]` → `[0.6.0]` consolidation per spec §11 | 9 |
| `MIGRATION.md` | `v0.5.x → v0.6.0` section extension per spec §9.1 (8 items) | 9 |

---

## Pre-Phase-0 — Setup verification (already done)

Sanity checks before starting; most already complete on the live branch.

- [x] **Step P0.1**: Feature branch `feature/v0.6-strip-layer-3` cut from main HEAD `93ac9ae` (commit `8e652b1`); pushed to GitHub.
- [x] **Step P0.2**: PR #2 open against main for visibility.
- [x] **Step P0.3**: Spec at `design/SPEC_v0_6_strip_layer_3.md` round-1 reviewed and revised (commit `8a0ac72`).
- [x] **Step P0.4**: Rationale doc at `design/MD_SCOPE_DECISION_2026-04-28.md` (commit `93ac9ae`).
- [x] **Step P0.5**: Forward-pointer added to Phase D agent report (commit `93ac9ae`).
- [x] **Step P0.6**: 7 strip FOLLOWUPS entries filed; 8 prior entries superseded (commit `dd38398`).

---

## Phase 1 — Tag enum rework

**Goal:** Rework `crates/md-codec/src/bytecode/tag.rs` to the v0.6 layout per spec §2.2/§2.3. Drop `Tag::Bare` and the 14 `Reserved*` variants. Allocate `Tag::SortedMultiA = 0x0B`. Renumber operators that move per the spec table.

**Files:**
- Modify: `crates/md-codec/src/bytecode/tag.rs`

### Task 1.1 — Write failing tests for the new Tag layout

**Files:**
- Modify: `crates/md-codec/src/bytecode/tag.rs` (test module at end of file)

- [ ] **Step 1.1.1: Write the new layout tests**

Add to the `#[cfg(test)] mod tests { ... }` block at the bottom of `tag.rs`:

```rust
#[test]
fn tag_v0_6_layout_top_level_descriptors() {
    assert_eq!(Tag::False.as_byte(), 0x00);
    assert_eq!(Tag::True.as_byte(), 0x01);
    assert_eq!(Tag::Pkh.as_byte(), 0x02);
    assert_eq!(Tag::Sh.as_byte(), 0x03);
    assert_eq!(Tag::Wpkh.as_byte(), 0x04);
    assert_eq!(Tag::Wsh.as_byte(), 0x05);
    assert_eq!(Tag::Tr.as_byte(), 0x06);
}

#[test]
fn tag_v0_6_layout_taptree_framing() {
    assert_eq!(Tag::TapTree.as_byte(), 0x07);
}

#[test]
fn tag_v0_6_layout_multisig_family_contiguous() {
    assert_eq!(Tag::Multi.as_byte(), 0x08);
    assert_eq!(Tag::SortedMulti.as_byte(), 0x09);
    assert_eq!(Tag::MultiA.as_byte(), 0x0A);
    assert_eq!(Tag::SortedMultiA.as_byte(), 0x0B);
}

#[test]
fn tag_v0_6_layout_wrappers() {
    assert_eq!(Tag::Alt.as_byte(), 0x0C);
    assert_eq!(Tag::Swap.as_byte(), 0x0D);
    assert_eq!(Tag::Check.as_byte(), 0x0E);
    assert_eq!(Tag::DupIf.as_byte(), 0x0F);
    assert_eq!(Tag::Verify.as_byte(), 0x10);
    assert_eq!(Tag::NonZero.as_byte(), 0x11);
    assert_eq!(Tag::ZeroNotEqual.as_byte(), 0x12);
}

#[test]
fn tag_v0_6_layout_logical() {
    assert_eq!(Tag::AndV.as_byte(), 0x13);
    assert_eq!(Tag::AndB.as_byte(), 0x14);
    assert_eq!(Tag::AndOr.as_byte(), 0x15);
    assert_eq!(Tag::OrB.as_byte(), 0x16);
    assert_eq!(Tag::OrC.as_byte(), 0x17);
    assert_eq!(Tag::OrD.as_byte(), 0x18);
    assert_eq!(Tag::OrI.as_byte(), 0x19);
    assert_eq!(Tag::Thresh.as_byte(), 0x1A);
}

#[test]
fn tag_v0_6_layout_keys_unchanged() {
    assert_eq!(Tag::PkK.as_byte(), 0x1B);
    assert_eq!(Tag::PkH.as_byte(), 0x1C);
    assert_eq!(Tag::RawPkH.as_byte(), 0x1D);
}

#[test]
fn tag_v0_6_layout_timelocks_unchanged() {
    assert_eq!(Tag::After.as_byte(), 0x1E);
    assert_eq!(Tag::Older.as_byte(), 0x1F);
}

#[test]
fn tag_v0_6_layout_hashes_unchanged() {
    assert_eq!(Tag::Sha256.as_byte(), 0x20);
    assert_eq!(Tag::Hash256.as_byte(), 0x21);
    assert_eq!(Tag::Ripemd160.as_byte(), 0x22);
    assert_eq!(Tag::Hash160.as_byte(), 0x23);
}

#[test]
fn tag_v0_6_layout_framing() {
    assert_eq!(Tag::Placeholder.as_byte(), 0x33);
    assert_eq!(Tag::SharedPath.as_byte(), 0x34);
    assert_eq!(Tag::Fingerprints.as_byte(), 0x35);
}

#[test]
fn tag_v0_6_unallocated_bytes() {
    // Reserved* range dropped (was 0x24-0x31); from_byte must return None
    for b in 0x24u8..=0x31 {
        assert!(
            Tag::from_byte(b).is_none(),
            "byte {b:#04x} should be unallocated in v0.6 (Reserved* range dropped)"
        );
    }
    // Byte 0x32 (formerly Placeholder) intentionally unallocated to surface
    // v0.5→v0.6 transcoder errors as clean from_byte=None
    assert!(
        Tag::from_byte(0x32).is_none(),
        "byte 0x32 should be unallocated in v0.6 (formerly Placeholder, intentionally vacant per spec §2.2)"
    );
    // 0x34 was already reserved-invalid pre-v0.6
    // (NB: in v0.6, 0x34 is now SharedPath since SharedPath moved up;
    // remove 0x34 from the unallocated list and put it in the SharedPath test.)
}

#[test]
fn tag_v0_6_high_bytes_unallocated() {
    // 0x36..=0xFF reserved (no change from v0.5)
    for b in 0x36u8..=0xFF {
        assert!(
            Tag::from_byte(b).is_none(),
            "byte {b:#04x} should be unallocated"
        );
    }
}

#[test]
fn tag_v0_6_round_trip_all_defined() {
    // All allocated bytes in v0.6: 0x00-0x23 (no gaps), 0x33-0x35.
    // Specifically NOT: 0x24-0x32 (Reserved* dropped + Bare dropped + Placeholder moved up).
    let v0_6_allocated: Vec<u8> = (0x00..=0x23).chain(0x33..=0x35).collect();
    for b in v0_6_allocated {
        let t = Tag::from_byte(b);
        assert!(t.is_some(), "byte {b:#04x} should be a valid v0.6 tag");
        assert_eq!(t.unwrap().as_byte(), b);
    }
}
```

- [ ] **Step 1.1.2: Run tests to verify they fail**

Run: `cargo test -p md-codec --lib bytecode::tag::tests::tag_v0_6 2>&1 | tail -30`

Expected: All `tag_v0_6_*` tests FAIL. Specifically: compile errors on `Tag::SortedMultiA` (variant doesn't exist yet), and runtime assertion failures on byte values that haven't been changed.

### Task 1.2 — Implement the new Tag layout

- [ ] **Step 1.2.1: Rewrite the Tag enum body**

Replace the entire `pub enum Tag { ... }` block in `crates/md-codec/src/bytecode/tag.rs` with:

```rust
/// Single-byte tag identifying an operator in the canonical bytecode.
///
/// v0.6 layout: descriptor-codec-vendored allocations dropped in favor
/// of a coherent grouping. `Reserved*` range 0x24-0x31 dropped entirely
/// (MD's BIP-388 framing forbids inline keys; see
/// `MD_SCOPE_DECISION_2026-04-28.md`). `Tag::Bare` dropped (never used
/// as inner; encoder rejects `Descriptor::Bare` via PolicyScopeViolation).
/// `Tag::SortedMultiA` (0x0B) NEW; needed for tap-context sorted multisig
/// shapes documented by Coldcard and Ledger.
///
/// Byte 0x32 is intentionally left unallocated to surface v0.5→v0.6
/// transcoder mistakes as clean `from_byte=None` rather than data
/// corruption (v0.5 emitted `Placeholder=0x32` in every encoded MD string).
///
/// Marked `#[non_exhaustive]` so adding new variants in v0.7+ does not
/// break downstream `match` consumers.
#[non_exhaustive]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tag {
    // Constants
    /// `0` — miniscript terminal always-false fragment.
    False = 0x00,
    /// `1` — miniscript terminal always-true fragment.
    True = 0x01,

    // Top-level descriptor wrappers
    /// `pkh(K)` — pay-to-pubkey-hash top-level descriptor.
    Pkh = 0x02,
    /// `sh(...)` — pay-to-script-hash top-level descriptor.
    Sh = 0x03,
    /// `wpkh(K)` — pay-to-witness-pubkey-hash top-level descriptor.
    Wpkh = 0x04,
    /// `wsh(...)` — pay-to-witness-script-hash top-level descriptor.
    Wsh = 0x05,
    /// `tr(...)` — taproot top-level descriptor.
    Tr = 0x06,

    // Tap-tree framing
    /// Taproot script tree inner-node framing (inside `tr(KEY, TREE)`).
    TapTree = 0x07,

    // Multisig family
    /// `multi(k, ...)` — k-of-n multisig (P2WSH-only by miniscript typing).
    Multi = 0x08,
    /// `sortedmulti(k, ...)` — sorted multisig (P2WSH-only by miniscript typing).
    SortedMulti = 0x09,
    /// `multi_a(k, ...)` — taproot k-of-n multisig (Tapscript-only by miniscript typing).
    MultiA = 0x0A,
    /// `sortedmulti_a(k, ...)` — taproot sorted multisig (Tapscript-only).
    /// NEW in v0.6.
    SortedMultiA = 0x0B,

    // Wrappers
    /// `a:` wrapper — toaltstack/fromaltstack.
    Alt = 0x0C,
    /// `s:` wrapper — swap.
    Swap = 0x0D,
    /// `c:` wrapper — checksig.
    Check = 0x0E,
    /// `d:` wrapper — dup-if.
    DupIf = 0x0F,
    /// `v:` wrapper — verify.
    Verify = 0x10,
    /// `j:` wrapper — non-zero.
    NonZero = 0x11,
    /// `n:` wrapper — zero-not-equal.
    ZeroNotEqual = 0x12,

    // Logical operators
    /// `and_v(X, Y)` — verify-and conjunction.
    AndV = 0x13,
    /// `and_b(X, Y)` — boolean-and conjunction.
    AndB = 0x14,
    /// `andor(X, Y, Z)` — if X then Y else Z.
    AndOr = 0x15,
    /// `or_b(X, Z)` — boolean-or disjunction.
    OrB = 0x16,
    /// `or_c(X, Z)` — or-continue disjunction.
    OrC = 0x17,
    /// `or_d(X, Z)` — or-dup disjunction.
    OrD = 0x18,
    /// `or_i(X, Z)` — or-if disjunction.
    OrI = 0x19,
    /// `thresh(k, ...)` — k-of-n threshold over fragments.
    Thresh = 0x1A,

    // Keys (byte values unchanged from v0.5)
    /// `pk_k(K)` — bare-key key script.
    PkK = 0x1B,
    /// `pk_h(K)` — keyhash key script.
    PkH = 0x1C,
    /// `pk_h(<20-byte hash>)` — raw-pubkeyhash key script.
    RawPkH = 0x1D,

    // Timelocks (byte values unchanged from v0.5)
    /// `after(n)` — absolute timelock.
    After = 0x1E,
    /// `older(n)` — relative timelock.
    Older = 0x1F,

    // Hashes (byte values unchanged from v0.5)
    /// `sha256(h)` — SHA-256 preimage commitment.
    Sha256 = 0x20,
    /// `hash256(h)` — double-SHA-256 preimage commitment.
    Hash256 = 0x21,
    /// `ripemd160(h)` — RIPEMD-160 preimage commitment.
    Ripemd160 = 0x22,
    /// `hash160(h)` — RIPEMD-160 of SHA-256 preimage commitment.
    Hash160 = 0x23,

    // Reserved (0x24-0x31): DROPPED in v0.6 — see crate-level rationale.
    // Byte 0x32: DROPPED — was v0.5 Placeholder; intentionally unallocated.

    // MD-specific framing (Placeholder + SharedPath bytes shifted +1 from v0.5
    // to leave 0x32 unallocated; Fingerprints byte unchanged from v0.5)
    /// MD extension: BIP 388 key placeholder (`@i/<a;b>/*`).
    Placeholder = 0x33,
    /// MD extension: shared-path declaration for placeholder framing.
    SharedPath = 0x34,
    /// MD extension: fingerprints block (Phase E v0.2).
    ///
    /// Byte value preserved across v0.5→v0.6 for wire-format continuity
    /// of the v0.2-shipped fingerprints framing.
    Fingerprints = 0x35,
}
```

- [ ] **Step 1.2.2: Rewrite the `from_byte` match**

Replace the `pub fn from_byte(b: u8) -> Option<Self>` body with:

```rust
pub fn from_byte(b: u8) -> Option<Self> {
    match b {
        // Constants
        0x00 => Some(Tag::False),
        0x01 => Some(Tag::True),
        // Top-level descriptor wrappers
        0x02 => Some(Tag::Pkh),
        0x03 => Some(Tag::Sh),
        0x04 => Some(Tag::Wpkh),
        0x05 => Some(Tag::Wsh),
        0x06 => Some(Tag::Tr),
        // Tap-tree framing
        0x07 => Some(Tag::TapTree),
        // Multisig family
        0x08 => Some(Tag::Multi),
        0x09 => Some(Tag::SortedMulti),
        0x0A => Some(Tag::MultiA),
        0x0B => Some(Tag::SortedMultiA),
        // Wrappers
        0x0C => Some(Tag::Alt),
        0x0D => Some(Tag::Swap),
        0x0E => Some(Tag::Check),
        0x0F => Some(Tag::DupIf),
        0x10 => Some(Tag::Verify),
        0x11 => Some(Tag::NonZero),
        0x12 => Some(Tag::ZeroNotEqual),
        // Logical operators
        0x13 => Some(Tag::AndV),
        0x14 => Some(Tag::AndB),
        0x15 => Some(Tag::AndOr),
        0x16 => Some(Tag::OrB),
        0x17 => Some(Tag::OrC),
        0x18 => Some(Tag::OrD),
        0x19 => Some(Tag::OrI),
        0x1A => Some(Tag::Thresh),
        // Keys
        0x1B => Some(Tag::PkK),
        0x1C => Some(Tag::PkH),
        0x1D => Some(Tag::RawPkH),
        // Timelocks
        0x1E => Some(Tag::After),
        0x1F => Some(Tag::Older),
        // Hashes
        0x20 => Some(Tag::Sha256),
        0x21 => Some(Tag::Hash256),
        0x22 => Some(Tag::Ripemd160),
        0x23 => Some(Tag::Hash160),
        // 0x24-0x32: unallocated (Reserved* dropped, Bare dropped, Placeholder moved)
        // MD-specific framing
        0x33 => Some(Tag::Placeholder),
        0x34 => Some(Tag::SharedPath),
        0x35 => Some(Tag::Fingerprints),
        // 0x36-0xFF: reserved
        _ => None,
    }
}
```

- [ ] **Step 1.2.3: Update the existing `tag_round_trip_all_defined` test**

The original test asserted `0x00..=0x33` are all valid plus 0x35. Update to assert v0.6's actual allocation pattern. Replace the existing test:

```rust
#[test]
fn tag_round_trip_all_defined() {
    // v0.6 allocation: 0x00-0x23 contiguous, then 0x33-0x35.
    // Gap: 0x24-0x32 (Reserved* dropped + Bare dropped + Placeholder moved up by 1).
    let v0_6_allocated: Vec<u8> = (0x00..=0x23).chain(0x33..=0x35).collect();
    for b in v0_6_allocated {
        let t = Tag::from_byte(b);
        assert!(t.is_some(), "byte {b:#04x} should be a valid v0.6 tag");
        assert_eq!(t.unwrap().as_byte(), b);
    }
}
```

- [ ] **Step 1.2.4: Update the existing `tag_rejects_unknown_bytes` test**

Replace with:

```rust
#[test]
fn tag_rejects_unknown_bytes() {
    // 0x24-0x31: formerly Reserved* range (dropped in v0.6)
    // 0x32: formerly Placeholder (dropped in v0.6 to surface transcoder errors)
    // 0x36-0xFF: reserved
    for b in 0x24u8..=0x32 {
        assert!(
            Tag::from_byte(b).is_none(),
            "byte {b:#04x} should be rejected (v0.6 unallocated)"
        );
    }
    for b in 0x36u8..=0xFF {
        assert!(
            Tag::from_byte(b).is_none(),
            "byte {b:#04x} should be rejected (high range reserved)"
        );
    }
}
```

- [ ] **Step 1.2.5: Update the existing `tag_specific_values` test**

Replace with v0.6 values:

```rust
#[test]
fn tag_specific_values() {
    assert_eq!(Tag::Wsh.as_byte(), 0x05);
    assert_eq!(Tag::PkK.as_byte(), 0x1B);
    assert_eq!(Tag::Sha256.as_byte(), 0x20);
    assert_eq!(Tag::Placeholder.as_byte(), 0x33);
    assert_eq!(Tag::SharedPath.as_byte(), 0x34);
    assert_eq!(Tag::Fingerprints.as_byte(), 0x35);
    // v0.6 NEW
    assert_eq!(Tag::SortedMultiA.as_byte(), 0x0B);
    assert_eq!(Tag::TapTree.as_byte(), 0x07);
}
```

- [ ] **Step 1.2.6: Run the new tests + verify pass**

Run: `cargo test -p md-codec --lib bytecode::tag 2>&1 | tail -20`

Expected: `tag_v0_6_*` tests + the updated 3 original tests all PASS. Test count should be 13 (10 new + 3 updated).

The rest of the crate WILL fail to compile at this point because encode.rs / decode.rs reference `Tag::Bare` and `Tag::Reserved*` variants that no longer exist. That's expected — Phase 2/3 fix it.

- [ ] **Step 1.2.7: Commit**

Run:
```bash
git add crates/md-codec/src/bytecode/tag.rs
git commit -m "feat(v0.6 phase 1): rework Tag enum to v0.6 layout"
```

The commit message body (multi-line via heredoc):

```bash
git commit -m "$(cat <<'EOF'
feat(v0.6 phase 1): rework Tag enum to v0.6 layout

- Drop Tag::Bare entirely (never used as inner; encoder rejects via PSV)
- Drop 14 Reserved* variants (0x24-0x31; descriptor-codec inline-key
  vendoring incompatible with MD's BIP-388 wallet-policy scope)
- Allocate Tag::SortedMultiA = 0x0B (Coldcard/Ledger documented)
- Reorganize: TapTree=0x07 (adjacent to Tr); multisig family contiguous
  at 0x08-0x0B; wrappers shift to 0x0C-0x12; logical ops to 0x13-0x1A
- Placeholder 0x32→0x33; SharedPath 0x33→0x34; byte 0x32 intentionally
  unallocated to surface v0.5→v0.6 transcoder errors
- Fingerprints=0x35 byte preserved for v0.2-shipped fingerprints framing

The crate will not compile after this commit until phases 2/3 update
encoder/decoder references to dropped variants; that is expected.

Spec reference: design/SPEC_v0_6_strip_layer_3.md §2.2/§2.3.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 1.3 — Phase 1 review

- [ ] **Step 1.3.1: Dispatch Opus reviewer on Phase 1 commit**

Use the Agent tool with `feature-dev:code-reviewer`, `model: opus`. Brief:
- File: `crates/md-codec/src/bytecode/tag.rs` after Phase 1 commit.
- Verify: layout matches spec §2.2 byte-for-byte; from_byte match is exhaustive over v0.6 allocations; all dropped variants (Bare, Reserved*) genuinely absent; rustdoc accurately describes the rework.
- Output: persist report to `design/agent-reports/v0-6-phase-1-review.md`.

- [ ] **Step 1.3.2: Address reviewer findings**

Critical/important inline; nits to FOLLOWUPS.

---

## Phase 2 — Encoder strip + new arms

**Goal:** Update `crates/md-codec/src/bytecode/encode.rs` to compile against the new Tag layout, remove the default-path `validate_tap_leaf_subset` call, make `encode_tap_terminal` exhaustive (option (a) per spec §3.2), add `Tag::SortedMultiA` arm, and update internal references to dropped Tag variants.

**Files:**
- Modify: `crates/md-codec/src/bytecode/encode.rs`

**Cross-cutting impact:** the encoder currently won't compile because `Tag::Bare` and `Tag::Reserved*` are referenced. This phase makes it compile + adopts the strip behavior.

### Task 2.0 — Add `BytecodeErrorKind::TagInvalidContext` variant (IMP-7 pre-decision)

Pre-pinned per plan review IMP-7 to avoid catch-all error-kind rework downstream. The new variant is used by Phase 3's decoder catch-all and by Phase 5's negative-vector audit (per CRIT-3 audit table).

**Files:**
- Modify: `crates/md-codec/src/error.rs` (add the variant)
- Modify: `crates/md-codec/tests/error_coverage.rs` (add to mirror)

- [ ] **Step 2.0.1: Add the variant to BytecodeErrorKind**

Locate the `pub enum BytecodeErrorKind { ... }` block in `crates/md-codec/src/error.rs`. Add:

```rust
/// A Tag valid in some context appears where it is not allowed.
///
/// E.g., a top-level descriptor tag (`Tag::Wsh`) where a tap-leaf
/// inner is expected. Distinct from `UnknownTag` (no Tag exists for
/// that byte) and `PolicyScopeViolation` (top-level admit decision).
TagInvalidContext {
    /// The tag byte that was structurally invalid in this context.
    tag: u8,
    /// Human-readable context name (e.g., "tap-leaf-inner",
    /// "wsh-inner").
    context: &'static str,
},
```

Place adjacent to `UnknownTag` for related-error grouping. Update the `Display` impl to format the new variant cleanly.

- [ ] **Step 2.0.2: Add to error_coverage mirror**

In `crates/md-codec/tests/error_coverage.rs`, add the variant to the BytecodeErrorKind exhaustiveness mirror (or to the test that derives expected names).

- [ ] **Step 2.0.3: Compile**

Run: `cargo check -p md-codec --tests 2>&1 | tail -5`

Expected: clean.

- [ ] **Step 2.0.4: Commit**

```bash
git add crates/md-codec/src/error.rs crates/md-codec/tests/error_coverage.rs
git commit -m "$(cat <<'EOF'
feat(v0.6 phase 2.0): add BytecodeErrorKind::TagInvalidContext variant

Pre-pinned per plan review IMP-7 to avoid catch-all error-kind rework
downstream. Used by Phase 3's decoder catch-all (Tag valid elsewhere
but not as a tap-leaf inner) and by Phase 5's negative-vector audit
table.

Variant shape: TagInvalidContext { tag: u8, context: &'static str }.
Distinct from UnknownTag (no Tag exists) and PolicyScopeViolation
(top-level admit).

error_coverage mirror updated.

Spec reference: design/SPEC_v0_6_strip_layer_3.md §4 (decoder strip).
Plan review reference: design/agent-reports/v0-6-plan-review-1.md IMP-7.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 2.1 — Audit encoder for dropped Tag references

- [ ] **Step 2.1.1: Find all Tag::Bare and Tag::Reserved* references in encode.rs**

Run: `grep -nE "Tag::(Bare|Reserved)" crates/md-codec/src/bytecode/encode.rs`

Document each hit with its purpose (top-level rejection check, validator, etc.) before editing. Most should be inside `Descriptor::Bare` rejection paths or symmetric structural-reject patterns.

### Task 2.2 — Remove default-path validator call

- [ ] **Step 2.2.1: Find the call site**

Run: `grep -nB2 -A2 "validate_tap_leaf_subset" crates/md-codec/src/bytecode/encode.rs | head -30`

The default-path call is in the `EncodeTemplate for Miniscript<DescriptorPublicKey, Tap>` impl, around the existing `encode_template` body. Identify the exact line to remove.

- [ ] **Step 2.2.2: Remove the call**

Replace the body of `EncodeTemplate for Miniscript<DescriptorPublicKey, Tap>::encode_template` so it no longer calls `validate_tap_leaf_subset`. The call to `encode_tap_terminal` (or whichever helper) remains, but the validator pre-check goes.

- [ ] **Step 2.2.3: Update rustdoc**

The function rustdoc currently says something like "validates against the per-leaf miniscript subset". Update to reflect that v0.6 does not validate by default; explicit-call validation is via `validate_tap_leaf_subset` retained as `pub fn`.

### Task 2.3 — Update `encode_tap_terminal` to exhaustive match

- [ ] **Step 2.3.1: Audit current arms**

Run: `grep -nE "Terminal::|=> \\{" crates/md-codec/src/bytecode/encode.rs | grep -v test | head -50`

Identify the existing `encode_tap_terminal` (or similar) function. List the current Terminal arms. The catch-all returns `TapLeafSubsetViolation`.

- [ ] **Step 2.3.2: Replace catch-all with exhaustive match**

The exhaustive match must cover **every** `Terminal<DescriptorPublicKey, Tap>` variant per spec §4.3's Add/Keep checklist (28 arms total). For each, emit the corresponding Tag byte and the operator-specific payload.

For previously-rejected variants, the new arms emit Tag bytes following the patterns established in `decode_terminal` (the Segwitv0 dispatcher) and the existing tap-context arms. Specifically:

- `Terminal::SortedMultiA(thresh)` — emit `Tag::SortedMultiA` byte, then `[k][n][key_1]...[key_n]` like `MultiA`. See `Terminal::MultiA` arm for the existing pattern.
- `Terminal::AndB(x, y)`, `Terminal::OrB(x, z)`, `Terminal::OrC(x, z)`, `Terminal::OrI(x, z)` — emit Tag byte, then encode each child. See `Terminal::AndV` arm for the binary pattern.
- `Terminal::AndOr(x, y, z)` — emit Tag byte, then encode three children.
- `Terminal::Thresh(thresh)` — emit Tag byte, then `[k][n]` then each `X_i`.
- `Terminal::Sha256(h)`, `Terminal::Hash256(h)`, `Terminal::Ripemd160(h)`, `Terminal::Hash160(h)` — emit Tag byte, then `h.as_byte_array()` (internal byte order per spec §6.3).
- `Terminal::After(lock)` — emit Tag byte, then varint of the lock value.
- `Terminal::Alt(x)`, `Terminal::Swap(x)`, `Terminal::DupIf(x)`, `Terminal::NonZero(x)`, `Terminal::ZeroNotEqual(x)` — emit Tag byte, then encode child (single recursive child).
- `Terminal::True`, `Terminal::False` — emit Tag byte; no payload.
- `Terminal::RawPkH(hash)` — emit Tag byte, then 20-byte hash.
- `Terminal::Multi(thresh)`, `Terminal::SortedMulti(thresh)` — tap-illegal but exhaustiveness requires arms; emit Tag byte + payload anyway per spec §3.2 option (a). Comment indicates miniscript's parser is the upstream gate.

The exact payload encoding for each variant must match the pattern in the Segwitv0 dispatcher (`crates/md-codec/src/bytecode/encode.rs` `EncodeTemplate for Miniscript<_, Segwitv0>`'s `encode_template` body). When in doubt, use the same byte sequence as the Segwitv0 path.

- [ ] **Step 2.3.3: Reference encoder commentary**

Preserve the existing `// hash terminal byte order: internal, NOT reversed-display` comment at encode.rs:316-319 (or wherever it currently sits) — it's invariant from v0.5 to v0.6 and locks the byte-order interpretation for Hash256.

### Task 2.4 — Audit and update Bare-rejection sites

- [ ] **Step 2.4.1: Find Descriptor::Bare rejection paths**

Run: `grep -nB2 -A3 "Descriptor::Bare\|Bare =>" crates/md-codec/src/bytecode/encode.rs`

Identify each spot. Most should be top-level dispatch arms returning `PolicyScopeViolation` with a "permanently rejected" message.

- [ ] **Step 2.4.2: Update arm to not reference `Tag::Bare`**

Wherever the rejection path emitted `Tag::Bare.as_byte()` (if any), replace with the rejection logic that doesn't use the dropped variant. The rejection itself stays — only the Tag reference goes.

If the encoder previously produced bytecode containing `Tag::Bare = 0x07` for any input shape, audit whether that path is reachable. If it was unreachable (rejected upstream), removing the byte emission is safe. If it was reachable, the rejection must produce a different error variant (likely `PolicyScopeViolation`).

### Task 2.5 — Reduce `validate_tap_leaf_subset` to pub fn for explicit-call use

- [ ] **Step 2.5.1: Verify function signature stays public**

The function should already be `pub fn validate_tap_leaf_subset(...)`. Confirm. Update rustdoc to reflect the new role:

```rust
/// Validate a tap-leaf miniscript against the historical Coldcard subset
/// (`pk_k`/`pk_h`/`multi_a`/`or_d`/`and_v`/`older` plus `c:`/`v:` wrappers).
///
/// **v0.6 note:** this function is no longer called by the encoder/decoder
/// default paths. It is retained as a public API for callers (typically
/// the layered `md-signer-compat` crate or an explicit pre-encode check)
/// who want signer-aware validation. The historical "encode rejects
/// out-of-subset operators" guarantee from v0.5 is GONE in v0.6 — see
/// `design/MD_SCOPE_DECISION_2026-04-28.md` for rationale.
///
/// On success returns `Ok(())`. On failure returns
/// [`Error::SubsetViolation`] (renamed from `TapLeafSubsetViolation` in
/// v0.6) carrying the offending operator name and optional leaf index.
pub fn validate_tap_leaf_subset(
    ms: &Miniscript<DescriptorPublicKey, Tap>,
    leaf_index: Option<usize>,
) -> Result<(), Error> {
    // body unchanged
}
```

`validate_tap_leaf_terminal` similarly stays pub (or `pub(crate)` if appropriate).

`tap_terminal_name` rustdoc updates to note its narrowed role:

```rust
/// Human-readable name for a tap-context Terminal variant, used in
/// `Error::SubsetViolation` messages produced by the explicit-call
/// `validate_tap_leaf_subset` path. Delegates to `tag_to_bip388_name`
/// for byte-identical encode/decode-side diagnostic equivalence.
///
/// **v0.6 note:** no longer the universal naming hook for tap-context
/// errors — only used by the retained `validate_tap_leaf_subset` `pub fn`.
fn tap_terminal_name(...) -> &'static str { ... }
```

### Task 2.6 — Compile and run encoder tests

- [ ] **Step 2.6.1: cargo check**

Run: `cargo check -p md-codec 2>&1 | tail -20`

Expected: compile errors should be down to **decoder-side issues only** (decode.rs still references dropped variants, addressed in Phase 3). Encoder should compile clean.

If encoder compile errors remain, fix them — they indicate the exhaustive match isn't actually exhaustive or a Bare/Reserved* reference was missed.

- [ ] **Step 2.6.2: Run encoder unit tests**

Run: `cargo test -p md-codec --lib bytecode::encode 2>&1 | tail -40`

Expected: many tests will FAIL because they assert specific bytecode that's now wrong (wrappers shifted by 2, etc.). That's expected — Phase 5 (corpus regen) is where we re-baseline. For now, audit the failures to ensure they're byte-value mismatches and not structural failures.

If any test fails because of a *missing arm* (panic or unreachable) that's a Phase 2 bug — fix before proceeding.

- [ ] **Step 2.6.3: Commit**

```bash
git add crates/md-codec/src/bytecode/encode.rs
git commit -m "$(cat <<'EOF'
feat(v0.6 phase 2): strip encoder default validator + exhaustive match

- Remove default-path validate_tap_leaf_subset call in
  EncodeTemplate for Miniscript<_, Tap>
- encode_tap_terminal becomes exhaustive (option (a) per spec §3.2):
  arms for every Terminal<_, Tap> variant including tap-illegal
  Multi/SortedMulti (miniscript parser is upstream gate)
- New SortedMultiA arm; new arms for Sha256/Hash256/Ripemd160/Hash160,
  After, AndB, AndOr, OrB, OrC, OrI, Thresh, Alt, Swap, DupIf, NonZero,
  ZeroNotEqual, RawPkH, True, False (~20 new arms total)
- Drop Tag::Bare references; Bare rejection routes via
  PolicyScopeViolation (unchanged behavior)
- validate_tap_leaf_subset retained as pub fn for explicit-call use;
  rustdoc clarifies the narrowed role
- tap_terminal_name rustdoc clarifies it's now only used by the
  explicit-call validator path

The crate still won't compile after this commit until phase 3 updates
the decoder.

Spec reference: design/SPEC_v0_6_strip_layer_3.md §3.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 2.7 — Phase 2 review

- [ ] **Step 2.7.1: Dispatch Opus reviewer**

Same pattern as Phase 1. Brief (expanded per plan review IMP-5):

- Files: `crates/md-codec/src/bytecode/encode.rs` after Phase 2 commit.
- Verify the basics:
  - Encoder admits the spec §4.3 admit set (every Terminal variant gets a Tag emit).
  - Exhaustive match has all 30 Terminal arms (no fallback wildcard).
  - No `Tag::Bare` or `Tag::Reserved*` references remain.
- Verify the option (a) decision specifically (per spec §3.2):
  - Tap-illegal `Multi`/`SortedMulti` arms have appropriate `// tap-illegal but exhaustive ...` comments.
  - Each arm emits the Tag byte unconditionally — no rejection logic in tap-illegal arms.
- Cross-check Tag byte emissions against the v0.6 layout (read `tag.rs` from Phase 1 commit):
  - New Hash256 arm emits `Tag::Hash256.as_byte() == 0x21`.
  - New SortedMultiA arm emits `Tag::SortedMultiA.as_byte() == 0x0B`.
  - All wrapper arms emit the renumbered v0.6 wrapper bytes (`Alt=0x0C`, etc.).
- Verify hash terminal byte order matches spec §6.3:
  - Encoder uses `h.as_byte_array()` directly for all four hash terminals.
  - NOT swapped to `h.to_byte_array()` or display-order encoding.
  - Reference comment at encode.rs:316-319 (or wherever it migrates after the rework) preserved.
- Verify rustdoc updates:
  - `validate_tap_leaf_subset` rustdoc clarifies "v0.6 note: no longer called by encoder/decoder default paths" per spec §3.2.
  - `tap_terminal_name` rustdoc clarifies "no longer the universal naming hook for tap-context errors — only used by the explicit-call validator path".
  - `validate_tap_leaf_subset` BODY is unchanged (only its rustdoc updates) — tempting to mistake "retain pub" as license to refactor.

- Output: `design/agent-reports/v0-6-phase-2-review.md`.

- [ ] **Step 2.7.2: Address findings**

Critical/important inline; nits to FOLLOWUPS.

---

## Phase 3 — Decoder strip + new arms

**Goal:** Update `crates/md-codec/src/bytecode/decode.rs` similarly: 20 new arms in `decode_tap_terminal`, removal of catch-all rejection arm and the explicit `validate_tap_leaf_subset` calls at decode.rs:295 + decode.rs:802. New `Tag::SortedMultiA` arm. Update Bare rejection paths.

**Files:**
- Modify: `crates/md-codec/src/bytecode/decode.rs`

### Task 3.1 — Audit decoder for dropped Tag references

- [ ] **Step 3.1.1: Find all references**

Run: `grep -nE "Tag::(Bare|Reserved)" crates/md-codec/src/bytecode/decode.rs`

Categorize each hit (top-level rejection, inner-rejection, debug-name lookup, etc.).

### Task 3.2 — Add 20 new arms to `decode_tap_terminal`

- [ ] **Step 3.2.1: Locate decode_tap_terminal**

Run: `grep -n "fn decode_tap_terminal" crates/md-codec/src/bytecode/decode.rs`

The function (per spec §4.3) currently covers 8 in-Tag-set arms (PkK, PkH, MultiA, Older, AndV, OrD, Check, Verify) + a defensive TapTree rejection + a catch-all returning TapLeafSubsetViolation.

- [ ] **Step 3.2.2: Add the 20 new arms in canonical order**

For each new arm in spec §4.3's Add column, write the arm. The implementations follow the patterns in the Segwitv0 dispatcher `decode_terminal` (decode.rs:324-583) but:
- Recurse via `decode_tap_miniscript` instead of `decode_miniscript`
- Construct `Terminal<_, Tap>` instead of `Terminal<_, Segwitv0>`
- Cite the spec section in arm-level comments where the byte format is non-obvious (e.g., the Hash256 internal-byte-order note).

Specific arms (in spec-order):

```rust
// --- New arms added in Phase 3 (v0.6) per spec §4.3 ---
Tag::False => Terminal::False,
Tag::True => Terminal::True,

Tag::RawPkH => {
    let hash = bitcoin::hashes::hash160::Hash::from_byte_array(cur.read_array::<20>()?);
    Terminal::RawPkH(hash)
}

Tag::Multi => {
    // Tap-illegal by miniscript typing, but the encoder's exhaustive match
    // emits this byte if hand-built ASTs reach it. Decode preserves
    // round-trip symmetry; miniscript's type system rejects downstream.
    let k = cur.read_byte()?;
    let n = cur.read_byte()?;
    let mut keys = Vec::with_capacity(n as usize);
    for _ in 0..n {
        keys.push(decode_descriptor_public_key(cur, keys_in_vector)?);
    }
    let thresh = Threshold::new(k as usize, keys)
        .map_err(|e| Error::InvalidBytecode { ... })?;
    Terminal::Multi(thresh)
}

Tag::SortedMulti => {
    // Same shape and tap-illegal note as Multi.
    let k = cur.read_byte()?;
    let n = cur.read_byte()?;
    let mut keys = Vec::with_capacity(n as usize);
    for _ in 0..n {
        keys.push(decode_descriptor_public_key(cur, keys_in_vector)?);
    }
    let thresh = Threshold::new(k as usize, keys)
        .map_err(|e| Error::InvalidBytecode { ... })?;
    Terminal::SortedMulti(thresh)
}

Tag::SortedMultiA => {
    // NEW in v0.6. Same shape as MultiA.
    let k = cur.read_byte()?;
    let n = cur.read_byte()?;
    let mut keys = Vec::with_capacity(n as usize);
    for _ in 0..n {
        keys.push(decode_descriptor_public_key(cur, keys_in_vector)?);
    }
    let thresh = Threshold::new(k as usize, keys)
        .map_err(|e| Error::InvalidBytecode { ... })?;
    Terminal::SortedMultiA(thresh)
}

Tag::Alt => {
    let child = decode_tap_miniscript(cur, keys_in_vector, leaf_index)?;
    Terminal::Alt(Arc::new(child))
}
Tag::Swap => {
    let child = decode_tap_miniscript(cur, keys_in_vector, leaf_index)?;
    Terminal::Swap(Arc::new(child))
}
Tag::DupIf => {
    let child = decode_tap_miniscript(cur, keys_in_vector, leaf_index)?;
    Terminal::DupIf(Arc::new(child))
}
Tag::NonZero => {
    let child = decode_tap_miniscript(cur, keys_in_vector, leaf_index)?;
    Terminal::NonZero(Arc::new(child))
}
Tag::ZeroNotEqual => {
    let child = decode_tap_miniscript(cur, keys_in_vector, leaf_index)?;
    Terminal::ZeroNotEqual(Arc::new(child))
}

Tag::AndB => {
    let x = decode_tap_miniscript(cur, keys_in_vector, leaf_index)?;
    let y = decode_tap_miniscript(cur, keys_in_vector, leaf_index)?;
    Terminal::AndB(Arc::new(x), Arc::new(y))
}
Tag::AndOr => {
    let x = decode_tap_miniscript(cur, keys_in_vector, leaf_index)?;
    let y = decode_tap_miniscript(cur, keys_in_vector, leaf_index)?;
    let z = decode_tap_miniscript(cur, keys_in_vector, leaf_index)?;
    Terminal::AndOr(Arc::new(x), Arc::new(y), Arc::new(z))
}
Tag::OrB => {
    let x = decode_tap_miniscript(cur, keys_in_vector, leaf_index)?;
    let z = decode_tap_miniscript(cur, keys_in_vector, leaf_index)?;
    Terminal::OrB(Arc::new(x), Arc::new(z))
}
Tag::OrC => {
    let x = decode_tap_miniscript(cur, keys_in_vector, leaf_index)?;
    let z = decode_tap_miniscript(cur, keys_in_vector, leaf_index)?;
    Terminal::OrC(Arc::new(x), Arc::new(z))
}
Tag::OrI => {
    let x = decode_tap_miniscript(cur, keys_in_vector, leaf_index)?;
    let z = decode_tap_miniscript(cur, keys_in_vector, leaf_index)?;
    Terminal::OrI(Arc::new(x), Arc::new(z))
}
Tag::Thresh => {
    let k = cur.read_byte()?;
    let n = cur.read_byte()?;
    let mut subs = Vec::with_capacity(n as usize);
    for _ in 0..n {
        subs.push(Arc::new(decode_tap_miniscript(cur, keys_in_vector, leaf_index)?));
    }
    let thresh = Threshold::new(k as usize, subs).map_err(|e| Error::InvalidBytecode { ... })?;
    Terminal::Thresh(thresh)
}

Tag::After => {
    let v = cur.read_varint_u64()?;
    let v32 = u32::try_from(v).map_err(|_| Error::InvalidBytecode { ... })?;
    let lock = miniscript::AbsLockTime::from_consensus(v32).map_err(|e| Error::InvalidBytecode { ... })?;
    Terminal::After(lock)
}

Tag::Sha256 => {
    let bytes = cur.read_array::<32>()?;
    // Internal byte order; see encode.rs:316-319 / spec §6.3
    Terminal::Sha256(bitcoin::hashes::sha256::Hash::from_byte_array(bytes))
}
Tag::Hash256 => {
    let bytes = cur.read_array::<32>()?;
    // Internal byte order, NOT reversed-display-order; see encode.rs:316-319 / spec §6.3
    Terminal::Hash256(miniscript::hash256::Hash::from_byte_array(bytes))
}
Tag::Ripemd160 => {
    let bytes = cur.read_array::<20>()?;
    Terminal::Ripemd160(bitcoin::hashes::ripemd160::Hash::from_byte_array(bytes))
}
Tag::Hash160 => {
    let bytes = cur.read_array::<20>()?;
    Terminal::Hash160(bitcoin::hashes::hash160::Hash::from_byte_array(bytes))
}

// --- Catch-all: structural rejection via TagInvalidContext ---
// In v0.6 the catch-all handles Tags valid in some context but not
// as a tap-leaf inner (e.g., a top-level descriptor tag like
// Tag::Wsh = 0x05 appearing where a tap-leaf inner is expected).
// Use the new BytecodeErrorKind::TagInvalidContext variant introduced
// for this purpose (see Step 3.0 below).
_ => {
    return Err(Error::InvalidBytecode {
        offset: tag_offset,
        kind: BytecodeErrorKind::TagInvalidContext {
            tag: tag.as_byte(),
            context: "tap-leaf-inner",
        },
    });
}
```

The new `BytecodeErrorKind::TagInvalidContext { tag: u8, context: &'static str }` variant is added in Step 3.0 (below) before adding the new tap-leaf arms. Decision pre-pinned per plan review IMP-7 to avoid cascading rework.

The exact `Threshold::new` error wrapping pattern is preserved from existing arms (look at the existing MultiA arm and copy its error-mapping shape).

### Task 3.3 — Remove explicit validate_tap_leaf_subset calls

- [ ] **Step 3.3.1: Find and remove decode.rs:295 call**

This is the single-leaf path's post-AST-reconstruction call. The line was added in Phase D. Removing it makes the single-leaf path admit any in-Tag-set Terminal.

- [ ] **Step 3.3.2: Find and remove decode.rs:802 call**

The multi-leaf path's per-leaf call (added in v0.5). Same treatment.

- [ ] **Step 3.3.3: Update surrounding comments**

The block comments referencing "the per-leaf subset gate" need updating to note that v0.6 removed the gate.

### Task 3.4 — Update Bare rejection paths

- [ ] **Step 3.4.1: Find Tag::Bare references**

Run: `grep -nB2 -A3 "Tag::Bare" crates/md-codec/src/bytecode/decode.rs`

Each top-level dispatch arm matching `Tag::Bare` to produce a permanent rejection becomes either a wildcard arm (since `Tag::Bare` no longer exists) or relies on `Tag::from_byte` returning `Some(other)` for byte 0x07 (now `Tag::TapTree`).

- [ ] **Step 3.4.2: Update tag_to_bip388_name**

Find the `Tag::Bare => "bare"` arm in `tag_to_bip388_name` (decode.rs:822 per the review report). Remove it — `Tag::Bare` no longer exists.

### Task 3.5 — Compile + test

- [ ] **Step 3.5.1: cargo check**

Run: `cargo check -p md-codec 2>&1 | tail -20`

Expected: clean compile (encoder + decoder both updated).

- [ ] **Step 3.5.2: Run unit tests**

Run: `cargo test -p md-codec --lib 2>&1 | tail -50`

Expected: many byte-value-pinning tests fail (corpus regen pending in Phase 5/8). Structural and round-trip tests should pass at the AST level.

- [ ] **Step 3.5.3: Commit**

```bash
git add crates/md-codec/src/bytecode/decode.rs
git commit -m "$(cat <<'EOF'
feat(v0.6 phase 3): strip decoder default validator + new tap-leaf arms

- Add 20 new arms to decode_tap_terminal per spec §4.3:
  False, True, RawPkH, Multi, SortedMulti, SortedMultiA (NEW),
  Alt, Swap, DupIf, NonZero, ZeroNotEqual,
  AndB, AndOr, OrB, OrC, OrI, Thresh, After,
  Sha256, Hash256, Ripemd160, Hash160
- Remove the catch-all TapLeafSubsetViolation arm; replace with a
  structural InvalidBytecode catch-all for "Tag valid elsewhere
  but not as a tap-leaf" cases
- Remove explicit validate_tap_leaf_subset calls at the single-leaf
  decode path (formerly decode.rs:295) and the multi-leaf per-leaf
  path (formerly decode.rs:802)
- Update Tag::Bare references; remove Tag::Bare arm from
  tag_to_bip388_name
- Update block comments to note v0.6's gate-removal

Spec reference: design/SPEC_v0_6_strip_layer_3.md §4.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 3.6 — Phase 3 review

- [ ] **Step 3.6.1: Dispatch Opus reviewer**

Brief:
- Files: `crates/md-codec/src/bytecode/decode.rs` after Phase 3 commit.
- Verify: 28 total arms in `decode_tap_terminal` (8 existing + 20 new); each arm matches encoder's emit path; tap-illegal Multi/SortedMulti arms have appropriate comments; catch-all error kind is sensible; no Tag::Bare or Tag::Reserved* references remain.
- Output: `design/agent-reports/v0-6-phase-3-review.md`.

- [ ] **Step 3.6.2: Address findings**

---

## Phase 4 — Error variant rename

**Goal:** Rename `Error::TapLeafSubsetViolation` to `Error::SubsetViolation` across the crate. Update `tests/error_coverage.rs` EnumIter mirror.

**Files:**
- Modify: `crates/md-codec/src/error.rs`
- Modify: all callers (encoder/decoder/tests)
- Modify: `crates/md-codec/tests/error_coverage.rs`

### Task 4.1 — Rename the variant

- [ ] **Step 4.1.1: Rename in error.rs**

Change `TapLeafSubsetViolation { operator: String, leaf_index: Option<usize> }` to `SubsetViolation { operator: String, leaf_index: Option<usize> }`. Update the variant's rustdoc per spec §5.2.

- [ ] **Step 4.1.2: Find all callers**

Run: `grep -rn "TapLeafSubsetViolation" crates/md-codec/`

Expect ~10-20 hits across `encode.rs`, `decode.rs`, `tests/`, `error.rs` (Display impl, etc.), `bin/`.

- [ ] **Step 4.1.3: sed substitute mechanically**

Run:
```bash
find crates/md-codec/src crates/md-codec/tests -type f -name "*.rs" -exec sed -i 's/TapLeafSubsetViolation/SubsetViolation/g' {} \;
```

- [ ] **Step 4.1.4: Update rustdoc references**

Run: `grep -rn "TapLeafSubsetViolation\|tap.leaf.subset" crates/md-codec/src/`

Update any rustdoc that still references the old name conceptually (not the variant). Check `validate_tap_leaf_subset` rustdoc, `tap_terminal_name` rustdoc, etc.

- [ ] **Step 4.1.5: Scrub design/ markdown for forward-pointing references** (per plan review CRIT-2)

Run: `grep -rn "TapLeafSubsetViolation" design/ | grep -v "agent-reports/"`

Audit each match. Past-tense/historical references (e.g., "v0.5 raised TapLeafSubsetViolation", "Phase D introduced TapLeafSubsetViolation") **stay** — they describe the v0.5 state accurately. Forward-pointing references (e.g., "callers get TapLeafSubsetViolation" in the spec or rationale doc) **update** to `SubsetViolation`.

Files likely needing update: `design/SPEC_v0_6_strip_layer_3.md`, `design/MD_SCOPE_DECISION_2026-04-28.md`, `design/FOLLOWUPS.md`. Skip `design/agent-reports/` — those are durable historical records.

### Task 4.2 — Update error_coverage CI gate

- [ ] **Step 4.2.1: Locate the EnumIter mirror**

Run: `grep -nE "EnumIter|TapLeafSubset|Subset" crates/md-codec/tests/error_coverage.rs`

The test file uses strum::EnumIter to assert exhaustiveness. The mirror table contains a list of all error variant names that must be kept in sync with `Error`.

- [ ] **Step 4.2.2: Update the variant name**

Replace `TapLeafSubsetViolation` with `SubsetViolation` in the mirror.

- [ ] **Step 4.2.3: Rename conformance.rs test for snake_case derivation** (per plan review IMP-4)

The error_coverage gate derives expected test names from variant names via snake_case. After the rename, the conformance.rs test currently named `rejects_tap_leaf_subset_violation` must rename to `rejects_subset_violation`.

Run: `grep -n 'rejects_tap_leaf_subset' crates/md-codec/tests/conformance.rs`

If the test exists, rename it. Verify by re-running the grep — should produce no matches.

### Task 4.3 — Compile + test

- [ ] **Step 4.3.1: cargo check**

Run: `cargo check -p md-codec --tests 2>&1 | tail -20`

Expected: clean compile.

- [ ] **Step 4.3.2: Run error_coverage gate**

Run: `cargo test -p md-codec --test error_coverage 2>&1 | tail -20`

Expected: pass.

- [ ] **Step 4.3.3: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
refactor(v0.6 phase 4): rename Error::TapLeafSubsetViolation → SubsetViolation

Per spec §5: variant name presumed Tap-context, but the explicit-call
validator infrastructure can plausibly extend to Segwitv0 subsets in
the future. Rename is breaking but cheap pre-1.0.

Field shape unchanged: { operator: String, leaf_index: Option<usize> }.

Updates:
- error.rs: variant rename + rustdoc clarification per spec §5.2
- All callers (~10-20 sites): mechanical sed substitution
- tests/error_coverage.rs: EnumIter mirror updated

Spec reference: design/SPEC_v0_6_strip_layer_3.md §5.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 4.4 — Phase 4 review folded into Phase 5 reviewer brief

Per plan review IMP-6: Phase 4 is mechanical but is a public-API breaking change spanning ~20 sites. Risk: a rustdoc link sed missed. The Phase 5 reviewer brief (Task 5.3) will include explicit Phase 4 verification points:
- Confirm every `TapLeafSubsetViolation` in `crates/md-codec/src/` is gone (post-Phase 4 sed).
- Confirm `rejects_tap_leaf_subset_violation` is renamed in conformance.rs.
- Confirm the EnumIter mirror in `tests/error_coverage.rs` updated.
- Confirm `design/` markdown forward-pointing references updated per Step 4.1.5; past-tense references preserved.
- Confirm rustdoc CI pre-passes with the rename (`RUSTDOCFLAGS="-D warnings" cargo doc -p md-codec --no-deps`).

---

## Phase 5 — Corpus expansion

**Goal:** Add 18+ new positive vectors per spec §6.1; audit existing negative vectors for now-flips; update test fixtures that asserted now-admitted-operator rejection.

**Files:**
- Modify: `crates/md-codec/src/vectors.rs`
- Audit: `crates/md-codec/tests/{taproot,conformance,corpus}.rs`

### Task 5.1 — Add new positive vectors

- [ ] **Step 5.1.1: Locate vectors.rs corpus structure**

Run: `grep -nE "CORPUS_FIXTURES|TAPROOT_FIXTURES|fn build_test_vectors" crates/md-codec/src/vectors.rs | head -20`

Identify the `TAPROOT_FIXTURES` array (or equivalent) where T1-T7 currently sit. New vectors append after T7.

- [ ] **Step 5.1.2: Add the 10 centerpiece + Ledger/Coldcard documented shapes**

Per spec §6.1, append:

```rust
// v0.6 admit-set widening — centerpiece + signer-documented shapes
("tr_sortedmulti_a_2of3_md_v0_6",
 "Taproot sortedmulti_a 2-of-3 (v0.6 SortedMultiA Tag round-trip anchor)",
 "tr(@0/**,sortedmulti_a(2,@1/**,@2/**,@3/**))"),

("tr_thresh_in_tap_leaf_md_v0_6",
 "Taproot thresh in tap leaf with s: wrappers",
 "tr(@0/**,thresh(2,pk(@1/**),s:pk(@2/**),s:pk(@3/**)))"),

("tr_or_b_in_tap_leaf_md_v0_6",
 "Taproot or_b in tap leaf with s: wrapper",
 "tr(@0/**,or_b(pk(@1/**),s:pk(@2/**)))"),

("tr_sha256_htlc_md_v0_6",
 "Taproot sha256 HTLC pattern in tap leaf",
 "tr(@0/**,and_v(v:sha256(deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef),pk(@1/**)))"),

("tr_after_absolute_height_md_v0_6",
 "Taproot absolute-height locked multisig (Ledger compound shape)",
 "tr(@0/**,and_v(v:multi_a(2,@1/**,@2/**),after(700000)))"),

("tr_after_absolute_time_md_v0_6",
 "Taproot absolute-time locked multisig (Ledger compound shape)",
 "tr(@0/**,and_v(v:multi_a(2,@1/**,@2/**),after(1734567890)))"),

("tr_older_relative_time_md_v0_6",
 "Taproot relative-time locked multisig (Ledger compound shape)",
 "tr(@0/**,and_v(v:multi_a(2,@1/**,@2/**),older(4194305)))"),

("tr_pkh_in_tap_leaf_md_v0_6",
 "Taproot pkh() round-trip in tap leaf via desugaring (Coldcard documented)",
 "tr(@0/**,and_v(v:pkh(@1/**),older(144)))"),

("tr_multi_leaf_with_sortedmulti_a_md_v0_6",
 "Taproot multi-leaf TapTree with sortedmulti_a (Coldcard documented)",
 "tr(@0/**,{sortedmulti_a(2,@1/**,@2/**),pk(@3/**)})"),

("tr_complex_recovery_path_md_v0_6",
 "Taproot complex recovery path (Coldcard documented)",
 "tr(@0/**,{and_v(v:pkh(@1/**),older(1000)),pk(@2/**)})"),
```

- [ ] **Step 5.1.3: Add the 8 per-Terminal coverage vectors**

```rust
// Per-Terminal coverage (v0.6 corpus expansion)
("tr_andor_in_tap_leaf_md_v0_6",
 "Taproot andor 3-arg in tap leaf",
 "tr(@0/**,andor(pk(@1/**),pk(@2/**),pk(@3/**)))"),

("tr_or_c_in_tap_leaf_md_v0_6",
 "Taproot or_c in tap leaf with v: wrapper",
 "tr(@0/**,or_c(pk(@1/**),v:pk(@2/**)))"),

("tr_or_i_in_tap_leaf_md_v0_6",
 "Taproot or_i in tap leaf",
 "tr(@0/**,or_i(pk(@1/**),pk(@2/**)))"),

("tr_hash256_htlc_md_v0_6",
 "Taproot hash256 HTLC pattern (locks internal-byte-order encoding)",
 "tr(@0/**,and_v(v:hash256(deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef),pk(@1/**)))"),

("tr_ripemd160_htlc_md_v0_6",
 "Taproot ripemd160 HTLC pattern",
 "tr(@0/**,and_v(v:ripemd160(deadbeefdeadbeefdeadbeefdeadbeefdeadbeef),pk(@1/**)))"),

("tr_hash160_htlc_md_v0_6",
 "Taproot hash160 HTLC pattern",
 "tr(@0/**,and_v(v:hash160(deadbeefdeadbeefdeadbeefdeadbeefdeadbeef),pk(@1/**)))"),

("tr_a_wrapper_in_tap_leaf_md_v0_6",
 "Taproot a: wrapper in tap leaf via and_b",
 "tr(@0/**,and_b(pk(@1/**),a:pk(@2/**)))"),

("tr_d_wrapper_in_tap_leaf_md_v0_6",
 "Taproot d: wrapper in tap leaf via andor",
 "tr(@0/**,andor(pk(@1/**),pk(@2/**),d:older(144)))"),
```

The `j:` and `n:` wrapper vectors are deferred — they require type-coercion patterns awkward to construct via BIP 388 source form. Hand-AST tests are OK substitutes (note for Phase 6 reviewer).

- [ ] **Step 5.1.4: Compile**

Run: `cargo check -p md-codec --lib 2>&1 | tail -15`

Expected: clean. The new fixtures are policy strings; rust-miniscript parses them.

- [ ] **Step 5.1.5: Run vector schema tests**

Run: `cargo test -p md-codec --test vectors_schema 2>&1 | tail -20`

Expected: SHA-pin tests fail (corpus content changed). That's expected — Phase 8 re-baselines SHAs.

- [ ] **Step 5.1.6: Add defensive hash-byte-order pin test** (per plan review §6.3 concern)

Add to `crates/md-codec/tests/taproot.rs` (or a new `tests/hash_byte_order.rs`):

```rust
#[test]
fn hash_terminals_encode_internal_byte_order_not_display_order() {
    // Defensive: the round-trip corpus alone cannot catch an encoder+decoder
    // both swapped to display-order (round-trip stable, but format changed).
    // Pin the bytes directly.
    use bitcoin::hashes::{Hash, hash160, ripemd160, sha256};
    use miniscript::hash256;
    use md_codec::bytecode::{Tag, encode_template};
    // ... construct minimal Sha256 / Hash256 / Ripemd160 / Hash160 Terminals
    // with known hashes (e.g., all-0xAA for 32 bytes), encode via
    // encode_template, and assert the bytecode contains the input bytes
    // in INTERNAL ORDER (not reversed-display-order).
    //
    // Reference: encode.rs:316-319 comment; spec §6.3.

    let known_32 = [0xAAu8; 32];
    let known_20 = [0xBBu8; 20];

    // Sha256: encode and verify the bytecode contains 32 bytes of 0xAA
    // immediately after the Tag::Sha256 byte.
    {
        let term: miniscript::Terminal<miniscript::DescriptorPublicKey, miniscript::Tap>
            = miniscript::Terminal::Sha256(sha256::Hash::from_byte_array(known_32));
        let mut out = Vec::new();
        // ... call the appropriate encode helper
        // assert!(out[1..33] == known_32);
        // (exact API call TBD by implementer; shape is "encode then assert byte-prefix match")
    }
    // Repeat for Hash256, Ripemd160, Hash160.
}
```

This test catches the case where encoder + decoder both regress to display-order (round-trip would still pass, but external decoders would interpret bytes differently).

- [ ] **Step 5.1.7: Run test to verify it passes**

Run: `cargo test -p md-codec --test taproot hash_terminals_encode_internal 2>&1 | tail -10`

Expected: PASS (since the encoder uses `as_byte_array()` directly).

- [ ] **Step 5.1.8: Commit** (this commit also captures the corpus expansion from Step 5.1.2-5.1.4 if not yet committed)

```bash
git add crates/md-codec/src/vectors.rs
git commit -m "$(cat <<'EOF'
test(v0.6 phase 5): expand positive corpus by 18 vectors

Per spec §6.1: every newly-admitted Terminal in v0.6 gets at least one
round-trip fixture locking its byte form under the v0.6 Tag layout.

10 centerpiece + signer-documented shapes:
- tr_sortedmulti_a_2of3 (NEW SortedMultiA Tag anchor)
- tr_thresh_in_tap_leaf (thresh + s: wrapper)
- tr_or_b_in_tap_leaf (or_b + s: wrapper)
- tr_sha256_htlc (hash terminal)
- tr_after_absolute_height/time (Ledger compound shapes)
- tr_older_relative_time (Ledger compound shape)
- tr_pkh_in_tap_leaf (Coldcard documented)
- tr_multi_leaf_with_sortedmulti_a (Coldcard documented)
- tr_complex_recovery_path (Coldcard documented)

8 per-Terminal coverage vectors:
- tr_andor (3-child)
- tr_or_c, tr_or_i
- tr_hash256, tr_ripemd160, tr_hash160 (locks all 4 hash terminals)
- tr_a_wrapper, tr_d_wrapper (a:/d: wrappers)

j:/n: wrapper vectors deferred — type-coercion patterns awkward via
BIP 388 source form; hand-AST tests during Phase 6 review.

SHA-pin tests fail after this commit; phase 8 re-baselines.

Spec reference: design/SPEC_v0_6_strip_layer_3.md §6.1.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 5.2 — Apply pre-pinned negative-vector audit

The negative-vector decisions are pre-pinned per the round-1 plan review (CRIT-3 / IMP-8). Apply the table below verbatim; do not re-derive judgment.

**Pre-pinned audit table:**

| Vector | Decision | Rationale |
|---|---|---|
| `n_tap_leaf_subset` (sha256 in tap leaf) | **DELETE** | sha256 is admitted in v0.6; redundant with new positive `tr_sha256_htlc_md_v0_6` |
| `n_taptree_inner_wpkh` | **KEEP, change `expected_error_variant`** to the structural catch-all variant Phase 3 introduces (`InvalidBytecode { kind: TagInvalidContext { tag: 0x04, context: "tap-leaf-inner" } }` per IMP-7) | `wpkh` is a top-level descriptor tag; structurally invalid as tap-leaf inner regardless of strip |
| `n_taptree_inner_sh` | **KEEP, change `expected_error_variant`** | Same; tag value 0x03 |
| `n_taptree_inner_wsh` | **KEEP, change `expected_error_variant`** | Same; tag value 0x05 |
| `n_taptree_inner_tr` | **KEEP, change `expected_error_variant`** | Same; tag value 0x06 |
| `n_taptree_inner_pkh` | **KEEP, change `expected_error_variant`** | Same; tag value 0x02. Distinct from policy-level `pkh()` (which desugars to `c:pk_h(...)`) — this vector tests the descriptor wrapper byte showing up where a tap-leaf inner is expected. |
| `n_sh_bare` | **KEEP, REBASE input bytes** | `expected_error_variant: PolicyScopeViolation` unchanged; input_strings shift because Tag layout shifted (Tag::Bare removed; byte 0x07 is now Tag::TapTree). Provenance prose updates. |
| `n_top_bare` | **KEEP, REBASE input bytes** | Same as `n_sh_bare`. After Phase 8/10 regen, byte 0x07 in input is interpreted as Tag::TapTree top-level — produces `PolicyScopeViolation` with a different message ("TapTree as top-level descriptor"); same error variant. Provenance updates. |

- [ ] **Step 5.2.1: Apply audit table — vectors.rs negative-vector definitions**

Find the negative-vector array in `crates/md-codec/src/vectors.rs` (likely `NEGATIVE_FIXTURES` or similar). Apply each row of the audit table:
- For DELETE rows: remove the entry.
- For KEEP-with-error-variant-change rows: update the `expected_error_variant` field to whatever Phase 3 chose for the structural catch-all (IMP-7 recommends `InvalidBytecode { kind: TagInvalidContext { tag, context } }`).
- For KEEP-with-input-rebase rows: update the input bytecode + provenance to use v0.6 Tag bytes. The input bytes regenerate at Phase 10 via `gen_vectors --output`.

- [ ] **Step 5.2.2: Update test fixtures referring to deleted/renamed vectors**

Run: `grep -rn "n_tap_leaf_subset\b" crates/md-codec/tests/`

Any test that expects `n_tap_leaf_subset` to exist needs updating (delete the test or repoint to a structural-rejection vector).

- [ ] **Step 5.2.3: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
test(v0.6 phase 5b): apply pre-pinned negative-vector audit

Per spec §6.2 + plan review CRIT-3/IMP-8, the negative-vector audit
decisions were pre-pinned in the plan and applied verbatim:

- DELETE n_tap_leaf_subset (sha256 in tap leaf — admitted in v0.6;
  redundant with new positive tr_sha256_htlc_md_v0_6)
- KEEP-with-error-variant-change for n_taptree_inner_{wpkh,sh,wsh,tr,pkh}:
  expected_error_variant updates from TapLeafSubsetViolation to
  InvalidBytecode { kind: TagInvalidContext { tag, context } } per
  Phase 3's structural catch-all (IMP-7)
- KEEP-with-input-rebase for n_sh_bare, n_top_bare: input_strings
  shift because Tag layout shifted (Tag::Bare removed; byte 0x07 is
  now Tag::TapTree); expected_error_variant: PolicyScopeViolation
  unchanged. Bytes regenerate at Phase 8/10.

Note: tests/vectors_schema.rs failing tests (committed_v0_2_json_matches
+ v0_2_sha256_lock) remain RED until Phase 10 regen; this is expected
and whitelisted in subsequent phase reviewer briefs.

Spec reference: design/SPEC_v0_6_strip_layer_3.md §6.2.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 5.3 — Phase 5 review (also covers Phase 4 per IMP-6)

- [ ] **Step 5.3.1: Dispatch Opus reviewer**

Brief (covers Phases 4 and 5):

**Phase 5 (corpus expansion):**
- Files: `crates/md-codec/src/vectors.rs` after Phase 5 commits.
- Verify: 18 new positive vectors; canonical form for each policy; per-Terminal coverage adequate.
- Verify the negative-vector audit was applied verbatim per the pre-pinned table in Step 5.2 (deletions, error-variant changes, input rebases match the plan).

**Phase 4 (rolled in per IMP-6):**
- Confirm every `TapLeafSubsetViolation` in `crates/md-codec/src/` is gone (post-Phase 4 sed).
- Confirm `rejects_tap_leaf_subset_violation` is renamed in conformance.rs.
- Confirm the EnumIter mirror in `tests/error_coverage.rs` updated.
- Confirm `design/` markdown forward-pointing references updated per Step 4.1.5; past-tense references preserved.
- Confirm rustdoc CI passes with the rename: `RUSTDOCFLAGS="-D warnings" cargo doc -p md-codec --no-deps` clean.

**Whitelist (per CRIT-1):** the following tests are EXPECTED to fail at this checkpoint and must NOT be reported as findings:
- `tests::vectors_schema::v0_2_sha256_lock_matches_committed_file`
- `tests::vectors_schema::committed_v0_2_json_matches_regenerated_if_present`
Both will pass after Phase 10 regen + SHA-pin update.

- Output: `design/agent-reports/v0-6-phase-5-review.md`.

- [ ] **Step 5.3.2: Address findings**

Critical/important inline; nits to FOLLOWUPS.

---

## Phase 6 — BIP draft updates

**Goal:** Rewrite `bip/bip-mnemonic-descriptor.mediawiki` per spec §7. MUST→MAY clause; new §"Signer compatibility (informational)"; Tag table to v0.6 layout.

**Files:**
- Modify: `bip/bip-mnemonic-descriptor.mediawiki`

### Task 6.1 — Rewrite §"Taproot tree" subset clause

- [ ] **Step 6.1.1: Locate line 547**

Run: `sed -n '540,560p' bip/bip-mnemonic-descriptor.mediawiki`

Confirm the MUST clause is at line 547.

- [ ] **Step 6.1.2: Replace with MAY-informational text**

Replace the existing paragraph with the spec §7.1 text:

```
Implementations MAY enforce a per-leaf miniscript subset matching their target hardware signer's documented admit list. The MD encoding format itself does not require this — see §"Signer compatibility (informational)" below for the layered-responsibility framing. Implementations SHOULD clearly document any such limitations per BIP 388 §"Implementation guidelines".
```

### Task 6.2 — Add new §"Signer compatibility (informational)"

- [ ] **Step 6.2.1: Determine insertion point**

After the §"Taproot tree" body but before the next major section. Likely a few paragraphs after line 547.

- [ ] **Step 6.2.2: Insert section per spec §7.2**

The exact text is in spec §7.2. Insert as a `====Signer compatibility (informational)====` subsection.

### Task 6.3 — Tag table update

- [ ] **Step 6.3.1: Locate Tag table at lines 371-453**

Run: `sed -n '370,460p' bip/bip-mnemonic-descriptor.mediawiki`

Identify the table structure (likely `{| class="wikitable"` with rows per Tag).

- [ ] **Step 6.3.2: Apply v0.6 layout per spec §2.2/§7.3**

Drop the `Tag::Bare` row. Add the `Tag::SortedMultiA` row at 0x0B. Renumber all rows for operators that move (per spec §2.3 table). Drop the 14 `Reserved*` rows.

This is mechanical but tedious; tabular editing.

### Task 6.4 — Reserved* paragraph rewrite

- [ ] **Step 6.4.1: Locate line 455**

Run: `sed -n '454,460p' bip/bip-mnemonic-descriptor.mediawiki`

- [ ] **Step 6.4.2: Replace with historical-orientation note per spec §7.3**

Use the exact text from spec §7.3:

```
Tags 0x24–0x31 are unallocated. (In MD v0.5 and earlier, these bytes were reserved for descriptor-codec inline-key compatibility; MD v0.6 dropped them since MD's BIP-388 wallet-policy framing forbids inline keys. See the project's <code>MD_SCOPE_DECISION_2026-04-28.md</code> design document for rationale.)
```

Plus the 0x32 unallocation note and 0x34/0x36+ status per spec §7.3.

### Task 6.5 — Verify BIP draft renders

- [ ] **Step 6.5.1: Spot-check structure**

Run: `wc -l bip/bip-mnemonic-descriptor.mediawiki && grep -nE "^==|^===|^====" bip/bip-mnemonic-descriptor.mediawiki | head -30`

The structure should still parse. Confirm headings are intact.

- [ ] **Step 6.5.2: Commit**

```bash
git add bip/bip-mnemonic-descriptor.mediawiki
git commit -m "$(cat <<'EOF'
docs(v0.6 phase 6): BIP draft MUST→MAY + Signer compatibility section

- Rewrite §"Taproot tree" subset clause from MUST to MAY-informational
- Add new §"Signer compatibility (informational)" section per spec §7.2
- Update Tag table to v0.6 layout (drop Bare row, add SortedMultiA row,
  renumber operators that moved, drop 14 Reserved* rows)
- Replace Reserved* paragraph at line 455 with historical-orientation
  note pointing at MD_SCOPE_DECISION_2026-04-28.md
- Add 0x32 / 0x34 / 0x36+ unallocation notes per spec §7.3

Spec reference: design/SPEC_v0_6_strip_layer_3.md §7.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 6.6 — Phase 6 review (also covers Phase 7 per IMP-6)

- [ ] **Step 6.6.1: Dispatch Opus reviewer**

Brief (covers Phases 6 and 7):

**Phase 6 (BIP draft):**
- File: `bip/bip-mnemonic-descriptor.mediawiki` after Phase 6 commit.
- Verify: MAY clause text matches spec §7.1; new §"Signer compatibility" reads as drafted; Tag table updated correctly (cross-check against the in-tree `tag.rs` to confirm byte-for-byte agreement); historical-orientation note matches spec §7.3 verbatim.

**Phase 7 (READMEs + CLI, rolled in per IMP-6):**
- Files: `README.md`, `crates/md-codec/README.md`, `crates/md-codec/src/bin/md/main.rs` after Phase 7 commit.
- Verify: recovery-responsibility paragraph wording per spec §8.1 reads as intended (clear, neutral, no signer-curation overclaim); no typos; cross-document tone consistency with the BIP §"Signer compatibility" section; CLI `md encode --help` long form contains the warning per spec §8.3.

**Whitelist (per CRIT-1):** the same two `tests::vectors_schema` failures whitelisted in the Phase 5 review remain whitelisted here. Both will pass after Phase 10 regen + SHA-pin update.

- Output: `design/agent-reports/v0-6-phase-6-review.md`.

- [ ] **Step 6.6.2: Address findings**

Critical/important inline; nits to FOLLOWUPS.

---

## Phase 7 — README + CLI updates

**Goal:** Add recovery-responsibility framing per spec §8.

**Files:**
- Modify: `README.md`
- Modify: `crates/md-codec/README.md`
- Modify: `crates/md-codec/src/bin/md/main.rs`

### Task 7.1 — Top-level README

- [ ] **Step 7.1.1: Locate "What is MD?" section**

Run: `head -80 README.md`

- [ ] **Step 7.1.2: Add recovery-responsibility paragraph per spec §8.1**

Insert the spec §8.1 paragraph in or near the "What is MD?" section.

### Task 7.2 — Crate README

- [ ] **Step 7.2.1: Same paragraph addition**

Run: `head -50 crates/md-codec/README.md`

Add the spec §8.1 paragraph, plus (if a "Limitations" / "Caveats" section exists) a sentence per spec §8.2.

### Task 7.3 — CLI help

- [ ] **Step 7.3.1: Locate `md encode` clap definition**

Run: `grep -nE "command|encode|about" crates/md-codec/src/bin/md/main.rs | head -20`

- [ ] **Step 7.3.2: Add long-help warning per spec §8.3**

Update the encode subcommand's `long_about` (or equivalent) to include:

> WARNING: This tool encodes any BIP 388 wallet policy. It does not check whether the policy is signable on any particular hardware wallet — that is your responsibility. See the project README for details.

### Task 7.4 — Compile + test

- [ ] **Step 7.4.1: cargo check**

Run: `cargo check -p md-codec --bins 2>&1 | tail -10`

- [ ] **Step 7.4.2: Test help output**

Run: `cargo run -q -p md-codec --bin md -- encode --help 2>&1 | tail -20`

Verify the new warning appears.

- [ ] **Step 7.4.3: Commit**

```bash
git add README.md crates/md-codec/README.md crates/md-codec/src/bin/md/main.rs
git commit -m "$(cat <<'EOF'
docs(v0.6 phase 7): recovery-responsibility framing in READMEs + CLI

Per spec §8: explicit responsibility-chain framing in user-visible docs.

- README.md: add recovery-responsibility paragraph in scope section
- crates/md-codec/README.md: same paragraph + Limitations addendum
- bin/md/main.rs: encode subcommand long-help gains the warning

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 8 — Vector regen + SHA pin updates

**Goal:** Regenerate v0.1.json + v0.2.json with v0.6 bytecode; update SHA pins; verify round-trip.

**Files:**
- Modify: `crates/md-codec/tests/vectors/v0.1.json` (regenerated)
- Modify: `crates/md-codec/tests/vectors/v0.2.json` (regenerated)
- Modify: `crates/md-codec/tests/vectors_schema.rs` (SHA pins)
- Modify: `crates/md-codec/src/vectors.rs` (GENERATOR_FAMILY token roll)

### Task 8.1 — Roll GENERATOR_FAMILY token

- [ ] **Step 8.1.1: Locate the token**

Run: `grep -nE "GENERATOR_FAMILY|md-codec 0\\.[0-9]" crates/md-codec/src/vectors.rs`

- [ ] **Step 8.1.2: Update from 0.5 to 0.6**

The `concat!` of `CARGO_PKG_VERSION_MAJOR` + `_MINOR` will roll automatically once Cargo.toml is bumped. Verify the constant uses the dynamic form. If it's hardcoded, update.

(Cargo.toml version bump is Phase 10. The GENERATOR_FAMILY token will roll at that point.)

### Task 8.2 — Regenerate vector files

- [ ] **Step 8.2.1: Run gen_vectors**

Run:
```bash
cargo run --bin gen_vectors -- --output crates/md-codec/tests/vectors/v0.1.json --schema 1
cargo run --bin gen_vectors -- --output crates/md-codec/tests/vectors/v0.2.json
```

This regenerates both files. Note: the GENERATOR_FAMILY token will still read "md-codec 0.5" until Cargo.toml is bumped (Phase 10). For Phase 8, the SHA pins must reflect the current state; Phase 10's version bump will require a final re-regen and SHA-pin update.

To avoid double-baselining, **defer the actual SHA-pinning regen to Phase 10** and use Phase 8 only to verify the regen succeeds structurally.

- [ ] **Step 8.2.2: Verify structural validity**

Run: `cargo run --bin gen_vectors -- --verify crates/md-codec/tests/vectors/v0.1.json && cargo run --bin gen_vectors -- --verify crates/md-codec/tests/vectors/v0.2.json`

Expected: both succeed (all generated entries verify against the same encoder).

### Task 8.3 — Defer SHA pinning to Phase 10

Mark Phase 8 as a "verify regen works" pass; the actual SHA pinning happens in Phase 10 alongside the version bump.

- [ ] **Step 8.3.1: Discard the regenerated files** (so Phase 10 produces them fresh post-bump)

Run: `git checkout -- crates/md-codec/tests/vectors/v0.1.json crates/md-codec/tests/vectors/v0.2.json`

(If they had no v0.5 baseline, this resets to whatever was committed; harmless.)

- [ ] **Step 8.3.2: Commit a verification placeholder**

No commit needed — Phase 8 produces no new commits. It's a verification gate before Phase 9/10. Note in the running PR that Phase 8 verified regen works.

---

## Phase 9 — CHANGELOG + MIGRATION

**Goal:** Consolidate `[Unreleased]` → `[0.6.0]`; extend MIGRATION.md `v0.5.x → v0.6.0` per spec §9.1's 8 items.

**Files:**
- Modify: `CHANGELOG.md`
- Modify: `MIGRATION.md`

### Task 9.1 — CHANGELOG consolidation

- [ ] **Step 9.1.1: Locate `[Unreleased]` section**

Run: `head -25 CHANGELOG.md`

- [ ] **Step 9.1.2: Replace with `[0.6.0] — <date>` content**

The existing `[Unreleased]` covers only the `DecodedString.data` removal. Extend to cover all 8 spec §9.1 items. Use the date of the eventual tag commit.

Sections: `### Changed (breaking)`, `### Added`, `### Removed`, `### Wire format`, `### Notes`, `### Closes FOLLOWUPS`.

### Task 9.2 — MIGRATION extension

- [ ] **Step 9.2.1: Locate `v0.5.x → v0.6.0` section**

Run: `head -60 MIGRATION.md`

- [ ] **Step 9.2.2: Extend to cover all 8 breaking changes**

The existing section covers only `DecodedString.data`. Extend per spec §9.1 to cover Tag-space rework, validator default flip, Reserved* drop, Tag::Bare drop, Error rename, wire-format break, etc.

### Task 9.3 — Commit

- [ ] **Step 9.3.1: Commit**

```bash
git add CHANGELOG.md MIGRATION.md
git commit -m "$(cat <<'EOF'
docs(v0.6 phase 9): CHANGELOG + MIGRATION for 0.6.0

- CHANGELOG: rename [Unreleased] → [0.6.0] — <date>; consolidate all
  v0.6 strip work entries; add Added/Removed/Wire format/Notes/Closes
  FOLLOWUPS subsections
- MIGRATION: extend v0.5.x → v0.6.0 section to cover all 8 breaking
  changes per spec §9.1

Spec reference: design/SPEC_v0_6_strip_layer_3.md §9 + §11.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 10 — Release plumbing

**Goal:** Cargo.toml version bump 0.5.0 → 0.6.0; verify GENERATOR_FAMILY rolls; regenerate vector files with new family token; update SHA pins; final tag.

**Files:**
- Modify: `crates/md-codec/Cargo.toml`
- Regenerate: `crates/md-codec/tests/vectors/v0.1.json`, `v0.2.json`
- Modify: `crates/md-codec/tests/vectors_schema.rs` (SHA pins)

### Task 10.1 — Version bump

- [ ] **Step 10.1.1: Update Cargo.toml**

Run: `grep -n '^version' crates/md-codec/Cargo.toml`

Replace `version = "0.5.0"` with `version = "0.6.0"`.

- [ ] **Step 10.1.2: Verify**

Run: `grep -A1 "^\\[package\\]" crates/md-codec/Cargo.toml | head -5`

### Task 10.2 — Regenerate vector files

- [ ] **Step 10.2.1: Confirm GENERATOR_FAMILY rolls**

Run: `cargo run --bin gen_vectors -- --output /tmp/test-v0.6.json && grep "generator" /tmp/test-v0.6.json | head -2`

Expected: `"generator": "md-codec 0.6"`.

- [ ] **Step 10.2.2: Regenerate both files**

Run:
```bash
cargo run --bin gen_vectors -- --output crates/md-codec/tests/vectors/v0.1.json --schema 1
cargo run --bin gen_vectors -- --output crates/md-codec/tests/vectors/v0.2.json
```

- [ ] **Step 10.2.3: Compute new SHAs**

Run:
```bash
sha256sum crates/md-codec/tests/vectors/v0.1.json crates/md-codec/tests/vectors/v0.2.json
```

Record both SHAs.

### Task 10.3 — Update SHA pins

- [ ] **Step 10.3.1: Locate SHA pin constants**

Run: `grep -nE "V0_[12]_SHA256\|V0\\.\\d_SHA" crates/md-codec/tests/vectors_schema.rs`

- [ ] **Step 10.3.2: Update both pins**

Replace the v0.5 SHAs with the v0.6 SHAs computed in Step 10.2.3.

- [ ] **Step 10.3.3: Run schema tests**

Run: `cargo test -p md-codec --test vectors_schema 2>&1 | tail -10`

Expected: pass.

### Task 10.4 — Full test suite

- [ ] **Step 10.4.1: Run all tests**

Run: `cargo test -p md-codec 2>&1 | tail -30`

Expected: all pass. If anything fails, debug and fix before proceeding.

- [ ] **Step 10.4.2: Run rustdoc CI gate**

Run: `RUSTDOCFLAGS="-D warnings" cargo doc -p md-codec --no-deps 2>&1 | tail -5`

Expected: clean.

- [ ] **Step 10.4.3: Run clippy**

Run: `cargo clippy --all-targets -p md-codec 2>&1 | tail -20`

Expected: clean.

### Task 10.5 — Commit + tag

- [ ] **Step 10.5.1: Commit**

```bash
git add crates/md-codec/Cargo.toml crates/md-codec/tests/vectors/v0.1.json crates/md-codec/tests/vectors/v0.2.json crates/md-codec/tests/vectors_schema.rs
git commit -m "$(cat <<'EOF'
release(v0.6.0): strip Layer 3 — bump 0.5.0 → 0.6.0; regen vectors

- Cargo.toml: 0.5.0 → 0.6.0
- GENERATOR_FAMILY rolls via Cargo dynamic concat → "md-codec 0.6"
- v0.1.json regenerated with v0.6 layout: SHA <new-v01-sha>
- v0.2.json regenerated with v0.6 layout: SHA <new-v02-sha>
- vectors_schema.rs SHA pins updated

Spec reference: design/SPEC_v0_6_strip_layer_3.md §10.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 10.5.2: Tag**

```bash
git tag -a md-codec-v0.6.0 -m "md-codec v0.6.0 — strip Layer 3 (signer-compatibility curation)

See design/MD_SCOPE_DECISION_2026-04-28.md for rationale.
See design/SPEC_v0_6_strip_layer_3.md for the spec.
See CHANGELOG.md [0.6.0] for the changelog.
See MIGRATION.md v0.5.x → v0.6.0 for the migration guide."
```

- [ ] **Step 10.5.3: Push tag**

```bash
git push origin md-codec-v0.6.0
git push origin feature/v0.6-strip-layer-3
```

---

## Phase 11 — Final reconciliation

**Goal:** Compare every agent report in `design/agent-reports/` produced during v0.6 work against `FOLLOWUPS.md` to ensure no flagged items were dropped.

### Task 11.1 — Inventory phase reviews

- [ ] **Step 11.1.1: List v0.6 agent reports**

Run: `ls design/agent-reports/v0-6-* 2>&1`

Expected: spec review (1), phase reviews (1, 2, 3, 5, 6), plus any others created during implementation.

- [ ] **Step 11.1.2: For each report, extract the "Follow-up items" section**

Manually scan each report. List every item flagged for FOLLOWUPS.

### Task 11.2 — Cross-check against FOLLOWUPS

- [ ] **Step 11.2.1: Run grep for each item**

For each item from 11.1.2, grep `design/FOLLOWUPS.md` to confirm an entry exists.

If missing, file a new entry now.

### Task 11.3 — Update memory

- [ ] **Step 11.3.1: Update project memory**

Per the user's standing directive ("CLAUDE.md / project-memory updates after strip ships"): update memory entries to reflect:
- v0.6.0 release shipped
- Layer 3 stripped; MD scope is encoding-only
- Open FOLLOWUPS for v0.7+ work

Use `Write` against `/home/bcg/.claude/projects/-scratch-code-shibboleth/memory/MEMORY.md` and the relevant memory files.

### Task 11.4 — Update PR

- [ ] **Step 11.4.1: Update PR #2 description**

Run: `gh pr view 2 --json body --jq .body`

Replace `[ ]` checkboxes with `[x]` for completed items. Note the v0.6.0 tag.

- [ ] **Step 11.4.2: Optional merge**

User authorized "continue working through release". Per safety guidance, the PR can be merged to main since the user explicitly endorsed shipping. Merge with `gh pr merge 2 --merge`.

---

## Self-review checklist (controller, before launching execution)

- [ ] **Spec coverage**: every spec §-section has a corresponding phase in this plan. (§1 = scope, §2 = Phase 1, §3 = Phase 2, §4 = Phase 3, §5 = Phase 4, §6 = Phase 5, §7 = Phase 6, §8 = Phase 7, §9 = Phase 9, §10 = Phase 10, §11 = covered by acceptance criteria, §12 = resolved.)

- [ ] **Placeholder scan**: no "TBD", "TODO", "implement later". Phase 8's "actual SHA pinning deferred to Phase 10" is explicit, not a placeholder.

- [ ] **Type consistency**: `Error::SubsetViolation` used consistently after Phase 4 rename. `Tag::SortedMultiA` used consistently from Phase 1 onward.

- [ ] **Bite-sized tasks**: most steps are 2-5 minutes; commits batched per phase.

- [ ] **Phase reviews**: Phases 1, 2, 3, 5, 6 each have a dedicated Opus review task. Phases 4 (mechanical rename), 7 (small docs change), 8 (deferred to 10), 9 (mechanical CHANGELOG/MIGRATION), 10 (release plumbing), 11 (reconciliation) skip dedicated reviews.

- [ ] **Reports persisted**: every dispatched review writes to `design/agent-reports/v0-6-phase-N-review.md`.

- [ ] **End-of-implementation reconciliation**: Phase 11 explicit step 11.2 grep cross-check.

---

## Execution handoff

Plan complete and saved to `design/IMPLEMENTATION_PLAN_v0_6_strip_layer_3.md`. Per user's autonomous-overnight directive:

- Execution model: **inline batch execution** with phase-level reviews dispatched to Opus subagents per the user's `executing-plans` workflow
- Reports persisted to `design/agent-reports/`
- Critical/important review items addressed inline; nits to FOLLOWUPS
- Final reconciliation pass per Phase 11

Begin execution at Phase 1.
