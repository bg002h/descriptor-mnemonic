# v0.5 Phase 6 Implementer Report

**Status**: DONE_WITH_CONCERNS

**Phase**: 6 — Test corpus (29 NEW + 1 RENAMED)

**Branch**: `feature/v0.5-multi-leaf-taptree`

**Commit**: `7d6e278` (test(v0.5 phase 6): multi-leaf TapTree corpus + hostile-input + round-trip tests)

**Pre-phase HEAD**: `3097c99` (Phase 5 commit)

---

## Summary per task

### Task 6.1 — Positive fixtures T1-T7 in TAPROOT_FIXTURES

Replaced the 3-entry `TAPROOT_FIXTURES` slice in `crates/md-codec/src/vectors.rs` with an 8-entry slice. Old IDs `tr_keypath` and `tr_pk` were renamed to `tr_keypath_only_md_v0_5` (T1) and `tr_single_leaf_pk_md_v0_5` (T2) per the v0.5 naming convention; `tr_multia_2of3` was preserved verbatim. NEW: T3-T7 multi-leaf fixtures.

T7's tree shape (`{{pk(@1),pk(@2)},{pk(@3),{pk(@4),{pk(@5),pk(@6)}}}}`) is the right-spine 6-leaf shape suggested by the plan; encoded length lands well within the regular-string single-chunk capacity, so this fixture exercises the multi-leaf encoder path under the SingleString chunking plan rather than crossing the chunked-plan boundary. **Concern**: see "Concerns" section below.

Coldcard-shape parity fixture: deferred per plan ("if no such corpus exists"). I checked the existing v0.4 Coldcard fixture (`coldcard` and `cs_coldcard_sh_wsh`) — both are sortedmulti shapes with no multi-leaf TapTree counterpart. Acceptable to defer.

### Task 6.2 — Negative fixtures N1-N9

