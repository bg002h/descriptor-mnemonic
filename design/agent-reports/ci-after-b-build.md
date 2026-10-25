
# Re-run after: ci: guard the release build jobs with ci/docs-only.sh

PR #39. Copies ci-skip-constellation reference/after-b/B-dm (SPEC-after-b.md).

## Change

```
 .github/workflows/release.yml | 30 ++++++++++++++++++++++++------
 1 file changed, 24 insertions(+), 6 deletions(-)
```

## Gate

`make-after-b-reference --check: PASS (12 files)`

## Local checks

```
ci/docs-only.sh: OK
ci/docs-only.test.sh: OK
ci/code-docs-check.sh: OK
ci/code-docs-check.test.sh: OK
docs-only.test.sh: 73 passed, 0 failed
code-docs-check.test.sh: 41 passed, 0 failed
code-docs check: ok (239 files scanned, 375 docs files)
PASS B-dm .github/workflows/release.yml: exactly the §4.2 edits to build
after-b-check B-dm: PASS
```

## L1 (code head 09cbf98876a0ea3c55d5ae09606e0ebfdebd9fb7)

```
run 40229204657    release                        pull_request   success    ok
run 40229204653    CI                             pull_request   success    ok
  release/release test (macos-latest)              success    ok
  release/build md (windows-x86_64)                success    ok
  release/build md (linux-aarch64)                 success    ok
  release/build md (macos-aarch64)                 success    ok
  release/build md (macos-x86_64)                  success    ok
  release/release test (ubuntu-latest)             success    ok
  release/build md (linux-x86_64)                  success    ok
  release/assemble + release                       skipped    ok (allowed skip)
  CI/freebsd compile-gate (whole-crate)            success    ok
  CI/musl compile/test (aarch64-unknown-linux-musl) success    ok
  CI/cargo test (ubuntu-latest)                    success    ok
  CI/cargo doc                                     success    ok
  CI/musl compile/test (x86_64-unknown-linux-musl) success    ok
  CI/cargo test (macos-latest)                     success    ok
  CI/cargo clippy                                  success    ok
  CI/cargo fmt                                     success    ok
  CI/cargo test (windows-latest)                   success    ok
pr-checks: PASS (descriptor-mnemonic 09cbf98876a0ea3c55d5ae09606e0ebfdebd9fb7)
```
