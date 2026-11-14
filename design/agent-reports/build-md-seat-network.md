# Build report: md-seat-network (item S, md C8b)

- Base: e2ee3206e78d5409ce51f200fd1d42ce7ff12a27 (md-remedies head, stacked on PR 53). Branch fix/md-seat-network.
- Source of decisions: coordinator `agent-reports/grade-md-c8b.md`; fix.diff and seat_network.rs from /scratch/cache/grade-md-c8b/.
- PR number: (filled in below after opening)

## RED
`cargo nextest run --locked -p md-cli --test seat_network --no-fail-fast` on the unmodified base (no feature flag needed):
`Summary [   0.134s] 31 tests run: 4 passed, 27 failed, 0 skipped` (log /scratch/cache/md-seat-network-logs/red.txt). The failing names equal the grader's red.txt set (diffed: same). RED commit 8da5bf0c.
One edit to the row file: a doc comment on the keyed-card control said "(grade C8 ruled ...)"; the brief bars `grade` inside, so it reads "(C8 ruled ...)". No test name changed. (The other `zz` hits are bech32 data in mk1 strings.)

## GREEN
`git apply -p1 --3way fix.diff` applied cleanly (both files); `cargo fmt --all` changed nothing. Same command:
`Summary [   0.143s] 31 tests run: 31 passed, 0 skipped` (green.txt). GREEN commit 664cd2c5.
The diff's CHANGELOG bullet matches the report's §4 text (compared by eye, same words); both bullets sit under `### Fixed`.

## Tests that changed
None existing. Only the new file crates/md-cli/tests/seat_network.rs.

## STOPs
None.

## Gate (logs in /scratch/cache/md-seat-network-logs/, rc per step in rcs.txt; commands as ci.yml and release.yml give them, plus `--locked`, `-j 8`)
- nextest: Summary [  31.187s] 1670 tests run: 1670 passed, 6 skipped
- release-nextest: Summary [  64.619s] 1624 tests run: 1624 passed (1 slow), 6 skipped
- nocompiler: Summary [   0.170s] 31 tests run: 31 passed, 0 skipped
- nextest rc=0
- doctest rc=0
- docsonly-test rc=0
- sharedci rc=0
- clippy rc=0
- nodefault rc=0
- fmtpin rc=0
- fmt rc=0
- doc rc=0
- freebsd rc=0
- nocompiler rc=0
- release-nextest rc=0
- release-doctest rc=0
- relbuild rc=0
- smoke rc=0

Steps: ci.yml cargo test (nextest all-features, doc-tests), clippy job (docs-only self-test, shared-ci + code-docs-check, clippy, no-default-features build), fmt job (checksum pin, `cargo +stable fmt --all --check`), doc job, freebsd compile-gate; release.yml PR jobs (nextest, doc-tests, release build `-j 4` with cli-compiler, smoke script). `nocompiler` is the seat_network file alone without `--features cli-compiler`. The nextest with `--all-features` includes cli-compiler. All rc=0; counts match the md-remedies baseline plus the 31 rows (1639 -> 1670; release-workflow 1593 -> 1624). No step differs from the baseline.

NOT RUN: musl x86_64 and aarch64 legs, Windows/macOS legs, the `cross` builds, `ci/docs-only.sh` itself (needs a PR context); other workflows (bitcoind-differential, fuzz-smoke, vendor-freshness, man-pages) are not triggered by this diff (md-cli src, CHANGELOG, a test file). The mutation gate was not run (not in the brief).

Counts: Critical 0, Important 0, STOPs 0, existing tests changed 0.
