# v0.17 Phase 1 — md-codec `Tag::TrUnspendable` addition (per-phase report)

**Date:** 2026-05-09
**Branch:** `feat/v0.17-tap-multi-leaf-policy`

## Scope

Wire-format addition: new tag for `tr(NUMS, ...)` shape with implicit BIP-341 NUMS H-point internal key. Distinct from `Tag::Tr` because NUMS isn't carried as a `key_index`.

## Artifacts

- `crates/md-codec/src/tag.rs`:
  - `Tag::TrUnspendable` variant added (extension space).
  - `codes()`: `(EXTENSION_PREFIX, Some(0x05))`.
  - `read()`: `0x05 => Ok(Tag::TrUnspendable)`.
  - Module header comment updated: "5 → 6 ops in extension 10-bit space."
  - `tag_unknown_extension_rejected` test re-pinned at 0x06 (next-free).
  - New `tag_tr_unspendable_extension` round-trip test.
- `crates/md-codec/src/tree.rs`:
  - `Body::TrUnspendable { tree: Option<Box<Node>> }` variant.
  - `write_node` arm: `[1-bit has_tree][optional tree]` (no key_index field).
  - `read_node` arm: symmetric.
  - Three round-trip tests: `tr_unspendable_no_tree_round_trip` (11 bits total — 5+5 ext-tag + 1 has_tree); `tr_unspendable_multi_a_2_of_3_round_trip` (canonical 2-of-3 hardware multisig); `tr_unspendable_and_v_inheritance_round_trip` (and-conjunction with verify wrapper).
- `crates/md-codec/src/validate.rs`:
  - `walk_for_placeholders` arm for `Body::TrUnspendable` (recurse into tree; no placeholder to register).
  - **Reviewer-driven addition**: `is_forbidden_leaf_tag` now includes `Tag::TrUnspendable`; pathological `tr(@0, tr_unspendable(...))` nesting rejected. Companion test `tap_tree_leaf_rejects_tr_unspendable` mirrors the existing `tap_tree_leaf_rejects_wsh`.
- `crates/md-codec/src/canonicalize.rs`:
  - `walk_collect_first` / `remap_indices` / `check_placeholder_bounds` arms (each recurses into the optional tree; no key_index because NUMS is implicit).

## Verification

- `cargo test -p md-codec` → all tests pass; +6 net new (5 in unit/tag/tree/validate + 1 reviewer-driven `tap_tree_leaf_rejects_tr_unspendable`).
- `cargo clippy -p md-codec --all-targets -- -D warnings` clean.
- Wire-layout assertion in `tr_unspendable_no_tree_round_trip` confirms 11-bit empty body.

## Per-phase code-reviewer round

`feature-dev:code-reviewer` dispatched. Findings:

- **C: 0**
- **I1** — `Tag::TrUnspendable` missing from `is_forbidden_leaf_tag` (validate.rs:146). Without this, a malformed `tr(@0, tr_unspendable(...))` nesting would be accepted. **Fixed inline** — added `Tag::TrUnspendable` to the forbidden list + companion test `tap_tree_leaf_rejects_tr_unspendable`.
- Reviewer signed off all other axes: wire format correctness, match coverage (4 sites total: tree/validate/canonicalize x3 — no other Body match sites need new arms; canonical_origin.rs and derive.rs catch via wildcard), determinism (no key_index ⇒ nothing to permute), forward compatibility (no existing wire layout changed; new tag is in extension space; v0.16-encoded payloads cannot contain the new bit pattern), doc-comment accuracy.

Net: 0C/0I after the I1 fix.

## Exit gate

- ✅ `Tag::TrUnspendable` round-trips through tag write/read.
- ✅ `Body::TrUnspendable` round-trips through tree write/read at three meaningful shapes (empty, multi_a 2-of-3, and_v inheritance).
- ✅ Validate rejects pathological nesting of TrUnspendable as a tap-script leaf.
- ✅ Canonicalize handles the new variant in all three tree-walk sites.
- ✅ No regressions in existing md-codec tests.
- ✅ Per-phase reviewer 0C/0I.

Phase 1 closed; proceeding to Phase 2.
