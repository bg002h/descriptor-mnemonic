# Build report: md output-sink fix (design/FIX_md_output_sink.md)

Start head 50cff6c5bf056ab6ded07f9755d2afc03758af08 (matched). Base main 6b70d5d9 (unmoved).

## Applied
- Rows (`rows.diff`, byte-identical to the document's section 4 blocks): commit 5624253d `RED:`.
- Blocks 3.1 to 3.5 (the document's blocks are byte-identical to the architect's `fix-with-docfix.diff`, checked with cmp): commit d1ba957 `GREEN:`, block 3.5 with the literal `<COMMIT>`.
- S2: next commit changes only the `md-output-sink` Status line to `resolved d1ba957`.
- No `.github/` file touched; the toolkit manual is not edited.

## RED (base code, section 4 commands as printed)
- All nine printed commands exit 100 and each output holds its `## expect:` substring (logs `red-1..9.log`).
- Whole set (with `--no-fail-fast`; without it nextest stops early at "8/11 tests run: 1 passed, 7 failed", so the flag was added exactly as the gate script does): `11 tests run: 2 passed, 9 failed`.

## GREEN
- Rows: `11 tests run: 11 passed`; `out::tests`: `2 tests run: 2 passed`.

## Gate (logs in /scratch/cache/build-fix-output-sink/md/, `<name>.log` and `<name>.rc`)
| command | rc | summary |
|---|---|---|
| nextest -p md-cli --features cli-compiler | 0 | 943 tests run: 943 passed, 3 skipped |
| nextest -p md-cli (default) | 0 | 905 tests run: 905 passed, 3 skipped |
| rows filter | 0 | 11 tests run: 11 passed |
| out::tests | 0 | 2 tests run: 2 passed |
| cargo test --workspace --doc --all-features | 0 | |
| cargo fmt --all --check (pinned 1.85.0) | 0 | |
| cargo clippy --workspace --all-targets --all-features -D warnings | 0 | |
| ci/code-docs-check.sh, code-docs-check.test.sh, docs-only.test.sh, shared-ci sha256 check, display-grouping vectors check | 0 each | |
| RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --all-features | 0 | |
| cargo check --target x86_64-unknown-freebsd -p md-cli | 0 | |
| ci.yml test job command: nextest --workspace --all-targets --all-features --no-fail-fast | 0 | 1620 tests run: 1620 passed, 6 skipped |
| nextest -p md-cli --target x86_64-unknown-linux-musl | 101 | build fails: `x86_64-linux-musl-gcc` not installed here (CI installs musl-tools); environmental |

The CI fmt job uses the stable toolchain; only the pinned 1.85.0 was run here.

## Left to CI
- macOS and Windows `cargo test` legs (other OS; the Windows closed-pipe mapping must be read first).
- musl x86_64 and aarch64 (release) legs: no musl C compiler here (x86_64 attempted, rc 101 at the cc step); aarch64 is release mode and cross.
- fmt on stable; vendor-freshness, fuzz-smoke, man-pages, bitcoind-differential, release workflows (not run here).
- The mutation gate and whole-diff review (the coordinator's).

## STOPs
None.

0 Critical, 0 Important, 0 Minor

## 2026-11-13 Whole-diff fold

Folded: the access-mode-3 port (ms review I-1 applied to md) and md's own whole-diff findings (`agent-reports/whole-diff-md-output-sink-s2.md`). Logs: `/scratch/cache/fold-output-sink-accmode/md/logs/`. The FIX document and its gate script are not edited; they predate the port, and the PR body says so.

- Port RED (`cargo nextest run --locked -j 8 -p md-cli -E 'test(no_access_mode_stdout_exits_1_with_the_line)'`): `1 test run: 0 passed, 1 failed, 908 skipped` (`left: Some(0)`, `right: Some(1)`). GREEN: `1 test run: 1 passed, 908 skipped`. Production change: the probe line and the `read_only()` doc comment only.
- I1: assert `out::exit_code(` in `every_binary_includes_the_out_module`. RED with `ExitCode::SUCCESS` planted in `gen_ap2_grind.rs` (reverted, not committed): `1 test run: 0 passed, 1 failed`; GREEN on the head: `1 test run: 1 passed`.
- Filed as `deferred-minor` (owner `md-output-sink-followups`): `md-output-sink-wd-m1-...` (M1), `md-output-sink-wd-n1-...` (N1), and `md-output-sink-wd-m2-...` (the ms review's M-2, run here: `ulimit -f 4` gives rc 153, 4096 bytes written, so it holds). N2 not filed (known).

| command | rc | summary |
|---|---|---|
| nextest -p md-cli --features cli-compiler | 0 | 944 tests run: 944 passed, 3 skipped |
| nextest -p md-cli (default) | 0 | 906 tests run: 906 passed, 3 skipped |
| rows filter (`--no-fail-fast`) | 0 | 12 tests run: 12 passed |
| out::tests | 0 | 2 tests run: 2 passed |
| ci.yml test job: nextest --workspace --all-targets --all-features --no-fail-fast | 0 | 1621 tests run: 1621 passed, 6 skipped |
| cargo test --workspace --doc --all-features | 0 | |
| cargo fmt --all --check (1.85.0) | 0 | |
| cargo clippy --workspace --all-targets --all-features -D warnings | 0 | |
| ci/code-docs-check.sh | 0 | |
| RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --all-features | 0 | |
| cargo check --target x86_64-unknown-freebsd -p md-cli | 0 | |

STOPs: none.
