# Spike report — Q11 per-`@N` override unification (v0.30 cycle)

**Topic:** Q11 from `design/FOLLOWUPS.md` `v2-design-questions` entry. Per-`@N` overrides currently use TWO different mechanisms — origin-path divergence via header bit 4 + inline block (dense), use-site-path overrides via TLV tag 0x00 (sparse). Proposal: unify both into one mechanism for consistency.

**Date:** 2026-05-10. Verdict: **DO NOT UNIFY (Q11 reclassified to wont-fix).**

**Resulting SPEC section:** `design/SPEC_v0_30_wire_format.md` §6 (Per-`@N` override encoding — intentionally not unified).

## Findings

1. **Framing confirmed against current code:** `crates/md-codec/src/header.rs:20` carries header-bit-4 divergent-origin-paths flag (dense inline block); `crates/md-codec/src/tlv.rs:11` carries `UseSitePathOverrides` (TLV tag 0x00, sparse).

2. **Corpus patterns:** origin-path divergence is rare in practice (1 vector, `wsh_divergent_paths`). Use-site overrides are sparse (typically 0–1 of n).

3. **TLV-for-both candidate** (move divergent-origin-paths to a new TLV tag, eliminating header bit 4): loses ~9 bits per encoding (5-bit tag + 4-bit length varint) vs the saved 1-bit header slot. **Net +8 bits per full-divergence encoding.** TLV-for-both is a wire-size loss in exchange for unification.

4. **Inline-for-both candidate** (move use-site overrides into a dense path-decl block): loses ~6–12 bits per sparse pattern (dense slots vs sparse TLV). Inline-for-both is also a wire-size loss in the common sparse-override case.

5. **Decoder simplicity:** current bifurcated design has no inter-field ordering constraints; TLV-for-both adds tag-order dependency for origin paths.

## Verdict

**DO NOT UNIFY.** The asymmetric design is structurally optimal: origin paths are dense BIP-32 identity → header bit; use-site overrides are sparse cosmetic → TLV. The Q11 framing in the original `v2-design-questions` catalog reflected an aesthetic preference for unification rather than an empirical efficiency case.

Q11 reclassified from "live design space" to **wont-fix in `design/FOLLOWUPS.md`**, with this spike report as the rationale. The current bifurcated design carries forward into v0.30 unchanged.
