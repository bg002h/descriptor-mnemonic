# v0.17 Phase 0 — V1 verification (per-phase report)

**Date:** 2026-05-09
**Branch:** `feat/v0.17-tap-multi-leaf-policy`

## Scope

Lock down exact pre-implementation behavior for three template shapes v0.17 will flip from failure → success. Test serves as a canary: failures during later phases confirm intentional behavior change.

## Artifacts

- `crates/md-cli/tests/v017_v1_encode_acceptance.rs` (new; long-term test, not throwaway).
- Three sub-tests:
  - V1.a `tr(@0,pk(@1))` — succeeds today; must keep passing through Phase 4.
  - V1.b `tr(@0,and_v(v:pk(@1),older(144)))` — fails today with `unsupported miniscript fragment: and_v`. Pinned. Phase 5 flips to assert success.
  - V1.c `tr(50929b74…,multi_a(2,@0,@1,@2))` — fails today with `synthetic key … not found in key map`. Pinned. Phase 5 flips to assert success.

## Verification

- `cargo test -p md-cli --test v017_v1_encode_acceptance` → 3 passed against pre-implementation state.
- `grep -n "Some(0x05)" crates/md-codec/src/tag.rs` returns empty → extension sub-code 0x05 confirmed unallocated.

## Per-phase code-reviewer round

`feature-dev:code-reviewer` dispatched. Findings:

- **C: 0**
- **I1** — V1.b/V1.c doc-comments said "demand an update" without naming the correct update phase. Reviewer noted plan §Phase 5 is where the assert-success update lands, not Phase 2/Phase 3. **Fixed inline** — doc-comments now say "update this test in Phase 5 (not Phase 2/3)".
- **I3** (borderline; reviewer flagged confidence 80) — V1.a no `--keys` could become fragile if future phases add key-count validation. **Mitigated** — added one-line comment on V1.a explaining keys-are-omitted-by-design (synthetic placeholders).
- Net: 0C/0I after fixes.

## Exit gate

- ✅ V1 documents exact pre-implementation failures.
- ✅ Tag extension code 0x05 unallocated.
- ✅ Per-phase reviewer 0C/0I.

Phase 0 closed; proceeding to Phase 1.
