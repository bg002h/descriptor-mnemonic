**⚠ STOP: step 6 is partial. The brief gives slugs only for R-15 and R-5; eleven entries have no slug, so none were filed.**

# Build report: md bug-hunt FIX-A + toolkit manual lockstep

## Commits (none pushed)
- md `fix/bughunt-a`: 42bd9c91 GREEN (R-1 R-3 R-4 R-10 R-11 R-12 R-14, spec 2.9 ignores and entries); 49dde934 followups (partial).
- toolkit `fix/bughunt-md-manual` (worktree `/scratch/code/shibboleth/tk-worktrees/bughunt-md-manual`, from origin/master f6355e22): db7a4f68 manual, 42-md.md lines 40 and 43 per spec 2.8.

## RED / GREEN (logs in `design/agent-reports/build-md-fix-a-logs/`, uncommitted)
- RED (red.log): 12-row filter, 12 run, 10 failed, 2 passed (04d, 04e), as spec 1 item 4 expects.
- GREEN: workspace nextest 1569 run, 1569 passed, 6 skipped (nextest.log); doc tests ok; fmt exit 0; clippy -D warnings exit 0; 12-row filter 12/12 (rows.log); renamed test 1/1 (rename.log).
- Toolkit manual job: doc-gate-guard test exit 0; `cargo build --bin mnemonic` ok; `make audit` exit 0 (9 anchor danglers, same as baseline); doc-flag-lint.test.sh 15 passed 0 failed; `make pdf` ok (xelatex present).

## STOP: step 6 slugs
Unfiled, need an exact slug (and tier) each, owners as the brief gives:
1. whole-diff-md-fix-b-s1 M-1 (shape-key refusal text; owner md toolkit migration of verify/inspect)
2. whole-diff-md-fix-b-s1 N-1 (R-4 check, to_miniscript.rs:175-185; next md-codec change)
3. whole-diff-md-fix-b-s2 M-1 (04f origin-and-alt case; next md-codec change)
4-6. fix-review-r1-md-b N-1, N-2, N-3 (end-of-bughunt batch)
7-11. fix-review-r1-md-a M1, M2, M3, N1, N2 (end-of-bughunt batch)
Filed: R-15 `md-bughunt-15-ap2-grind-determinism` (Nit) and R-5 `md-bughunt-05-policy-shape-cross-product`. The R-5 Tier is my choice (`v1+`), not decided by the brief; change if wrong.

## Deviations / notes
- Spec 2.9 cites lines :11 and :6 for the ignore attributes; the real lines are :12 and :7. I followed the "line after #[test]" rule.
- The installed `mk` is a dev build newer than the pinned tag, so the first audit/lint run failed on stale markers in 44-mk-cli.md (unrelated to md). I installed the pinned mk-cli 0.13.0 under the scratchpad and reran; both passed.
- A relative-path `git worktree add` first landed under mnemonic-toolkit; I removed it, deleted the branch and recreated it at the absolute path. Pre-existing untracked files there were untouched.
- The Tier values for R-15 (`nit`) and R-5 (`v1+`) follow the existing entry format; the brief did not state tiers.

Counts: 0 C, 0 I; 1 STOP; RED 10/12 failed as expected; GREEN 1569/1569, 12/12, 1/1.

## Step 6 completion
Coordinator ruling applied: the eleven entries (slugs and owners as ruled; Tier `v1+` for Minors, `nit` for Nits) are filed in commit 6d3c46c3 "followups: md bug-hunt deferred minors (review findings)" on fix/bughunt-a. `git diff HEAD~1 --stat` touches only `design/FOLLOWUPS.md` (88 insertions). Docs only, so no nextest run. The STOP above is closed; nothing pushed.
