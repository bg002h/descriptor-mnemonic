# Spike report — Q9 multi child packing (v0.30 cycle)

**Topic:** Q9 from `design/FOLLOWUPS.md` `v2-design-questions` entry. Multi-family children currently encoded as full Node = `Tag::PkK` + `key_index` (kiw bits). Proposal: drop the per-child tag and encode children as raw `kiw`-bit indices only.

**Date:** 2026-05-10. Verdict: **SHIP.**

**Resulting SPEC section:** `design/SPEC_v0_30_wire_format.md` §4 (Multi child packing).

## Findings

1. **Framing confirmed against current code:** `crates/md-codec/src/tree.rs:78-92` encodes multi children as full Node = `Tag::PkK` (5 bits) + `key_index` (kiw bits). Tag is redundant since multi-family operators (`multi`, `sortedmulti`, `multi_a`, `sortedmulti_a`) imply all children are keys.

2. **Corpus savings (existing test vectors):** 85 bits saved across 7 multi-shape vectors (`wsh_multi_2of2`, `wsh_multi_2of3`, `wsh_multi_chunked`, `wsh_sortedmulti`, `sh_wsh_multi`, `wsh_divergent_paths`, `wsh_with_fingerprints`).

3. **Realistic large multisigs (not in corpus):** 7-of-11 saves 55 bits; 11-of-15 saves 75 bits. A single large multisig dwarfs the existing corpus aggregate. The hardware-multisig use case is the headline beneficiary.

4. **Decoder cost:** localized tag-conditional branch in `read_node` (if parent ∈ {`Multi`, `SortedMulti`, `MultiA`, `SortedMultiA`}, skip child-tag reads). No new field beyond existing `k-1`, `n-1`. Negligible complexity addition.

5. **AST consequence:** `Body::Variable` for multi-family transitions to `Vec<u8>` key indices; `Thresh` retains `Vec<Node>`. SPEC §4.1 documents this split as `Body::MultiKeys { k, indices }` vs `Body::Variable { k, children }`.

## Verdict

**SHIP.** Average savings ≥ 1 codex32 character per shape threshold cleared. Adopted into v0.30 SPEC §4. Implementation lands in Phase C per `design/IMPLEMENTATION_PLAN_v0_30.md`.
