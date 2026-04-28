# Phase 6 — Test Corpus Expansion: Implementer Report

**Date:** 2026-04-27
**Branch:** `feature/v0.4-bip388-modern-surface`
**Phase:** 6 of design/IMPLEMENTATION_PLAN_v0_4_bip388_modern_segwit_surface.md

## Status: COMPLETE

All 4 gates green: build, test, clippy, fmt.

## Test Count

- Baseline (after Phase 5): 594 passing
- After Phase 6: **607 passing + 2 ignored** (+13 net; breakdown below)
- The 2 ignored tests: `v0_2_sha256_lock_matches_committed_file` and
  `committed_v0_2_json_matches_regenerated_if_present` — both pending Phase 7 regen.

## Files Modified

| File | Changes |
|------|---------|
| `crates/md-codec/src/vectors.rs` | +V0_4_DEFAULT_FIXTURES (5 entries), +build_v0_4_fingerprints_vectors (S2/S4/M3), +build_negative_v0_4_sh_matrix (9 negative vectors), updated build_positive_vectors_v2 and build_negative_vectors_v2 |
| `crates/md-codec/tests/conformance.rs` | +bitcoin::bip32::Fingerprint import, +6 hostile-input tests (Task 6.4), +8 round-trip tests (Task 6.5) |
| `crates/md-codec/tests/vectors_schema.rs` | +schema_2_contains_v0_4_corpus_additions test, updated build_test_vectors_has_expected_corpus_count (10→22 positive, >=18→>=27 negative), #[ignore] on both v0.2.json-dependent tests |

## Task 6.1: Positive Fixtures (8 added to schema-2)

Added as two arrays:
- `V0_4_DEFAULT_FIXTURES` (5 entries, no fingerprints block): s1_wpkh, s3_sh_wpkh, m1_sh_wsh_sortedmulti_1of2, m2_sh_wsh_sortedmulti_2of3, cs_coldcard_sh_wsh
- `build_v0_4_fingerprints_vectors()` (3 entries with fingerprints block): s2_wpkh_fingerprint, s4_sh_wpkh_fingerprint, m3_sh_wsh_sortedmulti_2of3_fingerprints

Cs cites Coldcard firmware 5.4.0 in provenance comment per spec. Schema-2 total: 22 positive vectors.

Note: S2/S4/M3 with "key-origin fingerprints" use `EncodeOptions::with_fingerprints` (the WDM fingerprints block mechanism), NOT embedded key-origin strings in the policy — because `[fingerprint/path]@i/**` is not a valid WDM policy string (rust-miniscript rejects it at parse time). The schema-1 fixture set remains 10 (unchanged).

## Task 6.2: Negative Fixtures (9 added as schema-2-only)

Added via `build_negative_v0_4_sh_matrix()` producing 9 `NegativeVector` entries with MD strings synthesized by hand-rolling bytecode. All 9 carry provenance strings. Schema-2 total: 39 negative vectors (30 pre-v0.4 + 9 new).

The 9 vectors: n_sh_multi, n_sh_sortedmulti, n_sh_pkh, n_sh_tr, n_sh_bare, n_sh_inner_script, n_sh_key_slot, n_top_pkh, n_top_bare.

Design decision: these are schema-2-only (not added to NEGATIVE_FIXTURES), mirroring the taproot/fingerprints precedent. The NEGATIVE_FIXTURES array stays at 30 entries for schema-1 stability.

## Task 6.3: Encode-Side Restriction Tests

ALL 5 ALREADY COVERED by Phase 1's `encode_rejects_*` tests in `crates/md-codec/src/bytecode/encode.rs`:
- `encode_rejects_sh_multi_legacy_p2sh` → enc_sh_multi
- `encode_rejects_sh_sortedmulti_legacy_p2sh` → enc_sh_sortedmulti
- `encode_rejects_top_level_pkh` → enc_top_pkh
- `encode_rejects_top_level_bare` → enc_top_bare
- `encode_rejects_sh_via_inner_ms_arbitrary_miniscript` → enc_sh_via_inner_ms

No duplicates added to conformance.rs. Task complete per "note if covered by Phase 1" instruction.

## Task 6.4: Hostile-Input Tests (6 added to conformance.rs)

- `rejects_sh_recursion_bomb` — 100 Sh tags via from_bytecode; rejects at depth 1 (PolicyScopeViolation)
- `rejects_sh_recursion_minimal` — [Sh][Sh][Wpkh][Placeholder][0]; immediate rejection
- `rejects_wpkh_trailing_bytes` — valid wpkh + 0xFF; InvalidBytecode(TrailingBytes)
- `rejects_sh_wpkh_trailing_bytes` — valid sh(wpkh) + 0xFF; InvalidBytecode(TrailingBytes)
- `rejects_sh_wpkh_non_placeholder` — Sh→Wpkh→Wsh (non-placeholder key slot); PolicyScopeViolation mentioning "Placeholder"
- `rejects_sh_inside_wsh_andv` — [Wsh][AndV][Sh][...]; PolicyScopeViolation or InvalidBytecode

**Finding for H5:** the decoder emits `PolicyScopeViolation("expected Tag::Placeholder, got Wsh at offset N")` (not `InvalidBytecode { kind: UnexpectedTag }`) when a non-placeholder tag appears in the key-slot position after Sh→Wpkh admission. Test updated to assert PolicyScopeViolation with "Placeholder" in the message, which provides the "distinct diagnostic" requirement.

## Task 6.5: Round-Trip Tests (8 added to conformance.rs)

8 tests: round_trip_s1_wpkh, round_trip_s2_wpkh_fingerprint, round_trip_s3_sh_wpkh, round_trip_s4_sh_wpkh_fingerprint, round_trip_m1_sh_wsh_sortedmulti_1of2, round_trip_m2_sh_wsh_sortedmulti_2of3, round_trip_m3_sh_wsh_sortedmulti_2of3_fingerprints, round_trip_cs_coldcard_sh_wsh.

S1/S3/M1/M2/Cs delegate to `common::round_trip_assert`. S2/S4/M3 use EncodeOptions::with_fingerprints and compare via `common::assert_structural_eq`.

## Task 6.6: Infrastructure Tests

- `build_test_vectors_has_expected_corpus_count`: positive bumped 10 → 22 (schema-2), negative >= 18 → >= 27 (schema-2). Schema-1 counts unchanged.
- Added `schema_2_contains_v0_4_corpus_additions` test checking all 8 positive IDs + all 9 negative IDs + fingerprints metadata on S2/S4/M3.

## Task 6.7: SHA Lock Tests Ignored

Both `v0_2_sha256_lock_matches_committed_file` and `committed_v0_2_json_matches_regenerated_if_present` annotated with `#[ignore]` + "TODO Phase 7: re-enable after vector regen" comment.

## Concerns

None. All gates green. The Coldcard Cs fixture uses the identical policy string as M2 (both are `sh(wsh(sortedmulti(2,@0/**,@1/**,@2/**)))`), which is correct — the Coldcard BIP 48/1' 2-of-3 export shape IS this policy. The two vectors have different IDs and descriptions confirming parity.
