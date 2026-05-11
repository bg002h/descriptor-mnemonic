# Phase C — code review r1 (2026-05-10)

**Working tree:** dirty at HEAD `7944f00`; not yet committed.

**Scope:** md-codec v0.30 Cycle 3 Phase C — introduce `Body::MultiKeys { k, indices: Vec<u8> }`; split encoder/decoder arms (Thresh stays on `Body::Variable`); sweep exhaustive matches; rewrite test fixtures; lift 1 FOLLOWUP-ignored test; rename 1 stale test.

**Files reviewed:**
- `crates/md-codec/src/{tree,canonicalize,canonical_origin,derive,identity,validate}.rs`
- `crates/md-cli/src/{format/{json,text},parse/template}.rs`
- `crates/md-codec/tests/{smoke,chunking,wallet_policy,address_derivation}.rs`
- `design/FOLLOWUPS.md`, `design/SPEC_v0_30_wire_format.md` §4

---

## Critical (block ship)

None.

## Important (must fix before ship)

### I-1 — `v0.30-phase-a-tree-tests-ignored-pending-corpus-regen` FOLLOWUP `Status:` not updated to reflect Phase C partial lift

- **Where:** `design/FOLLOWUPS.md:493`
- **What:** Entry body correctly strikethroughs `sortedmulti_2of3_bit_cost` as lifted in Phase C. But the `Status:` field still reads `open` — a Phase H implementer reading only the Status line cannot tell one test was already lifted, and may mis-count 11 remaining vs the original 12.
- **Fix:** update Status to `partial — sortedmulti_2of3_bit_cost lifted Phase C; 11 remain; Phase F lifts 5, Phase H lifts 6`.

### I-2 — `help_examples` test failure has no FOLLOWUP entry

- **Where:** `design/FOLLOWUPS.md` (missing entry)
- **What:** `decode_example_matches_actual_output` and `encode_example_matches_actual_output` (md-cli `help_examples`) fail because embedded literal md1 strings in `--help` text drift after multi-packing. The implementer recommends Phase H corpus regen but filed no tracking entry. Without a FOLLOWUP, the Phase H implementer has no signal to revisit these two tests.
- **Fix:** file `v0.30-phase-c-help-examples-md1-strings-drift` as v0.30 (active; lift gated by Phase H).

## Low (file as FOLLOWUP — ship can proceed)

### L-1 — `pkk` test helper `#[allow(dead_code)]` in two places in `canonicalize.rs`

- **Where:** `crates/md-codec/src/canonicalize.rs:474` and `:1028`.
- **What:** Both helpers have no live call sites after Phase C. Annotation reads `#[allow(dead_code)] // retained for non-multi-family fixtures; multi-family now uses MultiKeys`. Implementer kept them; reviewer prefers deletion (helpers, not deps).
- **Decision:** keep `#[allow(dead_code)]` for Phase C ship; file FOLLOWUP for Phase H cleanup decision (delete or rewire to MultiKeys fixtures).

### L-2 + L-3 — `//! #file location ...` / `//! file location ...` artifacts at line 1 of `derive.rs` and `tests/address_derivation.rs`

- **Where:** `crates/md-codec/src/derive.rs:1`, `crates/md-codec/tests/address_derivation.rs:1`
- **What:** Tooling-injected artifact strings that render literally in `cargo doc`. **Pre-exist on HEAD `7944f00`** — verified via `git show HEAD:path`. Not introduced by Phase C; out-of-scope for this commit per "Don't add features, refactor, or introduce abstractions beyond what the task requires" memory.
- **Decision:** file repo-hygiene FOLLOWUP; defer fix to opportunistic cleanup or Phase H.

## Nit (optional polish — not blocking)

### N-1 — `sortedmulti_2of3_bit_cost` doc-comment uses `Tag::SortedMulti(6)` which reads like a constructor

- **Where:** `crates/md-codec/src/tree.rs:371`.
- **Fix:** rephrase to SPEC §4.2 notation: `Tag(6-bit) | k-1(5) | n-1(5) | 3×kiw(2) = 22 bits`.
- **Decision:** apply inline in the Phase C commit (1-character class change; the comment is being touched anyway).

---

## Correctness checks (all passed)

1. **Wire encode/decode mirror (SPEC §4).** `write_node` for `Body::MultiKeys` at `tree.rs:117–121` emits `(k-1)(5) + (n-1)(5) + n×index(kiw)`. `read_node` for multi tags at `tree.rs:215–224` reads the same. Mirror correct. `sortedmulti_2of3_bit_cost` pin = 22 (= 6+5+5+3×2) matches SPEC §4.2 example.

2. **Exhaustive match coverage.** All 5 walk sites handle both `Body::Variable` (Thresh) and `Body::MultiKeys` (multi-family) with no silent wildcard: `validate.rs:62–83`, `canonicalize.rs:75–92,119–129,275–284`, `derive.rs:116–127` (tag-guarded). `canonical_origin.rs` + `identity.rs` don't walk body directly; confirmed safe.

3. **`derive.rs:122` swap safety.** `multi_threshold_and_sort` is only reached when `is_wsh_inner_multi(tag)` holds (`Tag::Multi | Tag::SortedMulti`). Tag-guard precedes body access at `derive.rs:117–121`. Swap from `Body::Variable` → `Body::MultiKeys` semantically necessary and safe.

4. **Canonicalization permutation.** `remap_indices` at `canonicalize.rs:124–129` applies `perm[*idx as usize]` in-place over `indices.iter_mut()`. Same pattern as `Body::KeyArg` at line 100. Correct.

5. **`derive_address` index traversal.** Iterates `expand_per_at_n` output (not `MultiKeys.indices`). Multi shape dispatched via `classify_derivable_shape` → `WshMulti`/`ShWshMulti`; `build_multi_script` receives full `pubkeys` slice. Semantically correct.

6. **New tree tests present and active.** `multi_keys_body_round_trip` (`tree.rs:389`), `sortedmulti_a_indices_round_trip` (`:407`), `sortedmulti_2of3_round_trip` (`:355` rewritten), `tr_with_single_leaf` (`:475` rewritten), `tr_sentinel_n_3_multi_a_2_of_3_round_trip` (`:683` rewritten). All active, no `#[ignore]`.

7. **FOLLOWUP entries.** `v0.30-phase-a-r1-low-1` correctly marked `resolved (phase c: tree.rs ripemd160 test renamed)`. I-1 gap addressed by Status update in Phase C commit.

8. **Integration fixture rewrites.** `smoke.rs`, `chunking.rs`, `wallet_policy.rs`, `address_derivation.rs` use `Body::MultiKeys` with canonical ascending `@N` indices. Correct.

## Verdict

**Iterate** (light touch): apply I-1 (Status field update), I-2 (file `help_examples` FOLLOWUP), N-1 (1-char doc tweak), plus L-1/L-2/L-3 as separate filed FOLLOWUPS. Then **ship**.

No r2 needed unless I-1 or I-2 fix changes correctness footprint (neither should).
