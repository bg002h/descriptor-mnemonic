# PLAN — md-codec seven-fragment render goldens (GAP 2)

**Date:** 2026-06-12 · **Crate:** `md-codec` · **SemVer:** NO-BUMP (test-only; PATCH *only* if a render bug surfaces during golden bring-up)
**Source SHA:** descriptor-mnemonic `origin/main` = `e798431` (recon ran at `422b049`; one docs-only commit since). Companion toolkit `origin/master` = `d3fafaf`.
**Recon (evidence base / spec round-1 log):** `mnemonic-toolkit/cycle-prep-recon-seven-fragment-render-tests.md`. **FOLLOWUP:** none exists; this cycle re-grounds two dangling resolved cites instead of filing-then-resolving.

## 1. Problem (grounded, recon-verified)

`md-codec`'s `to_miniscript.rs::node_to_miniscript` (the decoded-`Node` → miniscript-string renderer) has **seven arms with zero render-layer execution** in the entire suite — `DupIf` (`d:`, :337), `NonZero` (`j:`, :341), `ZeroNotEqual` (`n:`, :345), `OrB` (:368), `OrC` (:374), `False` (:458), `True` (:459). All seven are generated at the WIRE layer (`W_TARGET_TAGS[34]` @ `tests/proptest_to_miniscript.rs:727`, anti-vacuity-enforced by `w_generator_covers_all_fragments`), so they encode↔decode round-trip — but the **render** is verified only by P6, which runs **only over the T strategy**, and `T_TARGET_TAGS[23]` (@ `:765`) omits all seven. `to_miniscript.rs` has **no `#[cfg(test)] mod`**. So a mis-render (swapped `OrB`/`OrC` children, `True`/`False` transposed, a wrong wrapper) would ship silently → wrong descriptor → wrong address.

The recon empirically ran all seven through the full P6 chain (encode→decode→`to_miniscript_descriptor`→`Descriptor::from_str` reparse fixed-point→mainnet address) against the pinned miniscript 13.0.0: **7/7 pass, 0 are P7-only.** The exclusion is pure generator scope, not type difficulty.

Historical note: the byte-form pins for `or_c`/`d:`/`j:`/`n:`/`True`/`False` lived in `src/bytecode/hand_ast_coverage.rs` (637 LOC), **deleted at `5350f8a` (v0.12.0 strip)** and never migrated. ~7 entries cite that dead file (R0-R1 audit); the THREE that are fragment-coverage claims (not historical process records) are the `v06-corpus-*` entries re-grounded by (C) below: `v06-corpus-d-wrapper-coverage` (`:916-923`), `v06-corpus-or-c-coverage` (`:925-933`), `v06-corpus-j-n-wrapper-coverage` (`:934-942`).

## 2. The fix (test-only)

**(A) Seven deterministic P6 golden cells** in `tests/proptest_to_miniscript.rs`, mirroring the existing `self_test_*` house style (hand-built typed `Node` via `common::{descriptor_with_pubkeys, …}` → full `p6_chain(&d)` → assert the rendered-string fragment + a golden mainnet address literal). The seven shapes (recon §4, re-derive + prefix-verify addresses at impl time — the reparse fixed-point inside `p6_chain` is the mis-render oracle):

| Cell | Typed shape | Renders as (sugar to pin) |
|---|---|---|
| `self_test_wsh_or_b_pk_s_pk` | `wsh(or_b(pk(@0), s:pk(@1)))` | `or_b(pk,s:pk)` |
| `self_test_wsh_t_or_c_true` | `wsh(and_v(or_c(pk(@0), v:pk(@1)), True))` | `t:or_c(pk,v:pk)` (and_v(X,1)→`t:`) |
| `self_test_wsh_or_i_dupif_v_older` | `wsh(or_i(pk(@0), DupIf(Verify(older(144)))))` | `or_i(pk,dv:older(144))` |
| `self_test_wsh_nonzero_pk` | `wsh(NonZero(pk(@0)))` | `j:pk` |
| `self_test_wsh_or_i_zne_and_v` | `wsh(or_i(pk(@0), ZeroNotEqual(and_v(v:pk(@1), older(144)))))` | `or_i(pk,n:and_v(v:pk,older(144)))` |
| `self_test_wsh_or_i_false_u_sugar` | `wsh(or_i(pk(@0), False))` | `u:pk` (or_i(X,0)→`u:`) |
| `self_test_wsh_and_v_true_t_sugar` | `wsh(and_v(v:pk(@0), True))` | `tv:pk` |

Each cell asserts (i) `addr.starts_with("bc1q")`, (ii) the exact golden address literal, and (iii) a `contains()` on the rendered string's ANCHORED fragment marker (R0-M1): `or_b(`, `t:or_c(`, `dv:older(144)`, `j:pk(`, `n:and_v(`, `u:pk(`, `tv:pk(` — anchored forms (not bare `u:`/`j:`/`tv:`) so a wrapper-fusion regression is caught unambiguously. The `True`/`False` arms are pinned via their sugar consumers (`t:`/`u:`) since rust-miniscript never Displays a literal `1`/`0` here — that IS the contract.

**(B) Permanent property coverage** — extend `t_segwit_tree` (`tests/common/mod.rs`) with one production per fragment (the seven proven shapes) and grow `T_TARGET_TAGS` 23→30 so `t_generator_covers_all_fragments` anti-vacuity-enforces the seven under the P6 property forever (converts one-shot goldens into a standing gate). wsh-only (these are mostly sigless-branch shapes; tr() carries the from_str sanity branch, the rest of wsh does not — per the R4 spec).

