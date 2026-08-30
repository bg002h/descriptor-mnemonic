# Push report — 2026-08-30

Task: push descriptor-mnemonic and mnemonic-key to their origins.

## 1. descriptor-mnemonic

- Branch: `main`
- Pre-push tree: clean (`git status` showed "nothing to commit, working tree clean")
- Staging ritual check: no `scripts/push-via-staging.sh`, no "staging" mention in
  `README.md` or `CLAUDE.md`. Broader repo grep for "staging" hit only unrelated
  uses (git-add staging, plan-phase staging) in `design/*.md` — no push ritual.
  Used plain `git push origin main`.
- Commits ahead before push: 23 (`git rev-list --count origin/main..main`)
- Push result:
  ```
  remote: Bypassed rule violations for refs/heads/main:
  remote:
  remote: - 2 of 2 required status checks are expected.
  remote:
  To github.com:bg002h/descriptor-mnemonic.git
     6c4a56fd..f6127700  main -> main
  ```
  Push succeeded (exit 0) but printed a "Bypassed rule violations" message.
  Reported verbatim above per instructions; not retried.
- Final `git status -sb`: `## main...origin/main` (branch now in sync)

## 2. mnemonic-key

- Branch: `main`
- Pre-push tree: **DIRTY** — untracked file present:
  ```
  ## main...origin/main [ahead 2]
  ?? design/SPEC_chunk_set_id_verification.md
  ```
- Per task instructions, this is a STOP condition: did not push, did not commit,
  did not touch the untracked file.
- Commits ahead of origin (informational only, push not attempted): 2
- Staging ritual check (for reference, since push was not attempted): repo's
  `CLAUDE.md` documents a staging-ref ritual —
  `git push origin main:refs/heads/ci/staging` → `gh run watch` for
  `build (stable on ubuntu-latest)` → `git push origin main` →
  `git push origin --delete ci/staging` — with an explicit asymmetry note that
  `enforce_admins` is intentionally `false` (operator's own escape hatch) but
  automation must always use the staging path and report any "Bypassed rule
  violations" as a failure, not paper over it.
- Final `git status -sb`: unchanged, still `## main...origin/main [ahead 2]` with
  the untracked file present.

## Summary

| Repo | Branch | Ahead before | Push attempted | Result |
|---|---|---|---|---|
| descriptor-mnemonic | main | 23 | yes | succeeded, printed "Bypassed rule violations" (2/2 required checks expected) |
| mnemonic-key | main | 2 | **no** | STOPPED — working tree dirty (untracked `design/SPEC_chunk_set_id_verification.md`) |
