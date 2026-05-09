# v0.17 Phase 2 — md-cli walker extension Axis 1 (per-phase report)

**Date:** 2026-05-09
**Branch:** `feat/v0.17-tap-multi-leaf-policy`

## Scope

Extend md-cli's walker (`walk_miniscript_node`) with three new miniscript Terminal arms — `AndV`, `Older`, `Verify` — to enable the inheritance/timelock policy pattern (`or(pk(@0), and(pk(@1), older(N)))` and similar). md-codec's wire format already encodes these tags; the gap was purely in the md-cli bridge layer.

## Artifacts

### Walker (template.rs)

- `Terminal::AndV(l, r)` → `Tag::AndV` with `Body::Children([walk(l), walk(r)])`.
- `Terminal::Older(seq)` → `Tag::Older` with `Body::Timelock(seq.to_consensus_u32())` (preserves BIP-112 enable + lock-time-type bits verbatim).
- `Terminal::Verify(inner)` → `Tag::Verify` with `Body::Children([walk(inner)])`.
- `walk_tap_tree_v0_15` renamed to `walk_tap_tree`; error message dropped "v0.15" wording in favor of "multi-branch tap trees are not yet supported (got {n} leaves; single-leaf only). The policy compiler emits compact single-leaf miniscript fragments by default — file an issue if you need multi-branch support."
- `pub(crate) const NUMS_H_POINT_X_ONLY_HEX` added at the top of `parse/template.rs`. Lives unconditionally here (not in feature-gated `compile.rs`) so `format/text.rs`, `walk_tr` (Phase 3), and `compile.rs` (Phase 4) can all reference it.

### Renderer (format/text.rs)

Carry-over from Phase 1's md-codec wire change: text.rs's `render_node` only handled the original 5 tags. Phase 2 added arms for: `Tag::TrUnspendable` (renders `tr(<NUMS-hex>, ...)`), `Tag::AndV` (renders `and_v(left,right)`), `Tag::Verify` (renders `v:<inner>` prefix), `Tag::Older` (renders `older(N)`). Round-trip test pinned for the inheritance pattern: `tr(@0/<0;1>/*,and_v(v:pk(@1/<0;1>/*),older(144)))`.

### JSON serialization (format/json.rs)

Added `JsonBody::TrUnspendable { tree }` variant + `From<&Body>` arm. Mirror of the md-codec Body addition.

### Tests added

- `parse::template::tr_tests::tr_with_and_v_verify_older_inheritance` — full structural assertion on the walker output for the inheritance pattern.
- `parse::template::tr_tests::tr_multi_branch_rejected_with_v0_17_error_message` — pins the new error wording AND asserts "v0.15" is gone from the walker error path.
- `format::text::tests::roundtrip_tr_and_v_verify_older_inheritance` — encode-then-render round-trip for the decode-side path.

### V1 canary update (consolidated into Phase 2 commit)

`v017_v1_b_and_v_pre_phase_2_unsupported_fragment` was the canary that fired when Phase 2's walker arms made `tr(@0,and_v(v:pk(@1),older(144)))` encode successfully. Renamed to `v017_v1_b_and_v_inheritance_pattern_encodes` and flipped from "assert fail with `unsupported miniscript fragment: and_v`" to "assert success + stdout starts_with(`md1`)". Doc-comment preserves the historical pre-Phase-2 stderr for git-history readers.

The plan originally suggested updating V1.b in Phase 5. Coalescing to Phase 2 keeps each commit's test suite green (TDD red→green inside one phase). Per-phase reviewer endorsed the deviation.

## Verification

- `cargo test -p md-cli` (no features) → 63 unit + integration tests pass.
- `cargo test -p md-cli --features cli-compiler` → all tests pass.
- `cargo test --workspace --all-features` → no failures.
- `cargo clippy -p md-cli --all-targets --features cli-compiler -- -D warnings` clean.

## Per-phase code-reviewer round

`feature-dev:code-reviewer` dispatched. Findings:

- **C: 0**
- **I1** — `format/text.rs::render_node` had no arms for `Tag::AndV`, `Tag::Verify`, `Tag::Older`, or `Tag::TrUnspendable`; decode-side round-trip would fail silently. **Fixed inline** — added all four arms with structural body validation; pinned a round-trip test for the inheritance pattern.
- **I2** — `compile.rs:52` still has stale "v0.15 cli-compiler" wording, separate from the `walk_tap_tree` error path that the new test pins. **Mitigated** — added explicit comment to the test clarifying it covers ONLY `walk_tap_tree`'s error path; Phase 4 deletes compile.rs's bare-pk gate (and the v0.15 string with it).
- Reviewer confirmed all other axes: walker correctness (Verify/AndV recurse with `tap_context` preserved correctly; `seq.to_consensus_u32()` preserves BIP-112 bits), no other Body match sites missed, TDD canary update timing is sound.

Net: 0C/0I after fixes.

## Exit gate

- ✅ `walk_miniscript_node` handles AndV / Older / Verify.
- ✅ `walk_tap_tree` (renamed) emits v0.17 error wording for multi-branch.
- ✅ `format/text.rs::render_node` handles the four new tags for decode-side round-trip.
- ✅ `format/json.rs` JsonBody updated for Tag::TrUnspendable serialization.
- ✅ V1.b canary fired and was flipped to success in same commit.
- ✅ Per-phase reviewer 0C/0I.

Phase 2 closed; proceeding to Phase 3 (md-cli `walk_tr` NUMS recognition for Axis 2).