**(B) TYPING-BOUNDARY CONSTRAINT (R0-I2, load-bearing):** the new productions are the FIXED seven proven shapes — children drawn ONLY from `t_ka()` (keys) and `t_lock_node` in the two proven lock positions (`dv:LOCK`, `n:and_v(v:pk,LOCK)` — type-safe for BOTH lock classes, older/after both Bz, v:LOCK is Vz). Do NOT parameterize `or_b`/`or_c`/`j:`/`n:` child positions over the grammar's `leaf_any`/`bdu*`/`hash` pools — those compose type-INVALID combinations that P6 step-1 (`proptest_to_miniscript.rs:52-54`, no prop_filter) **panics** on (empirically confirmed counterexamples: `or_b(older(144),s:pk)` and `j:older(144)` both fail `to_miniscript_descriptor` typecheck — `or_b`/`j:` require a `d`/`n`-property child that locks/hashes lack). The house style IS pool-composition, so this constraint is explicit precisely because the naive extension would flake. **R0 confirmed (A)+(B) under this constraint** — (B) is what dissolves the gap class; (A) alone leaves the next new fragment uncovered.

**(C) Re-ground the THREE dangling `v06-corpus-*` FOLLOWUP cites (R0-I1):** `v06-corpus-d-wrapper-coverage` (`design/FOLLOWUPS.md:916-923`), `v06-corpus-or-c-coverage` (`:925-933`), `v06-corpus-j-n-wrapper-coverage` (`:934-942`) — all three mark themselves "resolved" pointing at deleted `hand_ast_coverage.rs` tests (`d_wrapper_tap_leaf_byte_form`, `or_c_unwrapped_tap_leaf_byte_form`/`t_or_c_tap_leaf_round_trips`, `j_wrapper_tap_leaf_byte_form`/`n_wrapper_tap_leaf_byte_form`). Note the bytecode-layer file was removed in the v0.12.0 strip (`5350f8a`) and re-point the "resolved" evidence at the new render-layer cells from (A) (which cover d:/or_c/j:/n: — the j-n entry was missed by R1, now included). The seven other historical/process entries that cite the dead file (`:904`, `:1000`, `:1031-35`, `:1041`, `:1074-78`, `:1093-96`, `:1135-39`) are process records, NOT fragment-coverage claims — left as-is (scope choice; not re-grounded by this cycle).

**(D) Toolkit companion — SPLIT to its own NO-BUMP edit (R0-Q2/M2):** `parse_descriptor.rs::arm_dup_if` (`d3fafaf:2421`) is `#[ignore]`d with a DISPROVEN reason ("DupIf descriptor-unreachable in v13" — the recon + R0 both parsed `wsh(or_i(pk,dv:older(144)))` via `Descriptor::from_str` on 13.0.0) AND has an EMPTY stub body — so the work is WRITING the body (`wsh(or_i(pk(@0/<0;1>/*),dv:older(144)))` + `find_tag(Tag::DupIf)`, mirroring `arm_non_zero` at `:2426`), not a pure de-ignore. Split from this md-codec-local cycle; **file a toolkit FOLLOWUP `toolkit-arm-dup-if-ignored-stub` in THIS cycle** (per R0-M2) so the disproven-ignore doesn't go tracking-less until the paired edit lands.

## 3. TDD / oracle

This is additive coverage, so "RED" is the golden-derivation step, not a pre-existing failure: build each cell, run it, and **if `p6_chain`'s reparse fixed-point fails for any of the seven, that is a FOUND RENDER BUG** → stop, escalate to a PATCH fix cycle (not expected — the recon's reparse oracle agreed on all seven). Capture the real derived address, prefix-verify (`bc1q`), pin it. For (B), `t_generator_covers_all_fragments` flips RED until the grammar productions are added (anti-vacuity proves the seven are actually generated). GREEN gate: full `cargo test -p md-codec` + `cargo clippy --all-targets -- -D warnings` + `cargo fmt --all --check`.

## 4. Lockstep / SemVer
- **NO-BUMP** (test + docs only). No public API/wire change → no version bump, no md-cli pin change, no crates.io publish, no toolkit tail. (Escalates to PATCH only if (3) finds a render bug.)
- No clap surface change → no manual mirror, no GUI schema_mirror.
- (D) if folded = a toolkit NO-BUMP edit (separate commit/repo).

## 5. R0 questions
1. (A)+(B) or (A)-only? (Recommend both — (B) is the standing gate that dissolves the gap class.)
2. Fold (D) toolkit `arm_dup_if` de-ignore here, or split? (Default: split.)
3. Golden discipline: confirm the existing `self_test_*` derive-once-then-pin pattern is the house standard to follow (it is — `self_test_wsh_and_v_pk_older_144` etc.), and that prefix-verify + reparse-fixed-point is sufficient address-oracle independence (the existing cells rely on exactly that).
4. Any of the seven better tested as a wire/decode cell too (defense beyond render)? (Lean no — wire is already anti-vacuity-covered; render is the gap.)

## 6. Risks
- **Low** — pure test-add against a renderer the recon already exercised. The one real possibility is a golden-bring-up mis-render (→ a genuine found bug, PATCH). Wrapper-sugar forms (`t:`/`u:`/`dv:`/`tv:`) must be pinned as rendered (not the desugared input) — the plan's `contains()` markers encode that.
