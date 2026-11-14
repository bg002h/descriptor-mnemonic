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
