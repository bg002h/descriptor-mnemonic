# Phase H — code review r1 (2026-05-10)

**Working tree:** dirty at HEAD `45d4702`; not yet committed.

**Scope:** md-codec v0.30 Cycle 4 Phase H — corpus regen + cleanup. Lift 6 `#[ignore]`d `tree.rs::tests`; fix 2 RED `help_examples` md-cli tests; delete 2 dead `pkk` test helpers in `canonicalize.rs`; resolve 3 FOLLOWUPs. Plus the implementer caught 4 additional RED tests at HEAD (`vectors_output_matches_committed_corpus`, `decode_json_snapshots`, `inspect_json_snapshots`, `encode_wpkh_default_phrase`) that the Cycle 3 exit verification missed and absorbed them into Phase H's atomic commit (corpus-regen-class; scope-consistent).

**Files reviewed:** `crates/md-codec/src/{tree, canonicalize}.rs`; `crates/md-cli/src/main.rs`; `crates/md-cli/tests/{smoke.rs, snapshots/*}`; `crates/md-codec/tests/vectors/*`; `design/FOLLOWUPS.md`.

---

## Critical (block ship)

None.

## Important (must fix before ship)

None.

## Low (file as FOLLOWUP)

None.

## Nit (optional polish — fixed inline)

### N-1 — Stale `pkk(...)` reference in test comment

- **Where:** `crates/md-codec/src/canonicalize.rs:975`
- **What:** Comment in `round_trip_canonicalize_encode_decode_canonicalize` test body: `"Children are pkk(@perm[i]) — but to match \`n\` we must use ..."`. Both `pkk()` helper functions are deleted (no call sites remain), so a future reader grepping `pkk` will find nothing.
- **Fix (applied):** Reworded to `"Children are pk_k(@perm[i]) — ..."` to avoid the dead-helper name.

---

## Correctness checks (all passed)

1. **All 6 tree.rs lifts.** Each `#[ignore]` removed; bit-count assertions match predictions (38, 262, 166, 262, 6, 32); semantics unchanged. ✓
2. **`hash256_round_trip` rename.** Function signature renamed; no in-repo references to `hash256_extension_round_trip` outside FOLLOWUPS.md bookkeeping. ✓
3. **`tap_tree_two_leaf_round_trip` doc-comment.** Math rewritten: `Tag::Tr (6) + is_nums (1) + kiw (2) + has_tree (1) + Tag::TapTree (6) + 2×(Tag::PkK (6) + kiw (2)) = 32 bits`. Verified. ✓
4. **`canonicalize.rs` pkk deletion.** Both functions removed; build clean; no broken references. ✓
5. **md-cli `main.rs` md1 string updates.** Both `after_long_help` strings embed `md1yqpqqxqq8xtwhw4xwn4qh`, consistent with `smoke.rs` pin and `wpkh_basic.phrase.txt` corpus. ✓
6. **15 insta snapshots.** Internal structure matches `Body::MultiKeys` shape for multi-family descriptors (correct post-Phase-C). ✓
7. **26 corpus vector files.** Triple consistency: `phrase.txt` md1 ↔ `bytes.hex` ↔ `descriptor.json` template. Regenerated via `md vectors --out` (standard tool). ✓
8. **md-cli `tests/smoke.rs::encode_wpkh_default_phrase` pin update.** Matches new post-v0.30 wire output. ✓
9. **FOLLOWUP entries.** All 3 marked `resolved (Phase H: ...)`:
   - `v0.30-phase-a-tree-tests-ignored-pending-corpus-regen` — 12/12 done
   - `v0.30-phase-c-help-examples-md1-strings-drift` — strings updated
   - `v0.30-phase-c-canonicalize-pkk-helpers-dead-code` — both deleted ✓
10. **Out-of-scope absorption.** The 4 RED-at-HEAD tests (insta snapshots + vector corpus + md-cli smoke pin) are corpus-regen-class — Phase H's named scope. The Cycle 3 exit verification I ran was incomplete (piecemeal named-crate tests instead of `cargo test --workspace`); absorbing the cleanup into Phase H is correct and scope-consistent. ✓
11. **Cycle 4 completion contract.** Workspace 451/0/0; 0 ignored in tree.rs; 3 v0.30-active FOLLOWUPs remain (correctly scoped for Cycle 5 / v1+): `v0.30-phase-b-r1-nit-1` → Phase J; `v0.30-phase-g-operator-context-violation-unwired` → v1+; `repo-hygiene-stale-file-location-doc-artifacts` → v1+. ✓

---

## Verdict

**Ship.** 0C/0I/0L/1N (fixed inline). Cycle 4 exit criteria fully satisfied. Cycle 5 (Phases I + J: BIP rewrite + final tag) inherits a workspace-green, zero-ignored, three-open-FOLLOWUP state.
