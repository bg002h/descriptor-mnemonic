r1: 0C/0I/0M/1N

# L3 delta review: descriptor-mnemonic PR #38 (feat/ci-docs-skip @ 4d47339f), round r1

Scope: the four deltas in `ci-skip-constellation/briefs/l3-descriptor-mnemonic-r1.md`
only. Shared files, the gate and L1/L2 were settled and not re-reviewed.

## 1. `ci/docs-only-paths.txt` additions and `ci/code-docs-exempt.txt`

- Added `- ^docs/verify-reproducibility\.md$`. The only code reference is
  `ci/repro/vendor-freshness.sh:85`, inside an error message. That script is run
  only by `.github/workflows/vendor-freshness.yml`, which `ci/docs-only.sh`
  does not guard, so the rule is conservative (it forces full CI on a change
  to that doc) and protects nothing wrongly. It matches the SPEC §3 table.
- Exempt: `^scripts/matrix-identity-check\.sh$`. A repo-wide grep finds it named
  only in README.md, design notes and a `//!` comment in
  `crates/md-cli/src/seat/mod.rs:62`. No workflow and no script a workflow
  runs invokes it. No guarded step runs it, so the exemption is sound.

## 2. Insertion sites and `if:` (SPEC §2.3)

- `ci.yml` job `clippy` (`runs-on: ubuntu-latest`): the step is right after
  `Self-test the docs-only guard`, with no `if:`, which matches the table.
- `release.yml` job `test`: the step is right after the self-test, with
  `if: runner.os == 'Linux' && github.event.inputs.tag == ''`, which matches
  the table. It sits before the `Docs-only change?` step, as specified.

## 3. `release.yml` `test` job

- `ref:` is `${{ github.event.inputs.tag || (startsWith(github.ref, 'refs/tags/') && github.ref) || github.sha }}`,
  `name:` is `release test (${{ matrix.os }})${{ (...) && ' (tag)' || '' }}` and
  `CHECK_NAME: release test (${{ matrix.os }})`. All three match SPEC §2.1 verbatim.
- Old name `test (<os>)`: nothing reads it.
  - `needs: [test, build]` (release.yml:279) uses the job id, which is unchanged.
  - The required contexts on `main` (branch protection) are `cargo test (ubuntu-latest)`
    and `cargo clippy`. There are no rulesets. Neither context is the release job.
  - `scripts/push-via-staging.sh:164` mentions "test (ubuntu-latest)" only in a
    comment about exact matching. It compares against the required contexts.
  - The other repo-wide hits are `cargo test (...)` and `musl compile/test (...)`
    in ci.yml or design notes. None refers to the release job.

## 4. PR description

Accurate. It states rule A′, the code-docs check, the release `test` checkout
change and the rename with the ` (tag)` suffix, and it flags the `.github/` and
`ci/` change as RULES.md requires. The PR is still a draft.

## Findings

- N1: the PR description does not mention this repo's own two path-file lines
  (`docs/verify-reproducibility.md` protected; `scripts/matrix-identity-check.sh`
  exempt). Optional wording only.
