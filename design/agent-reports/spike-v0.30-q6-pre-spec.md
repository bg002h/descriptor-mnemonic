# Spike report — Q6 BCH polynomial separation (v0.30 cycle)

**Topic:** Q6 from `design/FOLLOWUPS.md` `v2-design-questions` entry. v0.x reuses BIP-93's BCH polynomial across md1, mk1, ms1, with per-HRP target residues for cross-format domain separation. Proposal: each format adopts a distinct BCH polynomial for stricter formal cryptographic separation.

**Date:** 2026-05-10. Verdict: **DEFER (Q6 reclassified to v3+ or wont-fix).**

**Resulting SPEC section:** `design/SPEC_v0_30_wire_format.md` §10 (BCH polynomial layer — unchanged from v0.11).

## Findings

1. **Status quo confirmed against current code:** `crates/md-codec/src/bch.rs:7-17` uses shared BIP-93 polynomial (`GEN_REGULAR[5]`) with per-HRP target residue `MD_REGULAR_CONST` for md1; mk1's parallel implementation uses `MK_REGULAR_CONST`. Both target residues derive from `SHA-256("shibbolethnums")` with distinct bit extraction.

2. **Formal security gain of distinct polynomials:** would harden cross-format polynomial-collision attack from ~2^-32.5 per attempt (birthday on 65-bit residues with shared polynomial structure) to ~2^-65 (independent polynomials). This is **out-of-scope for the hand-transcription threat model** where user errors (transcription mistakes) dominate over cryptographic attacks.

3. **Cross-repo coordination burden of shipping Q6:**
   - `md-codec` ~100 LOC change (factorize BCH module per-format)
   - `mk-codec` mandatory coordinated change (companion BIP submission, polynomial spec amendment, conformance vector regen)
   - BIP draft contradicts current "reuse codex32's reviewed cryptography" design principle (per `bip/bip-mnemonic-descriptor.mediawiki:230-231`)
   - All conformance vectors invalidated; family-stable SHA token rolls

4. **Threat model reality check:** Per-HRP residue derivation produces residues that differ between any two HRPs sharing this polynomial (BIP-173 `hrp_expand` produces distinct expansion for `"md"` vs `"mk"`). Cross-format confusion attacks are mitigated by HRP-mismatch rejection at the codex32 layer, distinct residues at the BCH layer, and error-correction bounds (≤4 substitutions correctable).

## Verdict

**DEFER.** Per-HRP-residue + HRP-mixing provides adequate domain separation for the v0.x and v0.30 hand-transcription threat model. Cross-repo burden of shipping Q6 outweighs the formal cryptographic gain at this threat tier.

Q6 reclassified in `design/FOLLOWUPS.md`: **not shipped in v0.30**; revisit if a future major version (e.g., v3+) contemplates cryptographic-rigor as a priority and the cross-repo coordination is otherwise on the roadmap. v0.30 SPEC §10 documents the unchanged BCH layer.
