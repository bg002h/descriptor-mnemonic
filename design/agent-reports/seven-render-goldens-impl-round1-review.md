# Impl Review — md-codec seven-fragment render goldens — Round 1
Reviewer: Fable 5, 2026-06-12. Verified against the descriptor-mnemonic working tree.

## Verdict: GREEN (0C/0I)

## Critical
None.
## Important
None.
## Minor
- **m1 (version-cite imprecision in the (C) NOTEs + (D) toolkit FOLLOWUP).** All three re-grounding NOTEs and the toolkit FOLLOWUP attribute the new cells to the "md-codec 0.35.2 GAP-2 cycle", but tag `md-codec-v0.35.2` ALREADY EXISTS at 422b049 (the k>n encoder fix) and does NOT contain these tests — this cycle is NO-BUMP, landing post-tag. Suggest "post-0.35.2 NO-BUMP GAP-2 cycle" or the landing SHA. Docs-only. [FOLDED.]
- **m2 (conservative narrowing vs plan §2(B)).** The plan/R0-I2 permitted key children from `t_ka()` (pk | pkh); the `seven` production pins `keyarg(Tag::PkK, 0)` only. Strictly WITHIN the R0-I2 constraint (more conservative, matches the house `w0` style which hardcodes PkK for `s:`); placeholder-index-0 → `assign_sequential_indices` used correctly (no duplicate-key reparse risk). Acceptable as-is.

## Checks run
1. (A) 7 cells — `cargo test ... self_test_wsh_`: 9/9 PASS. Each is a valid typed wsh shape via house helpers, full `p6_chain`; anchored `contains()` markers match the plan exactly and are non-trivial; 7 DISTINCT 62-char bc1q goldens (no copy-paste dup). No no-op cells.
2. (B) `seven` / R0-I2 crux — children ONLY `keyarg(Tag::PkK,0)` + `t_lock_node(...)` in the two proven lock positions; NO leaf_any/bdu*/hash pool reaches an or_b/or_c/j:/n: child slot; constraint comment in-code. STRESS `PROPTEST_CASES=2000 ... p6_typed_to_miniscript_round_trip t_generator_covers_all_fragments` — BOTH PASS, zero panics (8.96s). PROPTEST_CASES honored (verified: 8-case 0.04s vs 2000 8.96s; no ProptestConfig override).
3. T_TARGET_TAGS = `[Tag; 30]` (+ the 7); compiles; `t_generator_covers_all_fragments` iterates the array (2048 samples); no hardcoded 23. Anti-vacuity PASS.
4. No regression — `cargo test -p md-codec` 16/16 binaries ok, exit 0; clippy `-D warnings` clean; `cargo fmt --all --check` clean.
5. (C) re-grounding — d-wrapper→dupif ✓, or-c→t_or_c ✓, j-n→nonzero+zne ✓; all correctly state `hand_ast_coverage.rs` (637 LOC) removed at `5350f8a` (`git show --stat 5350f8a` confirms the deletion).
6. Scope — descriptor-mnemonic diff = ONLY `tests/proptest_to_miniscript.rs` (+177/−1), `tests/common/mod.rs` (+65/−1), `design/FOLLOWUPS.md` (+3); `src/to_miniscript.rs` UNTOUCHED (renderer tested, not changed). Toolkit diff = `design/FOLLOWUPS.md` +10 (the FOLLOWUP). No stray churn.
7. NO-BUMP — no Cargo.toml/lock change; md-codec stays 0.35.2; md-cli pin unchanged; no wire/API change.
8. (D) FOLLOWUP claims verified vs toolkit source: `#[ignore]` parse_descriptor.rs:2420, `fn arm_dup_if()` :2421 comment-only body, `arm_non_zero` :2426; full path + `--bin mnemonic` note present.

## Plan conformance
Matches the GREEN round-2 plan on every load-bearing point. Only deviations: m1 (version phrasing) + m2 (PkK-only, conservative) — both Minor.

**Gate: GREEN — cleared to commit (fold m1).**
