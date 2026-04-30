# v0.11 Phase 14 Review — End-to-End Encoder + Decoder

**Date:** 2026-04-30
**Branch:** `feature/v0.11-impl-phase-1`
**Phase:** 14 — End-to-end encoder + decoder
**Status:** DONE

## Scope

Phase 14 wires the previously-built building blocks (header, path-decl, use-site-paths, tree, TLV) into a top-level `Descriptor` value with `encode_payload` / `decode_payload` entry points, and proves end-to-end round-trip with smoke tests for the two flagship descriptor shapes.

Spec citations:
- §3.2 — payload structure (Header → PathDecl → UseSite → Tree → TLV)
- §13.2 — decoder algorithm
- §13.3 — encoder algorithm

## Commits

| Task | Commit  | Description |
|------|---------|-------------|
| 14.1 | `e1ce971` | `BitReader::with_bit_limit` for exact payload-length decode |
| 14.1 | `4a681ca` | `Descriptor` type + `encode_payload` returning `(bytes, total_bits)` |
| 14.2 | `0ac0a96` | `decode_payload` + bip84 round-trip + 57-bit count smoke test |
| 14.3 | `6d43450` | BIP 48 2-of-3 sortedmulti round-trip smoke test |

## Verification

```
$ cargo test -p md-codec --lib v11::bitstream
test result: ok. 8 passed; 0 failed; 0 ignored

$ cargo test -p md-codec --test v11_smoke
test result: ok. 3 passed; 0 failed; 0 ignored
  - bip84_single_sig_round_trip
  - bip84_single_sig_bit_count
  - bip48_2of3_sortedmulti_round_trip
```

Cumulative v11 test count: **73 lib + 3 smoke = 76** (bitstream module went 7 → 8 with the `with_bit_limit` test added in 14.1).

## Bit-Cost Confirmation

BIP 84 single-sig payload measures **57 bits**, decomposing as:

| Section    | Bits | Source |
|------------|------|--------|
| Header     | 5    | §3.2 (3-bit version + 2 mode flags) |
| PathDecl   | 31   | path dictionary entry + child encoding |
| UseSite    | 16   | single use-site path |
| Tree       | 5    | leaf-only descriptor tree |
| TLV        | 0    | empty (no optional sections) |
| **Total**  | **57** | matches `encode_payload` second return value |

Empty-TLV handling validated end-to-end: `BitReader::with_bit_limit` plus `bit_position` tracking allows the decoder to terminate cleanly when payload bits are exhausted, without a sentinel tag.

## Major Milestone

**End-to-end encoder/decoder operational.** Both single-sig (BIP 84) and 2-of-3 multisig (BIP 48 sortedmulti) descriptors fully round-trip through `encode_payload` → bytes → `decode_payload` with structural equality.

This closes the core wire-format implementation arc that started in Phase 1. The remaining phases layer identity, ECC, and HRP framing on top of a payload codec that is now self-contained and testable.

## Carry-Forward Deferred Items

- **P1:** `read_past_end` error state-preservation; unused `BitStreamExhausted` variant.
- **P2:** `write_varint` `debug_assert` → `assert`; hand-crafted `L=0` test.
- **P4:** `PathDecl::write` `# Errors` rustdoc gap.
- **P5:** `UseSitePath::write` `# Errors` rustdoc gap.
- **P7:** RESOLVED in P14 — multi-arity dispatch is now exercised by the BIP 48 2-of-3 smoke test.
- **P12:** TLV decoder `>= 5` workaround can now be reverted given `with_bit_limit` is wired in. Defer to Phase 19 cleanup.
- **P13a:** `ForbiddenTapTreeLeaf` Debug formatting polish.
- **P13b:** Placeholder bounds `debug_assert` audit.

## CONCERNS

None blocking. P12 is a known-stale workaround that should be cleaned up in Phase 19 once the rest of the wire format stabilizes; not in scope for Phase 14.

## CONTEXT

- Header bit allocation finalized in `c30037b` (3-bit version + 2 mode flags) is what makes the 5-bit header line in the bit-cost table stable.
- `BitReader::with_bit_limit` is the structural fix that the §13.2 decoder pseudocode requires to terminate on byte-padded payloads without a length-prefixed TLV envelope.

## BLOCKED

None.

## Next

**Phase 15 — Identity:** `Md1EncodingId`, `WalletDescriptorTemplateId`, BIP-39 phrase derivation.

---

Reviewed commit: `6d43450`
