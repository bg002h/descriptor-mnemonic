# v0.10 spec review (opus, pass 3)

**Date:** 2026-04-29
**Spec:** design/SPEC_v0_10_per_at_N_paths.md (commit e09791f)
**Reviewer:** opus-4.7 (pass 3, narrow F16/F17 verification + end-to-end re-read)

## Summary

**Verdict: clean.** Pass-2's two findings (F16, F17) landed cleanly at commit `e09791f`. The encoder code sketch in §4 now uses `expect()` with documented BIP 388 invariant rather than the wrong-shaped `Error::OriginPathsCountMismatch` map_err. The §2 line-97 prose now references `BytecodeErrorKind::UnexpectedTag` with a §3 cross-reference. The §6 wire-format-break summary table no longer lists `Error::ConflictingPathDeclarations` and instead correctly itemizes the F4 split (`Error::OriginPathsCountMismatch` + `BytecodeErrorKind::OriginPathsCountTooLarge`). Surviving mentions of `ConflictingPathDeclarations` are limited to historical/explanatory text (lines 214, 397, 593, 606) — all four are status-block or history-callout context, not active spec content. End-to-end re-read surfaces no internal contradictions across §1 / §2 / §3 / §4 / §5 / §6. No new findings.

## Verifications

- **F17 fix landed: ✅.**
  - Line 97 (§2 prose) now reads "...surfaces as `Error::InvalidBytecode { offset: 1, kind: BytecodeErrorKind::UnexpectedTag { expected, got } }`, where `expected` is the tag the header bit predicted (`0x34` if bit 3 clear, `0x36` if set). See §3 'Path-decl dispatch' for the full match logic." Cross-reference is present and points correctly. The `where`-clause naming the expected value is a nice touch — matches the §3 dispatch sketch (line 203) exactly.
  - §6 wire-format-break summary table (lines 485–497) no longer lists `Error::ConflictingPathDeclarations`. Replacement rows itemize the F4 split correctly: `Error::OriginPathsCountMismatch` (line 493, semantic, policy layer), `Error::PathComponentCountExceeded` (line 494), and `BytecodeErrorKind::OriginPathsCountTooLarge` (line 495, structural, bytecode layer). The split shape matches §1 item 5 and §3 line 239.
  - Surviving mentions of `ConflictingPathDeclarations` (4 total): line 214 (parenthetical "Per F5: avoid introducing..." in §3 prose), line 397 (parenthetical drop-note in §4 error definitions), line 593 (self-review checklist history), line 606 (pass-1 finding status). All four are explicitly historical/explanatory; none claim it as an active variant. The variant has been cleanly excised from the active spec surface.

- **F16 fix landed: ✅.**
  - §4 encoder code sketch (lines 312–316) now uses:
    ```rust
    let count_u8 = u8::try_from(placeholder_paths.len())
        .expect("BIP 388 caps placeholder count at 32; upstream validation guarantees this");
    ```
    with a 3-line preceding comment block (lines 312–314) documenting "BIP 388 caps placeholder count at 32 upstream of `to_bytecode` (validated when the `WalletPolicy` is constructed); the cast is infallible. expect() documents the upstream invariant rather than silently masking a real bug." This matches reviewer's recommended shape (option 2 from F16, the `expect()` route). Documenting the invariant inline in the sketch is honest about where the cap is enforced.
  - The redundant `if count_u8 > 32 { return Err(...); }` block is **removed** (was at lines 313–315 in the prior commit; now absent in the post-fix sketch). This is correct: it would have been unreachable code given the `expect()` already covered the only failure mode of `try_from` for a `> 32` count, and BIP 388 enforces the cap upstream.
  - No remnants of `Error::OriginPathsCountMismatch` survive in the encoder sketch. The variant is now used exclusively in the decoder semantic-check path (§3 line 239) and negative vector `n_orig_count_mismatch` (line 454), both correct contexts.

- **End-to-end consistency: ✅.**
  - **§1 decision matrix → §2 wire format.** Q3-A (strict mutual exclusion) → §2 line 97 dispatches via header bit 3 with `UnexpectedTag` error shape. Q4 (header bit 3) → §2 line 73 (header layout) + §2 line 78 (`RESERVED_MASK = 0x03`) + §3 line 185 (`RESERVED_MASK: u8 = 0x03`). Q8 (`MAX_PATH_COMPONENTS = 10`) → §2 line 113 (path-decl explicit-form clause) + §2 line 125 (SharedPath cap) + §2 line 129 (constant location) + §3 line 232 (`decode_path` enforcement) + §4 line 360 (constant declaration). All cross-section references agree.
  - **§2 wire format → §3 decoder dispatch.** Header byte set `{0x00, 0x04, 0x08, 0x0C}` (line 80) matches the bit-2/bit-3 cross product. `UnexpectedTag { expected, got }` shape used at §2 line 97 matches §3 line 209 (`BytecodeErrorKind::UnexpectedTag { expected, got: other }`) and §5 line 459 (`{ expected: 0x36, got: 0x34 }`). Structural-vs-semantic count split at §2 lines 109–110 matches §3 lines 219–236 (`decode_origin_paths` body) and §3 line 239 (commentary).
  - **§3 decoder → §4 encoder symmetry.** The encoder dispatch (§4 lines 296–332) is a clean inverse of the decoder dispatch (§3 lines 198–212): encoder picks `SharedPath` vs `OriginPaths` by `all_share`, sets header bit 3 accordingly, emits the matching tag. The 4-tier precedence chain (§4 lines 285–294) feeds `placeholder_paths` to the dispatch.
  - **§4 type updates → §6 wire-format-break table.** `BytecodeHeader::new_v0(bool)` → `new_v0(bool, bool)` change (§4 line 363) matches §6 line 491 (`BytecodeHeader::new_v0` signature row) and §6 line 503 (migration table sed snippet).
  - **§5 negative vectors → §3 / §4 error variants.** Each negative vector cites the correct error variant: `n_orig_count_mismatch` → `Error::OriginPathsCountMismatch` (semantic, decoder, line 454); `n_orig_paths_count_zero` → `BytecodeErrorKind::OriginPathsCountTooLarge` (structural, line 457); `n_orig_paths_count_too_large` → same variant (line 458); `n_orig_paths_truncated` → `BytecodeErrorKind::UnexpectedEnd` (line 456); `n_conflicting_path_declarations` → `BytecodeErrorKind::UnexpectedTag` with the F5/F17 shape (line 459). All five are consistent with their respective §3 / §4 emit-sites.
  - **§6 migration table.** No claims contradict §2 wire format. The `0x00, 0x04` → `0x00, 0x04, 0x08, 0x0C` row matches §2 line 80; the reserved-bits mask row (`0x0B` → `0x03`) matches §2 line 78. The "v0.9 consumer code catching `Error::ReservedBitsSet { byte: 0x08 }`" migration guidance (§6 line 504) is consistent with the wire-format-break framing in §1 line 8 (pre-v0.10 decoders reject v0.10 OriginPaths-using encodings via `Error::ReservedBitsSet`).

  No internal inconsistencies introduced by the back-and-forth revisions.

## New findings (if any)

None. F1–F17 all addressed. Spec is clean.

## Greenlight

**Ready for plan-writing.** The spec has cleared three review passes; the structural-vs-semantic error split, the 4-tier precedence chain, and the strict mutual exclusion at the path-decl slot are all internally consistent across all six sections.
