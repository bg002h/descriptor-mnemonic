# v0.17 Phase 3 — md-cli `walk_tr` NUMS recognition (per-phase report)

**Date:** 2026-05-09
**Branch:** `feat/v0.17-tap-multi-leaf-policy`

## Scope

Wire the canonicalization invariant in md-cli's walker: `walk_tr` inspects the descriptor's `tr()` internal key string and emits `Tag::TrUnspendable` (no key_index field) iff the key matches the BIP-341 NUMS H-point. Otherwise it falls back to the existing `Tag::Tr` path with `lookup_key` against the placeholder map. Non-NUMS literal x-only hex is rejected with a clear actionable error.

## Artifacts

### walk_tr rewrite (template.rs)

```
1. Get t.internal_key().to_string()
2. Compute optional tap tree via walk_tap_tree
3. If key_str == NUMS_H_POINT_X_ONLY_HEX
   → Node { Tag::TrUnspendable, Body::TrUnspendable { tree } }
4. Else lookup_key against placeholder map
   - On miss + key looks like x-only hex (length 64, ASCII hex)
     → clear error pointing user to @N or NUMS
   - On miss otherwise → original lookup_key error
   - On hit → Node { Tag::Tr, Body::Tr { key_index, tree } }
```

Helper `is_x_only_hex(s)` added: returns `true` iff `s.len() == 64 && all_ascii_hex(s)`.

### Tests added

- `tr_tests::tr_with_nums_internal_key_emits_tr_unspendable` — positive case for the canonicalization invariant. Asserts Tag::TrUnspendable for `tr(<NUMS>, multi_a(2,@0,@1,@2))` and structural equality of the multi_a leaf.
- `tr_tests::tr_with_nums_key_only_no_tree_emits_tr_unspendable_with_none_tree` — key-path-only frozen output. Exercises the `Body::TrUnspendable { tree: None }` code path. (Reviewer-driven addition.)
- `tr_tests::tr_with_non_nums_literal_hex_rejected_with_clear_message` — negative case using the secp256k1 generator point's x-coord as a guaranteed-valid non-NUMS x-only key. Asserts the error message surfaces the offending hex AND includes the NUMS alternative.

### V1.c canary update (consolidated into Phase 3 commit)

`v017_v1_c_nums_pre_phase_3_synthetic_key_not_found` fired when Phase 3's NUMS recognition made `tr(<NUMS-hex>, multi_a(...))` encode successfully. Renamed to `v017_v1_c_nums_internal_key_encodes_via_tr_unspendable` and flipped to assert-success. Same red→green discipline as V1.b in Phase 2.

## Verification

- `cargo test -p md-cli` → 66 unit tests + integration tests pass.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean.
- Both V1 canaries (V1.b, V1.c) now assert success — Axis 1 + Axis 2 confirmed end-to-end through the encode pipeline.

## Per-phase code-reviewer round

`feature-dev:code-reviewer` dispatched. Findings:

- **C: 0**
- **I1** — Error message guidance inaccurate: said "Use a real DescriptorPublicKey (xpub) for the internal key" but the actual constraint is that the key must be `@N`-placeholder-derived. **Fixed inline** — error now reads "Use an @N placeholder (backed by an xpub via --keys) for the internal key".
- **I2** — `Body::TrUnspendable { tree: None }` code path was reachable but unexercised. **Fixed inline** — added `tr_with_nums_key_only_no_tree_emits_tr_unspendable_with_none_tree` test that confirms `tr(<NUMS>)` standalone parses through miniscript and produces the no-tree variant.
- Reviewer confirmed: canonicalization-invariant string comparison is sound (miniscript 13's Display impl emits lowercase hex; no origin-wrapping risk for x-only keys); `is_x_only_hex` predicate has no real-world false positives or negatives; TDD canary update timing acceptable; sub-`tr` recursion concern out of scope (taproot doesn't allow nested tr in script leaves).

Net: 0C/0I after fixes.

## Exit gate

- ✅ `walk_tr` recognizes NUMS H-point and emits Tag::TrUnspendable.
- ✅ Non-NUMS literal hex internal keys rejected with actionable error.
- ✅ `tr(<NUMS>)` no-tree case verified end-to-end.
- ✅ V1.c canary fired and was flipped to success in same commit.
- ✅ Per-phase reviewer 0C/0I.

Phase 3 closed; proceeding to Phase 4 (compile.rs rewrite + `--unspendable-key` flag).
