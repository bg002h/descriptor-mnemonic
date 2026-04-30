# v0.11 Phase 4 — Origin path-decl block

**Date:** 2026-04-30
**Branch:** `feature/v0.11-impl-phase-1`
**Scope:** Tasks 4.1, 4.2 (this phase review).

## Commits

- Task 4.1 — `e0bd61c` — `PathComponent` + `OriginPath` (3 tests)
- Task 4.2 — `5b878ae` — `PathDecl` shared + divergent (4 tests)

## Files added

- `crates/md-codec/src/v11/origin_path.rs` (new)
- `crates/md-codec/src/v11/mod.rs` — `pub mod origin_path;` addition
- `crates/md-codec/src/v11/error.rs` — error variants for depth/key-count bounds and divergent-path count mismatch

## Test count

7 unit tests in `v11::origin_path`, all passing under `cargo test -p md-codec --lib v11::origin_path`:

- `tests::origin_path_round_trip_bip84`
- `tests::origin_path_bit_cost_bip84`
- `tests::origin_path_rejects_depth_too_large`
- `path_decl_tests::path_decl_shared_round_trip`
- `path_decl_tests::path_decl_shared_bit_cost_bip84`
- `path_decl_tests::path_decl_divergent_round_trip`
- `path_decl_tests::path_decl_n_zero_rejected`

Cumulative `v11` test count: **29** (7 bitstream + 10 varint + 5 header + 7 origin_path), all passing under `cargo test -p md-codec --lib v11`.

## Spec coverage

- **§3.4** — origin path-decl block (shared and divergent forms).
- **§4.2** — count-1 offset encoding for `n` (key-count field; encoded value = `n - 1`, valid range `1..=32`).

## Bit cost

BIP 84 single-sig path-decl: **31 bits** (n=1 + depth=3 + 84' + 0' + 0'). Verified by `path_decl_shared_bit_cost_bip84`.

## Wire format

- Explicit-only encoding (no path dictionary, per D19′).
- Encoder rejects `depth > 15` (4-bit field) and `n` outside `1..=32`.
- Decoder dispatches Shared vs Divergent on header bit 4 (`divergent_paths` flag from Phase 3).

## TDD discipline note

Task 4.2 implementer skipped the explicit `unimplemented!()` red step and went straight to a passing implementation. The spec snippet was verbatim and correct, so no bug was masked here. Flagging as a process observation; future tasks should keep the red→green discipline so the test harness itself is exercised before the implementation lands.

## Deferred minor item from code-quality review

- **`PathDecl::write` `# Errors` doc gap.** The doc comment does not enumerate its two error conditions (`KeyCountOutOfRange`, `DivergentPathCountMismatch`). Worth a `# Errors` doc section in a future polish pass.

## Carry-forward deferred items (Phases 1–4)

- **Phase 1 — unused `BitStreamExhausted` variant.** Declared but not yet read by any call path; either wire it up or drop it in a polish pass.
- **Phase 1 — `read_past_end_errors` state-preservation assertion.** Test could additionally assert that a failed read leaves bit position unchanged.
- **Phase 2 — `write_varint` debug-assert → assert promotion.** Promote the bounds guard to an unconditional `assert!` for release-mode safety.
- **Phase 4 — `PathDecl::write` `# Errors` doc gap** (above).

All four are non-blocking; consolidating in a single polish commit remains the right disposition.

## Open items

None blocking phase exit.

## Next phase

Phase 5 — Use-site path-decl.
