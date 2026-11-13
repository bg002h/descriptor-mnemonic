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
