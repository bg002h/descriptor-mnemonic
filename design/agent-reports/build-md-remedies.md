# Build report: md-cli printed remedies (items L-Q, grade C1-C7)

- Base: ca65cd6d22a3c545d65809cd0fc2d7d7c9478d70 (origin/main did not move; `git apply --check -p1` of fix.diff passed).
- Branch: fix/md-remedies, worktree /scratch/cache/wt-md-fix-remedies. PR number: 53 (draft), https://github.com/bg002h/descriptor-mnemonic/pull/53.
- Source of decisions: coordinator `agent-reports/grade-md-mk-remedies.md`.

## RED
`cargo nextest run --locked -p md-cli --features cli-compiler --test printed_remedy_runs --no-fail-fast` on the unmodified base:
`17 tests run: 5 passed, 12 failed` (log /scratch/cache/md-remedies-logs/red.txt). Failing: c1 keyless, c2 tpub, c2 Vpub, c3, c4, c5 at-cap, c5 cost, c6 compile holder13, c6 encode holder13, c6 tap andor14, c6 or-under-thresh (each waited 30 s), c6 bound-nine (main compiles it). The 5 passes are the controls. Row file: the grader's rows with only the first doc line reworded (no `grade`/`zz` names); test function names unchanged.

## GREEN
fix.diff applied (`git apply -p1`), `cargo fmt --all` changed nothing further. Same command: `17 tests run: 17 passed, 0 skipped` (log green.txt).
fix.diff vs report text: compared C1-C7 line by line against grade §2; no difference, so the report text and the diff agree.
Also in the diff: the `.github/workflows/ci.yml` C7 step (a `.github/` change; the PR description says so).

## Other files
- CHANGELOG.md: one md-cli [Unreleased] / Fixed entry (C1-C5, the C6 refusal, C7). No version bump.
- design/FOLLOWUPS.md: `md-seat-remedy-chunk-cost-understated`, `deferred-minor`, tier `v1+`, owner "next md-cli change" (my wording of owner/tier: the grade names neither).
- Not done by design: C8 and C8b; the toolkit manual 42-md.md paragraph for C6 (grade §5, toolkit repo, not in this brief's job) is OWED in the toolkit repo.

## Tests that changed
No existing test changed (the whole existing suite passed untouched). Changes inside the new file, after the first GREEN: the 8 c6 rows and their helpers (`pk`, `chain`, `holder`, `andor`, `compile`, `c6_verdict`) and the `Duration` import carry `#[cfg(feature = "cli-compiler")]`, because the release workflow runs nextest without `cli-compiler`, where the `compile` verb does not exist and those 8 rows failed (`1601 tests run: 1593 passed, 8 failed`). Not foreseen by the grade. Without the feature the file runs `9 tests run: 9 passed` (RUSTFLAGS -D warnings); with it `17 tests run: 17 passed`.

## STOPs / deviations (for the coordinator)
1. **CI step name.** The grade's exact step name `md-cli builds without default features (crates/md-cli/README.md's serde-free install)` fails the repo's own gate: `bash ci/code-docs-check.sh` reports `FAIL README.md is named at .github/workflows/ci.yml:123` (log sharedci.txt). The check allows a reword for a line that names a docs file without reading it. I changed only the parenthesis to `(the serde-free install the md-cli readme gives)` (sharedci2: `code-docs check: ok`). The name's wording was not decided by the grade: say if you want another.
2. **FOLLOWUPS owner/tier.** The brief said `deferred-minor`; owner (`next md-cli change`) and tier (`v1+`) are my choice, copied from neighbouring entries.
3. **Toolkit manual** 42-md.md paragraph for C6 (grade §5) is owed in the toolkit repo; not part of this brief.

## Gate (final head before this report commit: see git log; logs in /scratch/cache/md-remedies-logs/)
ci.yml, every job for this diff (commands as the workflow gives them, plus `--locked`):
- cargo test: `cargo nextest run --workspace --all-targets --all-features --no-fail-fast`: `1639 tests run: 1639 passed, 6 skipped` (nextest.txt); `cargo test --workspace --doc --all-features` rc=0.
- clippy job: `bash ci/docs-only.test.sh` rc=0; shared-ci sha256 + `ci/code-docs-check.test.sh` (41 passed, 0 failed) + `ci/code-docs-check.sh` rc=0 (`ok (247 files scanned, 402 docs files)`); `cargo clippy --workspace --all-targets --all-features -- -D warnings` rc=0; new step `cargo build --locked -p md-cli --no-default-features` rc=0.
- fmt job: display-grouping checksum rc=0; `cargo +stable fmt --all --check` rc=0.
- doc job: `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps --document-private-items --all-features` rc=0.
- freebsd compile-gate: `cargo check --target x86_64-unknown-freebsd -p md-cli` rc=0.
release.yml PR jobs: `cargo nextest run --locked --workspace --no-fail-fast`: `1593 tests run: 1593 passed, 6 skipped` (release-nextest.txt); `cargo test --locked --workspace --doc` rc=0; `cargo build --release --locked --target x86_64-unknown-linux-gnu -p md-cli --features cli-compiler` rc=0 (first attempt killed 137 by memory pressure on the shared machine, rerun with 4 jobs) and the workflow's smoke-test script run against it: rc=0 (`acceptance ok`).
Other workflows: bitcoind-differential and fuzz-smoke trigger on md-codec src / fuzz paths only (not touched: not triggered). vendor-freshness triggers on Cargo.lock / Cargo.toml / vendor paths (not touched). man-pages: tags only.

NOT RUN: musl x86_64 (`cargo nextest run -p md-cli --target x86_64-unknown-linux-musl`: no `musl-gcc` on this machine); musl aarch64 (ARM runner); release Windows/macOS legs and the `cross` builds; `ci/docs-only.sh` (needs a PR context; the guard's self-test ran); the mutation gate the grade owes on compile.rs, encode.rs and descriptor.rs (not named in this brief's job; owed).

Counts: Critical 0, Important 0, Minor 0 found; STOPs 1 (CI step name) plus 2 notes; RED 12 failed/5 passed, GREEN 17/17; workspace 1639/1639.
