# R0 Review — md-codec seven-fragment render goldens (PLAN) — Round 2
Reviewer: Fable 5, 2026-06-12. Verified against descriptor-mnemonic origin/main e798431.
## Verdict: GREEN (0C/0I)

## Findings

**I1 (three v06-corpus-* entries, drop :1041) — LANDED.** §2(C) names exactly `v06-corpus-d-wrapper-coverage` (:916-923), `v06-corpus-or-c-coverage` (:925-933), `v06-corpus-j-n-wrapper-coverage` (:934-942), incl. the previously-missed j-n entry. Live `design/FOLLOWUPS.md` @ e798431: headers at :916/:925/:934; all three Status lines cite tests that lived only in the file deleted at 5350f8a (`d_wrapper_tap_leaf_byte_form`; `or_c_unwrapped_tap_leaf_byte_form`/`t_or_c_tap_leaf_round_trips`; `j_wrapper_tap_leaf_byte_form`/`n_wrapper_tap_leaf_byte_form`). :1041 correctly reclassified as `v07-phase2-or-c-unwrapped-test-docstring-drift` and left as-is.

**I2 (typing-boundary constraint) — LANDED.** §2(B) carries the load-bearing constraint: fixed seven shapes, children only from `t_ka()` (common/mod.rs:808) and `t_lock_node` (:788) in the two proven lock positions; prohibition on routing or_b/or_c/j:/n: children through leaf_any/bdu*/hash pools, with the type-fail counterexamples and the panic site. Verified P6 step 1 (proptest_to_miniscript.rs:52-54) is `unwrap_or_else(panic!)` with no prop_filter — constraint correctly framed as flake-prevention.

**M1 (anchored contains markers) — LANDED.** §2(A): `or_b(`, `t:or_c(`, `dv:older(144)`, `j:pk(`, `n:and_v(`, `u:pk(`, `tv:pk(`.

**M2 (arm_dup_if empty stub) — LANDED.** §2(D): body is EMPTY, work is WRITING it, file `toolkit-arm-dup-if-ignored-stub` in this cycle. Verified @ toolkit d3fafaf src/parse_descriptor.rs: `#[ignore=...]` :2420, `fn arm_dup_if()` :2421 (comment-only body), `arm_non_zero` :2426.

**M3 (§1 undercount) — LANDED.** §1 now "~7 entries."

## Unchanged core — re-confirmed sound
- NO-BUMP scope holds (tests + design/FOLLOWUPS.md only; (D) split to toolkit). No API/wire change; PATCH only on a found render bug. No scope creep.
- All citations live @ e798431: seven arms to_miniscript.rs :337/:341/:345/:368/:374/:458/:459; no `#[cfg(test)]` in to_miniscript.rs; `W_TARGET_TAGS:[Tag;34]` :727 (all 7); `T_TARGET_TAGS:[Tag;23]` :765 (omits 7; 23+7=30); anti-vacuity :811/:845; house cell `self_test_wsh_and_v_pk_older_144` :136; `t_segwit_tree` common/mod.rs:822.
- The 7 shapes double-verified in Round 1; the fold introduced no contradiction.

## Residual / Minor (cosmetic, non-blocking)
- **R2-m1:** §2(C) "The ~4 other historical/process entries" then enumerates SEVEN refs — change "~4" → "seven". (Folded in this round.)
- **R2-m2:** or-c/j-n ranges (:925-933, :934-942) include the trailing blank line (entries end :932/:941); harmless — header anchors exact.
- **R2-m3:** spell the full path `crates/mnemonic-toolkit/src/parse_descriptor.rs` (a bin-crate `#[cfg(test)]` module → needs `--bin mnemonic`, not `--lib`) in the toolkit FOLLOWUP when filed.

**Gate: GREEN — implementation may begin.**
