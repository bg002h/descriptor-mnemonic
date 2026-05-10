# v0.18 Phase 4b — Item A walker coverage (hashes + wrappers + True/False) (per-phase report)

**Date:** 2026-05-09
**Branch:** `feat/v0.18-full-tap-and-nums-engraving`

## Scope

Final 11 walker arms of Item A:

- 4 hash preimage locks: `Sha256`, `Hash256` (32-byte), `Ripemd160`, `Hash160` (20-byte).
- 5 single-arity wrappers: `Swap`, `Alt`, `DupIf`, `NonZero`, `ZeroNotEqual`.
- 2 boolean literals: `True`, `False` — initially planned as render-only with negative walker tests; Phase 4b probe revealed they're reachable via miniscript's `t:` sugar (= `and_v(X, 1)`), so walker arms added.

## Plan adjustments

- Dropped negative tests `walker_rejects_true_in_tap_top_level_context`, `walker_rejects_zero_at_top_level`. The probe demonstrated `t:or_c(B,V)` desugars to `and_v(or_c(B,V),1)` with a Tag::True child, reaching the walker. Walker arms added instead.
- Dropped `walker_rejects_raw_pkh_in_top_level_context`. Per miniscript 13's doc-comment, `RawPkH` is not constructible from any miniscript API; the walker can't be reached for this variant. The render arm covers the decode-side path.

## Artifacts

### Walker arms (`parse/template.rs`)

11 new arms in `walk_miniscript_node`, plus `use bitcoin::hashes::Hash` brought into scope:

- `Terminal::Sha256(h)` → `Tag::Sha256`, `Body::Hash256Body(h.to_byte_array())`
- `Terminal::Hash256(h)` → `Tag::Hash256`, `Body::Hash256Body(...)`
- `Terminal::Ripemd160(h)` → `Tag::Ripemd160`, `Body::Hash160Body(...)`
- `Terminal::Hash160(h)` → `Tag::Hash160`, `Body::Hash160Body(...)`
- `Terminal::Swap/Alt/DupIf/NonZero/ZeroNotEqual(inner)` → `Body::Children([walked_inner])`
- `Terminal::True` → `Tag::True`, `Body::Empty`
- `Terminal::False` → `Tag::False`, `Body::Empty`

### Render arms (`format/text.rs`)

11 new arms. Two structural refactors:

1. **Wrapper-chain canonicalization.** Naive `prefix + recurse` produced `s:n:j:X`; miniscript canonical is `snj:X`. Refactored 6 wrapper tags (Check + Swap + Alt + DupIf + NonZero + ZeroNotEqual) to dispatch into a single `render_wrapper_chain` helper that walks the spine, accumulates single-letter prefixes, and emits `<chain>:<deepest_inner>`.
2. **Tag::Check → pk shorthand.** Miniscript's canonical `Check(PkK)` renders as the shorthand `pk(K)` (not `c:pk(K)`); same for `Check(PkH) → pk_h(K)`. The chain helper detects this case (chain ends with `c` and deepest inner is PkK/PkH) and emits the shorthand. Pre-Phase-4b, `Tag::Check` had NO render arm — the missing arm surfaced via the and_b/swap test failure.

Two new helpers: `render_hash256` (32-byte hex literal), `render_hash160` (20-byte hex literal).

### Tests added (+9 net: 6 round-trip + 3 explicit walker)

**Round-trip (`format/text.rs::tests`):**

1. `roundtrip_tr_and_v_sha256_hash_lock`
2. `roundtrip_tr_and_v_hash160_hash_lock`
3. `roundtrip_wsh_thresh_with_non_key_fragment_child` — closes Phase 4a I3 deferral. Tests Thresh + Swap + chained `snj:` wrappers + AndV inner.
4. `roundtrip_wsh_and_b_with_swap_wrapper` — closes Phase 4a deferral.
5. `roundtrip_wsh_or_b_with_swap_wrapper` — closes Phase 4a deferral.
6. `roundtrip_tr_t_or_c_desugars_to_and_v_with_true` — closes Phase 4a deferral. Demonstrates the parse-vs-canonical drift: input `t:or_c(...)` parses; canonical render is `and_v(or_c(...),1)`.

**Explicit walker (`parse/template.rs::tr_tests`):**

1. `tr_with_sha256_emits_hash256_body` — pins Body::Hash256Body([32 bytes]).
2. `wsh_thresh_with_swap_wrapper_emits_children_one` — pins Body::Children([1]) for Swap; verifies inner is Tag::Check (segwitv0 context, walker doesn't collapse c:pk_k → PkK).
3. `tr_t_or_c_walker_emits_true_in_and_v_subtree` — pins t: desugaring sees Tag::True with Body::Empty.

## Verification

- `cargo build -p md-cli --features cli-compiler` clean.
- `cargo test --workspace --all-features` → 418 pass (was 409 pre-Phase-4b; +9 net).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean.
- Live probe: thresh+s:+snj:, sha256 lock, t:or_c all encode end-to-end.

## Per-phase code-reviewer round

`feature-dev:code-reviewer` dispatched. Findings:

- **C: 0 / I: 0 / L: 1 (informational, no code action)**
- **L1** — prompt described `*h.to_byte_array()`; actual code is `h.to_byte_array()` (no deref). Documentation imprecision in the prompt, not the code. Confirmed safe — `to_byte_array()` returns owned `[u8; N]` by value.
- All 8 review-focus areas passed: hash byte-array projection (clean); wrapper-chain canonicalization (correct for `Check(PkK)`, `Swap(Check(PkK))`, `Swap(NonZero(DupIf(X)))`, `Check(AndV(...))`); True/False wire encoding (matches md-codec extension tags 0x1F/0x04 and 0x1F/0x03); hash test fixture validity (miniscript accepts any 64-hex sha256); Phase 4a deferral closure (5/5 closed); walker/renderer layer separation (walker test asserts wire form Tag::Check; renderer collapses to `pk(K)` shorthand); dispatch completeness (`Tag::TapTree` correctly intercepted by `render_tap_node` before reaching catch-all); no Phase 4a regression.

Net: 0C/0I — clean SHIP.

## Item A complete (Phases 4a + 4b combined)

Total Item A coverage:

- **17 walker arms added** (8 in 4a + 11 in 4b — the 11 includes True/False that were initially "render-only" but proved reachable).
- **11 + 2 = 13 render arms added** (8 in 4a + 11 in 4b → counting Tag::Check as added in 4b). Plus the 2 wire-format-only render arms (Tag::True, Tag::False, Tag::RawPkH) and the wrapper-chain refactor.
- **17 tests added** total (8 in 4a + 9 in 4b).

The miniscript walker now handles every Terminal variant that's constructible from miniscript APIs. RawPkH is the lone variant skipped (per upstream's "not constructible from any API" guarantee); render-side coverage is in place for decode-side wire fidelity.

## Exit gate

- ✅ 11 walker arms added (4 hashes + 5 wrappers + True/False).
- ✅ 11 render arms added; wrapper chain canonicalized via `render_wrapper_chain`; Tag::Check shorthand collapse added.
- ✅ Tag::Check render arm added (was missing pre-Phase-4b, surfaced by tests).
- ✅ 6 round-trip tests + 3 explicit walker tests pinned.
- ✅ All 5 Phase 4a deferrals closed.
- ✅ Workspace tests + clippy clean (418 tests, +9 net Phase 4b; +17 net Item A combined).
- ✅ Per-phase reviewer 0C/0I.

Phase 4b closed; proceeding to Phase 5 (Item F round-trip integration test).
