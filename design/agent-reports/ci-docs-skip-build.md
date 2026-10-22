
# Re-run after: ci: skip a docs-only push to a PR that already passed (rule A')

PR #38. Applies ci-skip-constellation SPEC.md (GREEN at e8f831d), Part A.

## Change

```
 .github/workflows/ci.yml      |   6 ++
 .github/workflows/release.yml |  13 +++-
 ci/code-docs-check.sh         | 165 ++++++++++++++++++++++++++++++++++++++++++
 ci/code-docs-check.test.sh    | 128 ++++++++++++++++++++++++++++++++
 ci/code-docs-exempt.txt       |   7 ++
 ci/docs-only-paths.txt        |   2 +
 ci/docs-only.sh               | 158 +++++++++++++++++++++++++++++-----------
 ci/docs-only.test.sh          | 103 ++++++++++++++++++++++++--
 ci/shared-ci.sha256           |   4 +
 9 files changed, 534 insertions(+), 52 deletions(-)
```

## Gate

`gate-public: 0 failure(s)`

## Local checks

```
ci/docs-only.sh: OK
ci/docs-only.test.sh: OK
ci/code-docs-check.sh: OK
ci/code-docs-check.test.sh: OK
docs-only.test.sh: 73 passed, 0 failed
code-docs-check.test.sh: 41 passed, 0 failed
code-docs check: ok (239 files scanned, 373 docs files)
```

## L1 (code head b7428a522372d3dc24e8129f652abc8c0eb67531)

```
run 39803257224    release                        pull_request   success    ok
run 39803252317    CI                             pull_request   success    ok
  release/release test (macos-latest)              success    ok
  release/build md (macos-x86_64)                  success    ok
  release/build md (windows-x86_64)                success    ok
  release/build md (macos-aarch64)                 success    ok
  release/build md (linux-aarch64)                 success    ok
  release/release test (ubuntu-latest)             success    ok
  release/build md (linux-x86_64)                  success    ok
  release/assemble + release                       skipped    ok (allowed skip)
  CI/cargo clippy                                  success    ok
  CI/cargo doc                                     success    ok
  CI/musl compile/test (x86_64-unknown-linux-musl) success    ok
  CI/cargo fmt                                     success    ok
  CI/cargo test (windows-latest)                   success    ok
  CI/cargo test (ubuntu-latest)                    success    ok
  CI/cargo test (macos-latest)                     success    ok
  CI/musl compile/test (aarch64-unknown-linux-musl) success    ok
  CI/freebsd compile-gate (whole-crate)            success    ok
pr-checks: PASS (descriptor-mnemonic b7428a522372d3dc24e8129f652abc8c0eb67531)
```
