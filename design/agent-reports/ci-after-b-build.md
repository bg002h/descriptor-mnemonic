
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

## L2 (docs head 418896c6c111f486a73e090d4e56c356a628bfe1, previous head 09cbf98876a0ea3c55d5ae09606e0ebfdebd9fb7)

- run 40229942528 job 121279299756 `CI/musl compile/test (aarch64-unknown-linux-musl)`: `docs-only: all 1 files changed since the previous PR head 09cbf98876a0ea3c55d5ae09606e0ebfdebd9fb7 are documentation, the merged base a6929eb18f05ddfc106dabed513fd6981ca760bb is an ancestor of it, and 'musl compile/test (aarch64-unknown-linux-musl)' passed on it; skipping the guarded steps`
- run 40229942528 job 121279299877 `CI/freebsd compile-gate (whole-crate)`: `docs-only: all 1 files changed since the previous PR head 09cbf98876a0ea3c55d5ae09606e0ebfdebd9fb7 are documentation, the merged base a6929eb18f05ddfc106dabed513fd6981ca760bb is an ancestor of it, and 'freebsd compile-gate (whole-crate)' passed on it; skipping the guarded steps`
- run 40229942528 job 121279299891 `CI/cargo test (macos-latest)`: `docs-only: all 1 files changed since the previous PR head 09cbf98876a0ea3c55d5ae09606e0ebfdebd9fb7 are documentation, the merged base a6929eb18f05ddfc106dabed513fd6981ca760bb is an ancestor of it, and 'cargo test (macos-latest)' passed on it; skipping the guarded steps`
- run 40229942528 job 121279299924 `CI/cargo clippy`: `docs-only: all 1 files changed since the previous PR head 09cbf98876a0ea3c55d5ae09606e0ebfdebd9fb7 are documentation, the merged base a6929eb18f05ddfc106dabed513fd6981ca760bb is an ancestor of it, and 'cargo clippy' passed on it; skipping the guarded steps`
- run 40229942528 job 121279299929 `CI/cargo test (windows-latest)`: `docs-only: all 1 files changed since the previous PR head 09cbf98876a0ea3c55d5ae09606e0ebfdebd9fb7 are documentation, the merged base a6929eb18f05ddfc106dabed513fd6981ca760bb is an ancestor of it, and 'cargo test (windows-latest)' passed on it; skipping the guarded steps`
- run 40229942528 job 121279299931 `CI/cargo doc`: `docs-only: all 1 files changed since the previous PR head 09cbf98876a0ea3c55d5ae09606e0ebfdebd9fb7 are documentation, the merged base a6929eb18f05ddfc106dabed513fd6981ca760bb is an ancestor of it, and 'cargo doc' passed on it; skipping the guarded steps`
- run 40229942528 job 121279299949 `CI/musl compile/test (x86_64-unknown-linux-musl)`: `docs-only: all 1 files changed since the previous PR head 09cbf98876a0ea3c55d5ae09606e0ebfdebd9fb7 are documentation, the merged base a6929eb18f05ddfc106dabed513fd6981ca760bb is an ancestor of it, and 'musl compile/test (x86_64-unknown-linux-musl)' passed on it; skipping the guarded steps`
- run 40229942528 job 121279299955 `CI/cargo fmt`: `docs-only: all 1 files changed since the previous PR head 09cbf98876a0ea3c55d5ae09606e0ebfdebd9fb7 are documentation, the merged base a6929eb18f05ddfc106dabed513fd6981ca760bb is an ancestor of it, and 'cargo fmt' passed on it; skipping the guarded steps`
- run 40229942528 job 121279299966 `CI/cargo test (ubuntu-latest)`: `docs-only: all 1 files changed since the previous PR head 09cbf98876a0ea3c55d5ae09606e0ebfdebd9fb7 are documentation, the merged base a6929eb18f05ddfc106dabed513fd6981ca760bb is an ancestor of it, and 'cargo test (ubuntu-latest)' passed on it; skipping the guarded steps`
- run 40229942525 job 121279299681 `release/release test (macos-latest)`: `docs-only: all 1 files changed since the previous PR head 09cbf98876a0ea3c55d5ae09606e0ebfdebd9fb7 are documentation, the merged base a6929eb18f05ddfc106dabed513fd6981ca760bb is an ancestor of it, and 'release test (macos-latest)' passed on it; skipping the guarded steps`
- run 40229942525 job 121279299816 `release/build md (macos-x86_64)`: `docs-only: all 1 files changed since the previous PR head 09cbf98876a0ea3c55d5ae09606e0ebfdebd9fb7 are documentation, the merged base a6929eb18f05ddfc106dabed513fd6981ca760bb is an ancestor of it, and 'build md (macos-x86_64)' passed on it; skipping the guarded steps`
- run 40229942525 job 121279299884 `release/build md (macos-aarch64)`: `docs-only: all 1 files changed since the previous PR head 09cbf98876a0ea3c55d5ae09606e0ebfdebd9fb7 are documentation, the merged base a6929eb18f05ddfc106dabed513fd6981ca760bb is an ancestor of it, and 'build md (macos-aarch64)' passed on it; skipping the guarded steps`
- run 40229942525 job 121279299901 `release/build md (windows-x86_64)`: `docs-only: all 1 files changed since the previous PR head 09cbf98876a0ea3c55d5ae09606e0ebfdebd9fb7 are documentation, the merged base a6929eb18f05ddfc106dabed513fd6981ca760bb is an ancestor of it, and 'build md (windows-x86_64)' passed on it; skipping the guarded steps`
- run 40229942525 job 121279299962 `release/build md (linux-x86_64)`: `docs-only: all 1 files changed since the previous PR head 09cbf98876a0ea3c55d5ae09606e0ebfdebd9fb7 are documentation, the merged base a6929eb18f05ddfc106dabed513fd6981ca760bb is an ancestor of it, and 'build md (linux-x86_64)' passed on it; skipping the guarded steps`
- run 40229942525 job 121279299976 `release/build md (linux-aarch64)`: `docs-only: all 1 files changed since the previous PR head 09cbf98876a0ea3c55d5ae09606e0ebfdebd9fb7 are documentation, the merged base a6929eb18f05ddfc106dabed513fd6981ca760bb is an ancestor of it, and 'build md (linux-aarch64)' passed on it; skipping the guarded steps`
- run 40229942525 job 121279300041 `release/release test (ubuntu-latest)`: `docs-only: all 1 files changed since the previous PR head 09cbf98876a0ea3c55d5ae09606e0ebfdebd9fb7 are documentation, the merged base a6929eb18f05ddfc106dabed513fd6981ca760bb is an ancestor of it, and 'release test (ubuntu-latest)' passed on it; skipping the guarded steps`
