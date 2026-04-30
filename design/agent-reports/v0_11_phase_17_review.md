# v0.11 Phase 17 Review — Forward-compat tests

- Date: 2026-04-30
- Branch: `feature/v0.11-impl-phase-1`
- Phase: 17 (Forward-compat tests)
- Status: DONE

## Scope

Phase 17 adds a forward-compatibility test verifying that v0.11 decoders preserve unknown TLV records through a round-trip. This locks in the D6 decision (TLV forward-compat) and exercises the §3.7 unknown-tag handling rule (skip via length-prefix advancement, retain bytes for re-emission).

## Task

- **Task 17.1** — commit `b245d8f`: `unknown_tlv_round_trip_preserved` test added in `tests/v11_forward_compat.rs`. Encoder embeds a synthesized v0.12-style Xpubs TLV (tag `0x02`) into `TlvSection.unknown`; decoder preserves the bytes verbatim across a full round-trip.

## Verification

```
$ cargo test -p md-codec --test v11_forward_compat
running 1 test
test unknown_tlv_round_trip_preserved ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured
```

Cumulative v0.11 test count: 87 (86 + 1).

## Spec citations

- **D6 (TLV forward-compat)** — v0.11 readers MUST advance over unknown tags using the length prefix and retain the raw bytes for downstream re-emission, enabling minor-version TLV extensions without breaking older decoders.
- **§3.7 (TLV unknown-tag handling)** — unknown TLV records are collected into `TlvSection.unknown` (preserving wire order) and re-serialized on encode.

## Findings

The test confirms the core forward-compat property: a hypothetical v0.12 record (Xpubs at tag `0x02`) injected into a v0.11 payload is decoded into the `unknown` vec and re-emitted byte-identically. This validates both halves of D6 — length-prefix advancement on decode and faithful re-emission on encode.

## Deferred items (carry-forward)

Same set as prior phases — no change:
- P1, P2, P4, P5, P12, P13a, P13b.

## Next

Phase 18 — display rules (engraving layout helpers).

## Concerns

None.

## Context

Phase 17 is a single-task phase; the test is small but lock-in for the forward-compat invariant that gates safe v0.11 → v0.12 evolution.
