# Baseline GAP-B hardening, repo md (descriptor-mnemonic)

Model: Claude Sonnet 5.5 (claude-sonnet-5-5). Base: main 62280dd9539a494343054bd26a4bcdcc3f29bde1 (origin tip, unmoved).
Branch test/harden-baseline-gapb. Tests only, no production code (the diff only adds `#[cfg(test)]` modules and test files).

Method: each survivor was applied as a scratch edit (perl one-liner on the stated line, reverted after each run;
the loop is not committed), the one test run with `cargo nextest run --locked -j 8`, and the result line recorded.
Base: every new test passes on the base (md-codec 9/9, md-cli 8/8 integration + 3 unit tests; full workspace
1697 passed). Mutant logs: /scratch/cache/md-harden-gapb-logs/codec-mut.log and cli-mut.log (all rows FAIL;
a FAIL means the test compiled and ran against the mutant).
Row form: survivor -> test (file) ; base PASS ; mutant FAIL.

## md-codec (tests/baseline_gapb.rs), 10 killed

- KILLED bch.rs:100:5 bch_verify_regular -> true -> `bch_verify_regular_rejects_every_single_symbol_corruption`
  - base: PASS [   0.007s] (3/9) md-codec::baseline_gapb bch_verify_regular_rejects_every_single_symbol_corruption
- KILLED bch_decode.rs:268:21 > -> == -> `decode_regular_errors_corrects_a_three_error_word`
  - base: PASS [   0.009s] (5/9) md-codec::baseline_gapb decode_regular_errors_corrects_a_three_error_word
- KILLED bch_decode.rs:268:21 > -> < -> `decode_regular_errors_corrects_a_three_error_word`
  - base: PASS [   0.009s] (5/9) md-codec::baseline_gapb decode_regular_errors_corrects_a_three_error_word
- KILLED bch_decode.rs:432:17 `||`->`&&` -> `decode_regular_errors_refuses_a_word_past_capacity`
  - base: PASS [   0.008s] (4/9) md-codec::baseline_gapb decode_regular_errors_refuses_a_word_past_capacity
- KILLED canonical_origin.rs:75:41 `&&`->`||` -> `sh_wsh_non_multi_has_no_canonical_origin`
  - base: PASS [   0.008s] (6/9) md-codec::baseline_gapb sh_wsh_non_multi_has_no_canonical_origin
- KILLED tlv.rs:122:29 <= -> > -> `use_site_override_indices_must_be_strictly_ascending`
  - base: PASS [   0.009s] (7/9) md-codec::baseline_gapb use_site_override_indices_must_be_strictly_ascending
- KILLED validate.rs:301:66 == -> != -> `missing_explicit_origin_names_the_key_without_an_override`
  - base: PASS [   0.004s] (1/9) md-codec::baseline_gapb missing_explicit_origin_names_the_key_without_an_override
- KILLED validate.rs:637:9 delete Variable arm -> `nested_unspendable_under_a_variable_node_is_refused`
  - base: PASS [   0.013s] (9/9) md-codec::baseline_gapb nested_unspendable_under_a_variable_node_is_refused
- KILLED validate.rs:661:9 delete Children arm -> `sortedmulti_a_under_a_children_node_is_refused_under_liana_root`
  - base: PASS [   0.006s] (2/9) md-codec::baseline_gapb sortedmulti_a_under_a_children_node_is_refused_under_liana_root
- KILLED validate.rs:662:9 delete Variable arm -> `sortedmulti_a_under_a_variable_node_is_refused_under_liana_root`
  - base: PASS [   0.013s] (8/9) md-codec::baseline_gapb sortedmulti_a_under_a_variable_node_is_refused_under_liana_root

Mutant FAIL lines (codec-mut.log, one per survivor, in the order above):
```
== bch_decode 268 >->==
FAIL [   0.005s] (1/1) md-codec::baseline_gapb decode_regular_errors_corrects_a_three_error_word
== bch_decode 268 >-><
FAIL [   0.005s] (1/1) md-codec::baseline_gapb decode_regular_errors_corrects_a_three_error_word
== bch_decode 432 ||->&&
FAIL [   0.016s] (1/1) md-codec::baseline_gapb decode_regular_errors_refuses_a_word_past_capacity
== canonical_origin 75 &&->||
FAIL [   0.015s] (1/1) md-codec::baseline_gapb sh_wsh_non_multi_has_no_canonical_origin
== tlv 122 <=->>
FAIL [   0.016s] (1/1) md-codec::baseline_gapb use_site_override_indices_must_be_strictly_ascending
== validate 301 ==->!=
FAIL [   0.005s] (1/1) md-codec::baseline_gapb missing_explicit_origin_names_the_key_without_an_override
== validate 637 delete Variable arm
FAIL [   0.004s] (1/1) md-codec::baseline_gapb nested_unspendable_under_a_variable_node_is_refused
== validate 661 delete Children arm
FAIL [   0.005s] (1/1) md-codec::baseline_gapb sortedmulti_a_under_a_children_node_is_refused_under_liana_root
== validate 662 delete Variable arm
FAIL [   0.007s] (1/1) md-codec::baseline_gapb sortedmulti_a_under_a_variable_node_is_refused_under_liana_root
== bch.rs:100 ->true
FAIL [   0.003s] (1/1) md-codec::baseline_gapb bch_verify_regular_rejects_every_single_symbol_corruption
```

