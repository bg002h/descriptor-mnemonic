# Phase v0.2 D — Taproot Tr / single-leaf TapTree (`p2-taproot-tr-taptree`)

> **Forward note (added 2026-04-28):** the per-leaf Coldcard-subset enforcement
> shipped here (`validate_tap_leaf_subset` invoked on every encode and decode)
> is being **undone** in v0.6 as part of a deliberate scope reframing. The
> rationale and supersession plan are documented in
> [`design/MD_SCOPE_DECISION_2026-04-28.md`](../MD_SCOPE_DECISION_2026-04-28.md).
> This report stays unchanged as a faithful record of the v0.2 reasoning;
> read it for that context, not as a description of v0.6+ behaviour.

**Status:** DONE

**Commit SHA:** `6f6eae9` (cherry-picked onto main from worktree commit `267036f` because the worktree branch was cut from origin/main and didn't include the local-only Phase D decision-log commit `24a7a4b`)

**Branch:** `worktree-agent-a9faad48b7b3d2fc7` (worktree, now merged); upstream commit `6f6eae9` on `main`

## Summary

Removed the v0.1 `Descriptor::Tr` rejection on both encoder and decoder paths
and shipped single-leaf taproot per BIP §"Taproot tree" (D-1) with the
Coldcard per-leaf miniscript subset enforced (D-2). Multi-leaf TapTree
encoding remains reserved for v1+: any `Tag::TapTree` (`0x08`) in v0.2
input or any non-zero-depth leaf surfaces `PolicyScopeViolation` with a
clear "reserved for v1+" message. Top-level-only (D-3) and the shared
operator subset (D-4) are honoured: nested `tr()` inside `wsh()` is
rejected the same way nested `wsh()` is, and `multi_a` / `or_d` /
`and_v` / `older` / `pk` / `pkh` / `c:` / `v:` reuse their existing tag
encodings for cross-context byte-format consistency.

## Files changed

| File | Δ | Notes |
|---|---|---|
| `crates/wdm-codec/src/error.rs` | modified | New `Error::TapLeafSubsetViolation { operator: String }` variant; pipeline-stage doc updated. |
| `crates/wdm-codec/src/bytecode/cursor.rs` | modified | Added `is_empty()` and `peek_byte()` helpers (used by the optional-leaf decoder dispatch). |
| `crates/wdm-codec/src/bytecode/encode.rs` | modified (~+185 LOC) | Replaced the `Descriptor::Tr` rejection with the actual encoder (Tag::Tr + internal-key + optional single leaf). Added `EncodeTemplate` impls for `Miniscript<DescriptorPublicKey, Tap>` and `Terminal<DescriptorPublicKey, Tap>`. Added `validate_tap_leaf_subset()` (recursive AST walk) and `tap_terminal_name()` for diagnostics. Updated `rejects_tr_top_level` test to `rejects_tr_inline_internal_key` reflecting the new accept-then-reject-inline-key behaviour. |
| `crates/wdm-codec/src/bytecode/decode.rs` | modified (~+165 LOC) | Replaced the `Tag::Tr` rejection with `decode_tr_inner()`. Added `decode_tap_miniscript()` and `decode_tap_terminal()` for the Tap-context dispatch. Updated `decode_rejects_top_level_taproot` to `decode_top_level_taproot_truncated_internal_key`. |
| `crates/wdm-codec/tests/conformance.rs` | modified | New `rejects_tap_leaf_subset_violation` rejection test (Layer 7). |
| `crates/wdm-codec/tests/error_coverage.rs` | modified | New `TapLeafSubsetViolation` entry in the exhaustiveness mirror enum. |
| `crates/wdm-codec/tests/taproot.rs` | NEW (~190 LOC) | 8 tests: 4 round-trips (key-path-only, single-leaf `pk`, `multi_a`, nested or_d/and_v/older), 2 subset rejections (`sha256` and `thresh`), 2 wire-format rejections (decoded `Tag::TapTree` and nested `Tag::Tr` mid-tree). |
| `bip/bip-wallet-descriptor-mnemonic.mediawiki` | modified | (1) Heading `====Taproot tree (forward-defined)====` → `====Taproot tree====`. (2) Tag table entry for `0x08` clarifies "reserved for v1+ multi-leaf TapTree; v0 single-leaf encodes the leaf miniscript directly without `0x08`". (3) Subset clause expanded to mention the `c:` and `v:` wrappers. (4) New `=====Byte-layout examples=====` subsection with annotated bytecode for `tr(@0/**)`, `tr(@0/**, pk(@1/**))`, `tr(@0/**, multi_a(2, @1/**, @2/**, @3/**))`, and the nested `tr(@0/**, or_d(pk(@1/**), and_v(v:older(144), pk(@2/**))))` shape. All bytecode strings regenerated from the live encoder via `wdm bytecode` rather than hand-derived. |

## Wrapper-terminal subset decision

The Coldcard subset documents the surface forms `pk`, `pk_h`, `multi_a`,
`or_d`, `and_v`, `older` — but BIP 388 / miniscript expand `pk(K)` to
`c:pk_k(K)` and `and_v(v:..., ...)` to `and_v(Verify(...), ...)`. So the
canonical BIP 388 string form of any Coldcard-allowed leaf miniscript
necessarily includes `Terminal::Check` (`c:`) and `Terminal::Verify`
(`v:`) wrappers in the parsed AST. Rejecting either at the encoder would
make the entire Coldcard-allowed surface area unencodable.

**Decision:** allow `Terminal::Check` and `Terminal::Verify`; reject
every other miniscript wrapper (`a:` / `s:` / `d:` / `j:` / `n:`).
Every rejected wrapper is documented in `validate_tap_leaf_terminal()`
as a default-reject; the decision rationale is captured inline in
`validate_tap_leaf_subset()`'s rustdoc.

This decision is conservative: Coldcard's documented edge-firmware
support set may already accept additional wrappers (notably `s:` for
`thresh` chains in tap context), but the Coldcard public docs don't
enumerate those. Filing
`phase-d-tap-leaf-wrapper-subset-clarification` in FOLLOWUPS so v0.3
can revisit if a signer documents a wider safe wrapper set.

## Test coverage breakdown

Total new tests: 9 (8 in `tests/taproot.rs`, 1 in `tests/conformance.rs`).

### Round-trip positives (`tests/taproot.rs`)

| Test | Policy | Pinned tail bytes |
|---|---|---|
| `taproot_key_path_only_round_trips` | `tr(@0/**)` | `06 32 00` |
| `taproot_single_leaf_pk_round_trips` | `tr(@0/**, pk(@1/**))` | `06 32 00 0c 1b 32 01` |
| `taproot_single_leaf_multi_a_round_trips` | `tr(@0/**, multi_a(2, @1/**, @2/**, @3/**))` | `06 32 00 1a 02 03 32 01 32 02 32 03` |
| `taproot_nested_subset_round_trips` | `tr(@0/**, or_d(pk(@1/**), and_v(v:older(144), pk(@2/**))))` | structural round-trip only |

Each test calls `policy.to_bytecode → from_bytecode → to_bytecode` and asserts
byte-stable equality (mirroring the v0.1 `assert_roundtrips` discipline).

### Subset rejections (`tests/taproot.rs`)

| Test | Policy | Expected error |
|---|---|---|
| `taproot_rejects_out_of_subset_sha256` | `tr(@0/**, and_v(v:sha256(...), pk(@1/**)))` | `TapLeafSubsetViolation { operator: "sha256" }` |
| `taproot_rejects_wrapper_alt_outside_subset` | `tr(@0/**, thresh(2, pk(@1/**), s:pk(@2/**), s:pk(@3/**)))` | `TapLeafSubsetViolation { operator: "thresh" }` (or `"s:"` if walked first) |

### Wire-format rejections (`tests/taproot.rs`)

| Test | Synthetic input | Expected error |
|---|---|---|
| `taproot_rejects_decode_tag_taptree` | valid `tr(@0/**)` bytecode + appended `0x08` | `PolicyScopeViolation` mentioning "TapTree" / "v1+" |
| `taproot_rejects_nested_tr_inside_wsh` | spliced `wsh(...)` bytecode with `Tag::Tr` mid-tree | `PolicyScopeViolation` about "inner-fragment" |

### Conformance gate

`tests/conformance.rs::rejects_tap_leaf_subset_violation` — registers the new
`TapLeafSubsetViolation` variant against the exhaustiveness gate
(`tests/error_coverage.rs::ErrorVariantName`). Without this test the
`every_error_variant_has_a_rejects_test_in_conformance` test fails.

## BIP edits enumerated

1. **§"Operator tag map"** (line 314, tag table): `0x08 TapTree (reserved for v1+) — multi-leaf tree structure; v0 single-leaf form encodes the leaf miniscript directly without a TapTree node`.
2. **§"Taproot tree"** (line 421): heading dropped the `(forward-defined)` qualifier (the section is no longer a forward-reference; v0.2 ships it).
3. **§"Taproot tree"** body (line 423): rewritten to specify the wire layout (`tr()` tag + internal-key reference + optional single leaf, no intermediate `0x08` in v0). Adds an explicit "decoder receiving `0x08` in the tap-leaf position MUST reject" clause.
4. **§"Taproot tree"** subset clause (line 425): expanded to call out the `c:` and `v:` wrappers as part of the canonical Coldcard surface (the parser emits them when expanding `pk` and `and_v(v:..., ...)`).
5. **§"Taproot tree" / =====Byte-layout examples=====**: NEW subsection with four annotated example encodings (key-path-only, `pk` leaf, `multi_a` leaf, nested or_d/and_v/older shape), each showing the full bytecode and an inline annotation of the per-byte structure. Bytes regenerated from the actual v0.2 encoder.

## Out of scope (deferred / unchanged)

- Multi-leaf TapTree encoding (D-1; v1+).
- Unspendable internal-key (BIP 341 NUMS) handling.
- Tap-miniscript type-check parity beyond the Coldcard subset.
- `wsh()` handling, framing layer, chunking, BCH layer, `policy.rs::WalletPolicy`, `bin/wdm.rs` — all unchanged.
- `crates/wdm-codec/tests/vectors/v0.1.json` is unchanged. No new entries were added to `vectors.rs::CORPUS_FIXTURES` — `gen_vectors --verify v0.1.json` still passes byte-identically. Phase F is the dedicated test-vector phase; taproot fixtures are deferred to that phase via the FOLLOWUPS entry below.

## Quality gates

| Gate | Result |
|---|---|
| `cargo test -p wdm-codec` | PASS — **544 tests** (up from 535; +9 new); 5 doc-tests pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS — clean |
| `cargo fmt --all --check` | PASS — clean |
| `RUSTDOCFLAGS="-D warnings -D missing_docs" cargo doc --workspace --no-deps --document-private-items` | PASS — clean |
| `cargo run --bin gen_vectors -- --verify crates/wdm-codec/tests/vectors/v0.1.json` | PASS — `committed file matches regenerated vectors (10 positive, 30 negative)` |

## Deferred minor items (for FOLLOWUPS aggregation)

The controller should add the following entries to `design/FOLLOWUPS.md`
during phase closure:

1. **`phase-d-tap-leaf-wrapper-subset-clarification`** (v0.3) — Phase D
   defaulted to rejecting all miniscript wrappers other than `c:` / `v:`
   inside tap leaves. If Coldcard or another deployed signer publishes
   documented support for additional wrappers (`s:` for `thresh`-chain
   constructions, `a:` for some `or_b`-style branches, etc.), revisit
   `validate_tap_leaf_subset` in `crates/wdm-codec/src/bytecode/encode.rs`.

2. **`phase-d-taproot-corpus-fixtures`** (v0.2 Phase F) — extend
   `vectors.rs::CORPUS_FIXTURES` with a representative `tr(@0/**)`,
   `tr(@0/**, pk(@1/**))`, and `tr(@0/**, multi_a(...))` triple, plus a
   `NEGATIVE_FIXTURES` entry for `tr(@0/**, and_v(v:sha256(...), pk(@1/**)))`
   with `expected_error_variant: "TapLeafSubsetViolation"`. Held out of
   Phase D because adding to `CORPUS_FIXTURES` would break the v0.1.json
   verify gate (positive fixtures are byte-stable across the v0.1
   release lock); Phase F is the schema-bump + new vectors phase.

3. **`phase-d-tap-miniscript-type-check-parity`** (v0.3) — Phase D's
   subset validator is a structural AST walk, not a full miniscript
   type-checker. It accepts ASTs that the upstream miniscript
   `Tap::check_global_consensus_validity` would reject (e.g.
   `pk_h(@0/**)` as a bare leaf — K-typed, won't validate as the
   B-typed body that taproot expects). The current behaviour relies on
   miniscript's parser to filter mistyped fragments at parse time; if a
   user constructs a `Descriptor::Tr` programmatically (bypassing the
   parser), the encoder will accept it. Decide whether to harden the
   validator or document the parser-reliance in v0.3.

## Notes on workflow

- The workspace `[patch]` block uses a relative `../rust-miniscript-fork`
  path that doesn't resolve from the deeper worktree directory. The
  `--config 'patch...path=...'` flag worked for `cargo test` but not for
  `cargo clippy` (cargo seems to apply the override differently across
  subcommands). Worked around by creating a one-shot symlink at
  `.claude/worktrees/rust-miniscript-fork → /scratch/code/shibboleth/rust-miniscript-fork`
  before running clippy / fmt / doc / gen_vectors. The controller can
  remove this symlink after merging the worktree branch.
