# v0.11 Phase 15 Review — Identity & BIP-39 Phrase Rendering

**Date:** 2026-04-30
**Branch:** `feature/v0.11-impl-phase-1`
**Commits reviewed:**
- `9d9d109` feat(v0.11): Md1EncodingId computation (Task 15.1)
- `ac9eb71` feat(v0.11): WalletDescriptorTemplateId (γ-flavor) (Task 15.2)
- `0173733` feat(v0.11): BIP-39 12-word phrase rendering (Task 15.3)

## Scope

Phase 15 lands the v0.11 identity surface and human-facing phrase rendering. Three of the four spec-defined identifiers in §8 are addressed; the fourth (`WalletPolicyId`, §8.3) is explicitly deferred to v0.12 because it depends on the Xpubs TLV (D31′ + D34) not yet wired into the v0.11 payload.

## Task 15.1 — `Md1EncodingId` (§8.2)

`Md1EncodingId` is defined as the 16-byte (128-bit) prefix of `SHA-256` over the **full canonical wire bytes** of an md1 encoding (HRP-mixed BCH context, header + payload + TLV after canonicalization). It is engraving-specific: any change to header bits, payload routing, fingerprints, use-site-path overrides, or chunking representation produces a fresh ID. This is the right granularity for de-duping or referencing a particular engraved artifact.

The two tests cover:
1. **Determinism** — the same canonical wire bytes hash to the same ID across invocations.
2. **Path-sensitivity** — diverging origin paths between two encodings produces distinct IDs (sanity check that the hash domain truly is the full wire payload, not a content-restricted subset).

This complements §8.1 — `Md1EncodingId` is the identity that *changes* with engraving choices, paired with `WalletDescriptorTemplateId` which *does not*.

## Task 15.2 — `WalletDescriptorTemplateId` (§8.1, γ-flavor)

The γ-flavor of `WalletDescriptorTemplateId` is the shape-identifying hash: `SHA-256` over BIP 388 template content only — specifically the use-site-path-decl structure, the descriptor tree, and the `UseSitePathOverrides` TLV bits — explicitly excluding origin-path declarations and master-key fingerprints. This captures "is this the same wallet template?" while remaining invariant to:

- **Origin-path divergence** (test: `wdt_id_invariant_to_origin_path_change`) — different signers with different BIP 32 derivation roots still resolve to the same template ID.
- **Fingerprint addition** (test: `wdt_id_invariant_to_fingerprint_addition`) — adding the optional Fingerprints TLV does not perturb the template hash.
- **Use-site-path divergence** (test: `wdt_id_differs_for_different_use_site_paths`) — confirms the negative direction: when use-site-path-decl actually differs, IDs differ. This guards against accidentally over-broadening the invariance window.

The γ-flavor choice (over α/β alternatives discussed in §8.1) keeps the ID stable across the largest class of cosmetic engraving variation while still distinguishing genuinely different wallet shapes (different `@N` use-site policies, different tap-script-tree topology, etc.).

## Task 15.3 — BIP-39 phrase rendering (§8.4)

The `Phrase` type wraps `[String; 12]` and is constructed via `Phrase::from_id_bytes(&[u8; 16])`, which feeds the 128-bit ID into the `bip39` crate's `Mnemonic::from_entropy` (English word list, default). The `Display` impl produces a single space-separated 12-word string suitable for human transcription or verbal exchange.

This gives a one-way human projection of `Md1EncodingId` (or `WalletDescriptorTemplateId`, or — once it lands in v0.12 — `WalletPolicyId`) that fits within the 128-bit BIP-39 entropy bracket without ambiguity. Three tests cover word-count (12), determinism for a fixed input, and the space-separated `Display` formatting.

## §8.3 deferral — `WalletPolicyId`

`WalletPolicyId` is intentionally not implemented in v0.11. Per the spec, it requires the Xpubs TLV (per D31′ and D34) to canonicalize the policy-with-keys representation. Phase 11 shipped `UseSitePathOverrides` and `Fingerprints` TLVs, but the Xpubs TLV is on the v0.12 docket. Implementing a stub now would risk locking in a hash domain that changes once Xpubs lands, so deferral is the correct call.

## Test confirmation

```
$ cargo test -p md-codec --lib v11::identity
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 560 filtered out

$ cargo test -p md-codec --lib v11::phrase
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 562 filtered out

$ cargo test -p md-codec --lib v11
test result: ok. 81 passed; 0 failed; 0 ignored; 0 measured; 484 filtered out
```

**Note on cumulative count:** the prompt anticipated 84 (76 prior + 8 new). Observed cumulative is **81** (73 prior + 8 new). The +8 new — 5 identity, 3 phrase — matches expectations exactly, so the discrepancy is in the prior-count baseline carried forward (the Phase 14 review's +N tally vs. the actual filter for `v11::*` modules), not in Phase 15 itself. All Phase 15 tests pass.

## Carried-forward deferred items

- **P1:** `read_past_end_errors` state-preservation; unused `BitStreamExhausted` variant.
- **P2:** `write_varint` `debug_assert!` should harden to `assert!`; `L=0` hand-crafted decode test.
- **P4:** `PathDecl::write` `# Errors` doc gap.
- **P5:** `UseSitePath::write` `# Errors` doc gap.
- **P12:** TLV decoder `>= 5` workaround — likely revertable now that `BitReader::with_bit_limit` (Phase 13) is operational; explicit re-evaluation deferred.
- **P13a:** `ForbiddenTapTreeLeaf` `Debug` formatting polish.
- **P13b:** placeholder bounds `debug_assert!`.

No new deferrals introduced in Phase 15.

## Forward look — Phase 16 (chunking)

Phase 16 introduces `ChunkSetId` (a 20-bit truncated derivation from `Md1EncodingId`, sized to the codex32 chunk-set-identifier field) and `ChunkHeader` (37 bits). These are the metadata-only pieces; the actual split/reassemble against codex32 lives in Phase 21. Pulling `ChunkSetId` derivation forward into Phase 16 is sensible — it's a pure function of `Md1EncodingId` (now landed) and unblocks header layout work without waiting on the chunking codec itself.

## Verdict

**DONE.** All three tasks land cleanly with TDD discipline; the §8.3 deferral is well-motivated; the γ-flavor invariance properties for `WalletDescriptorTemplateId` are exercised in both directions (invariance and sensitivity). Ready to proceed to Phase 16.
