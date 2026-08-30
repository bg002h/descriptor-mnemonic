#!/usr/bin/env bash
# push-via-staging.sh -- the ci/staging push ritual as a command, for THIS repo.
#
# WHY IT EXISTS. A required status check binds to a COMMIT SHA, not a branch, so
# a commit pushed straight to `main` carries no check when the protection rule
# is evaluated: GitHub reports the contexts as "expected" and the push is
# BYPASSED rather than satisfied. Measured here 2026-08-30 on a plain
# `git push origin main`:
#
#     remote: Bypassed rule violations for refs/heads/main:
#     remote: - 2 of 2 required status checks are expected.
#
# `strict: false` on the rule is what makes that fixable -- GitHub asks only
# whether the commit carries a passing context, not whether it is up to date --
# so let the SHA earn one first on `ci/staging`, then push the branch. That is
# the whole ritual, and this script IS it; running it is the discipline.
#
#     scripts/push-via-staging.sh          # pushes the current branch
#     scripts/push-via-staging.sh main     # explicit branch
#
# THE FREEZE RULE -- READ THIS BEFORE RUNNING.
# The ritual assumes the branch tip DOES NOT MOVE for the whole window. Make no
# commits to the branch between the staging push and the final push. Measured on
# a sibling repo 2026-08-16: a controller committed twice while CI ran, the
# final push carried a tip two commits past the gated one, `strict: false`
# accepted it against the older gated ancestor, and two commits reached
# origin/main with ZERO CI signal while the push printed "Bypassed rule
# violations". Empty your hands first: commit everything, verify a clean tree,
# THEN run this. The script re-checks the tip before pushing and aborts if it
# moved -- but the fix at that point is to re-stage the new tip, not to push.
#
# REQUIRED CONTEXTS, resolved against the live rule on 2026-08-30
# (`gh api repos/bg002h/descriptor-mnemonic/branches/main/protection`):
#
#     cargo test (ubuntu-latest)
#     cargo clippy
#
# BOTH are waited for. The other ci.yml jobs (fmt, doc, the macOS/Windows test
# legs, freebsd-compile-gate, musl-check) are NOT required by the rule; they are
# reported after the push, informationally, and never gate it. Override with
# REQUIRED_CONTEXTS="a|b" if the rule changes -- and update this header when you
# do, because a stale list here silently waits for the wrong job.
#
# `.github/workflows/ci.yml` builds `ci/**` for exactly this reason and explains
# it at the trigger. `enforce_admins` is false DELIBERATELY -- the maintainer's
# own escape hatch, ruled 2026-08-15, and NOT to be flipped. The no-bypass rule
# binds automation, not the human.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

BRANCH="${1:-$(git rev-parse --abbrev-ref HEAD)}"
REPO="${REPO:-bg002h/descriptor-mnemonic}"
# Pipe-separated, so a context containing spaces stays one field.
REQUIRED_CONTEXTS="${REQUIRED_CONTEXTS:-cargo test (ubuntu-latest)|cargo clippy}"

[ -z "$(git status --porcelain)" ] || {
  echo "FATAL: working tree is dirty -- commit or stash before staging a push" >&2
  git status --short >&2
  exit 1; }

TIP=$(git rev-parse HEAD)   # full 40 chars: an abbreviated SHA makes gh queries
                            # return empty, which reads exactly like "no run".
AHEAD=$(git rev-list --count "origin/$BRANCH..HEAD" 2>/dev/null || echo '?')
echo "== staging $TIP (branch $BRANCH, $AHEAD ahead of origin/$BRANCH)"
echo "== FREEZE $BRANCH now: no commits until this script finishes"
git push origin "HEAD:refs/heads/ci/staging"

RUN_ID=""
for _ in $(seq 1 30); do
  RUN_ID=$(gh run list --repo "$REPO" --commit "$TIP" --json databaseId -q '.[0].databaseId' 2>/dev/null || true)
  [ -n "$RUN_ID" ] && break
  sleep 10   # an empty gh result can be a race, never a conclusion
done
[ -n "$RUN_ID" ] || { echo "FATAL: no workflow run appeared for $TIP" >&2; exit 1; }
echo "== run $RUN_ID; waiting for required contexts: $REQUIRED_CONTEXTS"

# Judge PER-JOB conclusions. A run-level conclusion is the wrong question here:
# it can be 'failure' because a NON-required job failed, and it can still be
# 'in_progress' after both required jobs are green.
IFS='|' read -r -a WANTED <<< "$REQUIRED_CONTEXTS"
for _ in $(seq 1 180); do
  JOBS=$(gh run view "$RUN_ID" --repo "$REPO" --json jobs -q '.jobs[] | .name + "\t" + (.conclusion // "pending")' 2>/dev/null || true)
  all_green=1
  for name in "${WANTED[@]}"; do
    conc=$(printf '%s\n' "$JOBS" | awk -F'\t' -v n="$name" '$1==n {print $2}')
    case "$conc" in
      success) ;;
      failure|cancelled|timed_out|action_required)
        echo "FATAL: required job '$name' concluded '$conc' -- NOT pushing $BRANCH" >&2; exit 1 ;;
      "") all_green=0 ;;   # the job has not appeared in the run yet
      *) all_green=0 ;;    # pending / in_progress
    esac
  done
  [ "$all_green" = 1 ] && break
  sleep 10
done
for name in "${WANTED[@]}"; do
  conc=$(gh run view "$RUN_ID" --repo "$REPO" --json jobs -q ".jobs[] | select(.name==\"$name\") | .conclusion // empty" 2>/dev/null || true)
  [ "$conc" = success ] || {
    echo "FATAL: timed out waiting for required context '$name' (last: '${conc:-absent}')" >&2; exit 1; }
done

[ "$(git rev-parse HEAD)" = "$TIP" ] || {
  echo "FATAL: the tip moved during the window -- the freeze rule was broken." >&2
  echo "       Re-run this script to stage the NEW tip; do not push now." >&2
  exit 1; }

OUT=$(git push origin "HEAD:$BRANCH" 2>&1); echo "$OUT"
if printf '%s' "$OUT" | grep -qi "bypassed rule violations"; then
  echo "FATAL: bypass message detected -- the check was NOT satisfied." >&2
  echo "       ci/staging is left in place for forensics; do not delete it." >&2
  exit 1
fi
git push origin --delete ci/staging

echo "== post-push straggler report (non-required jobs, informational):"
gh run view "$RUN_ID" --repo "$REPO" --json jobs -q '.jobs[] | .name + ": " + (.conclusion // .status)' || true
echo "== OK: $TIP is on $BRANCH with both required checks earned"
