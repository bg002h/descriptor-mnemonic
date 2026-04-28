# Phase F review — Opus 4.7

**Status:** APPROVE
**Subject:** commit `5348b12` (`8-negative-fixture-dynamic-generation` + `phase-d-taproot-corpus-fixtures`)
**Reviewer model:** Opus 4.7 via general-purpose subagent
**Stage:** combined spec + corpus + provenance + backward compat + BIP-edit + code quality
**Role:** reviewer

## Findings

### Spec deviations

- **D-F7 (acknowledged, sound; nit)**: `tr_multia_2of3` uses `tr(@0/**, multi_a(2,@1/**,@2/**,@3/**))` (4 distinct keys) instead of the decision-doc original (3 keys reusing `@0`). Verified against `tests/taproot.rs:80-83` precedent (`taproot_single_leaf_multi_a_round_trips` uses identical policy). BIP 388's disjoint-paths constraint is real; the original would not encode. JSON's `description` field flags "(4 distinct keys)". Documented in agent report and commit message.

All other F-1..F-12 honored.

### Corpus + generator correctness

- Spot-checked 10 generators (n01, n02, n03, n05, n06–n11, n13, n14, n15, n20, n_taptree_multi_leaf). Provenance text matches code construction recipes exactly.
- `gen_vectors --verify v0.1.json` PASS (regenerates schema-1 byte-identical via `build_test_vectors_v1()`).
- `gen_vectors --verify v0.2.json` PASS via `build_test_vectors_v2()`.
- 5 empty `input_strings` sets (n12, n17, n30, n_tap_leaf_subset, n_fingerprints_count_mismatch) carry honest provenance naming the lower-level API or encode-side rejection.
- `multi_2of2_with_fingerprints` populates both `expected_fingerprints_hex: ["deadbeef", "cafebabe"]` and `encode_options_fingerprints: [[222,173,190,239],[202,254,186,190]]`; round-trips clean.

### Backward compat

- `v0.1.json` does NOT appear in the commit's content diff. `sha256sum` confirms `1957b542ed0388b51f01a7b467c8e802942dc6d6507abffaefaf777c90f3cd2c` unchanged.
- Schema-1 forward-compat: `Vector` and `NegativeVector` lack `deny_unknown_fields`; the 3 new fields use `serde(default, skip_serializing_if = "Option::is_none")`. Schema-1-only deserializers parse v0.2.json cleanly.

### BIP-edit correctness

- Both SHAs present (v0.1 `1957b542…`, v0.2 `92f0d5b2f365df38a6b22fcf24c3f0bc493883fd14f1db591f82418c001e0e42`).
- New subsections `===Schema versioning===`, `===Schema 1 (v0.1.0) contents===`, `===Schema 2 (v0.2.0) contents===`, `===Generation and verification===` present and sound.
- Vector counts: v0.1 = 10+30 (preserved), v0.2 = 14+34 (matches JSON).
- `provenance` field documented; schema-2 additions described accurately.

### Quality blockers

(none)

### Quality important

(none)

### Quality nits (3)

- `vectors.rs:514` doc comment for `build_test_vectors()` could read more cleanly on the alias chain (`build_test_vectors` → `build_test_vectors_v1`). Minor.
- 29 `unwrap()`/`panic!`/`unreachable!` sites in `vectors.rs` are all in vector-build-time paths over fixed-corpus inputs (compile-time-constant policy strings + byte sequences). Cannot fire except on a genuine codec bug — exactly what `gen_vectors` should panic on. Acceptable.
- `EncodedChunkRaw._header` and `_fragment` are dead-code-but-named-with-underscore for future debug. Fine.

## Disposition

| Finding | Action |
|---|---|
| All F-1..F-12 honored (D-F7 sound deviation) | No action |
| 10 generators spot-checked | No action |
| Backward compat verified (SHA unchanged, forward-compat) | No action |
| BIP edits accurate | No action |
| 3 nits | All cosmetic; no FOLLOWUPS entries needed |

## Verdict

**APPROVE** — Phase F clear; no follow-ups required. The cleanest reviewer outcome of any v0.2 phase to date.
