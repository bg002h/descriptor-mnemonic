# v0.11 Phase 5 — Use-site path-decl

**Date:** 2026-04-30
**Branch:** `feature/v0.11-impl-phase-1`
**Scope:** Task 5.1 (this phase review).

## Commits

- Task 5.1 — `0603dd5` — `UseSitePath` encode/decode (7 tests)

## Files added

- `crates/md-codec/src/v11/use_site_path.rs` (new)
- `crates/md-codec/src/v11/mod.rs` — `pub mod use_site_path;` addition
- `crates/md-codec/src/v11/error.rs` — error variant(s) for use-site alt-count bounds

## Test count

7 unit tests in `v11::use_site_path`, all passing under
`cargo test -p md-codec --lib v11::use_site_path`:

- `use_site_path_standard_round_trip`
- `use_site_path_standard_bit_cost`
- `use_site_path_bare_star_round_trip`
- `use_site_path_bare_star_bit_cost`
- `use_site_path_hardened_wildcard_round_trip`
- `use_site_path_alt_count_too_small_rejected`
- `use_site_path_alt_count_too_large_rejected`

Cumulative `v11` test count: **36** (7 bitstream + 10 varint + 5 header
+ 7 origin_path + 7 use_site_path), all passing under
`cargo test -p md-codec --lib v11`.

## Spec coverage

- **§3.5** — use-site path-decl block.

## Bit costs

- Standard `<0;1>/*` use-site path: **16 bits**.
- Bare `*` (no alt list, single child branch): **2 bits**.

Both verified by dedicated bit-cost tests.

## Wire format

- Explicit-only encoding (no use-site path dictionary, per D26′).
- Encoder rejects alt-count outside `2..=9` (the inclusive range
  permitted by the field's count-2 offset encoding).

## Carry-forward deferred items (Phases 1–5)

- **Phase 1 — unused `BitStreamExhausted` variant.** Declared but not
  yet read by any call path; either wire it up or drop in a polish pass.
- **Phase 1 — `read_past_end_errors` state-preservation assertion.**
  Test could additionally assert that a failed read leaves bit position
  unchanged.
- **Phase 2 — `write_varint` debug-assert → assert promotion.** Promote
  the bounds guard to an unconditional `assert!` for release-mode safety.
- **Phase 2 — `L=0` hand-crafted decode test.** Add a fixture-driven
  decode test exercising the L=0 single-byte varint path.
- **Phase 4 — `PathDecl::write` `# Errors` doc gap.** Doc comment does
  not enumerate its error conditions.
- **Phase 5 — `UseSitePath::write` `# Errors` doc gap.** Same pattern as
  Phase 4 — the doc comment omits a `# Errors` section enumerating the
  alt-count-out-of-range error.

All non-blocking; consolidating in a single polish commit remains the
right disposition.

## Open items

None blocking phase exit.

## Next phase

Phase 6 — `Tag` enum + tree skeleton.
