**⚠ Open point: spec §5 ends "Then the mutation gate per the coordinator's rules." No command or scope is given, so I did not run it. All other gate steps pass.**

# Build report: md FIX-B

Commit: 0141ea57 on fix/bughunt-b (one GREEN commit: §2 code + BIP table row + nine ignores). Not pushed.

- RED (unmodified head b190cbf5): 14 run, 1 passed (inverted #9), 13 failed. Log: design/agent-reports/build-md-fix-b-logs/red.log
- GREEN step 1 nextest: 1591 passed, 13 skipped (s1-nextest.log)
- Step 2 doc tests ok (s2-doc.log); step 3 fmt --check clean; step 4 clippy -D warnings clean
- Step 5 row filter: 14 matched, 14 passed (s5-rows.log); filter text in filter.txt
- Step 0 cargo fmt --all reflowed only the R-8 return and R-16 chunk.rs statement (and the new canonicalize chain).

Deviations: none in production text. R-7 code is not given verbatim in §2.4, only behaviour; I wrote a chain over the four TLV idx lists in the stated order (use_site_path_overrides, fingerprints, pubkeys, origin_path_overrides), returning PlaceholderIndexOutOfRange { idx, n: d.n }. Tests are untouched except the ignore attributes.

Counts: RED 14/1p/13f; GREEN 1591p/13s; rows 14/14.
