# v0.5 Phase 2 Implementer Report — Type wiring + decoder helper

**Status**: DONE
**Phase**: 2 of 11 (v0.5 multi-leaf TapTree)
**Branch**: `feature/v0.5-multi-leaf-taptree`
**Commit**: `5c12672` (tip; based on `d72b159`)
**Model**: opus 4.7 (1M context)
**Date**: 2026-04-28

## Summary

Phase 2 lands the type-wiring and decoder-helper layer for v0.5 multi-leaf TapTree
admission. Decoder routes `Tag::TapTree` (0x08) into a new recursive helper that
threads a DFS pre-order leaf index through to `Error::TapLeafSubsetViolation`.
Encoder side is intentionally unchanged in this phase (Phase 4) and per-leaf
report population is also deferred (Phase 5). All existing v0.4.x decode
behavior is preserved verbatim for non-multi-leaf inputs (single-leaf path
not detoured through the helper).

Final test count: **615 passing + 1 ignored, 0 failing** (baseline 609 +
6 new tests in `v0_5_type_wiring.rs`; `multi_leaf_two_leaf_symmetric_round_trips`
is the deliberate `#[ignore]` pinning the end-state).

## Files changed

```
crates/md-codec/src/bytecode/decode.rs  | recursive helper + routing + leaf_index plumbing
crates/md-codec/src/bytecode/encode.rs  | validate_tap_leaf_subset signature + 2 catch-all sites
crates/md-codec/src/decode.rs           | DecodeReport literal site
crates/md-codec/src/decode_report.rs    | TapLeafReport struct + tap_leaves field + tests
crates/md-codec/src/error.rs            | Error::TapLeafSubsetViolation variant + #[non_exhaustive]
crates/md-codec/src/lib.rs              | TapLeafReport re-export
crates/md-codec/src/vectors.rs          | n_taptree_multi_leaf vector reflects v0.5 semantics
crates/md-codec/tests/conformance.rs    | TapLeafSubsetViolation pattern updated to use ..
crates/md-codec/tests/taproot.rs        | TapLeafSubsetViolation patterns + renamed obsolete test
crates/md-codec/tests/v0_5_type_wiring.rs (new) | 7 type-wiring + behavioral tests
crates/md-codec/tests/vectors/v0.2.json | regenerated; reflects updated negative vector
crates/md-codec/tests/vectors_schema.rs | SHA pin updated for regenerated v0.2.json
```

## Per-task summary

### Task 2.1 — `Error::TapLeafSubsetViolation` `#[non_exhaustive]` + `leaf_index`
Variant marked `#[non_exhaustive]` and gained `leaf_index: Option<usize>` field
per spec §4. Failing-test cycle: pinned variant shape via destructure (NOT
direct construction — see Deviation 1).

### Task 2.2 — Update 3 construction sites
- `encode.rs:443` Terminal-encoder catch-all: `leaf_index: None`
- `encode.rs:487` `validate_tap_leaf_terminal` catch-all: `leaf_index: None`
- `decode.rs:691` `decode_tap_terminal` catch-all: `leaf_index: leaf_index`
  (forward-references the param added in Task 2.6)

### Task 2.3 — `validate_tap_leaf_subset` signature
Added `leaf_index: Option<usize>` parameter; outer `map_err` re-wraps the
`None`-tagged inner error with the caller-supplied index. Doc-comment
captures the convention.

