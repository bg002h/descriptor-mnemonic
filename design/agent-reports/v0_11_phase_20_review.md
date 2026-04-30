# v0.11 Phase 20 Review — Top-Level String API + Multisig Round-Trip

- **Date:** 2026-04-30
- **Branch:** `feature/v0.11-impl-phase-1`
- **Phase:** 20 — `encode_md1_string` + `decode_md1_string` + multisig md1 round-trip
- **Spec refs:** §3.1 (HRP + character set), §13.2 (single-string mode wire layout), §13.3 (BCH checksum framing)

## Summary

Phase 20 lands the canonical single-string-mode public API for the md1 codec.
After this phase, callers can move a wallet descriptor through the full md1
pipeline using only the top-level entry points:

```text
encode_md1_string(&Descriptor) -> String
decode_md1_string(&str)        -> Descriptor
```

Both the BIP 84 single-sig fixture and the BIP 48 2-of-3 sortedmulti fixture
round-trip cleanly through `HRP + GF(32)-encoded payload + BCH checksum`
(§3.1, §13.2, §13.3). This is the **major usability milestone** for v0.11:
the codec is now exercisable end-to-end without reaching into internal payload
or chunking helpers.

## Tasks landed

| Task | Commit | Deliverable |
|------|--------|-------------|
| 20.1 | `49640c1` | `encode_md1_string` in `encode.rs` + `bip84_emit_md1_string` smoke |
| 20.2 | `c386908` | `decode_md1_string` in `decode.rs` + `bip84_md1_string_round_trip` smoke |
| 20.3 | `736f736` | `bip48_2of3_md1_string_round_trip` smoke |

## Verification

```
$ cargo test -p md-codec --test v11_smoke
running 6 tests
test bip48_2of3_sortedmulti_round_trip ... ok
test bip48_2of3_md1_string_round_trip ... ok
test bip84_emit_md1_string ... ok
test bip84_md1_string_round_trip ... ok
test bip84_single_sig_payload_bit_count ... ok
test bip84_single_sig_round_trip ... ok

test result: ok. 6 passed; 0 failed; 0 ignored
```

**Cumulative v11 test count:** 100 (97 prior + 3 added in P20).

## Spec alignment

- **§3.1** — emitted strings carry the `md1` HRP and are restricted to the
  codex32 GF(32) alphabet; both fixtures round-trip without alphabet drift.
- **§13.2** — single-string-mode payload framing (header + tagged sections)
  is preserved bit-exactly across the encode/decode boundary for both
  single-sig and multisig descriptors.
- **§13.3** — BCH checksum is appended on encode and verified on decode via
  the shared BIP 93 polynomial with HRP-mixing and md1 target residue.

## Carry-forward deferred items

Unchanged from Phase 19. P12 was resolved in P19; no new deferrals introduced
in P20.

- **P1, P2** — (per phase-19 review)
- **P4, P5** — (per phase-19 review)
- **P13a, P13b** — (per phase-19 review)

## Next phase

**Phase 21: multi-card chunking with codex32 integration.** Extends the
single-string API into the multi-card share form so that descriptors which
exceed a single card's payload budget split deterministically across
codex32-style shares.
