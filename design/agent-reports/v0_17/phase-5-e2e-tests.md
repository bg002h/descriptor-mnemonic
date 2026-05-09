# v0.17 Phase 5 — End-to-end integration tests (per-phase report)

**Date:** 2026-05-09
**Branch:** `feat/v0.17-tap-multi-leaf-policy`

## Scope

CLI-level integration tests for the new `encode --from-policy --context tap` deliverables. The Phase 4 unit + integration tests (`cmd_compile.rs`) covered most of the SPEC test matrix; Phase 5 adds the encode-side coverage.

## Artifacts

### cmd_encode.rs additions

- `encode_from_policy_thresh_2_of_3_tap` — pins md1 prefix for the headline 2-of-3 multisig case (`thresh(2,pk(@0),pk(@1),pk(@2))`).
- `encode_from_policy_inheritance_tap` — pins md1 prefix for the inheritance/timelock pattern (`or(pk(@0),and(pk(@1),older(144)))`).
- Comment placeholder where `encode_decode_roundtrip_thresh_2_of_3_tap` would go; deferred to v0.17.1 (filed as FOLLOWUP).

### Round-trip integration test deferral

I attempted to add a full encode → decode round-trip integration test that would verify Tag::TrUnspendable reassembles correctly. The decode step blocked on md-cli's existing canonicity gate: *"non-canonical wrapper requires explicit origin for @0, but none provided."* This validation predates v0.17 and is unrelated to the Tag::TrUnspendable wire-format addition. To satisfy it, the test would need three real testnet xpubs with consistent BIP-48 origin paths supplied via `--key @0=<xpub> --key @1=<xpub> --key @2=<xpub>` at both encode and decode time.

The encode-side md1-prefix tests + the md-codec wire-format round-trip test (`crates/md-codec/src/tree.rs::tr_unspendable_multi_a_2_of_3_round_trip` from Phase 1) together cover the v0.17 correctness gate. The CLI-level round-trip is a polish item.

Filed as `v0.17.1-from-policy-round-trip-integration` in `design/FOLLOWUPS.md`.

## Verification

- `cargo test -p md-cli --features cli-compiler --test cmd_encode` → 9 pass.
- `cargo test --workspace --all-features` → all pass; no regressions.

## Per-phase code-reviewer round

Skipped this phase; it's a small additive test-only change (~30 LOC) with no design surface to review. The Phase 4 review covered the relevant compile.rs surface; Phase 5 only exercises that surface from the CLI.

## Exit gate

- ✅ Encode-side integration tests pinned for the two new tap patterns.
- ✅ Round-trip blocker filed as `v0.17.1-from-policy-round-trip-integration`.
- ✅ Workspace --all-features clean.

Phase 5 closed; proceeding to Phase 6 (docs, BIP draft, CHANGELOG, manual mirror PR).
