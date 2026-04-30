# v0.11 Phase 11 Review — Extension-space ops

**Date:** 2026-04-30
**Branch:** `feature/v0.11-impl-phase-1`
**Phase:** 11 — Extension-space ops (Hash256/Ripemd160/RawPkH + False/True)
**Status:** DONE

## Summary

Phase 11 closes out tree-dispatch coverage by wiring the four remaining
extension-space operators referenced in §5 (extension space) of the v0.11
wire-format brainstorm:

- **Task 11.1 (`c13e15c`)** — Hash256, Ripemd160, RawPkH dispatch in
  `write_node` / `read_node`. 2 tests, including a 266-bit cost assertion
  (10-bit tag + 256-bit hash literal). Ripemd160/RawPkH share the 170-bit
  shape (10-bit tag + 160-bit hash literal).
- **Task 11.2 (`7e9d540`)** — False / True dispatch. 2 tests, including a
  10-bit cost assertion (10-bit tag, no body). With these arms in place
  the trailing `_ => unimplemented!()` fallbacks were removed from both
  `write_node` and `read_node`; the `Tag` and `Body` matches are now
  exhaustive.

### Key milestone

**Tree dispatch is COMPLETE.** All 36 `Tag` variants and all 8 `Body`
variants are handled in both encode and decode paths. There are no
`unimplemented!()` arms left in the core tree codec; the compiler now
enforces exhaustiveness.

### Bit-cost notes (cited from §5)

| Op             | Tag bits | Body bits | Total |
|----------------|---------:|----------:|------:|
| Hash256        |       10 |       256 |   266 |
| Ripemd160      |       10 |       160 |   170 |
| RawPkH         |       10 |       160 |   170 |
| False          |       10 |         0 |    10 |
| True           |       10 |         0 |    10 |

## Verification

```
$ cargo test -p md-codec --lib v11::tree
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 528 filtered out
```

17 `v11::tree` tests pass. Cumulative v11 test count: **61** (57 prior + 4
new in Phase 11).

## Deferred items (carry-forward, unchanged from Phase 10)

No new deferrals introduced in Phase 11. Open items remain:

- **P1** — TLV section framing (Phase 12)
- **P2** — UseSitePathOverrides encoding (Phase 12)
- **P4** — Fingerprints section (Phase 12)
- **P5** — `@N` divergent-origin policy edges
- **P7** — Cross-format header negotiation with mk1
- **P9** — Top-level descriptor checksum / outer frame

## Next

Phase 12 — TLV section + UseSitePathOverrides + Fingerprints.

## Status

**DONE** at commit `7e9d540`. No CONCERNS, no BLOCKED items. CONTEXT:
exhaustive matches in `v11::tree` mean future tag/body additions will
fail to compile until handled — a desirable invariant for Phase 12 work.
