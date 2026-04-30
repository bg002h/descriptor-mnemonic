# v0.11 Phase 22 Review — End-to-end Engrave/Restore Tests

- **Date:** 2026-04-30
- **Branch:** `feature/v0.11-impl-phase-1`
- **Phase:** 22 — End-to-end engrave/restore tests
- **Status:** DONE

## Scope

Phase 22 closes out the v0.11 implementation work modulo the final cutover by
exercising the full encode → md1 codex32 string → decode → `Descriptor`
pipeline against the four representative wallet shapes from the spec's
worked-examples appendix. Per spec §1 (purpose), the md1 format exists to make
descriptor-template engrave/restore practical for real wallets; per §13
(implementation guidance), end-to-end tests are the gating evidence that the
codec subsystems compose correctly; per §14 (worked examples), the four shapes
covered here are the canonical reference set every implementation must
round-trip.

## Task 22.1 — commit `2ee415f`

`test(v0.11): BIP 86 + vault end-to-end md1 string round-trip`

- Adds two tests to `crates/md-codec/tests/v11_smoke.rs`:
  - `bip86_taproot_md1_string_round_trip` — `tr(@0/<0;1>/*)` with
    `m/86'/0'/0'`, no script tree (§14 BIP 86 single-sig taproot).
  - `vault_or_d_pk_older_md1_string_round_trip` —
    `wsh(or_d(pk(@0), and_v(v:older(144), pk(@1))))` with a single
    144-block CSV recovery branch (§14 vault recovery).
- Each test constructs the source `Descriptor`, encodes via
  `encode_md1_string`, decodes via `decode_md1_string`, and asserts structural
  equality with the original.
- +71 lines, single-file diff (tests-only).

## Verification

`cargo test -p md-codec --test v11_smoke`:

```
running 8 tests
test bip48_2of3_sortedmulti_round_trip ... ok
test bip48_2of3_md1_string_round_trip ... ok
test bip84_emit_md1_string ... ok
test bip84_single_sig_round_trip ... ok
test bip84_single_sig_payload_bit_count ... ok
test bip86_taproot_md1_string_round_trip ... ok
test bip84_md1_string_round_trip ... ok
test vault_or_d_pk_older_md1_string_round_trip ... ok

test result: ok. 8 passed; 0 failed
```

8 PASS (6 prior + 2 new). Cumulative v11 tests: 105 (103 + 2).

## End-to-end coverage achieved

End-to-end engrave/restore is now validated for all four §14 reference shapes:

- **BIP 84 single-sig** — `wpkh(@0/<0;1>/*)`, `m/84'/0'/0'`.
- **BIP 86 taproot single-sig** — `tr(@0/<0;1>/*)`, `m/86'/0'/0'`, no script tree.
- **BIP 48 2-of-3 sortedmulti** — `wsh(sortedmulti(2, @0, @1, @2))`, `m/48'/0'/0'/2'`.
- **Vault recovery** — `wsh(or_d(pk(@0), and_v(v:older(144), pk(@1))))`, single
  144-block CSV recovery branch.

Each shape round-trips structurally identical:
`encode_md1_string(d) → s; decode_md1_string(s) → d'; assert_eq!(d, d')`.

This is the major v0.11 milestone — every wallet shape the spec commits to
supporting has been demonstrated to engrave and restore through the full
codex32-string boundary.

## Carry-forward deferred items

Same set as Phase 21: P1, P2, P4, P5, P13a, P13b. P12 resolved in Phase 19.
None block Phase 23 cutover.

## Next

Phase 23 — final cutover: re-export the v11 surface as the canonical
`md-codec` API in `lib.rs`.

---

**DONE** — Task 22.1 verified, commit `2ee415f`.
**CONCERNS** — none.
**CONTEXT** — End-to-end engrave/restore validated for all four §14 worked
examples (BIP 84, BIP 86, BIP 48 2-of-3, vault or_d/older). Cumulative v11
test count: 105. Carry-forward deferred items unchanged from Phase 21.
**BLOCKED** — none.
