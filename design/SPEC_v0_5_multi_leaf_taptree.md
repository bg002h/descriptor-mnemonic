# v0.5 Design Spec: Multi-Leaf TapTree Admission

**Brainstormed**: 2026-04-28 via `superpowers:brainstorming` skill
**Status**: Approved by user; ready for writing-plans handoff
**Per-section agent reviews**: Sections 1-5 reviewed by Opus 4.7 peer agents (revisions folded inline)
**Closes FOLLOWUPS at v0.5.0 ship**: `v0-5-multi-leaf-taptree`
**Carry-forward (NOT closing)**: 9 other open items (verified count post-close from `design/FOLLOWUPS.md`) including `phase-d-tap-leaf-wrapper-subset-clarification`, `phase-d-tap-miniscript-type-check-parity`, `external-pr-1-hash-terminals` (apoelstra/rust-miniscript#1), `slip-0173-register-md-hrp` (PR filed; awaiting registry merge), v0.3-deferred carried items; plus 3 `wont-fix` legacy-exclusion entries (`legacy-pkh-permanent-exclusion`, `legacy-sh-multi-permanent-exclusion`, `legacy-sh-sortedmulti-permanent-exclusion`)

**Wire-format-additive release.** v0.4.x-produced strings continue to validate identically in v0.5.0; only v0.5.0-produced strings using non-trivial TapTrees are rejected by older v0.4.x decoders. This is the same framing as v0.3.x → v0.4.0 (NOT the v0.2.x → v0.3.0 rename pattern).

---

## §1. Scope and Goals

**Goal.** v0.5.0 of `md-codec` admits **multi-leaf taproot trees** under the existing `tr(...)` top-level descriptor — extending v0.4.x's `tr(...)` admittance (currently keypath-only `tr(KEY)` + single-leaf `tr(KEY, leaf)`) to BIP 388's full `tr(KEY, TREE)` form where `TREE` is a non-trivial script tree per BIP 388 §"Taproot tree".

**Decision matrix locked during brainstorming:**

| Knob | Choice | Rationale |
|---|---|---|
| Scope | (a) **pure admission** | Do not broaden the per-leaf miniscript subset. Each leaf is admissible on the same terms as v0.4.x's single-leaf `tr` admits (i.e., `validate_tap_leaf_subset` unchanged). |
| Depth ceiling | (a) **BIP 341 consensus depth (128)** | Same ceiling that the Bitcoin consensus rules impose on control-block paths. No tighter MD-specific cap. |
| Per-leaf cap | (α) **none** | The depth-128 gate is the only structural ceiling. A tree could in principle hold up to 2^128 leaves; the chunking layer + capacity bound the practical maximum well below that. |
| Hardening posture | (B) **peek-before-recurse + depth check** | Re-uses the v0.4 Sh recursion-bomb defense pattern. Decoder peeks `Tag::TapTree (0x08)` BEFORE recursive descent and gates depth so a hostile producer cannot blow the stack with deeply nested `0x08` framing. |

**Non-goals (out of scope for v0.5):**

- Broadening the per-leaf miniscript subset. `validate_tap_leaf_subset` is unchanged. (Carried-forward FOLLOWUPS `phase-d-tap-leaf-wrapper-subset-clarification` and `phase-d-tap-miniscript-type-check-parity` remain open — independent of v0.5 scope.)
- Adding a per-leaf cap. Choice α stands.
- Closing the legacy P2SH gap (`pkh`, `sh(multi)`, `sh(sortedmulti)`). Permanently rejected by v0.4 carve-out; unchanged.
- Inline xpubs / foreign keys (descriptor-codec tag range 0x24–0x31). Still v1+.

**Sh-shape parity (already done at v0.4):** v0.4 added a 3-cell `Sh` restriction matrix with peek-before-recurse defense. v0.5 mirrors that pattern at the TapTree node level: `0x08` is a structural framing tag whose inner byte must be a recognized leaf-script tag — and the decoder peeks it without consuming so a depth-overflow input cannot wedge cursor state.

---

## §2. Wire Format

### Bytecode shape

The taproot TLV under `tr(...)` already exists at v0.4.x in single-leaf form:

```
[Tr=0x06][Placeholder][key_index]                   ← tr(@0/**) — KeyOnly form (no leaf)
[Tr=0x06][Placeholder][key_index][LEAF]             ← tr(KEY, pk(@1/**)) — single-leaf form (v0.4 baseline)
```

v0.5 extends the **leaf-position** with recursive `Tag::TapTree` framing:

```
[Tr=0x06][Placeholder][key_index][Tag::TapTree=0x08][LEFT_SUBTREE][RIGHT_SUBTREE]
```

where each `SUBTREE` is **either** a leaf script (`pk(...)`, `multi(...)`, etc. — same admissibility as v0.4 single-leaf) **or** another `[Tag::TapTree=0x08][LEFT][RIGHT]` framing.

`Tag::TapTree = 0x08` was reserved in v0.2 Phase D and rejected with `Error::PolicyScopeViolation("multi-leaf TapTree reserved for v1+")`. v0.5 activates it.

### Examples

`tr(@0/**, {pk(@1/**), pk(@2/**)})` — symmetric depth-1 tree:
```
[Tr=0x06][Placeholder][0][TapTree=0x08]
  [Pk=??][Placeholder][1]              ← left leaf (depth 1)
  [Pk=??][Placeholder][2]              ← right leaf (depth 1)
```

`tr(@0/**, {pk(@1/**), {pk(@2/**), pk(@3/**)}})` — asymmetric (left depth 1, right depth 2):
```
[Tr=0x06][Placeholder][0][TapTree=0x08]
  [Pk=??][Placeholder][1]                  ← left leaf (depth 1)
  [TapTree=0x08]                           ← right inner-node
    [Pk=??][Placeholder][2]                ← right-left leaf (depth 2)
    [Pk=??][Placeholder][3]                ← right-right leaf (depth 2)
```

### Family-stable SHAs

Generator token bumps `"md-codec 0.4"` → `"md-codec 0.5"`. Both `v0.1.json` and `v0.2.json` SHAs CHANGE at v0.5.0 (new positive fixtures + new negatives per §5). v0.5.x patches will produce byte-identical SHAs (same family-stable promise as v0.2.1 → v0.2.3, v0.4.0 → v0.4.1).

### What does NOT change at v0.5.0

- Single-leaf `tr(KEY, leaf)` bytecode is byte-identical to v0.4.x. Only NON-trivial trees emit the new `0x08` framing.
- KeyOnly `tr(KEY)` (no leaf) is byte-identical to v0.4.x.
- All other top-level descriptors (`wsh`, `wpkh`, `sh(wpkh)`, `sh(wsh)`) are unaffected.
- Per-leaf miniscript subset unchanged (`validate_tap_leaf_subset` constants and call sites preserved).

---

## §3. Decoder Design

### Recursive helper with peek-before-recurse + depth gate

```rust
fn decode_tap_subtree(
    cur: &mut Cursor<'_>,
    keys: &[DescriptorPublicKey],
    depth: usize,
    leaf_counter: &mut usize,
) -> Result<TapTree<DescriptorPublicKey>, Error> {
    let inner_byte = cur.peek_byte()?;          // peek, don't consume
    match Tag::from_byte(inner_byte) {
        Some(Tag::TapTree) => {
            cur.read_byte()?;                   // consume the TapTree byte, then gate; cursor offset on rejection points past the violation byte (matches v0.4 Sh diagnostic precedent)
            if depth > 128 {
                return Err(Error::PolicyScopeViolation(
                    "TapTree depth exceeds BIP 341 consensus maximum (128)".to_string()
                ));
            }
            let left  = decode_tap_subtree(cur, keys, depth + 1, leaf_counter)?;
            let right = decode_tap_subtree(cur, keys, depth + 1, leaf_counter)?;
            TapTree::combine(left, right).map_err(|_| Error::PolicyScopeViolation(
                "TapTree::combine rejected (depth limit at upstream miniscript layer)".to_string()
            ))
        }
        Some(_other_leaf_tag) => {
            // `TapTree::leaf(leaf)` returns a depth-0 seed; each enclosing
            // `TapTree::combine` post-increments depth as the recursion unwinds,
            // so a leaf encountered at recursion-depth N ends up at miniscript-depth N
            // in the final tree (see `taptree.rs:40-50` upstream).
            let index = *leaf_counter;
            *leaf_counter += 1;
            let leaf = decode_tap_miniscript(cur, keys, Some(index))?;
            Ok(TapTree::leaf(leaf))
        }
        None => Err(Error::InvalidBytecode {
            offset: cur.offset(),
            kind: BytecodeErrorKind::UnknownTag(inner_byte),
        }),
    }
}
```

**Routing changes in `decode_tr_inner`** (existing function):

- Read `[Placeholder][key_index]` as today
- Peek next byte:
  - If `Tag::TapTree (0x08)` → call `decode_tap_subtree(cur, keys, depth=1, leaf_counter=&mut 0)`, attach as `tap_tree`
  - If a leaf-script tag → existing v0.4.x single-leaf path (preserved verbatim — no behavior change for v0.4.x-shaped inputs)
  - If end-of-bytecode → `tr(KEY)` KeyOnly form (preserved verbatim)
  - If unknown tag → `InvalidBytecode { kind: UnknownTag }`

**Depth semantics.** The `depth` argument represents "the framing-level this call is about to read". Initial value = 1 (caller `decode_tr_inner` invokes the helper to read the FIRST `[TapTree]` framing at the root of the script-tree subtree). After a TapTree byte is consumed at depth=N, the recursion re-enters at depth=N+1 to read children. Each TapTree framing consumed at depth=N produces leaves under it at miniscript-depth N. BIP 341's `TAPROOT_CONTROL_MAX_NODE_COUNT = 128` therefore caps the helper at consuming framings up to depth=128 inclusive — leaves discovered at depth=129 (no framing read there) end up at miniscript-depth 128, the legal maximum. The gate `if depth > 128` fires at depth=129 when the next byte is `Tag::TapTree`, rejecting only the case that would push leaves past depth 128. H1 fixture (§5) covers the legal-128 boundary; H2 covers the 129 rejection.

**Hostile-input invariants** (peek-before-recurse rationale):

- `peek_byte()` does NOT advance the cursor. If the depth gate fails BEFORE consume, cursor state is fully recoverable for diagnostics (`offset()` reports the byte that triggered).
- The `cur.read_byte()` to commit-consume `0x08` happens AFTER `Tag::from_byte` returns `Some(TapTree)` BUT BEFORE the depth check — meaning for a too-deep tree, the cursor IS advanced past the `0x08` of the bottommost violation. This is intentional: `offset()` then points at the next-frame's first byte, which is what diagnostic tooling expects (cf. v0.4 Sh restriction matrix diagnostics).

### Leaf-index propagation

The `leaf_counter` argument provides **DFS pre-order leaf indexing** — left-subtree leaves are numbered before right-subtree leaves at any given inner node.

**Signature changes (NEW at v0.5; not present at v0.4.x):**

- `decode_tap_miniscript(cur, keys)` → `decode_tap_miniscript(cur, keys, leaf_index: Option<usize>)` (private)
- `decode_tap_terminal(cur, keys, ...)` → adds `leaf_index: Option<usize>` parameter (private)
- `validate_tap_leaf_subset(ms)` → `validate_tap_leaf_subset(ms, leaf_index: Option<usize>)` (**public**, additive but technically breaking — no known external callers)

The single-leaf decode path in `decode_tr_inner` passes `Some(0)`; the recursive helper threads the running counter as `Some(index)`. The leaf-index value is plumbed through to `validate_tap_leaf_subset` so violation diagnostics can name the offending leaf.

Leaf index propagates into:
- `Error::TapLeafSubsetViolation { operator, leaf_index }` (NEW field on existing variant; see §4)
- `decode_report.tap_leaves[]` (NEW field on `DecodeReport`, populated for all `tr` decodes — single-leaf and multi-leaf alike; see §4)
- BIP 388 §"Taproot tree" key derivation paths (out-of-band; v0.5 records the index, callers may use it)

### Single-leaf path preserved

The existing v0.4.x single-leaf decode path is preserved verbatim (no detour through `decode_tap_subtree`). This guarantees **byte-identical decode of v0.4.x-shaped inputs** — no risk that the new framing accidentally changes single-leaf decode semantics.

---

## §4. Encoder Design + Type/Error Updates

### Encoder helper

```rust
fn encode_tap_subtree(
    leaves: &[(u8, &Arc<Miniscript<DescriptorPublicKey, Tap>>)],
    cursor: &mut usize,
    target_depth: u8,
    out: &mut Vec<u8>,
    placeholder_map: &HashMap<DescriptorPublicKey, u8>,
) -> Result<(), Error> {
    let leaf_depth = leaves[*cursor].0;
    if leaf_depth == target_depth {
        let leaf_index = *cursor;
        let ms = leaves[*cursor].1;
        validate_tap_leaf_subset(ms, Some(leaf_index))?;
        ms.encode_template(out, placeholder_map)?;
        *cursor += 1;
    } else if leaf_depth > target_depth {
        out.push(Tag::TapTree.as_byte());
        encode_tap_subtree(leaves, cursor, target_depth + 1, out, placeholder_map)?;
        encode_tap_subtree(leaves, cursor, target_depth + 1, out, placeholder_map)?;
    }
    // leaf_depth < target_depth is unreachable given DFS pre-order from upstream `TapTree::leaves()`
    Ok(())
}
```

**Routing changes in `encode_tr`** (existing function):

```rust
match desc.tap_tree() {
    None => {
        // tr(KEY) KeyOnly — bytecode unchanged from v0.4.x
    }
    Some(tap_tree) => {
        let leaves: Vec<(u8, &Arc<_>)> = tap_tree.leaves()
            .map(|item| (item.depth(), item.miniscript()))
            .collect();
        if leaves.len() == 1 && leaves[0].0 == 0 {
            // single-leaf tr(KEY, leaf) — bytecode unchanged from v0.4.x
            let leaf_index = 0;
            validate_tap_leaf_subset(leaves[0].1, Some(leaf_index))?;
            leaves[0].1.encode_template(out, placeholder_map)?;
        } else {
            // multi-leaf — new 0x08 framing
            let mut cursor = 0;
            encode_tap_subtree(&leaves, &mut cursor, 1, out, placeholder_map)?;
            // post: cursor == leaves.len()
        }
    }
}
```

Single-leaf detection: `leaves.len() == 1 && leaves[0].0 == 0`. (The depth-0 check matters: rust-miniscript `TapTree::leaf(ms)` produces depth 0; deeper single-leaf trees are not produced by upstream's API but the `== 0` guard makes the carve-out tight.)

**Encoder depth invariant.** The encoder does NOT add a depth-128 check of its own. Justification: any `Descriptor::Tr` reaching `encode_tr` was constructed via rust-miniscript's `TapTree::combine`, which already enforces depth ≤ 128 at construction time (verified at `rust-miniscript-fork/src/descriptor/tr/taptree.rs:40-50`). A defensive over-deep check in the encoder would be unreachable code. The decoder's depth gate is the actual ceiling; encoder relies on the upstream invariant. (If a future rust-miniscript change relaxes `combine`'s rejection, this assumption breaks and a defensive check should be added at that time.)

**v0.4 single-leaf-with-non-zero-depth subsumption.** v0.4 explicitly rejected single-leaf `TapTree`s with `depth != 0` via `PolicyScopeViolation`. Under v0.5 such inputs (which rust-miniscript doesn't normally produce, but aren't structurally impossible) flow through the multi-leaf path and emit `0x08` framing. No known producer emits this shape; the v0.4 rejection was theoretical defense. Any v0.4 test asserting that rejection must be removed or re-classified — see §5 infrastructure-modifications.

### Error type extension (additive on `#[non_exhaustive]` enum)

```rust
// Before (v0.4.x):
TapLeafSubsetViolation { operator: String },

// After (v0.5.0):
TapLeafSubsetViolation { operator: String, leaf_index: Option<usize> },
```

`leaf_index` is `Option<usize>` to remain ergonomic for paths that don't yet plumb the index. The 3 construction sites in the codebase (`encode.rs:443` Terminal encoder catch-all; `encode.rs:487` `validate_tap_leaf_terminal` catch-all; `decode.rs:691` `decode_tap_terminal` catch-all) all get explicit `leaf_index: Some(idx)` or `None`. Separately, the 2 call sites of `validate_tap_leaf_subset` (`encode.rs:154`, `decode.rs:276`) get the new `Some(leaf_index)` argument plumbed.

**Backwards compatibility**: `Error` is `#[non_exhaustive]`. Adding a field to an existing struct variant is non-breaking for **constructors outside this crate** (covered by `#[non_exhaustive]`) and **wildcard `match` arms** (`_` or `..`). It IS breaking for **field-exhaustive destructure patterns** like `Error::TapLeafSubsetViolation { operator } => ...`, which become "missing field `leaf_index`" errors. To avoid this, also annotate the variant itself as `#[non_exhaustive]` (rust 1.84+; project MSRV 1.85). Downstream destructure patterns will then need to add `..` (a future-proof idiom anyway). No known external destructure sites exist today.

### `decode_report.tap_leaves[]` (NEW field; NEW struct)

v0.5 introduces a NEW `tap_leaves: Vec<TapLeafReport>` field on `DecodeReport` (verified absent at v0.4.x: `crates/md-codec/src/decode_report.rs` defines `DecodeReport` with `outcome`, `corrections`, `verifications`, `confidence` only). Addition is non-breaking because `DecodeReport` is `#[non_exhaustive]`.

NEW public type `TapLeafReport`:

```rust
#[non_exhaustive]
pub struct TapLeafReport {
    pub leaf_index: usize,
    pub miniscript: Arc<Miniscript<DescriptorPublicKey, Tap>>,
    pub depth: u8,
}
```

Populated for all `tr(...)` decodes in DFS pre-order — single-leaf decodes get a 1-element vec (`{ leaf_index: 0, miniscript, depth: 0 }`); KeyOnly `tr(KEY)` gets an empty vec; multi-leaf decodes get N elements with `leaf_index: 0..N` and depths matching the tree shape. `leaf_index` aligns with the leaf-index propagation through decoder + encoder, completing the round-trip.

### BIP draft updates (line-level inventory)

Checked against `bip/bip-mnemonic-descriptor.mediawiki` at HEAD (v0.4.1 ship state):

| Section | Lines | Change |
|---|---|---|
| §"Top-level descriptor scope" | 85-89 | Add `tr(KEY, TREE)` to admittance list (alongside existing `tr(KEY)`) |
| §"Taproot tree" | 534-540 | Substantive rewrite. Drop single-leaf-only carve-out at line 536; specify recursive bytecode form via `Tag::TapTree` framing. Note BIP 388 grammar uses curly-brace source form `{LEFT, RIGHT}`; MD bytecode is the encoding of that grammar. |
| Tag table | 391 | Whole-row rewrite for `0x08 TapTree`: status (was reserved/rejected → now active), description (multi-leaf TapTree inner-node framing), reference column (this BIP §"Taproot tree") |
| §FAQ "Why was multi-leaf TapTree deferred?" | (existing) | KEEP as history. |
| §FAQ "Why does v0.5 admit multi-leaf TapTree?" | (NEW) | Resolution Q&A: scope (a) pure admission, depth-128 ceiling, peek-before-recurse hardening |
| §FAQ "What about `tr(KEY)` single-leaf?" | (existing) | EXPAND: single-leaf is now a degenerate case of multi-leaf admission; bytecode unchanged |
| §"Test vectors" | 884-892 | Update fixture references (renamed positive fixture per §5) |
| §"Status" | (line near top) | UNCHANGED. Still "Pre-Draft, AI + reference implementation, awaiting human review". |
| TODO Phase 7 markers | 860-861 | Resolve markers placed during v0.2 Phase D when `0x08` was reserved |

---

## §5. Test Corpus + Hostile-Input Fixtures

**Target enumeration** (count is bounded by the table totals below; final harness count may add 2-3 implicit per-fixture variants depending on how `gen_vectors` expansion lands):

- Positive fixtures NEW: 6 (T1, T3-T7); RENAMED: 1 (T2)
- Negative fixtures NEW: 9 (N1-N9)
- Hostile inline tests NEW: 5 (H1-H5)
- Round-trip inline tests NEW: 4 (RT1-RT4)
- Leaf-index inline tests NEW: 3 (LI1-LI3)
- Parser-roundtrip inline tests NEW: 2 (PR1-PR2)

**Sum**: 29 NEW + 1 RENAMED listed in tables. RENAMED adds 0 to the count (existing v0.4.x test under new filename). Floor: 609 baseline + 29 = **≥638 tests + 0 ignored**. Realistic target: ≥640 once `gen_vectors` expansion produces encode/decode variants for the 6 NEW positive fixtures (at v0.4 cadence, +1 round-trip pair per positive fixture).

### T1-T7: Positive corpora

Added to `crates/md-codec/tests/vectors/v0.2.json` (fixture file format unchanged):

| ID | Fixture | Tree shape | Notes |
|---|---|---|---|
| T1 | `tr_keypath_only_md_v0_5` | KeyOnly | Byte-identical to v0.4.x; included as a regression anchor |
| T2 | `tr_single_leaf_pk_md_v0_5` (RENAMED from v0.4 fixture) | Single leaf, depth 0 | Byte-identical to v0.4.x single-leaf bytecode |
| T3 | `tr_two_leaf_symmetric_md_v0_5` | `{pk(@1), pk(@2)}` — depth 1 / depth 1 | Smallest multi-leaf case |
| T4 | `tr_three_leaf_left_heavy_md_v0_5` | `{pk(@1), {pk(@2), pk(@3)}}` — depth 1 / 2 / 2 | Asymmetric tree |
| T5 | `tr_three_leaf_right_heavy_md_v0_5` | `{{pk(@1), pk(@2)}, pk(@3)}` — depth 2 / 2 / 1 | Mirror of T4 (different bytecode by construction) |
| T6 | `tr_multi_leaf_with_multi_md_v0_5` | `{pk(@1), multi(2, @2, @3)}` | Mix of leaf script types |
| T7 | `tr_multi_leaf_chunking_boundary_md_v0_5` | Tree sized to push 1-string regular boundary into chunking | Chunking-boundary regression coverage |

Also: at least one **Coldcard-shape parity fixture** if such a multi-leaf shape exists in the Coldcard test suite (defer to implementer if no such corpus exists).

### N1-N9: Negative decode-side fixtures (`v0.2.json`)

All N3-N7 fixtures use the offending operator as the LEFT leaf of a 2-leaf depth-1 tree (`{<offender>, pk(@1)}`), so the offending leaf is at `leaf_index = 0`.

| ID | Fixture | Hostile shape | Expected error |
|---|---|---|---|
| N1 | `n_taptree_single_inner_under_tr` | `[Tr][Placeholder][0][TapTree][LEFT_LEAF]` (only 1 child) | `InvalidBytecode { kind: UnexpectedEnd }` (cursor runs out reading right child) |
| N2 | `n_taptree_three_inners_under_tr` | `[TapTree][LEAF][LEAF][LEAF]` (3 children) | Excess byte after right child → `InvalidBytecode { kind: TrailingBytes }` |
| N3 | `n_taptree_inner_wpkh` | `{wpkh-leaf, pk(@1)}` | `TapLeafSubsetViolation { operator: "wpkh", leaf_index: Some(0) }` |
| N4 | `n_taptree_inner_sh` | `{sh-leaf, pk(@1)}` | `TapLeafSubsetViolation { operator: "sh", leaf_index: Some(0) }` |
| N5 | `n_taptree_inner_wsh` | `{wsh-leaf, pk(@1)}` | `TapLeafSubsetViolation { operator: "wsh", leaf_index: Some(0) }` |
| N6 | `n_taptree_inner_tr` | `{tr-leaf, pk(@1)}` | `TapLeafSubsetViolation { operator: "tr", leaf_index: Some(0) }` |
| N7 | `n_taptree_inner_pkh` | `{pkh-leaf, pk(@1)}` | `TapLeafSubsetViolation { operator: "pkh", leaf_index: Some(0) }` |
| N8 | `n_taptree_unknown_tag_inner` | `[TapTree]` containing an unallocated tag byte | `InvalidBytecode { kind: UnknownTag }` |
| N9 | `n_taptree_at_top_level` | `[TapTree]` as top-level descriptor (no `Tr` outer) | `PolicyScopeViolation` from top-level dispatcher (existing dispatcher message at `decode.rs:98-100` says "v0.4 does not support top-level tag TapTree" — v0.5 must update the message text since `0x08` is now active inside `tr`; suggested replacement: "TapTree (0x08) is not a valid top-level descriptor; it appears only inside `tr(KEY, TREE)`") |

**Critical corrections folded inline (Section 5 review + holistic review):** (1) N3-N7 produce `TapLeafSubsetViolation { operator, leaf_index }`, NOT `InvalidBytecode` or `PolicyScopeViolation`. The decoder routes these through `decode_tap_terminal`, which calls `validate_tap_leaf_subset` and produces the operator-named subset-violation diagnostic. (2) N3-N7 leaf-index pinned to `Some(0)` for deterministic test assertions. (3) N9 produces `PolicyScopeViolation` from the top-level dispatcher (NOT `UnknownTag` — `Tag::from_byte(0x08)` returns `Some(Tag::TapTree)` so the "unknown tag" path is not hit; only the "valid tag in wrong scope" path applies). (4) N1 uses `UnexpectedEnd` (real variant) not `TruncatedBytecode` (does not exist).

### H1-H5: Hostile-input fixtures

Inline Rust tests (NOT fixture-driven; depth construction via direct bytecode emission):

| ID | Test name | Construction | Expected behavior |
|---|---|---|---|
| H1 | `accepts_taptree_with_leaves_at_miniscript_depth_128` | Build a left-spine of 128 `[TapTree]` framings + leaves at the bottom (final tree has at least one leaf at miniscript-depth 128) | Decode succeeds; helper consumes its 128th framing at recursion-depth=128 (gate `> 128` does NOT fire); leaves at miniscript-depth 128 |
| H2 | `rejects_taptree_at_miniscript_depth_129` | 129 `[TapTree]` framings + leaves at the bottom | `PolicyScopeViolation("TapTree depth exceeds BIP 341 consensus maximum (128)")`; gate fires at recursion-depth=129 reading the 129th `[TapTree]` byte |
| H3 | `rejects_taptree_with_truncated_subtree` | `[TapTree]` then EOF | `InvalidBytecode { kind: UnexpectedEnd }` |
| H4 | `rejects_deeply_nested_recursion_bomb` | Pathological construction: 10K `[TapTree]` bytes with no leaves | Rejection at recursion-depth 129 BEFORE stack overflow (rationale: peek-before-recurse + depth gate fires at the BIP 341 boundary, well below stack-overflow risk) |
| H5 | `rejects_taptree_unrecognized_inner_at_depth` | `[TapTree][TapTree][unallocated_byte]` | `InvalidBytecode { kind: UnknownTag }` (depth-aware error reporting at the violation site) |

**Hostile-input rationale:** H1 is the legal-128 boundary; H2 is the first illegal step beyond. The gate fires when `depth > 128` — at recursion-depth 129 reading a `[TapTree]` byte. This is the smallest 1-byte difference between a legal and rejected input. H3-H5 cover orthogonal hostile shapes (truncation, recursion bomb, depth-deep unknown tag).

### Round-trip + index propagation tests

| ID | Test name | Asserts |
|---|---|---|
| RT1 | `roundtrip_two_leaf_symmetric` | `decode(encode(T3)) == T3` |
| RT2 | `roundtrip_three_leaf_asymmetric` | `decode(encode(T4)) == T4` |
| RT3 | `roundtrip_multi_leaf_with_multi` | `decode(encode(T6)) == T6` |
| RT4 | `t4_left_heavy_and_t5_right_heavy_emit_distinct_bytecodes` | `encode(T4) != encode(T5)` (defense against accidental sym-bug — mirrors v0.4 Sh inner-shape coverage) |
| LI1 | `decode_report_populates_leaf_index_dfs_preorder` | `decode_report.tap_leaves` is `[(0, depth_1), (1, depth_2), ...]` for T4 |
| LI2 | `tap_leaf_subset_violation_carries_leaf_index` | N3-N7 errors expose `leaf_index: Some(<expected index>)` |
| LI3 | `single_leaf_tr_uses_leaf_index_zero` | T2 produces `leaf_index = 0`, byte-identical to v0.4.x |

### Parser-roundtrip equivalence

| ID | Test name | Asserts |
|---|---|---|
| PR1 | `parser_roundtrip_t4` | `Descriptor::from_str(encode_then_decode(T4_source)).unwrap() == Descriptor::from_str(T4_source).unwrap()` |
| PR2 | `parser_roundtrip_t6_with_multi` | Same for T6 |

### Infrastructure modifications (NOT new tests)

These are **modifications to existing tests**, not separate test cases:

- `gen_vectors --verify` regenerates v0.1.json + v0.2.json with bumped generator token `"md-codec 0.5"`; existing test that asserts vector SHA pinning is updated to the new SHAs.
- Existing v0.4.x single-leaf round-trip test is preserved verbatim (regression anchor).
- Renamed fixture (T2): the v0.4.x single-leaf fixture renamed to follow v0.5 naming convention; bytecode unchanged.
- Any v0.4 test asserting `PolicyScopeViolation` for "single-leaf TapTree with depth ≠ 0" (subsumed by §4 encoder change) must be removed or re-classified as a positive test. Implementer to enumerate during Phase 4; no known producer emits this shape so the test (if it exists) is theoretical-defense.
- Any v0.4 test asserting the old top-level dispatcher message "v0.4 does not support top-level tag TapTree" must be updated to the new message text per §6 CHANGELOG (or have its assertion loosened).

### Coverage gap closures (folded from Section 5 review)

- **Gap 1** (chunking boundary) — closed by T7
- **Gap 2** (T4-vs-T5 symmetric/asymmetric distinction) — closed by RT4
- **Gap 3** (`decode_report.tap_leaves[]` index plumbing) — closed by LI1
- **Critical 1** (depth-test naming ambiguity) — H1/H2 renamed
- **Critical 2** (wrong expected errors for inner-tag violations) — N3-N7 corrected to `TapLeafSubsetViolation`

---

## §6. Migration + Release Framing

### SemVer cut: v0.4.x → v0.5.0

Minor bump (0.4.x → 0.5.0). Same rationale as v0.3.x → v0.4.0:

- **Wire-format-additive**: v0.4.x-produced strings continue to validate identically in v0.5.0. The `0x08 = TapTree` tag was already RESERVED at v0.2 Phase D and rejected with `PolicyScopeViolation("multi-leaf TapTree reserved for v1+")`. v0.5.0 admits it. Older v0.4.x decoders reject v0.5.0-produced multi-leaf strings; v0.5.0 decoders accept everything v0.4.x produced.
- **No breaking wire change**, no rename, no MSRV bump.
- **Family-stable promise** carries forward: generator token bumps `"md-codec 0.4"` → `"md-codec 0.5"`. Both `v0.1.json` and `v0.2.json` SHAs CHANGE at v0.5.0; v0.5.x patches will produce byte-identical SHAs.

### Past-release framing

**No deprecation banners on v0.4.x tags.** v0.4.x remains a valid smaller-surface subset — same pattern as v0.3.x → v0.4.0. Users pinned to v0.4.x can stay there indefinitely if they don't need multi-leaf TapTree.

### Documentation deltas

**MIGRATION.md** — new section `## v0.4.x → v0.5.0`:
- "What changed": `tr(KEY, TREE)` admittance with non-trivial script trees (BIP 388 §"Taproot tree" subset)
- "What didn't change": wire format for v0.4.x-shaped inputs is byte-identical
- "How to upgrade": `cargo update -p md-codec --precise 0.5.0`; no code changes required for users not constructing multi-leaf trees
- "New encoder behavior": `Descriptor::Tr` with non-trivial `TapTree` (anything other than `TapTree::leaf(ms)` or KeyOnly) now encodes successfully instead of returning `PolicyScopeViolation`
- "New decoder behavior": bytecode containing `Tag::TapTree (0x08)` now decodes successfully instead of returning `PolicyScopeViolation("multi-leaf TapTree reserved for v1+")`

**CHANGELOG.md** — full v0.4.x → v0.5.0 changelog under `## [0.5.0] — 2026-MM-DD`:
- Added: `tr(KEY, TREE)` multi-leaf TapTree admittance
- Added: `Tag::TapTree (0x08)` now active (was reserved/rejected since v0.2 Phase D)
- Added: BIP 341 control-block depth-128 enforcement during decode (peek-before-recurse)
- Added: `DecodeReport.tap_leaves: Vec<TapLeafReport>` field (NEW field on existing struct — non-breaking via `#[non_exhaustive]`)
- Added: `TapLeafReport` public struct (`leaf_index`, `miniscript`, `depth`)
- Changed: `Error::TapLeafSubsetViolation` extended with `leaf_index: Option<usize>` field; variant now `#[non_exhaustive]` so destructure patterns must use `..` (additive — non-breaking for wildcard `match` arms; breaking for field-exhaustive destructures, but no known external consumers)
- Changed: `validate_tap_leaf_subset(ms)` → `validate_tap_leaf_subset(ms, leaf_index: Option<usize>)` — public API additive but technically breaking (no known external callers)
- Changed: top-level dispatcher message for `0x08`-at-top-level updated from "v0.4 does not support top-level tag TapTree" to "TapTree (0x08) is not a valid top-level descriptor; it appears only inside `tr(KEY, TREE)`"
- Removed: v0.4 single-leaf-with-non-zero-depth `PolicyScopeViolation` rejection (subsumed by multi-leaf path; theoretical-only, no producer emits this shape)
- Changed: `v0.1.json` SHA `<new>`, `v0.2.json` SHA `<new>` (new fixtures; family generator token `"md-codec 0.5"`)
- Wire format: v0.4.x-shaped inputs byte-identical

### FOLLOWUPS housekeeping

**Close at v0.5.0 ship**:
- `v0-5-multi-leaf-taptree` → resolved (this release IS the resolution)

**Carry forward (NOT closing — independent of v0.5 scope)**:
- `phase-d-tap-leaf-wrapper-subset-clarification` — v0.5 does NOT broaden the per-leaf miniscript subset
- `phase-d-tap-miniscript-type-check-parity` — v0.5 does NOT touch the type-check-parity question
- `apoelstra/rust-miniscript#1` — external, unchanged
- BIP header / SLIP-0173 / 4 v0.3-deferred items / 5 wont-fix entries — all unchanged

**Net FOLLOWUPS state at v0.5.0 ship**: 9 open + 3 wont-fix (was 10 open + 3 wont-fix at v0.4.1; close 1 = `v0-5-multi-leaf-taptree`, no new opens unless something is discovered during implementation). Counts verified against `design/FOLLOWUPS.md` HEAD by status-line tally.

### Release sequencing

**11-phase plan template** (mirrors v0.4 cadence):

1. Spec ratification + plan draft (THIS document is the spec; plan comes from `writing-plans` skill next)
2. Type wiring + decoder — extend `Error::TapLeafSubsetViolation` with `leaf_index` field (variant `#[non_exhaustive]`); add `validate_tap_leaf_subset(_, leaf_index)` parameter; add `DecodeReport.tap_leaves` field + `TapLeafReport` struct; add `decode_tap_subtree` recursive helper (peek-before-recurse + `depth > 128` gate); route `Tag::TapTree` in `decode_tr_inner` → recurse. Land call-site updates in the same phase to keep tree green.
3. Top-level dispatcher message update — replace "v0.4 does not support top-level tag TapTree" string per §6 CHANGELOG.
4. Encoder — add `encode_tap_subtree` walking depth-annotated `TapTree::leaves()`, dispatch from `encode_tr` based on multi-leaf detection; remove v0.4 single-leaf-non-zero-depth rejection (subsumed); enumerate any tests that assert it.
5. Roundtrip glue — `decode_report.tap_leaves[]` population for single + multi + KeyOnly tr; parser-roundtrip equivalence; tap_tree comparator helper
6. Test corpus — add 29 NEW + 1 RENAMED fixtures and inline tests per §5; regenerate `v0.1.json` + `v0.2.json`
7. BIP doc updates — line edits per §4 inventory
8. CLI surface — `md encode "tr(@0/**, {pk(@1/**), pk(@2/**)})"` works automatically (no new flags); add at least one CLI integration test
9. Final cumulative reviewer pass (Opus 4.7) — verdict gate before tag
10. CHANGELOG + MIGRATION + release notes draft
11. Tag + push + GitHub release

**Audit trail expectations**:
- `design/SPEC_v0_5_multi_leaf_taptree.md` (this spec)
- `design/IMPLEMENTATION_PLAN_v0_5_multi_leaf_taptree.md` (next, via writing-plans)
- `design/agent-reports/v0-5-multi-leaf-phase-N-implementer.md` per phase
- `design/agent-reports/v0-5-multi-leaf-final-reviewer.md` cumulative

### Quality gates (target at tag)

- **≥638 tests passing + 0 ignored** (609 baseline + 29 new per §5; ≥640 once `gen_vectors` expansion adds round-trip pairs for new positives)
- **3-OS CI green**
- **MSRV 1.85** (unchanged)
- **`gen_vectors --verify` PASS** for both `v0.1.json` and `v0.2.json` post-regeneration
- **Final reviewer pass** verdict READY or READY-WITH-MINOR-FIXES (fixes inline before tag)

### What this release deliberately does NOT do

- **Does NOT broaden tap-leaf miniscript subset.** `validate_tap_leaf_subset` is unchanged. Per-leaf admissibility is exactly what v0.4.x admits in single-leaf `tr(KEY, leaf)`.
- **Does NOT add a per-leaf cap.** Choice α (no per-leaf cap) — depth-128 is the only structural ceiling.
- **Does NOT add hostile-input metering beyond the depth check.** Peek-before-recurse + depth-128 gate is sufficient (H4 fixture demonstrates 129-deep rejection cleanly).
- **Does NOT change `tr(KEY)` single-leaf encoding.** Single-leaf bytecode remains exactly what v0.4.x produces (no leading `Tag::TapTree`); only multi-leaf trees emit the new `0x08` framing.
- **Does NOT close the BIP 388 modern surface gap.** Legacy `pkh`, `sh(multi)`, `sh(sortedmulti)` remain permanently rejected (the v0.4 carve-out stands).
