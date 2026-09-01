# Staging push attempt — seat-ap cycle tip 4251069a — RED, NOT SHIPPED

**SHA staged:** `4251069a0b01779d93115a1db4e64b42fea61b81` (main, tree clean at start)
**CI run:** [33484120763](https://github.com/bg002h/descriptor-mnemonic/actions/runs/33484120763) on branch `ci/staging`, conclusion **failure**

## Job conclusions (9 jobs)

| Job | Conclusion |
| --- | --- |
| freebsd compile-gate (whole-crate) | success |
| musl compile/test (aarch64-unknown-linux-musl) | success |
| cargo clippy | success |
| musl compile/test (x86_64-unknown-linux-musl) | **failure** |
| cargo fmt | success |
| cargo test (windows-latest) | **failure** |
| cargo doc | **failure** |
| cargo test (macos-latest) | **failure** |
| **cargo test (ubuntu-latest)** (required check) | **failure** |

Required checks: `cargo clippy` — success. `cargo test (ubuntu-latest)` — **failure**. Run is not fully green (5 of 9 jobs failed); does not meet the "every job success/skipped" bar regardless of which two are formally required.

## Failing job detail

**`cargo test (ubuntu-latest)`** — 3 test failures in `crates/md-cli/src/seat/partition.rs`, all decode-count assertions that expected 0 decode calls on over-budget/refusal paths but observed nonzero counts:

- `seat::partition::tests::group_cap_set_refuses_with_sigma_k_six` — panicked at `partition.rs:424`: `assertion left == right failed: the cap fires before any candidate is enumerated or decoded` (left: 8, right: 0)
- `seat::partition::tests::over_budget_synthetic_set_refuses_statically_with_zero_decodes` — panicked at `partition.rs:476`: `assertion left == right failed: over-budget must refuse with ZERO decode calls -- no hang` (left: 61, right: 0)
- `seat::partition::tests::sum_still_refuses_a_genuinely_over_budget_two_class_group_i1` — panicked at `partition.rs:660`: `assertion left == right failed: over-budget must refuse with ZERO decode calls` (left: 25, right: 0)

Overall: `test result: FAILED. 327 passed; 3 failed; 1 ignored`. Same 3-test failure pattern also took down `cargo test (windows-latest)` and `cargo test (macos-latest)` (platform-independent logic bug, not a platform-specific flake).

**`cargo doc`** — separate defect, 3 broken intra-doc links, `error: could not document `md-cli``:
- `crates/md-cli/src/seat/canonical.rs:10:7` — `` [`super::input::canonicalize_group`] `` — no item named `canonicalize_group` in module `input`
- `crates/md-cli/src/seat/partition.rs:118:7` — same broken link
- `crates/md-cli/src/seat/partition.rs:120:32` — `` [`super::p0_shapes`] `` — no item named `p0_shapes` in module `seat`

**`musl compile/test (x86_64-unknown-linux-musl)`** — failed at the `cargo test -p md-cli --target x86_64-musl` step (same partition-budget defect class, not independently triaged here).

## Actions taken (ritual halted per STOP condition)

- Pushed `main` → `ci/staging` (step 1) — done, SHA `4251069a`.
- Watched + queried the CI run (step 2) — done, run is red.
- **Did NOT run** `git push origin main` (step 3) — halted per "STOP, do not push main" on red.
- **Did NOT delete** `ci/staging` (step 4) — left in place for post-mortem, still present at `4251069a`.
- `origin/main` unchanged: `54ab1cd606ed98140508ea7e2736e0764d63fd83` (verified via `git rev-parse origin/main` after `git fetch`).

## Disposition

Not a CI/infra flake — real logic regression: the seat partition budget check is enumerating/decoding candidates on paths documented and tested to refuse with zero decodes, plus 3 stale rustdoc intra-doc links from a prior rename (`input::canonicalize_group`, `seat::p0_shapes` no longer resolve). Needs a fix-and-recheck cycle on `main` before the next staging attempt; not pushed.

## Attempt 2 — GREEN, SHIPPED

**SHA staged:** `ed20ff856a845c0ec8c7b108d94815abf6a6e1b1` (main, tree clean at start; verified via `git status --short` before pushing)
**CI run:** [33484872891](https://github.com/bg002h/descriptor-mnemonic/actions/runs/33484872891) on branch `ci/staging`, conclusion **success**, headSha confirmed `ed20ff856a845c0ec8c7b108d94815abf6a6e1b1`

Old `ci/staging` ref (attempt 1, left in place at `4251069a`) was updated by a plain fast-forward push — no force-with-lease needed (`4251069a..ed20ff85`).

## Job conclusions (9 of 9, all success)

| Job | Conclusion |
| --- | --- |
| cargo doc | success |
| freebsd compile-gate (whole-crate) | success |
| musl compile/test (x86_64-unknown-linux-musl) | success |
| cargo clippy | success |
| cargo fmt | success |
| cargo test (windows-latest) | success |
| cargo test (macos-latest) | success |
| **cargo test (ubuntu-latest)** (required check) | success |
| musl compile/test (aarch64-unknown-linux-musl) | success |

Both previously-failing classes from attempt 1 are gone: the 3 seat-partition decode-count assertions pass, and the 3 broken rustdoc intra-doc links resolve (`cargo doc` succeeded outright). Required checks `cargo clippy` and `cargo test (ubuntu-latest)` — both success. Every job success (none skipped, none failed) — meets the "every job success/skipped" bar.

## Actions taken

1. `git push origin main:refs/heads/ci/staging` — plain push succeeded (fast-forward, no lease needed): `4251069a..ed20ff85`.
2. Found run `33484872891` for the exact new SHA on `ci/staging`; `gh run watch 33484872891 --repo bg002h/descriptor-mnemonic --exit-status` completed clean; `gh run view --json conclusion,status,headSha,jobs` confirmed `conclusion: success`, `headSha` matches, all 9 jobs `success`.
3. `git push origin main` — output `54ab1cd6..ed20ff85  main -> main`, **no "Bypassed rule violations" line** — the required-check gate was satisfied by the staged SHA's own green run, not bypassed.
4. `git push origin --delete ci/staging` — deleted.
5. `git rev-parse origin/main` == `ed20ff856a845c0ec8c7b108d94815abf6a6e1b1` == local `HEAD`. Confirmed match.

## Disposition

Shipped. `origin/main` now at `ed20ff856a845c0ec8c7b108d94815abf6a6e1b1`. Whole staging run fully green across all 9 jobs (doc, musl x2, clippy, fmt, windows/macos/ubuntu test, freebsd compile-gate) — the exact set that was red or absent-context in attempt 1. `ci/staging` ref cleaned up. No force used, `enforce_admins` untouched, no commits made to `main` between staging and final push.
