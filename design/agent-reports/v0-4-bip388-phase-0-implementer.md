# Phase 0 Implementer Report — decode_wsh_inner body/wrapper refactor

**Date**: 2026-04-27
**Branch**: feature/v0.4-bip388-modern-surface
**Commit**: 8911ed5

## Status: DONE

## Summary

Refactored `decode_wsh_inner` in `crates/md-codec/src/bytecode/decode.rs` to
split into:

- `pub(crate) decode_wsh_body(cur, keys) -> Result<Wsh<DescriptorPublicKey>, Error>` — pure wsh body decoder, both SortedMulti and generic-miniscript arms now return `Ok(wsh)` instead of `Ok(Descriptor::Wsh(wsh))`.
- `fn decode_wsh_inner(cur, keys) -> Result<Descriptor<DescriptorPublicKey>, Error>` — thin wrapper calling `Ok(Descriptor::Wsh(decode_wsh_body(cur, keys)?))`

Added one new unit test `decode_wsh_body_returns_inner_wsh_not_descriptor` that
type-ascribes the return as `Wsh<DescriptorPublicKey>` — the type ascription is
the assertion.

## Files Changed

```
crates/md-codec/src/bytecode/decode.rs | 46 insertions(+), 6 deletions(-)
```

(From `git diff HEAD~1 --stat`)

## Test Results

```
test result: ok. 392 passed; 0 failed; 0 ignored  (decode.rs inline tests)
test result: ok. 42 passed; 0 failed; 0 ignored
test result: ok. 37 passed; 0 failed; 0 ignored
test result: ok. 18 passed; 0 failed; 0 ignored
... (all harnesses)
Total: 566 passed; 0 failed; 0 ignored
```

565 pre-existing + 1 new = 566 passing.

## Gate Results

- **build**: `Finished dev profile` — clean
- **test**: 566 passed, 0 failed, 0 ignored
- **clippy**: `Finished dev profile` — 0 warnings
- **fmt**: no output (clean)

## Structural Observations

The function had exactly two `Ok(Descriptor::Wsh(wsh))` return paths as
documented (one per match arm). The split was straightforward — no shared
helper functions complicating the extraction. The `inner_tag_offset` variable
used in error construction is local to `decode_wsh_body` and does not leak.

## Self-Review Findings

None — the refactor is mechanical. The `pub(crate)` visibility matches the
plan requirement and makes `decode_wsh_body` accessible to the upcoming
`decode_sh_inner` in Phase 2 without exposing it publicly.

## Concerns

None.
