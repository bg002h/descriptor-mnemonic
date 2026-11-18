# Review: baseline GAP-B hardening PR #56 (descriptor-mnemonic), base 62280dd9, head 6ce4952b

Reviewer: Claude Sonnet 5.5 (claude-sonnet-5-5). Author: Sonnet implementer (Claude Sonnet 5.5). Round 1 (not a re-review, 0 findings from a prior fix commit).
No Critical, no Important found.

## Lens 1: production code
`git diff --stat` shows only crates/md-codec/tests/baseline_gapb.rs, crates/md-cli/tests/baseline_gapb.rs, the author report, and three src/ files
(parse/template.rs +37, seat/compose.rs +36, seat/partition.rs +23). I read every src/ hunk: all are pure additions, inside a new
`#[cfg(test)] mod gapb_resolve_tests` (template.rs) or the existing `mod tests` (compose.rs, partition.rs). No non-test line changed. cargo fmt --check clean.

Private-path need (settled addition a):
- template.rs `resolve_placeholders`: genuinely needs the unit test. Run through the CLI, `md encode` of `@0/<0;1>/*,@0/<0;2>/*` and of `/*` vs `/*h`
  refuses upstream (OVERLAP / HARDENING messages) before reaching line 846, so no CLI test can kill it. Confirmed.
- seat/compose.rs `comparison_form`: needs a hand-built `tr(NUMS, sortedmulti_a)` tree swapped onto a composed descriptor; private fn, sound.
- seat/partition.rs `partition`: sits beside existing partition unit tests using the same fixtures (v-ap-shared / v-ap-incomplete); acceptable. (Nit N1.)

## Lens 2: mutants re-applied (scratch edits, one test each, reverted; worktree clean after)
All 38 claimed kills reproduced; I re-ran every row (all 10 md-codec, all 28 md-cli), each FAILs on the mutant and the 9+8+3 tests PASS on head:
- md-codec: bch.rs:99/100 verify -> true FAIL; bch_decode.rs:268 `>`->`==` FAIL, `>`->`<` FAIL; :432 `||`->`&&` FAIL; canonical_origin.rs:75 `&&`->`||` FAIL;
  tlv.rs:122 `<=`->`>` FAIL; validate.rs:301 `==`->`!=` FAIL; :637 Variable arm FAIL; :661 Children arm FAIL; :662 Variable arm FAIL.
- md-cli: compose parse_u16 Ok(0) and Ok(1) FAIL; :89, :106 guards -> true FAIL; walk.rs:248 `||`->`&&` FAIL; walk.rs hash translators 332/335/338/341
  (String::new and "xyzzy" variants sampled across all four) FAIL; text.rs fmt_md1_id/fmt_template_id/fmt_policy_id (empty and "xyzzy") FAIL;
  template.rs:846 FAIL (unit); 1542, 1548, 1731, 1752 arm deletions FAIL; 2937, 2940 `minting`->false FAIL; compose.rs:148 arm FAIL (unit); partition.rs:296 `&&`->`||` FAIL (unit).
All wrong-result survivors the grader flagged (bch 100, bch_decode 432/268, canonical_origin 75, validate 301/661/662, parse_u16, :89/:106, walk 248, fmt ids) are in the above.
(Note: a first few of my own perl edits missed their line and showed PASS; I re-applied at the right lines and confirmed non-empty `git diff` before reading each result.)

## Lens 3: expected values
Independent of the code under test: `older(4194404)` = BIP 68 type-flag bit 22 (4194304) + 100; heightlock/timelock mix uses the 500,000,000 threshold of
BIP 65/miniscript; sh(wsh(multi)) canonical origin m/48'/0'/0'/1' is BIP 48 script-type 1; hash digests are fixed literals round-tripped; round-trip
template strings are literal. md-codec inspect ids are computed by md-codec identity fns and compared with the printed lines: self-consistency, but the
mutants are on the CLI formatters so the pin is real (M2).

## Lens 4: robustness
No timing, no randomness. Runtimes: md-codec baseline_gapb 9 tests 0.022s total; md-cli integration 8 tests 0.045s; 3 unit tests 0.021s. (Profile is
the repo's optimized+debuginfo test profile; no `--release`.)

## Lens 5: EQUIV
None claimed (EQUIV 0). Grader's 10 md-codec and 28 md-cli GAP-B rows are all accounted for (10 + 28 = 38).

## Findings
- M1 (Minor) crates/md-codec/tests/baseline_gapb.rs `decode_regular_errors_corrects_a_three_error_word`: asserts only `pos.len()==3` and `mag.len()==3`. It kills the :268 mutants, but does not check the correction is right. Fix text: after the asserts, apply the magnitudes at `pos` to a codeword-residue check, or at minimum assert `pos` are distinct and each `< 66`... (the kills do not depend on it).
- M2 (Minor) md-cli `inspect_prints_the_three_ids_in_full` takes expected ids from md-codec's compute_* fns, not pinned hex constants; a regression in the md-codec id derivation would be invisible here (covered by md-codec's own vectors).
- N1 (Nit) partition.rs stray-piece unit test might also be reachable via `md seat`; unit placement is consistent with neighbours, no action.
- N2 (Nit) the author's report says "md-cli 8/8 integration + 3 unit"; I measured the same counts.

C 0 / I 0 / M 2 / N 2
