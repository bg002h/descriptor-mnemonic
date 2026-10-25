r1: 0C/0I/0M/2N

# L3 review r1: descriptor-mnemonic PR #39 (after-Part-B B-dm)

Head 1f1b0db755ddbf481e1b9e894cf29fef19b57cfa. Diff `origin/main...head`: `.github/workflows/release.yml`
(+24/-6, `build` job only) and `design/agent-reports/ci-after-b-build.md`.

**The question:** can the change skip `build`'s build steps on an event whose build output is used, or on a
change that is not documentation, or break the release (tag) path? **No.**

## Per event (what is checked out, the check run name, what the guard does)

| event | guard step runs? | `ci/docs-only.sh` path | checkout `ref` | check run name | build steps |
| --- | --- | --- | --- | --- | --- |
| pull_request into main | yes | PR_BASE_SHA, then A′ | `github.sha` (the test-merge commit; same as `refs/pull/N/merge` at event time) | `build md (<leg>)` | skip only on proof |
| push to main | yes | PUSH_BEFORE | `github.sha` | `build md (<leg>)` | skip only on proof |
| push to `ci/**` | yes | origin/main tip | `github.sha` | `build md (<leg>)` | skip only on proof; `assemble` skipped |
| tag push | yes (`inputs` is null; null == `''` is true) | `*) full "push to refs/tags/…"` | `github.ref` (the tag) | `build md (<leg>) (tag)` | always run |
| dispatch, no `tag` (incl. `sign_dry_run`) | yes | `*) full "event workflow_dispatch"` | `github.sha`, or `github.ref` when dispatched from a tag ref | `build md (<leg>)`, or `… (tag)` when dispatched from a tag ref | always run |
| dispatch with `tag` | no (`if:` false) | n/a; `steps.docs.outputs.docs_only` is `''`, so `!= 'true'` holds | `inputs.tag` | `build md (<leg>) (tag)` | always run |

Every event where `assemble` runs (tag, `inputs.tag`, `sign_dry_run`) takes a full path in the guard, so
`Upload artifact` always runs before `assemble`'s `Download all artifacts` / `pack`. The only skip writer
is `skip()`, reached only on `pull_request` into main or a branch push. Every other outcome, a missing
output included, runs the steps (`!= 'true'`).

## Look-at items

1. **Steps.** Guarded steps: toolchain, cross install, both builds, smoke test, stage, upload. None is
   an attestation, a signature or a cache save. The upload's only consumer is `assemble`, which runs only
   on the always-full events above. `needs: [test, build]` is met by a success job with skipped steps.
2. **CHECK_NAME vs real name.** The untagged rendered name equals `CHECK_NAME` (`build md (<leg>)`).
   Ran `gh api …/commits/09cbf988…/check-runs -f check_name=…`: `build md (linux-x86_64)` returns
   exactly that run (completed/success); the prefix `build md (linux` returns nothing; `… (tag)`
   returns nothing. The filter is exact, so another leg, or a ` (tag)` run, cannot answer for this leg.
   A leg skipped at job level concludes `skipped`, so it fails `passed()`. A docs-skipped leg concludes
   `success`, which is the induction settled in Part A. No other workflow defines a `build md (…)` job
   (grepped `.github/workflows/`).
3. **Permissions.** `checkout` needs `contents: read`. The guard's compare call needs `contents: read`
   and its check-runs call needs `checks: read`. `upload-artifact` uses the runtime token, not
   `GITHUB_TOKEN`, and `cross` pulls public ghcr images without the token. Nothing needs `id-token`,
   `attestations` or `contents: write`; those stay on `assemble` through the workflow-level block.
   The L1 head ran all five legs green under the new block, `linux-aarch64` (cross) included.
4. **` (tag)` and the checkout `ref`.** See the table. A ` (tag)` run cannot stand in for an untagged
   lookup (exact name). The reverse would need a run named `build md (<leg>)` on a commit whose tree it
   did not build. After this PR, only a dispatch with `tag` builds a tree other than its commit's, and it
   is now suffixed. Before this PR there were three dispatch runs (`gh run list --event
   workflow_dispatch`: 0a6cf8f3 on main, 2026-09-17, and two on `sign-releases`). R is always a recent
   tip (the PR base or PUSH_BEFORE), never a September commit, so that history cannot be reached today.
   For PRs and branch pushes, checking out `github.sha` instead of `github.ref` pins the commit the
   check run is filed on. The guard's two-parent HEAD test still holds: L2's A′ line appears on every
   leg.
5. **PR description.** Accurate on the three skip cases, the permissions, the `github.sha` checkout and
   the ` (tag)` suffix. It says the PR changes `.github/workflows/`, as §7 step 4 requires (this PR has
   no `ci/` change). Draft. Footer present.

## Nits

- N1: The PR description says "the release build jobs" and "Each job now has …", but B-dm has one
  job (`build`, five legs). The wording comes from the spec's shared B-* text.
- N2: The PR description cites the spec by a local path that public readers cannot open (constellation
  convention; noted only).
