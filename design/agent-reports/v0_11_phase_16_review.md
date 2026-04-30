# v0.11 Phase 16 Review — ChunkHeader + ChunkSetId Derivation

- Date: 2026-04-30
- Branch: `feature/v0.11-impl-phase-1`
- Phase: 16 (ChunkHeader + ChunkSetId derivation)
- Spec: §9.3 (chunk header)

## Status: DONE

## Scope

Phase 16 ships only the chunk-header data structure and chunk-set-id
derivation. The actual split/reassemble of chunks across multiple md1
codex32 strings is delivered in Phase 21 (after codex32 BCH wiring in
Phase 19 and md1 string emit/parse in Phase 20). This was an explicit
plan revision after the third spec review, replacing what was
previously a `todo!()` placeholder.

## Commits

- `95c729c` — Task 16.1: ChunkHeader encode/decode.
  - 37 bits total: 3 version + 1 chunked + 1 reserved + 20 chunk_set_id
    + 6 count + 6 index.
  - `chunked` flag (bit 3) = 1 unconditionally for chunk headers; this
    is the dispatch mechanism distinguishing chunk headers from
    payload headers (where bit 3 is reserved=0).
  - `count` is encoded as count-1 offset (range 1..=64 → 6-bit value
    0..=63).
  - 3 tests, 4 error variants.
- `dbf136a` — Task 16.2: `derive_chunk_set_id`.
  - Top 20 bits of `Md1EncodingId` (MSB-first):
    `(bytes[0] << 12) | (bytes[1] << 4) | (bytes[2] >> 4)`.
  - 2 tests (deterministic + MSB-first extraction).

## Verification

`cargo test -p md-codec --lib v11::chunk` — 5 passed:

- `chunk_set_id_tests::derive_chunk_set_id_deterministic`
- `chunk_set_id_tests::derive_chunk_set_id_msb_first_extraction`
- `tests::chunk_header_count_zero_rejected`
- `tests::chunk_header_round_trip`
- `tests::chunk_header_count_64_round_trip`

Cumulative v11 lib + smoke tests: 86 (81 + 5).

## Spec alignment (§9.3)

ChunkHeader bit layout, count-1 offset encoding, 20-bit chunk-set-id
derivation, and the `chunked` dispatch flag all match §9.3.

## Carry-forward deferred items

P1, P2, P4, P5, P12, P13a, P13b — same set as prior phase, no new
deferrals from Phase 16.

## Next

Phase 17 — forward-compat tests.
