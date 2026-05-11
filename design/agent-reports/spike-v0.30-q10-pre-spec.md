# Spike report — Q10 in-band discriminator + header layout (v0.30 cycle)

**Topic:** Q10 from `design/FOLLOWUPS.md` `v2-design-questions` entry. v0.x payload header (5 bits: `[paths:1][reserved=0:1][version:3]`) and chunk header first 5 bits (`[version:3][chunked=1:1][reserved=0:1]`) place version and discriminator at different bit positions across modes — defeating in-band auto-dispatch. v0.x decode requires a caller hint (single vs chunked).

**Date:** 2026-05-10. Verdict: **PROPOSED-LAYOUT-OK** (with version-value lock refined post-architect-review; see below).

**Resulting SPEC section:** `design/SPEC_v0_30_wire_format.md` §2 (Header layout).

## Findings

1. **Framing confirmed against current code:** `crates/md-codec/src/header.rs:1-5` payload header is `[paths:1][reserved=0:1][version:3]` (bits 4..0); `crates/md-codec/src/chunk.rs:23-48` chunk header first 5 bits are `[version:3][chunked=1:1][reserved=0:1]`. Bit positions misaligned across modes; current dispatch is API-routed (caller hint), not in-band.

2. **Proposed v0.30 single-payload header (5 bits):** `[paths:1][v3:1][v2:1][v1:1][v0:1]` — bits 4..0.

3. **Proposed v0.30 chunk header first symbol:** `[v3:1][v2:1][v1:1][v0:1][chunked:1]` — bits 4..0. **Mode discriminator at bit 0** (consistent and unambiguous; round-1 spike notes said bit 1 — corrected to bit 0 in SPEC §2.1/2.2 after architect-review caught the discriminator-collision concern).

4. **Version field:** 4 bits (16 representable values) — **absorbs SW1** (reserved bit folded into version) and **achieves Q8** (4-bit version up from current 3-bit) in one change.

5. **Auto-dispatch:** decoder reads first 5-bit symbol's bit 0; if 1 → chunked mode (read 32 more bits); if 0 → single-payload mode. Works without caller hint.

6. **Safe v0.x rejection:**
   - v0.x single-payload (version=0) → first-symbol bits `[paths][0][0][0][0]`; v0.30 reads bit 0 = 0 → single-payload; version bits 3..0 = `0000` = 0 → `WireVersionMismatch { got: 0 }`.
   - v0.x chunked (version=0, chunked=1) → first-symbol bits `[0][0][0][1][0]`; v0.30 reads bit 0 = 0 → single-payload; version bits 3..0 = `0010` = **2** → `WireVersionMismatch { got: 2 }`.

7. **Version value selection (refined post-architect-review):** v0.30 uses **`version = 4`**. The auto-dispatch design constrains usable values to those with `v0 = 0` (else single-payload misclassifies as chunked). Additionally, version=0 collides with v0.x single-payload; version=2 collides with v0.x chunked-misread. **Usable subset: {4, 8, 12}.** v0.30 uses 4; future major breaks would use 8 then 12. After 12 is consumed, the next break requires a format-layer change (e.g., widening the version field to 5 bits). The 3-version lifetime is intentional pre-alpha discipline.

## Verdict

**LAYOUT-OK.** Adopted into v0.30 SPEC §2 with the version-value refinement (`version = 4`; usable set {4, 8, 12}). Q10 is also a **prerequisite** for the f.1 clean-break safety (without an in-band wire-version discriminator at a known bit position, a v0.30 decoder fed an v0.x payload could silently mis-decode rather than cleanly reject).

Implementation lands in Phase B per `design/IMPLEMENTATION_PLAN_v0_30.md`. Phase B's stop condition includes 2 explicit v0.x-fed rejection tests (single-payload + chunked-misread), grounded by SPEC §2.5's auto-dispatch trace.
