# Push report — converter cycle, via scripts/push-via-staging.sh (first run)

**Date:** 2026-08-30 (task filename says 2026-08-31; run actually executed 2026-08-30 per system clock)
**Repo:** `/scratch/code/shibboleth/descriptor-mnemonic`, branch `main`
**Script:** `scripts/push-via-staging.sh` — this was its first execution.

## Pre-flight verification

- `git status` (before run): clean working tree, branch `main`, "ahead of
  'origin/main' by 52 commits."
- `git log --oneline -1`: `d3676fb1 report: persist REVIEW-converter-whole-diff-r2 -- GREEN, 0C/0I`
  — matches the expected tip exactly.
- `git remote -v`: `origin` = `git@github.com:bg002h/descriptor-mnemonic.git` (both fetch/push).
- `gh auth status`: logged in as `bg002h`, active account, scopes include `repo`.
- Branch protection required contexts (`gh api
  repos/bg002h/descriptor-mnemonic/branches/main/protection --jq
  '.required_status_checks.contexts'`): `["cargo test (ubuntu-latest)","cargo clippy"]`
  — matches exactly what the script's header documents and what
  `REQUIRED_CONTEXTS` defaults to.
- Script was read in full before running (`scripts/push-via-staging.sh`,
  119 lines). It implements: dirty-tree check, full-40-char SHA capture, push
  to `ci/staging`, poll for the workflow run (up to 300s), poll **per-job**
  conclusions for each required context (up to 1800s) rather than the
  run-level conclusion, a tip-move check before the final push (freeze-rule
  enforcement), a bypass-message grep on the final push output, `ci/staging`
  deletion only on success, and an informational post-push straggler report
  for the non-required jobs.

## Run

Executed in the background (script's own wait loop can run up to ~30 min;
a single foreground shell call is capped at 10 min) with full output logged
and monitored to completion.

1. **Stage:** `git push origin "HEAD:refs/heads/ci/staging"` — succeeded,
   new branch `ci/staging` created at `d3676fb1d43c9b71ddab5799e933718914d8b4dc`.
2. **Run discovery:** run id `33340055177` found on first poll.
3. **Required-context wait:** both required jobs reached `success`:
   - `cargo test (ubuntu-latest)`: `success`
   - `cargo clippy`: `success`
4. **Tip-move check:** passed (`git rev-parse HEAD` == staged SHA at push time)
   — no commits landed on `main` during the window, freeze held.
5. **Final push:** `git push origin "HEAD:main"` →
   ```
   To github.com:bg002h/descriptor-mnemonic.git
      f6127700..d3676fb1  HEAD -> main
   ```
   **No** "Bypassed rule violations" text present in the output (checked by
   the script's own grep, and independently re-read from the captured log).
6. **Cleanup:** `git push origin --delete ci/staging` → succeeded
   (`- [deleted]  ci/staging`). Confirmed independently post-run via
   `git ls-remote origin refs/heads/ci/staging` — empty, i.e. deleted.
7. **Script exit code:** `0`. Final line: `== OK:
   d3676fb1d43c9b71ddab5799e933718914d8b4dc is on main with both required
   checks earned`.

## Post-run independent verification (not just trusting the script's own report)

- `git -C descriptor-mnemonic status`: "Your branch is up to date with
  'origin/main'." Clean.
- `git rev-parse HEAD origin/main` (after `git fetch`): both
  `d3676fb1d43c9b71ddab5799e933718914d8b4dc` — local and remote `main` agree.
- `git rev-list --count f6127700..d3676fb1`: **52** — matches the "52 ahead"
  reported before the run; all 52 commits landed, none dropped or squashed.
- `gh run view 33340055177 --json jobs` (queried independently, after the
  script exited): full per-job table —

  | job | conclusion | status |
  | --- | --- | --- |
  | freebsd compile-gate (whole-crate) | success | completed |
  | cargo doc | success | completed |
  | musl compile/test (aarch64-unknown-linux-musl) | success | completed |
  | cargo fmt | success | completed |
  | **cargo clippy** (required) | **success** | completed |
  | musl compile/test (x86_64-unknown-linux-musl) | success | completed |
  | cargo test (windows-latest) | *(null)* | in_progress |
  | cargo test (macos-latest) | success | completed |
  | **cargo test (ubuntu-latest)** (required) | **success** | completed |

  Both required contexts: `success`. The one still-`in_progress` job
  (`cargo test (windows-latest)`) is **not** a required context — matches the
  script's documented behavior of treating non-required jobs as informational
  only, and the run-level conclusion (`in_progress`) is correctly *not* what
  the script (or this report) judges by.

## CI run

- Run ID: `33340055177`
- URL: https://github.com/bg002h/descriptor-mnemonic/actions/runs/33340055177
- Commit: `d3676fb1d43c9b71ddab5799e933718914d8b4dc`
- Required-context conclusions: `cargo test (ubuntu-latest)` = success,
  `cargo clippy` = success.
- All other completed jobs: success (freebsd compile-gate, cargo doc, musl
  aarch64, cargo fmt, musl x86_64, cargo test macos-latest). Only
  `cargo test (windows-latest)` remained `in_progress` at push time
  (non-required, informational, does not gate).

## Commits pushed

**52** commits (`f6127700..d3676fb1`), from `origin/main`'s prior tip
(`f6127700`) up to `d3676fb1` (`report: persist
REVIEW-converter-whole-diff-r2 -- GREEN, 0C/0I`).

## Verdict on the script's first run

**Clean pass, no deviations.** `scripts/push-via-staging.sh` behaved exactly
as its header documents on its first live execution against
`bg002h/descriptor-mnemonic`: staged correctly, discovered the run, waited on
the correct two required per-job contexts (verified independently against
live branch protection before the run), detected no tip movement, pushed
`main` with no bypass message, deleted `ci/staging` only after confirming
success, and reported non-required jobs informationally without letting them
gate. No script misbehavior to report; no manual-ritual fallback was needed.