## md-cli, 28 killed (tests/baseline_gapb.rs; unit tests in src/parse/template.rs, src/seat/compose.rs, src/seat/partition.rs)

Private/crate-private items (bin-only crate, no lib): `resolve_placeholders` (template.rs:846), `comparison_form`
(seat/compose.rs:148) and `partition` (seat/partition.rs:296) are killed by unit tests in `src/`; the rest go
through the `md` binary.

| survivor(s) | test |
|---|---|
| compose.rs:46:5 parse_u16 -> Ok(0), Ok(1) | compose_older_units_carries_the_typed_value (`older=100u` -> `older(4194404)` = 2^22 + 100, BIP 68) |
| compose.rs:89:24 older guard -> true | compose_refuses_older_after_another_lock |
| compose.rs:106:24 after guard -> true | compose_refuses_after_after_another_lock |
| walk.rs:248:52 \|\| -> && | decompose_does_not_recognise_a_liana_key_at_a_nonstandard_multipath (`<0;2>`, `<2;1>`) |
| walk.rs:332/335/338/341 hash Translator, String::new() and "xyzzy" (8 rows) | decompose_template_carries_each_hash_literal_verbatim |
| text.rs:322/330/338 fmt_md1_id/fmt_template_id/fmt_policy_id, String::new() and "xyzzy" (6 rows) | inspect_prints_the_three_ids_in_full (full values compared with md-codec's computed ids) |
| template.rs:846:17 \|\| -> && | gapb_resolve_tests::resolve_placeholders_refuses_a_repeat_differing_in_one_respect (unit; the CLI path is shadowed by the reuse gate, so a direct call is the only observable) |
| template.rs:1542/1548/1731/1752 delete PkK/PkH/DupIf/False arms | encode_decode_round_trips_pk_k_pk_h_dupif_and_false |
| template.rs:2937:37, 2940:37 minting guard -> false | experimental_still_refuses_timelock_mixing_under_wsh_and_sh_wsh |
| seat/compose.rs:148:9 delete Tr arm | comparison_form_absorbs_a_swap_inside_a_sortedmulti_a_in_a_tap_tree (unit) |
| seat/partition.rs:296:53 && -> \|\| | a_stray_piece_that_completes_no_card_refuses_the_class (unit) |

Mutant FAIL lines (cli-mut.log, in survivor order; mutant text for the deleted-arm rows is `if false =>`, for the
function-body rows an early `return`):
```
== compose:46 parse_u16->Ok(0)
FAIL [   0.009s] (1/1) md-cli::baseline_gapb compose_older_units_carries_the_typed_value
== compose:46 parse_u16->Ok(1)
FAIL [   0.013s] (1/1) md-cli::baseline_gapb compose_older_units_carries_the_typed_value
== compose:89 older guard->true
FAIL [   0.008s] (1/1) md-cli::baseline_gapb compose_refuses_older_after_another_lock
== compose:106 after guard->true
FAIL [   0.008s] (1/1) md-cli::baseline_gapb compose_refuses_after_after_another_lock
== walk:248 ||->&&
FAIL [   0.008s] (1/1) md-cli::baseline_gapb decompose_does_not_recognise_a_liana_key_at_a_nonstandard_multipath
== walk:332 ->Ok(String::new())
FAIL [   0.013s] (1/1) md-cli::baseline_gapb decompose_template_carries_each_hash_literal_verbatim
== walk:332 ->Ok(xyzzy)
FAIL [   0.007s] (1/1) md-cli::baseline_gapb decompose_template_carries_each_hash_literal_verbatim
== walk:335 ->Ok(String::new())
FAIL [   0.015s] (1/1) md-cli::baseline_gapb decompose_template_carries_each_hash_literal_verbatim
== walk:335 ->Ok(xyzzy)
FAIL [   0.015s] (1/1) md-cli::baseline_gapb decompose_template_carries_each_hash_literal_verbatim
== walk:338 ->Ok(String::new())
FAIL [   0.015s] (1/1) md-cli::baseline_gapb decompose_template_carries_each_hash_literal_verbatim
== walk:338 ->Ok(xyzzy)
FAIL [   0.020s] (1/1) md-cli::baseline_gapb decompose_template_carries_each_hash_literal_verbatim
== walk:341 ->Ok(String::new())
FAIL [   0.022s] (1/1) md-cli::baseline_gapb decompose_template_carries_each_hash_literal_verbatim
== walk:341 ->Ok(xyzzy)
FAIL [   0.030s] (1/1) md-cli::baseline_gapb decompose_template_carries_each_hash_literal_verbatim
== text:322 ->String::new()
FAIL [   0.007s] (1/1) md-cli::baseline_gapb inspect_prints_the_three_ids_in_full
== text:322 ->xyzzy
FAIL [   0.011s] (1/1) md-cli::baseline_gapb inspect_prints_the_three_ids_in_full
== text:330 ->String::new()
FAIL [   0.008s] (1/1) md-cli::baseline_gapb inspect_prints_the_three_ids_in_full
== text:330 ->xyzzy
FAIL [   0.008s] (1/1) md-cli::baseline_gapb inspect_prints_the_three_ids_in_full
== text:338 ->String::new()
FAIL [   0.020s] (1/1) md-cli::baseline_gapb inspect_prints_the_three_ids_in_full
== text:338 ->xyzzy
FAIL [   0.010s] (1/1) md-cli::baseline_gapb inspect_prints_the_three_ids_in_full
== template:846 ||->&&
FAIL [   0.011s] (1/1) md-cli::bin/md parse::template::gapb_resolve_tests::resolve_placeholders_refuses_a_repeat_differing_in_one_respect
== template:1542 delete arm
FAIL [   0.010s] (1/1) md-cli::baseline_gapb encode_decode_round_trips_pk_k_pk_h_dupif_and_false
== template:1548 delete arm
FAIL [   0.022s] (1/1) md-cli::baseline_gapb encode_decode_round_trips_pk_k_pk_h_dupif_and_false
== template:1731 delete arm
FAIL [   0.026s] (1/1) md-cli::baseline_gapb encode_decode_round_trips_pk_k_pk_h_dupif_and_false
== template:1752 delete arm
FAIL [   0.038s] (1/1) md-cli::baseline_gapb encode_decode_round_trips_pk_k_pk_h_dupif_and_false
== template:2937 guard->false
FAIL [   0.008s] (1/1) md-cli::baseline_gapb experimental_still_refuses_timelock_mixing_under_wsh_and_sh_wsh
== template:2940 guard->false
FAIL [   0.011s] (1/1) md-cli::baseline_gapb experimental_still_refuses_timelock_mixing_under_wsh_and_sh_wsh
== seat/compose:148 delete Tr arm
FAIL [   0.007s] (1/1) md-cli::bin/md seat::compose::tests::comparison_form_absorbs_a_swap_inside_a_sortedmulti_a_in_a_tap_tree
== partition:296 &&->||
FAIL [   0.005s] (1/1) md-cli::bin/md seat::partition::tests::a_stray_piece_that_completes_no_card_refuses_the_class
```

## Gates (run once each, logs in /scratch/cache/md-harden-gapb-logs/)

cargo 1.85.0, `--locked`, CARGO_TARGET_DIR=/scratch/cache/target-md-harden-gapb, never `--release`.
- `cargo fmt --all --check`: pass.
- `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`: pass (after fixing a `format_collect` lint in my own helper).
- `cargo nextest run --locked -j 8 --workspace --all-targets --all-features --no-fail-fast`: 1697 passed, 6 skipped.
- `cargo test --locked --workspace --doc --all-features`: pass.
- `cargo build --locked -p md-cli --no-default-features`: pass.
- `cargo doc --workspace --no-deps --document-private-items --all-features`: pass.
- display-grouping vectors sha256 check, `ci/docs-only.test.sh`, `sha256sum --check ci/shared-ci.sha256`,
  `ci/code-docs-check.test.sh`, `ci/code-docs-check.sh`: pass.
- NOT run: freebsd cross-check, musl (x86_64/aarch64) jobs, man-pages.yml, vendor-freshness.yml, fuzz-smoke.yml,
  bitcoind-differential.yml (need cross targets, a native arm64 runner, bitcoind, network or the fuzz toolchain; none
  is touched by a tests-only change). cargo-mutants / mutation-gate.sh not run, per the brief.

No STOP, no base failure, no equivalent mutant.

KILLED 38 / EQUIV 0 / STOP 0 / BASE-FAIL 0