### Task 2.4 — 2 call-site updates
- `encode.rs:154` Tr arm: `Some(0)` for single-leaf preservation
- `decode.rs:276` (now in `decode_tr_inner`'s single-leaf branch): `Some(0)`

### Task 2.5 — `TapLeafReport` + `DecodeReport.tap_leaves`
- New `#[non_exhaustive]` `TapLeafReport` struct with `leaf_index: usize`,
  `miniscript: Arc<Miniscript<DescriptorPublicKey, Tap>>`, `depth: u8`
- `DecodeReport` gains `tap_leaves: Vec<TapLeafReport>` (always empty until
  Phase 5)
- `lib.rs` re-export added; in-tree literal constructors at `decode.rs:183`
  and `decode_report.rs:233,253` (test fns) updated

### Task 2.6 — `decode_tap_miniscript` + `decode_tap_terminal` signatures
Both functions gain `leaf_index: Option<usize>`. Recursive sites inside
`decode_tap_terminal` (`Tag::AndV`/`OrD`/`Check`/`Verify`) propagate the
caller's `leaf_index` (same leaf, same index).

### Task 2.7 — `decode_tap_subtree` recursive helper
New helper at the bottom of the tap-context decoder section.
Peek-before-recurse semantics: the `0x08` framing byte IS consumed before
the depth gate fires (matches v0.4 Sh diagnostic offset convention). Depth
gate is `> 128` per BIP 341 `TAPROOT_CONTROL_MAX_NODE_COUNT`. Per-leaf
subset gate (`validate_tap_leaf_subset`) is invoked inline at the leaf
arm, mirroring single-leaf parity.

### Task 2.8 — Routing in `decode_tr_inner`
`decode_tr_inner` peeks the first post-key byte; `Tag::TapTree (0x08)`
routes to `decode_tap_subtree(depth=1, leaf_counter=&mut 0)`, single-leaf
preserved verbatim, KeyOnly via `cur.is_empty()` (the cursor's actual
predicate; the plan's `is_at_end()` is not the in-tree name).

### Task 2.9 — Defense-in-depth message at `decode_tap_terminal`
The line-680 `Tag::TapTree` arm message updated from "reserved for v1+"
to a "must be routed via decode_tap_subtree" advisory. This arm is
no longer reachable on the happy path but retained as a guard against
future direct-call regressions.

### Task 2.10 — Behavioral test (ignored)
`multi_leaf_two_leaf_symmetric_round_trips` added with `#[ignore]` and an
unblock note pointing at Phase 4 (encoder) and Phase 5 (tap_leaves
population).

### Task 2.11 — Commit
Single commit `5c12672` with conventional-commit-style message and
`Co-Authored-By: Claude Opus 4.7` trailer. 12 files changed,
394 insertions(+), 53 deletions(-).

## Deviations from the plan

### Deviation 1 — `Error::TapLeafSubsetViolation` test must NOT directly construct
The plan provided test code that constructs `Error::TapLeafSubsetViolation { ... }`
from an integration test (separate crate). However, the plan also requires
the variant be `#[non_exhaustive]` (per spec §4 and Step 3 of Task 2.1).
These two are mutually exclusive: `#[non_exhaustive]` blocks external
struct-expression construction.

**Resolution**: kept `#[non_exhaustive]` (spec is explicit) and replaced
the direct construction in the test with obtaining a real instance via
the public encode API:

```rust
fn trigger_tap_leaf_subset_violation() -> Error {
    let policy: WalletPolicy =
        "tr(@0/**,and_v(v:sha256(<32B-hex>),pk(@1/**)))".parse()...;
    policy.to_bytecode(&EncodeOptions::default()).expect_err(...)
}
```

The destructure pattern (which is the load-bearing assertion — pinning
the field name) still exercises the type. Functionally equivalent;
faithful to the spec's intent.

### Deviation 2 — `taproot_rejects_decode_tag_taptree` test obsoleted
The existing `tests/taproot.rs:163` test asserted v0.4's
"PolicyScopeViolation reserved for v1+" rejection of `Tag::TapTree`. v0.5
admits that byte as multi-leaf framing, so the test's intent is gone.
**Resolution**: renamed to `taproot_decodes_tag_taptree_routes_into_subtree_helper`
and updated the assertion to expect `InvalidBytecode/UnexpectedEnd`
(the new behavior of "framing byte without children → truncation
reported by the helper"). Phase 6 will replace this with the canonical
N1-N9 negative set per spec §5.

### Deviation 3 — `n_taptree_multi_leaf` negative vector behavior changed
The same v0.4-era assertion lived in `vectors.rs:1528` as a fixture.
**Resolution**: updated `description`, `expected_error_variant`
(`PolicyScopeViolation` → `InvalidBytecode`), and `provenance` to match
the v0.5 routing. Regenerated `tests/vectors/v0.2.json` (1 affected
entry) and updated the SHA pin in `vectors_schema.rs`.

The Phase 6 plan calls for further regeneration with the
`"md-codec 0.5"` family generator token and the new N1-N9 negative
fixtures. The Phase 2 regeneration is mid-stream and will be
superseded; doing it now keeps CI green throughout the phase.

### Deviation 4 — Cursor predicate name
Plan suggested `cur.is_at_end()` or `cur.remaining() == 0` for the
KeyOnly check. The actual cursor API exposes `is_empty()` (verified
at `cursor.rs:104`). Used `cur.is_empty()` accordingly; doc-comment
in the helper notes the convention.

## Self-review notes

- The v0.5 routing in `decode_tr_inner` preserves the v0.4 single-leaf path
  byte-for-byte (the helper is only entered when the peek byte equals
  `Tag::TapTree`). Existing v0.4.x corpus continues to round-trip.
- Recursive calls in `decode_tap_terminal` (`AndV`/`OrD`/`Check`/`Verify`)
  pass the *same* `leaf_index` through (a single leaf has one index;
  inside the leaf's miniscript AST every node shares it).
- The depth gate (`> 128`) is intentionally evaluated AFTER consuming
  the `0x08` byte. This matches the v0.4 Sh diagnostic offset
  convention (cursor-on-rejection points past the violating byte).
  Spec H1/H2 hostile-input fixtures (Phase 6) will pin this exact
  semantic.
- `validate_tap_leaf_subset` is invoked inline at the leaf arm of
  `decode_tap_subtree` so multi-leaf decode applies the same Coldcard
  subset gate per leaf as the single-leaf path. Without this, a
  hostile multi-leaf input could smuggle out-of-subset operators.
- `clippy --workspace --all-targets -- -D warnings` is clean.
  `cargo fmt --check` is clean.

## Deferred items (FOLLOWUPS)

None. All in-scope items landed; obsolete v0.4-era tests / fixtures
adjusted to keep CI green. The planned Phase 6 corpus rework will
produce the canonical N1-N9 / T1-T7 fixtures and supersede the
mid-phase adjustments made here.

No new entries filed in `design/FOLLOWUPS.md`.

## Test counts

```
Baseline (start of Phase 2): 609 passing, 0 ignored
End of Phase 2:              615 passing, 1 ignored, 0 failing

Delta: +6 active tests (5 type-wiring smoke tests +
       1 helper-pinning placeholder), 1 deliberate #[ignore]
       (multi_leaf_two_leaf_symmetric_round_trips, unblocks Phase 4+5)
```

## Self-review gate results

```
RUSTUP_TOOLCHAIN=stable cargo test --workspace --no-fail-fast    PASS (615 + 1 ignored)
RUSTUP_TOOLCHAIN=stable cargo fmt --check                        PASS
RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets   PASS (-D warnings)
                       -- -D warnings
```

## Next phase

Phase 3 will update the top-level dispatcher message at `decode.rs:98-100`
("v0.4 does not support top-level tag TapTree") to reflect that 0x08 is
now an active inner-node framing inside `tr(KEY, TREE)` but is still
not a valid top-level descriptor. Phase 4 then lands the encoder
rewrite (multi-leaf + recursive `encode_tap_subtree`).
