# Phase 19 Review — Codex32 BCH wiring + BitReader save/restore + TLV rollback

- **Date:** 2026-04-30
- **Branch:** `feature/v0.11-impl-phase-1`
- **HEAD:** `133a306`
- **Phase:** 19 (Codex32 BCH wiring symbol-aligned + BitReader save/restore + TLV rollback)

## Status: DONE

## Scope

- **Task 19.1** (commit `39c26bc`): `v11::codex32` module — symbol-aligned `wrap_payload` + `unwrap_string` returning `(bytes, symbol_aligned_bit_count)`. Bypasses v0.x's `encode_string`/`decode_string`; uses lower-level `bch_create_checksum_regular` / `bch_verify_regular` directly. 4 tests including BIP 84 single-sig 28-char round-trip and N=3 chunk byte-count recovery.
- **Task 19.2** (commit `133a306`): `BitReader::save_position` / `restore_position`; `TlvSection::read` rewritten with rollback semantics; reverts the Phase 12 `>= 5` workaround. 1 new bitstream test.

## Verification (cargo test, evidence before assertions)

| Suite | Expected | Actual |
|---|---|---|
| `v11::codex32` | 4 | 4 PASS |
| `v11::bitstream` | 9 (was 8) | 9 PASS |
| `v11` (lib cumulative) | 93 | 93 PASS |
| `--test v11_smoke` | 3 | 3 PASS |
| `--test v11_forward_compat` | 1 | 1 PASS |
| **Total v11** | **97** | **97 PASS** |

## Major milestones

1. **Codex32 wiring complete.** v0.11 produces real `md1...` codex32 strings — BIP 84 single-sig is 28 chars. Symbol-aligned packing saves ~1 char per encoding versus byte-alignment, per spec §3.1 (md1 string layout: HRP | payload | BCH checksum).
2. **TLV rollback operational.** Phase 12's `>= 5` heuristic workaround is gone, replaced by proper `BitReader` save/restore semantics. Tolerates up to 7 bits of trailing zero-padding. End-of-stream phantom-tag scenarios (e.g., trailing zeros parsing as `tag=0, length=0` after a valid `tag=1` entry) are handled correctly via the rollback path. Aligns with spec §3.7 (TLV section) and brainstorm §4f S5-2 (no payload-bit-count communication; encoder deterministic; rollback for end-of-stream).
3. **Carry-forward P12 RESOLVED.** TLV decoder loop is now the proper bit-precise version.

## Wire-format properties locked

- HRP `"md"`, separator `"1"`, BCH polynomial constants from v0.x's `MD_REGULAR_CONST` / `MD_LONG_CONST` (still SHA-256-derived from `"shibbolethnums"`).
- No leading sentinel byte (D38) — per brainstorm §4f S5-1.
- Symbol-aligned bit count communicated via the codex32 layer; `unwrap_string` returns `5 × data_symbol_count` as the precise bit count (no bit-count field in the wire format, per §4f S5-2).
- `Md1EncodingId = SHA-256(payload_bytes)[0..16]` — over byte-padded payload bytes only.

## Carry-forward deferred items (post-P12 resolution)

- **P1:** `read_past_end_errors` state-preservation; unused `BitStreamExhausted`.
- **P2:** `write_varint` `debug_assert`→`assert`; L=0 hand-crafted test.
- **P4:** `PathDecl::write` `# Errors` doc gap.
- **P5:** `UseSitePath::write` `# Errors` doc gap.
- **P12:** RESOLVED in P19 (TLV rollback shipped).
- **P13a:** `ForbiddenTapTreeLeaf` Debug formatting.
- **P13b:** placeholder bounds `debug_assert`.

## CONCERNS

None. All test counts match plan exactly; the P12 carry-forward is closed by a principled mechanism rather than a heuristic.

## CONTEXT

- Phase 19 is the final infrastructure phase before user-facing top-level codecs. With `wrap_payload`/`unwrap_string` and rollback-capable TLV decoding both shipped, the encode/decode pipelines now have all primitives needed.
- BIP 84 single-sig 28-char round-trip is a concrete proof point that wire-format §3.1 is honored end-to-end including BCH checksum.

## BLOCKED

Nothing.

## Next

**Phase 20:** `encode_md1_string` + `decode_md1_string` — top-level public API composing payload codecs with the codex32 wrapper.
