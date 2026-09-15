# Push report — descriptor-mnemonic `main` to `c61ff4ef`

**Date**: 2026-09-15
**Agent**: push agent (dispatched by controller)
**Repo**: bg002h/descriptor-mnemonic
**Ritual**: manual staging-then-push (`ci/staging`), per dispatch brief

## Outcome

**CLEAN.** Pushed `main` from `65eab419` to `c61ff4ef` (1 commit, docs-only).
No "Bypassed rule violations" message on the final push. `ci/staging`
deleted and confirmed gone. `origin/main` verified at
`c61ff4efeb87eec26f53011f750583ec302f3936`.

## Preconditions verified before starting

- `cargo --version` → `cargo 1.85.0 (d73d2caf9 2024-12-31)`.
- `git status` → clean working tree, branch ahead of `origin/main` by 1
  commit.
- `git rev-parse main` → `c61ff4efeb87eec26f53011f750583ec302f3936`.
- `git rev-parse origin/main` (before push) →
  `65eab4193684cee64f160d60b7e622b8f3e908cd` — matches the dispatch brief's
  "prior tip" claim exactly.
- `git log origin/main..main --oneline` → 1 commit:
  `c61ff4ef report: dm push 65eab419 via ci/staging -- both required
  contexts success, no bypass; verbatim`, whose own commit message cites
  `65eab419` as the prior tip.
- `git diff origin/main..main --stat` → 1 file changed, 118 insertions,
  0 deletions: `design/agent-reports/push-dm-65eab419.md` (new file). Docs
  only — matches the brief exactly.

All preconditions matched the brief; nothing to stop for.

## Pre-flight gates run

- `cargo nextest run --locked --all-targets` → **1316 tests run: 1316
  passed, 3 skipped, 0 failed** (22.348s wall). Matches the expected numbers
  exactly.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` → clean,
  0 warnings (`Finished dev profile [optimized + debuginfo] target(s) in
  0.08s`, nothing else).
- `rustup toolchain list` confirmed `1.95.0-x86_64-unknown-linux-gnu`
  present. `cargo +1.95.0 fmt --all -- --check` → exit code 0, no diff
  output.

## Required contexts discovered

```
gh api repos/bg002h/descriptor-mnemonic/branches/main/protection \
  --jq '.required_status_checks.contexts'
```
→ `["cargo test (ubuntu-latest)", "cargo clippy"]`

`enforce_admins.enabled` → `false` (the documented, deliberate maintainer
hatch — left untouched, per standing instruction).

## Docs-only path-filter check (explicitly asked for in the brief)

Read `.github/workflows/ci.yml` in full: its `on.push` trigger is
`branches: [main, 'ci/**']` with **no `paths:` filter** — the workflow's own
header comment documents this staging ritual verbatim and names the two
required contexts. So `ci.yml` (the workflow carrying both required
contexts) runs unconditionally on any push to `main` or `ci/**`, regardless
of whether the diff is docs-only.

The two workflows that DO have path filters — `fuzz-smoke.yml`
(`fuzz/**`, `crates/md-codec/src/**`, `.github/workflows/fuzz-smoke.yml`)
and `bitcoind-differential.yml` (three specific `md-codec` source files) —
correctly did **not** fire for this commit, since it touches only
`design/agent-reports/*.md`. `vendor-freshness.yml` also builds on
`ci/**`/`main` but did not trigger here either (confirmed via
`gh run list --commit`, one workflow run only — see below); its own header
comment notes the `pull_request:` trigger doesn't fire for this
constellation's direct-to-main push flow, and it evidently has no
unconditional `push:` path either.

**Net: only the `CI` workflow fired for this SHA, and it fired in full
(all 9 jobs, not just the 2 required ones) — the required contexts RAN, not
SKIPPED.** No docs-only skip behavior was observed for the gating workflow.

## Staging push

`git push origin main:refs/heads/ci/staging` → `[new branch] main ->
ci/staging`.

One workflow run fired on SHA `c61ff4efeb87eec26f53011f750583ec302f3936`:

| Run ID | Workflow | Conclusion |
| --- | --- | --- |
| 35011776245 | CI | success |

`gh run watch 35011776245 --repo bg002h/descriptor-mnemonic` ran to
completion, all jobs green. Independently re-verified per-job conclusions
via `gh run view 35011776245 --json jobs` and again via
`gh api repos/bg002h/descriptor-mnemonic/commits/<sha>/check-runs`:

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

Both required contexts present and green (RAN to success, not skipped).
Overall run conclusion: `success`.

## Final push

```
git push origin main
```
Output (verbatim, no bypass line):
```
To github.com:bg002h/descriptor-mnemonic.git
   65eab419..c61ff4ef  main -> main
```

Cleanup:
```
git push origin --delete ci/staging
```
```
To github.com:bg002h/descriptor-mnemonic.git
 - [deleted]           ci/staging
```

Note: pushing to `main` itself re-triggered the `CI` workflow again on the
same SHA (since `ci.yml` also matches `branches: [main]`) — a second `CI`
run (`35012445817`) appeared `queued`/`in_progress` immediately after. This
is expected, harmless, and not part of the gate (the gate was already
satisfied by the `ci/staging` run against the identical SHA); it was not
waited on.

## Final verification

`git fetch origin` then `git rev-parse origin/main` →
`c61ff4efeb87eec26f53011f750583ec302f3936`. Matches local `main` exactly.

## Report persistence note

Per this dispatch's explicit instruction, this report is **left untracked**
(not committed) — the controller will handle persistence/commit decisions
itself.
