# v0.18 Phase 4a — Item A walker coverage (binary fragments + Thresh + After) (per-phase report)

**Date:** 2026-05-09
**Branch:** `feat/v0.18-full-tap-and-nums-engraving`

## Scope

Phase 4 of the v0.18 cycle covers the 17 miniscript Terminal arms not handled by v0.17 (which carried only AndV, Older, Verify). Per the plan §"Phase 4 — Item A" pre-emptive sub-split note, Phase 4a addresses 8 walker arms — binary fragments (AndB, AndOr, OrB, OrC, OrD, OrI), Thresh, and After. Phase 4b will cover the remaining 9 (4 hashes + 5 wrappers + True/False/RawPkH negative tests).

## Artifacts

### Walker arms (`parse/template.rs`)

8 new arms in `walk_miniscript_node`:

- `Terminal::AndB(l, r)` → `Tag::AndB`, `Body::Children([l, r])`
- `Terminal::AndOr(a, b, c)` → `Tag::AndOr`, `Body::Children([a, b, c])` (the only ternary fragment)
- `Terminal::OrB(l, r)` → `Tag::OrB`, `Body::Children([l, r])`
- `Terminal::OrC(l, r)` → `Tag::OrC`, `Body::Children([l, r])`
- `Terminal::OrD(l, r)` → `Tag::OrD`, `Body::Children([l, r])`
- `Terminal::OrI(l, r)` → `Tag::OrI`, `Body::Children([l, r])`
- `Terminal::Thresh(thresh)` → `Tag::Thresh`, `Body::Variable { k, children }` (distinct from Multi/MultiA — accepts arbitrary fragments, not just keys). Includes bounds-check `k ∈ 1..=32` before narrowing to u8 (reviewer I1 fix).
- `Terminal::After(abs)` → `Tag::After`, `Body::Timelock(abs.to_consensus_u32())` (BIP-65 absolute timelock; companion to v0.17's `Older` BIP-112 relative timelock).

All arms recurse via `walk_miniscript_node(child, km, tap_context)`, preserving the tap-context flag for nested-fragment dispatch.

### Render arms (`format/text.rs`)

8 new render arms; binary fragments factored into a small `render_binary` helper (5 callers: and_b, or_b, or_c, or_d, or_i):

- AndB/OrB/OrC/OrD/OrI → `name(l,r)` via `render_binary`
- AndOr → `andor(a,b,c)` (3-child variant)
- Thresh → `thresh(k,c1,c2,...)` (variable-arity; mirrors `render_multi` shape)
- After → `after(N)` (mirrors existing Older render)

All arms thread `n: u8` through to recursive children, consistent with the Phase 3 render_node signature change.

### Tests added (+8 net: 4 round-trip + 4 explicit walker)

**Round-trip tests in `format/text.rs::tests`:**

1. `roundtrip_tr_or_d_recovery_pattern` — `tr(@0,or_d(pk(@1),and_v(v:pk(@2),older(144))))`. Common BOLT-3-style hot-cold split.
2. `roundtrip_tr_or_i_disjunction` — `tr(@0,or_i(pk(@1),pk(@2)))`.
3. `roundtrip_tr_and_or_ternary` — `tr(@0,andor(pk(@1),pk(@2),pk(@3)))`. Pins the ternary shape.
4. `roundtrip_tr_and_v_after_absolute_timelock` — `tr(@0,and_v(v:pk(@1),after(700000)))`.

**Explicit walker tests in `parse/template.rs::tr_tests`:**

1. `tr_with_and_or_ternary_emits_three_children` — pins Body::Children([3]) for AndOr.
2. `tr_with_after_absolute_timelock_emits_timelock_body` — pins Body::Timelock(700000) for After (distinct tag from Older).
3. `tr_with_or_d_recovery_pattern_walker_shape` — pins Body::Children([2]) with PkK + AndV children.
4. `tr_with_thresh_1_of_1_emits_variable_body` — pins Body::Variable{k=1, children=[PkK]} for Thresh (reviewer I3 fix; uses 1-of-1 form to avoid Phase 4b's wrapper dependency).

### Tests deferred to Phase 4b (documented gaps)

- `walker_handles_and_b_inheritance` — AndB requires Swap-wrapped right child for typecheck.
- `walker_handles_or_b` — same reason.
- `walker_handles_thresh_segwitv0` and `walker_handles_thresh_with_non_key_fragment_child` — Thresh requires Swap-wrapped position-2+ children.
- `roundtrip_tr_or_c_with_verify` — or_c is V-typed at top level; needs `t:` wrapper to be a valid root expression.

Phase 4b's wrapper coverage (Swap, Alt, DupIf, NonZero, ZeroNotEqual) unblocks all four. The walker arms ARE in place and structurally sound; they're just not exercisable end-to-end in 4a.

## Verification

- `cargo build -p md-cli --features cli-compiler` clean.
- `cargo test --workspace --all-features` → 409 pass (was 401 baseline pre-Phase-4a; +8 net).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean.
- Live probe: or_d, or_i, andor, after, 1-of-1 thresh via `md encode <miniscript-string>` all produce md1 phrases.

## Per-phase code-reviewer round

`feature-dev:code-reviewer` dispatched. Findings:

- **C: 0 / I: 2 / L: 3 (3 retracted/no-action)**
- **I1** — `thresh.k() as u8` silent truncation for k > 255. **Fixed inline** — added explicit `if !(1..=32).contains(&k)` bounds check before narrowing, returning `CliError::TemplateParse` with a clear message. Codec's `ThresholdOutOfRange` is a fallback; the walker error surfaces earlier with user-readable text.
- **I3** — Thresh has no dedicated walker test in 4a despite the plan's I3 finding calling for one. Reviewer suggested a 1-of-1 form that doesn't need wrappers. **Fixed inline** — added `tr_with_thresh_1_of_1_emits_variable_body` test.
- L1/L2/L3 — non-actionable observations (pre-existing patterns or already addressed in the diff).

Reviewer also identified a parity concern: `build_multi_node` (which handles Multi/MultiA) has the same pre-existing `k as u8` truncation. Pre-existing, out of scope for Phase 4a; **filed as FOLLOWUP** `v0.18-phase-4a-build-multi-node-k-bounds-parity`.

Net: 0C/0I after I1+I3 fixes.

## Exit gate

- ✅ 8 walker arms added (AndB, AndOr, OrB, OrC, OrD, OrI, Thresh, After).
- ✅ 8 render arms added (5 via render_binary helper + Thresh + AndOr + After).
- ✅ 4 round-trip tests + 4 explicit walker tests pinned.
- ✅ Thresh k-bounds check added (reviewer I1).
- ✅ Thresh 1-of-1 walker test added (reviewer I3).
- ✅ Phase 4b deferrals explicitly documented (or_c top-level + AndB/OrB/Thresh end-to-end).
- ✅ build_multi_node parity concern filed as FOLLOWUP.
- ✅ Workspace tests + clippy clean (409 tests).
- ✅ Per-phase reviewer 0C/0I after inline fixes.

Phase 4a closed; Phase 4b (hashes + wrappers + negative tests) staged for a future session per the user's pause directive.
