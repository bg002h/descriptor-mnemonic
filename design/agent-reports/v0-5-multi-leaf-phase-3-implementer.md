# v0.5 Phase 3 Implementer Report — Top-level dispatcher message update

**Status**: DONE
**Phase**: 3 of 11 (v0.5 multi-leaf TapTree)
**Branch**: `feature/v0.5-multi-leaf-taptree`
**Commit**: `59797ef` (tip; based on `a82ef2c`)
**Model**: Claude Sonnet 4.6
**Date**: 2026-04-28

## Summary

Phase 3 updates the top-level dispatcher (`decode_descriptor` in
`crates/md-codec/src/bytecode/decode.rs`) to:

1. Add an explicit `Tag::TapTree` arm with a TapTree-specific diagnostic
   message that identifies the byte (`0x08`) and explains the correct context
   (`only inside tr(KEY, TREE)`).
2. Drop the stale "v0.4" prefix from three messages that were written when
   v0.4 was the latest cut — now version-agnostic per spec §6 CHANGELOG.

Final test count: **616 passing + 1 ignored, 0 failing** (baseline 615 + 1 ignored
from Phase 2; +1 new unit test in `decode.rs::tests`).

## Files changed

```
crates/md-codec/src/bytecode/decode.rs  | 37 insertions, 3 deletions
```

1 file changed: adds the `Tag::TapTree` arm before the catch-all, updates the
Pkh/Bare message, updates the inline-key message, updates the catch-all message,
and adds the unit test `taptree_at_top_level_produces_specific_diagnostic`.

## Per-task summary

### Task 3.1 — Add explicit Tag::TapTree arm to `decode_descriptor`

**Step 1 — Write failing test**

Added unit test `taptree_at_top_level_produces_specific_diagnostic` inside
`bytecode::decode::tests` (crate-private `mod tests` block). The test:
- Constructs `bytes = vec![Tag::TapTree.as_byte()]` (0x08 raw).
- Calls `decode_template(&bytes, &[])` and expects `Err`.
- Asserts the message contains "TapTree" AND "0x08".
- Asserts the message contains "only inside" OR "tr(KEY".

**Step 2 — Confirmed failure**

Test failed with: `"policy violates v0.1 scope: v0.4 does not support top-level
tag TapTree"` — which correctly lacks "0x08" and the context clause.

**Step 3 — Updated `decode_descriptor`**

Four changes applied:

1. `Some(Tag::Pkh) | Some(Tag::Bare)` message:
   - Old: `"v0.4 does not support top-level pkh()/bare() (legacy non-segwit out of scope)"`
   - New: `"top-level pkh()/bare() is permanently rejected (legacy non-segwit out of scope per design)"`

2. New arm inserted before the inline-key block:
   ```rust
   Some(Tag::TapTree) => Err(Error::PolicyScopeViolation(
       "TapTree (0x08) is not a valid top-level descriptor; \
        it appears only inside `tr(KEY, TREE)` as multi-leaf inner-node framing"
           .to_string(),
   )),
   ```

3. Inline-key catch-arm message:
   - Old: `"v0.4 rejects inline-key tag 0x{tag_byte:02x} (deferred to v1+)"`
   - New: `"inline-key tag 0x{tag_byte:02x} is reserved (deferred to v1+ per descriptor-codec scope)"`

4. Catch-all `Some(other)` message:
   - Old: `"v0.4 does not support top-level tag {other:?}"`
   - New: `"tag {other:?} is not a valid top-level descriptor (recognised but out of scope)"`

**Step 4 — Confirmed pass**

Test passed: 1 passed; 0 failed.

**Step 5 — Audit for old "v0.4 does not support" strings**

```
grep -rn '"v0.4 does not support' crates/md-codec/tests/
```

Zero hits in `tests/`. No existing tests asserted the old string.

Remaining occurrences in `src/` (NOT in `decode_descriptor`):
- `encode.rs:116,123,163` — encoder messages, out of Phase 3 scope.
- `decode.rs:167` — `decode_sh_inner` message, out of Phase 3 scope.

These are NOT part of the Phase 3 changes per the plan's "Files" specification
(`decode.rs:67-100` only).

**Step 6 — All workspace tests pass**

```
cargo test --workspace --no-fail-fast
→ 616 passing, 1 ignored, 0 failing
```

### Task 3.2 — Commit Phase 3

Single commit `59797ef` with conventional-commit message and
`Co-Authored-By: Claude Sonnet 4.6` trailer. 1 file changed,
37 insertions(+), 3 deletions(-).

## Deviations from the plan

### Deviation 1 — Test placed in unit-test block, not integration test file

The plan's Step 1 initially suggested appending to
`crates/md-codec/tests/v0_5_type_wiring.rs` (an integration test), but
included a note (plan line 1077) that `decode_template` and `Cursor::new` are
crate-private (`pub(crate)`) and therefore inaccessible from integration tests.
The plan directed: "use a unit test in `crates/md-codec/src/bytecode/decode.rs`
instead — the test then has crate-private access."

**Resolution**: Test was placed in the existing `#[cfg(test)] mod tests` block
at `decode.rs:901`. This is the correct approach per the plan's own footnote.
The plan's `git add` line in Task 3.2 Step 1 lists `tests/v0_5_type_wiring.rs`
as a staged file — this is stale in the plan (the file was not modified in
Phase 3). Only `decode.rs` was staged.

### Deviation 2 — "v0.4 does not support" strings in encode.rs and decode_sh_inner not updated

The plan's scope is `decode.rs:67-100` (the `decode_descriptor` match). Several
"v0.4 does not support" strings remain in `encode.rs` and `decode_sh_inner` —
these are encoder-side and sh-dispatcher messages outside the Phase 3 scope.
No test asserts them, and the plan does not require updating them.

These may be worth updating in a follow-up pass for consistency, but are not
deferred as FOLLOWUPS entries since they have no user-visible semantic impact
(they are error messages for permanently rejected legacy forms).

## Self-review gate results

```
RUSTUP_TOOLCHAIN=stable cargo test --workspace --no-fail-fast    PASS (616 + 1 ignored)
RUSTUP_TOOLCHAIN=stable cargo fmt --check                        PASS (no output = clean)
RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets   PASS (no warnings)
                       -- -D warnings
```

## Deferred items (FOLLOWUPS)

None. All in-scope items landed. Remaining "v0.4" strings in `encode.rs` and
`decode_sh_inner` are out of Phase 3 scope and have no test coverage pinning
them, so they do not require FOLLOWUPS tracking at this point.

No new entries filed in `design/FOLLOWUPS.md`.

## Test counts

```
Baseline (start of Phase 3): 615 passing, 1 ignored, 0 failing
End of Phase 3:              616 passing, 1 ignored, 0 failing

Delta: +1 unit test (taptree_at_top_level_produces_specific_diagnostic)
```

## Next phase

Phase 4 will rewrite the `Descriptor::Tr(tr)` arm in `encode.rs:126-158` to
support multi-leaf TapTree via a new `encode_tap_subtree` recursive helper.
The single-leaf and KeyOnly encode paths must be preserved byte-identically
(existing corpus must still round-trip).
