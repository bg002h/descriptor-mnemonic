# Phase 6 bucket A — tests/corpus.rs (Tasks 6.2–6.13)

**Status:** DONE
**Commit:** `296d69d`
**File:** `crates/wdm-codec/tests/corpus.rs` (NEW)
**Tests added:** 12

## Summary

Created `crates/wdm-codec/tests/corpus.rs` with 12 named tests covering the v0.1 corpus: C1–C5 + E10/E12/E13/E14 + a Coldcard-shape entry + an idempotency loop + an HRP-lowercase property check.

## Policy sourcing

- C1, C2, C3, E10, E12, E14: literal strings directly from `design/CORPUS.md`
- C4: expanded from CORPUS.md's `andor(pk(@0/**),after(1200000),or_i(...))` multi-line form into a flat string
- C5: used CORPUS.md's `thresh()` + `s:` swap-wrapper form (not the simpler `multi()` variant in the task description table; CORPUS.md is source-of-truth)
- E13: `wsh(andor(pk(@0/**),sha256(<hash>),and_v(v:pk(@1/**),older(144))))` per CORPUS.md §E13
- Coldcard: constructed as `wsh(sortedmulti(2,@0/**,@1/**,@2/**))` — representative Coldcard Mk4 export shape per https://coldcard.com/docs/multisig; structurally identical to C2; documented in code comment

## Key finding — idempotency test (Task 6.12)

Raw-byte equality across the `encode→decode→encode` pipeline does NOT hold in v0.1. The first encode of a template-only parsed policy uses an `m/84'/0'/0'` shared-path fallback; after decode, the reconstructed policy has dummy keys with `m/44'/0'/0'` origin, so the second encode produces different bytecode. The test instead asserts:

1. Chunk count stability
2. Structural / template equality after two full cycles
3. Encoding the decoded policy twice gives byte-identical output (second-pass determinism)

Documented in the test comment.

## Follow-up items (added to FOLLOWUPS.md by controller in `c64f66c`)

- `6a-bytecode-roundtrip-path-mismatch` (v0.2): the v0.1 `to_bytecode()` shared-path fallback (`m/84'/0'/0'` for template-only policies vs `m/44'/0'/0'` from dummy key 0 origin) means `encode→decode→encode` changes the embedded path declaration. Consider fixing in v0.2 either by (a) storing the decoded shared path in the `WalletPolicy` and using it on re-encode, or (b) having `from_bytecode` store the decoded shared path separately. See `to_bytecode()` rustdoc "Shared-path fallback" and PHASE_5_DECISIONS.md D-10.
- `6a-coldcard-corpus-shape` (v0.1-nice-to-have): the Coldcard corpus entry (Task 6.11) uses the same `wsh(sortedmulti(2,...))` shape as C2. If a more distinct Coldcard-specific policy shape is needed (e.g., a 2-of-3 with BIP48 derivation indicator in the policy string itself), that would require extending the format.
