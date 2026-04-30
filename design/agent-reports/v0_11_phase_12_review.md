# Phase 12 Review — TLV Section + UseSitePathOverrides + Fingerprints

- **Date:** 2026-04-30
- **Branch:** `feature/v0.11-impl-phase-1`
- **Phase:** 12 — TLV section, UseSitePathOverrides (tag 0x00), Fingerprints (tag 0x01)
- **Spec reference:** §3.7 (TLV section)
- **Status:** DONE

## Summary

Phase 12 lands the TLV (tag-length-value) section that follows the Tag stream
in v0.11 wire format per spec §3.7. Two concrete TLV tag handlers ship in this
phase:

- `0x00` UseSitePathOverrides — sparse list `[(@N-index, UseSitePath)]`,
  ascending by `@N`, non-empty when present.
- `0x01` Fingerprints — sparse list `[(@N-index, [u8; 4])]`, ascending by
  `@N`, non-empty when present.

Tag `0x02` is reserved for the v0.12 Xpubs TLV (not yet implemented). Unknown
TLVs are preserved verbatim in `unknown: Vec<(u8, Vec<u8>, usize)>` for
forward-compatible round-trip behavior.

Wire encoding per entry: `[tag:5 bits][length:LP4-ext varint][payload:length
bits]`, with entries strictly ascending by tag.

## Task 12.1 — commit `215ac02`

`feat(v0.11): TLV section with UseSitePathOverrides and Fingerprints`

Tests added (4):

- `empty_tlv_section_round_trip`
- `use_site_path_override_round_trip`
- `fingerprint_round_trip`
- `ascending_tag_order_enforced_in_encoder`

New error variants (4):

- `TlvOrderingViolation`
- `PlaceholderIndexOutOfRange`
- `OverrideOrderViolation`
- `EmptyTlvEntry`

Helper added: `BitReader::bit_position_for_test` (gated for tests).

## Verification

```
cargo test -p md-codec --lib v11::tlv
```

```
running 4 tests
test v11::tlv::tests::ascending_tag_order_enforced_in_encoder ... ok
test v11::tlv::tests::empty_tlv_section_round_trip ... ok
test v11::tlv::tests::fingerprint_round_trip ... ok
test v11::tlv::tests::use_site_path_override_round_trip ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 545 filtered out
```

Cumulative v11 test count: **65** (61 prior + 4 new).

## Authorized Deviation — TLV decoder loop condition

Decoder loop condition changed from `while r.remaining_bits() > 0` to
`while r.remaining_bits() >= 5`.

**Reason:** `BitWriter` pads to a byte boundary, leaving up to 7 trailing zero
bits at end-of-stream. Without a bit-precise reader, those pad bits cause
spurious tag-read attempts (a 5-bit tag of `0x00` would be misread as an
unintended UseSitePathOverrides entry).

**Scope of workaround:** Acceptable for Phase 12 standalone TLV testing.
Proper resolutions land later in the plan:

- **Phase 14** introduces `BitReader::with_bit_limit` to bound reads to the
  exact bit count carried in the framing, eliminating reliance on byte-padded
  end-of-stream heuristics.
- **Phase 19** adds full rollback semantics for graceful end-of-stream
  handling across all decoders.

This deviation is tracked in the deferred-items list below.

## Deferred Items (carried forward)

Same set as Phase 11, with Phase-12 workaround note appended:

- **P1** — (carried)
- **P2** — (carried)
- **P4** — (carried)
- **P5** — (carried)
- **P7** — (carried)
- **P9** — (carried)
- **Phase 12 workaround:** TLV decoder loop uses `remaining_bits() >= 5`
  rather than bit-exact termination. Bit-bounded reader lands in
  Phase 14 (`BitReader::with_bit_limit`); full rollback semantics land in
  Phase 19.

## Next

Phase 13 — Validation invariants.

## Commit

```
docs(v0.11): phase 12 review report
```

---

DONE — Phase 12 complete at `215ac02`.
