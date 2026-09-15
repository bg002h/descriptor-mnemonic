# Push report — descriptor-mnemonic `main` to `65eab419`

**Date**: 2026-09-15
**Agent**: push agent (dispatched by controller)
**Repo**: bg002h/descriptor-mnemonic
**Ritual**: manual staging-then-push (`ci/staging`), per dispatch brief

## Outcome

**CLEAN.** Pushed `main` from `40c400de` to `65eab419` (5 commits). No
"Bypassed rule violations" message on the final push. `ci/staging` deleted
and confirmed gone. `origin/main` verified at `65eab4193684cee64f160d60b7e622b8f3e908cd`.

## Preconditions verified before starting

- `cargo --version` → `cargo 1.85.0 (d73d2caf9 2024-12-31)`.
- `git status --porcelain` empty.
- `git rev-parse main` → `65eab4193684cee64f160d60b7e622b8f3e908cd`.
- `git rev-list --count origin/main..main` → `5`, matching the brief:
  `65eab419` compose: warn when the preimage is the ONLY way to spend;
  `496b0767` fold: P1 R0 round 1; `83cd53e4` release: md-codec 0.43.0 /
  md-cli 0.15.0; `af3ba0b2` vectors: hashlock-gated vectors + help text;
  `064e107f` compose: author all four hash kinds.

## Pre-flight gates run

- `cargo nextest run --locked --all-targets` → **1316 tests run: 1316
  passed, 3 skipped, 0 failed** (22.421s wall). Matches the brief exactly.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` → clean,
  0 warnings (`Finished dev profile [optimized + debuginfo] target(s) in
  2.65s`, nothing but the two `Checking` lines).
- `rustup toolchain list` confirmed `1.95.0-x86_64-unknown-linux-gnu`
  present. `cargo +1.95.0 fmt --all -- --check` → clean, no diff output.

## Required contexts discovered

```
gh api repos/bg002h/descriptor-mnemonic/branches/main/protection \
  --jq '.required_status_checks.contexts'
```
→ `["cargo test (ubuntu-latest)", "cargo clippy"]`

`enforce_admins.enabled` → `false` (the documented, deliberate maintainer
hatch — left untouched, per standing instruction).

Confirmed `.github/workflows/ci.yml` (the workflow carrying both required
contexts) triggers on `branches: [main, 'ci/**']` — its own header comment
documents this exact staging ritual verbatim. `vendor-freshness.yml` also
builds `ci/**`. `fuzz-smoke.yml` triggers on `crates/md-codec/src/**` path
changes (matched this push's diff, so it ran too, unrequired).
`bitcoind-differential.yml` and `man-pages.yml` did not trigger (no matching
paths/tags).

## Staging push

`git push origin main:refs/heads/ci/staging` → `[new branch] main ->
ci/staging`.

Three workflow runs fired on SHA `65eab4193684cee64f160d60b7e622b8f3e908cd`:

| Run ID | Workflow | Conclusion |
| --- | --- | --- |
| 34996578322 | CI | success |
| 34996578295 | vendor-freshness | success |
| 34996578315 | fuzz-smoke | success |

`gh run watch 34996578322 --repo bg002h/descriptor-mnemonic` ran to
completion, all jobs green. Independently re-verified per-job conclusions
(not just the run-level conclusion) via
`gh run view 34996578322 --json status,conclusion,jobs`:

| Job | Conclusion |
| --- | --- |
| cargo doc | success |
| musl compile/test (x86_64-unknown-linux-musl) | success |
| **cargo clippy** | **success** |
| musl compile/test (aarch64-unknown-linux-musl) | success |
| cargo fmt | success |
| freebsd compile-gate (whole-crate) | success |
| cargo test (windows-latest) | success |
| cargo test (macos-latest) | success |
| **cargo test (ubuntu-latest)** | **success** |

Both required contexts present and green. The other two triggered runs
(`vendor-freshness`, `fuzz-smoke`) also checked as `success` (not required,
checked anyway for red-CI discipline).

## Final push

```
git push origin main
```
Output (verbatim, no bypass line):
```
To github.com:bg002h/descriptor-mnemonic.git
   40c400de..65eab419  main -> main
```

Cleanup:
```
git push origin --delete ci/staging
```
```
To github.com:bg002h/descriptor-mnemonic.git
 - [deleted]           ci/staging
```

## Final verification

`git fetch origin` then `git rev-parse origin/main` →
`65eab4193684cee64f160d60b7e622b8f3e908cd`. Matches local `main` and the
brief's expected tip.

## Report persistence note

Per this dispatch's explicit instruction, this report is **left untracked**
(not committed) — the controller will handle persistence/commit decisions
itself.
