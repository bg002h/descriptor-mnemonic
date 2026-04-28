# Phase E review — Opus 4.7

**Status:** APPROVE_WITH_FOLLOWUPS
**Subject:** commit `6559c17` (`p2-fingerprints-block`)
**Reviewer model:** Opus 4.7 via general-purpose subagent
**Stage:** combined spec compliance + algorithmic + BIP-edit + test coverage + code quality
**Role:** reviewer

## Findings

### Spec deviations (vs `design/PHASE_v0_2_E_DECISIONS.md`)

(none) — E-1 through E-12 all honored as documented.

### Algorithmic correctness

- **`key_count()` semantics (E-12)**: returns `max_index + 1` by scanning for `@N` tokens (`policy.rs:238-258`), exactly matches the BIP MUST clause "highest `@i` index plus 1" (BIP line 406). Correctly handles non-contiguous indices (e.g., `@0,@2,@5` → 6, not 3). Reuse is sound.
- **Public API preserved**: `from_bytecode` at `policy.rs:454-456` is `pub fn from_bytecode(bytes: &[u8]) -> Result<Self, Error>` — signature unchanged. The new `from_bytecode_with_fingerprints` is the internal helper returning the tuple; legacy callers unaffected.
- **Encoder validation order**: `policy.rs:339-346` validates count BEFORE descriptor materialization (step 1) and any `out.push()`. No partial-write risk on error.
- **Header byte construction**: encoder uses `BytecodeHeader::new_v0(opts.fingerprints.is_some()).as_byte()` at `policy.rs:402`. Returns `0x04` when `Some`, `0x00` otherwise.
- **Tag dispatch airtight**: fingerprints block read inline in `from_bytecode_with_fingerprints` at `policy.rs:478-538` BEFORE `decode_template` is called. `decode_template` only sees post-fingerprints bytes. A mid-tree `0x35` falls through to `UnknownTag` via `Tag::from_byte`'s caller chain (existing behavior). Cannot be mistaken for a fingerprints block — dispatch is gated on `header.fingerprints()`.

### BIP-edit correctness

- Privacy paragraph at BIP line 415 with normative MUST/SHOULD language ("implementations SHOULD NOT emit", "MUST warn", "MUST flag"). Reads correctly.
- Byte-layout example at lines 417-447 with annotated table. Hex `0433033502deadbeefcafebabe0519020232003201` matches the live encoder via the pinning test `fingerprints_block_byte_layout_matches_bip_example`.
- Tag-table 0x35 row at BIP line 374 carries "(implemented v0.2)" annotation.
- Reserved tag note at line 379 correctly says 0x34 is reserved (was 0x34-0x35; now correctly 0x34 only).

### Test coverage

All 8 dispatch-list items present (in `tests/fingerprints.rs`):
1. `round_trip_with_fingerprints_two_keys`
2. `round_trip_without_fingerprints_two_keys`
3. `encoder_rejects_fingerprints_too_many` + conformance `rejects_fingerprints_count_mismatch` (asymmetric "too few")
4. `decoder_rejects_missing_fingerprints_tag`
5. `decoder_rejects_fingerprints_count_mismatch`
6. `decoder_rejects_fingerprints_truncated_mid_block`
7. `error_coverage.rs::ErrorVariantName::FingerprintsCountMismatch` registered
8. `from_bytecode_with_fingerprints_flag_no_block_is_truncated` (E-6 repurposed test; asserts `UnexpectedEnd` instead of `PolicyScopeViolation`)

Plus bonus: `decoder_rejects_fingerprints_missing_count_byte` and `fingerprints_block_byte_layout_matches_bip_example`. Total 10 new tests.

### Quality blockers

(none)

### Quality important

- **N-1 (nit-leaning-important)**: encoder cast `fps.len() as u8` at `policy.rs:410` gated only on `debug_assert!`. Since `key_count()` is bounded by BIP 388's 32-key cap and the validation above ensures `fps.len() == count`, the cast is provably safe in current code paths. But a future refactor that bypasses the validation funnel could silently truncate. Defense-in-depth: replace with `u8::try_from(fps.len()).map_err(...)?` for release-mode safety. **(Filed as `phase-e-encoder-count-cast-hardening`.)**

### Quality nits

- **N-2**: rustdoc redundancy — `error.rs` Stage-5 list mentions `FingerprintsCountMismatch` twice (encode side + decode side). Accurate but redundant.
- **N-3**: pre-existing `unwrap()` at `policy.rs:247` in `key_count` is well-guarded by `peek().is_some_and(...)`. Not introduced by this commit; noted only because the audit covered it.
- **N-4**: asymmetry between `WdmBackup.fingerprints` (reflects what was encoded) and `DecodeResult.fingerprints` (reflects what was decoded from the wire). Both sound; rustdoc could clarify which is authoritative on which side. Cosmetic only.

## Disposition

| Finding | Action |
|---|---|
| All E-1..E-12 honored | No action |
| Algorithmic correctness verified | No action |
| BIP-edit byte-layout reproduces exactly | No action |
| All 8 + 2 bonus tests present | No action |
| N-1 (cast hardening) | New FOLLOWUPS: `phase-e-encoder-count-cast-hardening` (v0.2-nice-to-have) |
| N-2/N-3/N-4 | Acknowledged; cosmetic |

Plus 2 deferred per the decision log:
- `phase-e-cli-fingerprint-flag` (E-10 deferred at dispatch)
- `phase-e-fingerprints-behavioral-break-migration-note` (E-9 deferred at dispatch; Phase G)

## Verdict

APPROVE_WITH_FOLLOWUPS — Phase E clear; E-1 through E-12 all honored.
