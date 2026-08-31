# Push report — mdcli-mini cycle to origin/main

**Date**: 2026-08-31
**Agent**: push agent (dispatched by controller)
**Repo**: bg002h/descriptor-mnemonic
**Ritual**: `scripts/push-via-staging.sh` (ci/staging staging-then-push)

## Outcome

Pushed. `origin/main` == `bdb031a4cb54a9f57510af98db81386c360e9b70`, 68 commits
ahead of the pre-push tip (`d3676fb1d43c9b71ddab5799e933718914d8b4dc`). No
"Bypassed rule violations" message on the final push. `ci/staging` deleted and
confirmed gone.

## Preconditions verified before starting

- Tree clean, `HEAD` == `bdb031a4cb54a9f57510af98db81386c360e9b70`, branch `main`.

## Commands run, in order

```
git status --porcelain=v1 -uno
git rev-parse HEAD
git branch --show-current
scripts/push-via-staging.sh main
  # staged HEAD to refs/heads/ci/staging successfully; script's own
  # `gh run list --commit <TIP> -q '.[0].databaseId'` selected run 33364380344
  # and began polling it for the required contexts. Left running.
```

## Anomaly found and resolved

**The script polled the wrong CI run.** This SHA triggers three separate
GitHub Actions workflows: `fuzz-smoke`, `CI`, and `bitcoind-differential`. All
three complete and report against the same commit SHA. The script's run
discovery,

```sh
RUN_ID=$(gh run list --repo "$REPO" --commit "$TIP" --json databaseId -q '.[0].databaseId' ...)
```

is **order-dependent** across multiple workflow runs for one SHA — it takes
whichever run `gh run list` returns first, with no filter on workflow name.
For this push it selected `bitcoind-differential` (run `33364380344`, single
job `bitcoind v27.0 address differential`), not `CI` (run `33364380379`),
which is the workflow that actually contains the two required contexts
(`cargo test (ubuntu-latest)`, `cargo clippy`). The script then looped waiting
for job names that could never appear in run `33364380344` — a silent stall
that would have run its full 1800s budget and FATAL'd with `last: absent`,
having pushed nothing.

**Verification performed independently** (full 40-char SHA, `--repo` explicit
throughout):

- Run `33364380344` (`bitcoind-differential`, wrong run): 1 job —
  `bitcoind v27.0 address differential` → `success`. Not a required context.
- Run `33364380379` (`CI`, correct run), `headSha` ==
  `bdb031a4cb54a9f57510af98db81386c360e9b70`: 9 jobs, all `success`:
  `cargo doc`, **`cargo clippy`**, `musl compile/test (x86_64-unknown-linux-musl)`,
  `freebsd compile-gate (whole-crate)`, `cargo fmt`, `cargo test (macos-latest)`,
  `cargo test (windows-latest)`, `musl compile/test (aarch64-unknown-linux-musl)`,
  **`cargo test (ubuntu-latest)`**. Both required contexts present and
  `success`.

Safety state at time of finding (no bypass had occurred):

- Local `main` unchanged (frozen): `bdb031a4cb54a9f57510af98db81386c360e9b70`,
  tree clean.
- `origin/main` unchanged: `d3676fb1d43c9b71ddab5799e933718914d8b4dc`.
- `refs/heads/ci/staging` present on origin at
  `bdb031a4cb54a9f57510af98db81386c360e9b70`.

Stopped the stalled background script (task `becrde1p8`) rather than let it
run out its 1800s timeout. Reported the exact state to the controller
(per-job conclusions, full SHAs, `--repo` on every `gh` query) before taking
any further action, per standing discipline — did not retry-push, did not
restart the ritual unilaterally.

Controller reviewed the independent verification and confirmed the actual
gate condition (both required contexts green on the exact staged SHA, tip
unmoved, no bypass) was satisfied, and directed executing the ritual's two
remaining steps manually on the still-frozen tip.

## Final steps (manual, on controller's explicit direction)

```
git rev-parse HEAD                       # bdb031a4... (unmoved, re-checked)
git status --porcelain=v1 -uno           # clean
git push origin HEAD:main
```

Full push output (verbatim, no "Bypassed rule violations" anywhere):

```
To github.com:bg002h/descriptor-mnemonic.git
   d3676fb1..bdb031a4  HEAD -> main
```

```
git fetch origin main --quiet
git rev-parse origin/main                # bdb031a4cb54a9f57510af98db81386c360e9b70
git push origin --delete ci/staging
```

Delete output:

```
To github.com:bg002h/descriptor-mnemonic.git
 - [deleted]           ci/staging
```

```
git ls-remote origin refs/heads/ci/staging   # empty — ref confirmed gone
```

## Suggested fix direction (for a FOLLOWUPS entry)

`scripts/push-via-staging.sh`'s run discovery should filter by workflow name
(or by the presence of the required-context job names within the run),
not take `gh run list`'s first result unfiltered. E.g.:

```sh
RUN_ID=$(gh run list --repo "$REPO" --commit "$TIP" --workflow CI \
  --json databaseId -q '.[0].databaseId')
```

or, more robustly against a future rename, select the run whose `jobs[].name`
set is a superset of `REQUIRED_CONTEXTS` rather than trusting workflow name or
list order. This repo has at least 3 workflows landing on the same push
(`ci.yml`, `fuzz-smoke`, `bitcoind-differential`); any future added workflow
reintroduces the same race unless discovery is filtered.

## Report persistence note

This report is written and committed by the push agent itself, in its own
commit, as the mandatory final action — the one exception to the `main`
freeze, taken only after the push ritual had fully completed (final push +
verification + staging-ref deletion), not before.
