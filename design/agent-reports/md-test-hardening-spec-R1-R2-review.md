# R1 + R2 fold-verify — `SPEC_md_codec_test_hardening.md`

## R1 (sonnet) — RED 0C/2I/1M (all fold-introduced from R0's folds)
- **I-R1-A:** §3.1 templated-shapes bullet still said `n∈1..=8` for wsh(multi/sortedmulti) while Varied-params said `n∈1..=32` (I3 fold updated one bullet, missed the sibling) → defeats the kiw=4→5 goal. FOLDED → `n∈1..=32`.
- **I-R1-B:** §3.2 P2 still used the type-incorrect nested `decode_payload(encode_payload(d))` (M4 fixed P1, missed P2). FOLDED → destructured `(bytes,total_bits)`.
- **M-R1-a:** §10 still cited `canonicalize.rs:168,954` (M1 fixed §2.1 only). FOLDED → `:955`.
Confirmed correct: I1 (decode_payload debug-assert pin), I2 (placeholder-set-derived n + ≥1 key-bearing leaf per @i), I3 (n∈1..=32 vs MAX_PATH_COMPONENTS=15 distinct; kiw=4→5 at n=16→17), I4 (#![allow(dead_code)]), M2 (T3c broad Codex32DecodeError — bch_verify precedes too-short), M3 (position-0 note), M4 (P1 destructure).

## R2 (sonnet) — GREEN 0C/0I
All 3 R1 folds confirmed: no stale `1..=8`, no nested `decode_payload(encode_payload(...))` in prescriptive position (only the prohibition note), no `:954`. §3.1 bullets agree on n-range; P2 matches P1's destructure; headline calls (canonical-fixpoint P1, `!= Ok(original)` T2c) intact. No fold-introduced contradictions.

## Gate: R0 RED 0C/4I/4M → R1 0C/2I/1M → R2 GREEN 0C/0I. Cleared for the implementation plan.
