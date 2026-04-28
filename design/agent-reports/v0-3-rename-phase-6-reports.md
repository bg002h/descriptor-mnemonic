# Phase 6 (test vector regeneration / wire-format crystallization) — agent reports

**Phase**: regenerate v0.1.json + v0.2.json with HRP "md", capture new SHAs, update lock + BIP + decision log
**Branch**: `rename/v0.3-wdm-to-md`
**Commits**: `c89be41` (vectors regen + lock update), `03fa977` (BIP + decision log)
**Date**: 2026-04-27

---

## Implementer report (Sonnet)

**Status**: DONE

**New SHAs** (the v0.3.0 family-stable values):
- `v0.1.json`: `aac3677fd84f06915c7bb5148a25ed80c399daa4f9bf56c8052ed84f83c9b71b`
- `v0.2.json`: `18804929d54f94fe4b83a135f3e53d3a26b6ae3565729970ce02ef38f74e9909`

**Steps executed in order**:
1. `gen_vectors --output v0.1.json --schema 1` (10 positive + 30 negative; family generator `md-codec 0.3`)
2. `gen_vectors --output v0.2.json --schema 2` (14 positive + 34 negative; family generator `md-codec 0.3`)
3. SHAs captured
4. `V0_2_SHA256` constant updated (no `V0_1_SHA256` exists per plan correction)
5. 3 `#[ignore]`d tests re-enabled: `v0_2_sha256_lock_matches_committed_file`, `committed_json_matches_regenerated_if_present`, `committed_v0_2_json_matches_regenerated_if_present`
6. `gen_vectors --verify` PASS for both files
7. Full test suite: 565 passing + 0 ignored
8. BIP TODO Phase 6 marker removed (BIP discusses SHA-256 as a protocol primitive only; doesn't embed file checksum — Phase 2 marker was for a non-existent SHA reference)
9. Decision log Open Items section filled with new SHAs + discovery surprises summary + final touch-point counts

**Concerns**: None. All gates clean.

---

## Combined spec + quality review (Sonnet) — APPROVED

All 13 verifications passed:
- Vector files present + non-empty + valid JSON
- SHA-256 of files matches claims exactly
- Vector content uses `md1` prefix (43 occurrences in v0.1.json, 49 in v0.2.json); ZERO `wdm1` occurrences
- Generator field is `"md-codec 0.3"` in both
- `V0_2_SHA256` constant matches actual file SHA
- `gen_vectors --verify` PASS for both
- 3 previously-ignored tests now have NO `#[ignore]` attribute
- No remaining `TODO Phase 6` markers in `crates/md-codec/` or `bip/`
- BIP correctly does NOT embed file-checksum (only describes SHA-256 protocol usage); Phase 2 marker was for a non-existent reference
- Decision log Open Items filled (no `<TODO>` placeholders)
- Build clean, **565 passing + 0 ignored** across 15 test binaries, clippy clean, fmt clean
- Diff scope: exactly 5 files changed (BIP, 2 vector JSONs, vectors_schema.rs, decision log) — nothing extraneous
- Vector structure: v0.1 schema=1 with 10/30 fixtures; v0.2 schema=2 with 14/34 fixtures; matches expected counts

**Quality observations**: Diff is minimal (124+/124− churn from HRP-byte changes in JSON; 4 lines in vectors_schema.rs; 4 lines in BIP). Decision log captures the 5 discovered surprises at appropriate detail for future auditability.

**Concerns flagged**: None.

---

## Phase 6 closure

✅ Wire format crystallized at v0.3.0. New family-stable promise begins: future v0.3.x patches will produce byte-identical vector files (per the design from v0.2.1). Phases 2-6 substantive work complete.

**Test count progression**:
- Pre-rename (v0.2.3 baseline): 565 passing
- Phase 4 close: 564 passing + 1 ignored (SHA-lock pending Phase 6)
- Phase 5 close: 562 passing + 3 ignored (added 2 regen-comparison ignores)
- Phase 6 close: **565 passing + 0 ignored** (all 3 re-enabled and green)

Phase 7 (CI verification — mostly no-op) unblocked.
