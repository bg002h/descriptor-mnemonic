
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

## L2 (docs head 529442a89605f836111221b06bc8cf82bdbe9c5d, previous head b7428a522372d3dc24e8129f652abc8c0eb67531)

- run 39806069082 job 119919499208 `CI/cargo fmt`: `docs-only: all 1 files changed since the previous PR head b7428a522372d3dc24e8129f652abc8c0eb67531 are documentation, the merged base 9033386838868499243a9a778ef15d95dc71f054 is an ancestor of it, and 'cargo fmt' passed on it; skipping the guarded steps`
- run 39806069082 job 119919499542 `CI/musl compile/test (x86_64-unknown-linux-musl)`: `docs-only: all 1 files changed since the previous PR head b7428a522372d3dc24e8129f652abc8c0eb67531 are documentation, the merged base 9033386838868499243a9a778ef15d95dc71f054 is an ancestor of it, and 'musl compile/test (x86_64-unknown-linux-musl)' passed on it; skipping the guarded steps`
- run 39806069082 job 119919499547 `CI/cargo doc`: `docs-only: all 1 files changed since the previous PR head b7428a522372d3dc24e8129f652abc8c0eb67531 are documentation, the merged base 9033386838868499243a9a778ef15d95dc71f054 is an ancestor of it, and 'cargo doc' passed on it; skipping the guarded steps`
- run 39806069082 job 119919499548 `CI/cargo test (windows-latest)`: `docs-only: all 1 files changed since the previous PR head b7428a522372d3dc24e8129f652abc8c0eb67531 are documentation, the merged base 9033386838868499243a9a778ef15d95dc71f054 is an ancestor of it, and 'cargo test (windows-latest)' passed on it; skipping the guarded steps`
- run 39806069082 job 119919499550 `CI/musl compile/test (aarch64-unknown-linux-musl)`: `docs-only: all 1 files changed since the previous PR head b7428a522372d3dc24e8129f652abc8c0eb67531 are documentation, the merged base 9033386838868499243a9a778ef15d95dc71f054 is an ancestor of it, and 'musl compile/test (aarch64-unknown-linux-musl)' passed on it; skipping the guarded steps`
- run 39806069082 job 119919499667 `CI/freebsd compile-gate (whole-crate)`: `docs-only: all 1 files changed since the previous PR head b7428a522372d3dc24e8129f652abc8c0eb67531 are documentation, the merged base 9033386838868499243a9a778ef15d95dc71f054 is an ancestor of it, and 'freebsd compile-gate (whole-crate)' passed on it; skipping the guarded steps`
- run 39806069082 job 119919499723 `CI/cargo test (ubuntu-latest)`: `docs-only: all 1 files changed since the previous PR head b7428a522372d3dc24e8129f652abc8c0eb67531 are documentation, the merged base 9033386838868499243a9a778ef15d95dc71f054 is an ancestor of it, and 'cargo test (ubuntu-latest)' passed on it; skipping the guarded steps`
- run 39806069082 job 119919499750 `CI/cargo test (macos-latest)`: `docs-only: all 1 files changed since the previous PR head b7428a522372d3dc24e8129f652abc8c0eb67531 are documentation, the merged base 9033386838868499243a9a778ef15d95dc71f054 is an ancestor of it, and 'cargo test (macos-latest)' passed on it; skipping the guarded steps`
- run 39806069082 job 119919499824 `CI/cargo clippy`: `docs-only: all 1 files changed since the previous PR head b7428a522372d3dc24e8129f652abc8c0eb67531 are documentation, the merged base 9033386838868499243a9a778ef15d95dc71f054 is an ancestor of it, and 'cargo clippy' passed on it; skipping the guarded steps`
- run 39806069143 job 119919499605 `release/release test (macos-latest)`: `docs-only: all 1 files changed since the previous PR head b7428a522372d3dc24e8129f652abc8c0eb67531 are documentation, the merged base 9033386838868499243a9a778ef15d95dc71f054 is an ancestor of it, and 'release test (macos-latest)' passed on it; skipping the guarded steps`
- run 39806069143 job 119919499779 `release/release test (ubuntu-latest)`: `docs-only: all 1 files changed since the previous PR head b7428a522372d3dc24e8129f652abc8c0eb67531 are documentation, the merged base 9033386838868499243a9a778ef15d95dc71f054 is an ancestor of it, and 'release test (ubuntu-latest)' passed on it; skipping the guarded steps`
