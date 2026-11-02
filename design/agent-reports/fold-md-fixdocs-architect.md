fold-md-fixdocs-architect: no C/I, no STOP; one open note (S-2 left in FIX-A)

FIX-B commit 7b33977c (fix/bughunt-b); FIX-A commit 71c2971c (fix/bughunt-a).

## FIX-B items applied (`design/FIX_bughunt_md_b.md`)
- Q1/Q2: STOPs S-1 and S-2 deleted; header paragraph now cites the ruling.
- Q2: §2.5 sentence added ("All alternatives are checked ... first offending value is reported (key order, then origin components, then alternatives)").
- Q1: row 04f added to §3, §4 (rows list) and the §5 filter; count 13 -> 14.
- §1 item 3 sub-bullet: "Under S-1 option (i)" wording dropped.
- Q4: §5 "input_bughunt.rs 13" text changed to "(Minor, deferred; FIX-A `#[ignore]`s 06 and 13)".
- Q1: `grade-bughunt-md.md` R-4 sentence replaced verbatim.
- Row 04f written in `crates/md-codec/tests/bughunt.rs` (row + control).

## FIX-A items applied (`design/FIX_bughunt_md_a.md`)
- Q1: §1 item 3 (04d/04e depend on FIX-B), item 4 (10 fail, 04d/04e pass on B's head); §2.3 sentence replaced by the corrected R-4 sentence; §3 rows 04d/04e; §6 filter and "12 run, 12 passed"; S-1 deleted.
- Q3: §2.3 "unless S-3" dropped, exact text added; §3 row 04 second assertion; S-3 deleted.
- Q4: new §2.9 with both `#[ignore = ...]` attributes and both FOLLOWUPS entries verbatim (GREEN commit); §4 files; §6 sentence; S-4 deleted.
- Rows 04d, 04e and the row-04 extra assertion written in `crates/md-cli/tests/bughunt.rs`. No `#[ignore]` or FOLLOWUPS edits made.

## RED commands and counts
- FIX-B: `cargo nextest run --locked --workspace -E 'test(/(^|::)bughunt_md_04f_codec_unselected_alt_above_2_29_varint_overflow$/)'`: 1 run, 0 passed, 1 failed. Reason: `to_miniscript_descriptor(chain 0)` left `Ok("wpkh(xpub6BemY.../0/*)#lj5fafuw")`, right `Err(VarintOverflow { value: 536870912 })`.
- FIX-B §5 filter (with `--no-fail-fast`): 14 matched, 1 passed (09 inverted), 13 failed (incl. 04f and the policy_shape row). `cargo fmt --all --check` clean.
- FIX-A: `cargo nextest run --locked -p md-cli -E 'test(/(^|::)bughunt_md_(04_multipath_hardened_range_refused_as_hardened|04d_...|04e_...)$/)'`: 3 run, 0 passed, 3 failed. 04d: exit 0 stdout `bc1qghsy27425xapuv6gltah6uf2q9w5usrjucr5uy` (want 1); 04e: exit 0 `...#akxwxgma`; 04 fails on its first assertion (varint text, "refused, but not as hardened").
- FIX-A §6 filter `bughunt_md_(01|03|04|04b|04c|04d|04e|10|11|11b|12|14)_`: 12 matched, 0 passed, 12 failed (on unmodified code, as expected; 04d/04e pass only on B's head).

## Notes (not decided, not changed)
- FIX-A S-2 (names of 04b/04c) is not covered by the ruling; left in §7 as the only STOP.
- FIX-B §3 intro still says "(log: 14 run, 2 passed, 12 failed)" for the pre-fold run; not in the ruling's folds, left as is. The FIX-A §3 "Evidence" block counts (10 run, 1560 workspace) are likewise pre-fold.
- Row 04d/04e names collide with nothing; md-codec 04b/04c vs md-cli 04b/04c collision is harmless per ruling.
- Logs in the session scratchpad only.

Counts: 0 Critical, 0 Important; 0 STOPs raised; 2 commits; rows added 3 (+1 assertion).