Removed the legacy `build_negative_n_taptree_multi_leaf` builder (Phase 2's stop-gap that morphed the v0.4 reservation rejection into UnexpectedEnd). Added 9 new builders: `build_negative_n1_taptree_single_inner_under_tr` through `build_negative_n9_taptree_at_top_level`.

`expected_error_variant` follows the existing convention from the harness's `error_variant_name` map (variant name only, e.g. `"InvalidBytecode"`, `"PolicyScopeViolation"`, `"TapLeafSubsetViolation"`). The plan's `expected_error_variant: "InvalidBytecode { kind: UnexpectedEnd }"` strings would not have matched — the variant name is the contract.

N3-N7 use a shared helper `build_negative_taptree_inner_off_subset(id, offender_tag, operator_name)` to avoid copy-paste; each fixture varies only in tag (Wpkh/Sh/Wsh/Tr/Pkh) and operator name. The shared helper does a debug-time decode-and-expect-error sanity check (rather than the variant-equality check that `debug_assert_decode_matches` does) because we expect a `TapLeafSubsetViolation` variant, which is in a separate match arm from the harness's `InvalidBytecode` cases — the family-level guard is sufficient.

`vectors_schema.rs` updated to reflect the rename:
- `schema_2_contains_v0_2_corpus_additions` checks for `tr_keypath_only_md_v0_5` / `tr_single_leaf_pk_md_v0_5` (was `tr_keypath` / `tr_pk`)
- Negative-fixture check switched from `n_taptree_multi_leaf` to `n_taptree_single_inner_under_tr` (semantic successor: N1)
- Positive corpus count bumped from 22 to 27 (added T1, T3, T4, T5, T6, T7 = 5 positive vectors net; T2 is a rename)

### Task 6.3 — Hostile-input inline tests H1-H5

Created `crates/md-codec/tests/v0_5_taptree_hostile.rs` with H1-H5.

**Significant deviation from plan**: the plan's helpers `build_left_spine_taptree_bytecode` and `encode_bytecode_to_md_string` had two unworkable assumptions:

1. The bytecode they constructed was missing the `[header byte][SharedPath][indicator]` prefix that the bytecode-level decoder requires — the plan's helper started directly with `[Tag::Tr]`.
2. `encode_bytecode_to_md_string` referenced `md_codec::chunking::encode_to_string`, which does not exist (the actual chunking encoder is `chunk_bytes` + `encoding::encode_string`, and is size-bounded to 1692 bytes — H4's 10K-byte recursion bomb cannot fit through it).
3. The plan's left-spine helper used 130 distinct placeholder indices (N+2 unique keys for N=128 framings), exceeding the 32-key cap enforced both at the bytecode-decode placeholder lookup (`keys.len() <= 32`) and the BIP 388 wallet-policy re-derivation (which enforces monotonic `@N` index ordering — `prev.index == curr.index || prev.index + 1 == curr.index`).

**Resolution**: tests call `md_codec::bytecode::decode::decode_template` directly with a 32-key dummy slice, bypassing both the chunking layer (allowing H4's 10K-byte recursion bomb) and the BIP 388 re-derivation (allowing 129-leaf trees with cyclic placeholder indices `1..32`). For H3 (truncation), I use `decode_bytecode` because it tests the bytecode-level header-and-template path, which is well-covered by that API.

H1-H5 all pass. The depth boundary semantics confirmed: the gate `depth > 128` in `decode_tap_subtree` fires at recursion-depth 129 reading a 129th `[TapTree]` byte; 128 framings + leaves at the bottom is legal (deepest leaf at miniscript-depth 128); 129 framings rejects.

### Task 6.4 — Round-trip + leaf-index + parser-roundtrip tests

Created `crates/md-codec/tests/v0_5_taptree_roundtrip.rs` with RT1-RT4, LI1-LI3, PR1-PR2 (9 tests total).

**Deviation from plan**: the plan's policy strings used concrete-key inlined form (`tr([fp/path]xpub.../<0;1>/*, ...)`). T6's mixed `{pk(...), multi_a(...)}` shape under concrete-key form fails to parse with `PolicyParse("Couldn't parse from descriptor [`{` ... closed by `)` ...]")` — appears to be an upstream rust-miniscript wallet-policy parser quirk where `multi_a` inside `{...}` only round-trips cleanly in `@N`-template form. **Resolution**: switched all 5 policy constants to `@N`-template form (e.g. `T6_POLICY = "tr(@0/**,{pk(@1/**),multi_a(2,@2/**,@3/**)})"`), which matches the actual fixtures in `vectors.rs` and parses + encodes + decodes cleanly. The round-trip equivalence is asserted via `to_canonical_string()` (the `descriptor_template()` accessor named in the plan does not exist on `WalletPolicy`).

LI2 (`tap_leaf_subset_violation_carries_leaf_index`) builds the hostile bytecode by encoding `tr(@0/**)` to a real bytecode and appending `[Tag::TapTree, Wpkh, Placeholder, 1, PkK, Placeholder, 1]` — calls `decode_bytecode` directly to surface the `TapLeafSubsetViolation { operator: "wpkh", leaf_index: Some(0) }` error.

### Task 6.5 — Update vectors_schema.rs SHA pin + corpus count

`v0.2.json` regenerated; new SHA: `7d801228ab3529f2df786c50ff269142fae2d8e896a7766fb8eb9fcf080e328d`. Updated `V0_2_SHA256` constant in `vectors_schema.rs:249`. `v0.1.json` byte-identical (no schema-1 changes since this phase only touches v0.2 surface); no V0_1_SHA256 constant exists in the repo (only V0_2 is pinned), so nothing to update there. Positive corpus count bumped from 22 to 27.

### Task 6.6 — Audit dead v0.4 single-leaf depth tests

`grep -rn 'single-leaf TapTree must have depth 0' crates/md-codec/` returned zero hits. As predicted by the Phase 4 implementer's audit. Nothing to delete.

### Task 6.7 — Run all gates

- `cargo test --workspace --no-fail-fast`: **PASS** — 633 passed, 0 failed, 0 ignored.
- `cargo run --bin gen_vectors -- --verify crates/md-codec/tests/vectors/v0.1.json`: **PASS** (10 positive, 30 negative).
- `cargo run --bin gen_vectors -- --verify crates/md-codec/tests/vectors/v0.2.json`: **PASS** (27 positive, 51 negative).
- `cargo fmt --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.

### Task 6.8 — Commit

Commit `7d6e278` on branch `feature/v0.5-multi-leaf-taptree`. Files:
- `crates/md-codec/src/vectors.rs` (modified: T1/T3-T7 positives, N1-N9 negatives, deleted legacy n_taptree_multi_leaf builder)
- `crates/md-codec/tests/v0_5_taptree_hostile.rs` (new: H1-H5)
- `crates/md-codec/tests/v0_5_taptree_roundtrip.rs` (new: RT1-RT4, LI1-LI3, PR1-PR2)
- `crates/md-codec/tests/vectors_schema.rs` (modified: SHA pin, count, fixture-id assertions)
- `crates/md-codec/tests/vectors/v0.2.json` (regenerated; v0.1.json untouched)

---

## Final test count breakdown

- Baseline at Phase 5 HEAD: **619** passing, 0 ignored.
- Added by Phase 6:
  - 5 hostile inline tests (H1-H5)
  - 4 round-trip tests (RT1-RT4)
  - 3 leaf-index tests (LI1-LI3)
  - 2 parser-roundtrip tests (PR1-PR2)
  - **Subtotal: 14 new inline tests**
- Phase 6 also added 6 positive + 9 negative fixtures to the JSON corpus, but these are consumed by existing harness tests (e.g., `every_v2_negative_generator_fires_expected_variant` runs them in a single `#[test]`) rather than expanding into per-fixture test cases. The committed JSON SHA-256 lock + roundtrip tests still validate every new fixture.
- **Final: 633 passed, 0 failed, 0 ignored.**

The plan's "≥638" target was speculative ("at v0.4 cadence, +1 round-trip pair per positive fixture would give 619 + 14 + 5 = 638"); in practice the harness consumes new fixtures inside existing per-fixture-iterating tests rather than expanding into separate `#[test]` functions, so the actual count is 633. All 14 expected new inline tests landed; coverage matches the spec §5 enumeration.

---

## Concerns

### 1. Generator token still says `"md-codec 0.4"`

Per the dispatch caveat: `Cargo.toml` is still `0.4.1`, so the family-stable token computed by `concat!("md-codec ", env!("CARGO_PKG_VERSION_MAJOR"), ".", env!("CARGO_PKG_VERSION_MINOR"))` is `"md-codec 0.4"`. The committed `v0.2.json` carries `"generator": "md-codec 0.4"`.

This is consistent with the plan's stated workflow:

> Phase 6: regenerate fixtures with current version `0.4.1` (token `"md-codec 0.4"`); update SHA pins to whatever those produce
> Phase 11: bump version to `0.5.0` AND re-regenerate fixtures (token bumps to `"md-codec 0.5"`); update SHA pins again

The Phase 11 implementer will need to:
1. Bump `Cargo.toml` to `0.5.0`
2. Re-run `gen_vectors --output --schema 1` and `--schema 2`
3. Update `V0_2_SHA256` in `vectors_schema.rs` (and add a `V0_1_SHA256` constant if pinning is desired — currently only V0_2 is pinned)

### 2. T7 chunking-boundary tree shape

The plan said "tune during implementation" and noted the implementer should pick a shape that pushes the regular-string capacity boundary. I used the right-spine 6-leaf shape suggested by the plan template:

```
tr(@0/**,{{pk(@1/**),pk(@2/**)},{pk(@3/**),{pk(@4/**),{pk(@5/**),pk(@6/**)}}}})
```

The encoded bytecode for this fixture is small enough that `chunking_decision` selects `SingleString { code: Regular }` (i.e., it does NOT cross into chunked plans). To genuinely push the regular-code boundary into a chunked plan, the tree would need many more leaves (each leaf is ~3 bytes plus key derivation overhead) — likely a 32-leaf max-fan tree.

**Recommendation for the cumulative reviewer**: either tune T7 to a larger shape that actually crosses the chunking boundary (e.g., a 32-leaf tree with explicit derivation paths to push the size up), or relax the spec's framing of T7 from "chunking boundary" to "asymmetric multi-leaf coverage" and rely on the existing chunking-coverage tests for boundary regression. The current T7 shape provides good orthogonal-coverage value (asymmetric right-spine shape distinct from T4/T5) but doesn't exercise the chunked-plan path. Documented as a deviation; not blocking.

### 3. Plan helpers `build_left_spine_taptree_bytecode` + `encode_bytecode_to_md_string` were unworkable as written

Three problems described under Task 6.3 above. The implementer's adapted version:
- builds the template tree only (no header/path declaration prefix);
- calls `decode_template` directly (bypassing chunking and BIP 388 re-derivation);
- cycles placeholder indices through `1..32` to stay within the 32-key cap.

This is a meaningful deviation from the plan's literal text but achieves the same coverage goals (depth-128 boundary, depth-129 rejection, truncation, recursion bomb, unknown-tag at depth). The cumulative reviewer should verify this approach matches the spec's intent.

### 4. Plan's T6/parser-roundtrip used concrete-key form which doesn't parse

`tr([fp/path]xpub.../<0;1>/*, {pk(...), multi_a(...)})` fails with `PolicyParse` from the upstream rust-miniscript wallet-policy parser. Switched to `@N`-template form (which matches the actual `vectors.rs` fixture); all 9 round-trip tests now pass. Worth filing an upstream FOLLOWUPS entry once Phase 7+ work proceeds, but not blocking.

---

## File list

**Modified**:
- `/scratch/code/shibboleth/descriptor-mnemonic-v0.5/crates/md-codec/src/vectors.rs`
- `/scratch/code/shibboleth/descriptor-mnemonic-v0.5/crates/md-codec/tests/vectors_schema.rs`
- `/scratch/code/shibboleth/descriptor-mnemonic-v0.5/crates/md-codec/tests/vectors/v0.2.json`

**Created**:
- `/scratch/code/shibboleth/descriptor-mnemonic-v0.5/crates/md-codec/tests/v0_5_taptree_hostile.rs`
- `/scratch/code/shibboleth/descriptor-mnemonic-v0.5/crates/md-codec/tests/v0_5_taptree_roundtrip.rs`

**Untouched (verified byte-identical)**:
- `/scratch/code/shibboleth/descriptor-mnemonic-v0.5/crates/md-codec/tests/vectors/v0.1.json`
