# Spike report — Q13 tag-space rework (v0.30 cycle)

**Topic:** Q13 from `design/FOLLOWUPS.md` `v2-design-questions` entry. The 5-bit primary bytecode tag space is exhausted in v0.18 (every slot 0x00–0x1E allocated; 0x1F is the extension prefix; 5 of 32 extension slots used). Future operators must always pay the 5-bit extension prefix overhead. Proposal: pick a larger primary space.

**Date:** 2026-05-10. Verdict: **Candidate B (6-bit primary + 4-bit extension subspace).**

**Resulting SPEC section:** `design/SPEC_v0_30_wire_format.md` §3 (Bytecode tag space + encoding uniformity).

**Subsequent revisit:** Phase 3.6 lock split the tag space — bytecode tags adopt Q13 Candidate B (6-bit primary), TLV section tags retained at v0.x 5-bit width. See SPEC §3.1.

## Findings

1. **Tag inventory:** 31 primary slots (0x00–0x1E) all allocated in v0.18; 5 extension slots used (Hash256, Ripemd160, RawPkH, False, True); 27 extension slots free. Net 0 new operators across v0.12–v0.18 (7 minor releases) — operator addition is rare but the existing exhaustion blocks any cheap future addition.

2. **5-year operator ceiling (conservative):** ~41 total operators (current 36 + likely additions: ≤5 decorator/wrapper variants, ≤3 BIP-additive operators, plus framing/escape). Hardware-signer realistic max well within fixed-5's range, but the 6-bit space buys headroom for the broader miniscript / BIP universe.

3. **Candidate evaluation:**
   - **Candidate A (status quo + restructure ext subspace, e.g., 8-bit extension):** +3 bits per extension tag (13 vs 10); decoder gains a single branch but incurs wire cost on every extension-space operator usage.
   - **Candidate B (6-bit primary + 4-bit extension):** 64 + 16 = 80 equivalent slots; **2.5× headroom over the 5-year ceiling**; neutral wire cost (single 6-bit read, no "if extension" branch penalty); simplest decoder. 0x20–0x3F free for future operators.
   - **Candidate C (8-bit flat):** +3 bits per operator × ~30 operators per descriptor ≈ +90 bits per structure. Wasteful.
   - **Candidate D (context-sensitive width):** premature optimization; rejected.

4. **Promoted operators:** Hash256, Ripemd160, RawPkH, False, True move from extension subspace into primary slots 0x1F–0x23. Saves 4 bits per usage (was 10-bit extension; now 6-bit primary).

## Verdict

**Candidate B.** Adopted into v0.30 SPEC §3.1–3.2.

Phase 3.6 split the tag space further: TLV section tags retain v0.x 5-bit width (preserves wire-size for high-TLV use cases per Phase 3.5b empirical data; TLV tags grow only by 0 bits, eliminating the +1/TLV-tag cost that dominated Mode B/C overhead). Implementation lands in Phase A per `design/IMPLEMENTATION_PLAN_v0_30.md`.
