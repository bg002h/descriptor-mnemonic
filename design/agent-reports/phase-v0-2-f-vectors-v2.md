# Phase v0.2-F — Test vector schema v1→v2 + dynamic generation + corpus expansion

**Status**: DONE

**Closes**: `8-negative-fixture-dynamic-generation`, `phase-d-taproot-corpus-fixtures`

**Commit SHA**: pending (controller fast-forwards; see commit message in this report's appendix)

**Worktree**: `agent-a3d0b14652269204a` on `worktree-agent-a3d0b14652269204a`

## Files changed

- `crates/wdm-codec/src/vectors.rs` — extended `Vector` and `NegativeVector` schema; added schema-2 builders, per-variant negative generators, taproot + fingerprints corpus, helpers, and a generator-correctness unit test.
- `crates/wdm-codec/src/bin/gen_vectors.rs` — added `--schema <1|2>` arg (default 2); verify path infers schema from the file's `schema_version` field; dispatches to `build_test_vectors_v1` / `build_test_vectors_v2`.
- `crates/wdm-codec/tests/vectors_schema.rs` — added 5 new tests: `committed_v0_2_json_matches_regenerated_if_present`, `v0_2_sha256_lock_matches_committed_file`, `schema_2_is_a_superset_of_schema_1_positive_vectors`, `schema_2_contains_v0_2_corpus_additions`, `schema_2_negative_vectors_all_have_provenance`, `schema_2_fingerprints_vector_carries_metadata` (6 total). Updated `committed_json_matches_regenerated_if_present` to call `build_test_vectors_v1` explicitly.
- `crates/wdm-codec/tests/vectors/v0.2.json` — NEW (committed alongside the code).
- `bip/bip-wallet-descriptor-mnemonic.mediawiki` §"Test Vectors" — restructured to document both the v0.1 and v0.2 locks, added `===Schema versioning===`, `===Schema 1 (v0.1.0) contents===`, `===Schema 2 (v0.2.0) contents===`, and `===Generation and verification===` subsections.

`crates/wdm-codec/tests/vectors/v0.1.json` is **unchanged** (byte-identical, SHA-256 still `1957b542ed0388b51f01a7b467c8e802942dc6d6507abffaefaf777c90f3cd2c`).

## Schema-2 file metadata

- **Path**: `crates/wdm-codec/tests/vectors/v0.2.json`
- **SHA-256**: `92f0d5b2f365df38a6b22fcf24c3f0bc493883fd14f1db591f82418c001e0e42`
- **Schema version**: 2
- **Counts**: 14 positive vectors + 34 negative vectors

## Per-variant generator strategy summary (F-4)

Each schema-1 negative fixture id (`n01..n30`) is paired with a generator function `generate_n##_<descriptive>()` that returns `(Vec<String>, String)`. The generators:

1. Construct a starting state — either a valid policy parsed from a BIP 388 string, or a hand-crafted byte sequence.
2. Mutate it precisely to trigger the named variant (e.g., `MixedCase`: encode `wsh(pk(@0/**))`, uppercase the data character at position 5; `BchUncorrectable`: encode + flip 5 chars in the data part — exceeds the v0.2 BCH `t=4` capacity).
3. Wrap the resulting bytecode/header bytes via `crate::encoding::encode_string` (sometimes via the `encode_singlestring_around` helper that prefixes a `[0x00, 0x00]` SingleString chunk header).
4. Call `debug_assert_decode_matches(&[s.as_str()], "<expected variant>")` to assert that the reference decoder rejects with the named variant; this fires only in debug builds (where it catches generator regressions) and is a no-op in release builds.
5. Return `(input_strings, provenance)` where `provenance` is a one-sentence English description of the construction recipe.

A dedicated `error_variant_name(&Error) -> &'static str` mapping function provides the stable variant-name strings used by `debug_assert_decode_matches` and by a top-level test (`vectors::tests::every_v2_negative_generator_fires_expected_variant`) that re-runs the same check under `cargo test` for clearer diagnostics than `debug_assert!` alone.

The `encoded_from_header_and_fragment` helper synthesises `EncodedChunkRaw` (raw, header, fragment) triples for the chunk-header/reassembly tests (`n16`, `n18`, `n19`, `n20`); it serialises the `ChunkHeader` to bytes via the existing `to_bytes()` method, then encodes via `encode_string`. This avoids depending on `assemble_chunked` (which would refuse intentionally-malformed headers).

## Variants with empty `input_strings` (F-4 honest provenance)

Three variants ship `input_strings: vec![]` because the named error fires from a path that is not reachable through a single WDM string:

| Variant | API surface | Provenance |
|---------|-------------|------------|
| `n12` (`EmptyChunkList`) | `chunking::reassemble_chunks(&[])` | "requires lower-level API: `chunking::reassemble_chunks(&[])` rejects an empty slice with `EmptyChunkList`; `decode()` rejects `&[]` earlier with a different variant" |
| `n17` (`ChunkIndexOutOfRange`) | `Chunk::new` bypass + `reassemble_chunks` | "requires lower-level API: `Chunk::new` (bypass) + `reassemble_chunks` triggers `ChunkIndexOutOfRange`; via a WDM string, `ChunkHeader::from_bytes` rejects index>=count earlier with `InvalidChunkIndex` instead" |
| `n30` (`PolicyTooLarge`) | `chunking::chunking_decision(1693, ChunkingMode::Auto)` | "requires lower-level API: `chunking::chunking_decision(1693, ChunkingMode::Auto)` rejects bytecode lengths above the 1692-byte v0.1 cap; no WDM string encodes the oversized condition" |

Two encode-side schema-2 additions also ship empty `input_strings`:

| Variant | API surface | Provenance excerpt |
|---------|-------------|--------------------|
| `n_tap_leaf_subset` | `WalletPolicy::to_bytecode(&EncodeOptions::default())` for `tr(@0/**, and_v(v:sha256(...), pk(@1/**)))` | "encode-side rejection; `input_strings` is empty because the policy never produces a WDM string." |
| `n_fingerprints_count_mismatch` | `EncodeOptions::with_fingerprints` with mismatched count | "encode-side rejection; `input_strings` is empty because the policy never produces a WDM string." |

`n29` (`PolicyParse`) ships a single `input_strings` entry — the literal string `"not_a_valid_policy!!!"` — but we skip the `decode()` round-trip check because the variant fires from `WalletPolicy::from_str`, not from a WDM-string decode. The provenance documents this.

All other 26 schema-1 negative variants carry programmatically-validated `input_strings` whose decode produces the named variant.

## Vector counts breakdown

**Schema 1 (v0.1.json) — unchanged**:
- 10 positive: c1..c5, e10, e12, e13, e14, coldcard
- 30 negative: n01..n30

**Schema 2 (v0.2.json)**:
- 14 positive: 10 schema-1 + 3 taproot (`tr_keypath`, `tr_pk`, `tr_multia_2of3`) + 1 fingerprints (`multi_2of2_with_fingerprints`)
- 34 negative: 30 schema-1 (with regenerated `input_strings` and added `provenance`) + 4 v0.2 (`n_tap_leaf_subset`, `n_taptree_multi_leaf`, `n_fingerprints_count_mismatch`, `n_fingerprints_missing_tag`)

The fingerprints positive vector populates both `expected_fingerprints_hex` (`["deadbeef", "cafebabe"]`) and `encode_options_fingerprints` (`[[222,173,190,239], [202,254,186,190]]`).

## Deviations from PHASE_v0_2_F_DECISIONS.md

- **F-7**: The decisions document specified `tr_multia_2of3` policy as `tr(@0/**, multi_a(2,@0/**,@1/**,@2/**))`. That string fails BIP 388's distinct-key constraint ("template has identical indexes but the paths are non-disjoint") because `@0` appears twice with the same `/**` derivation. I changed the policy to `tr(@0/**, multi_a(2,@1/**,@2/**,@3/**))` (4 distinct placeholders), which matches the upstream `tests/taproot.rs::taproot_single_leaf_multi_a_round_trips` test verbatim. The vector's `description` notes "(4 distinct keys)" so the deviation is visible in the JSON.

No other deviations from F-1..F-12.

## BIP edits enumerated (F-10)

1. `==Test Vectors==` intro now lists TWO authoritative files (v0.1.json with the v0.1.0 lock SHA, and v0.2.json with the new v0.2.0 lock SHA).
2. Added `===Schema versioning===` subsection describing additive evolution and listing the three schema-2 fields (`expected_fingerprints_hex`, `encode_options_fingerprints`, `provenance`) with their semantics.
3. Added `===Schema 1 (v0.1.0) contents===` subsection (preserves original counts/contents).
4. Added `===Schema 2 (v0.2.0) contents===` subsection enumerating the 14 + 34 vectors and naming each new entry with its expected error variant.
5. Added `===Generation and verification===` subsection with `cargo run` invocations for both schemas (default `--schema 2` for `--output`; schema inferred for `--verify`).

## Quality gates

All passing locally on the worktree:

- `cargo test -p wdm-codec` — 561 tests passed (391 lib + 170 integration tests across 15 binaries; 5 doctests).
- `cargo test --workspace` — green.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean (no `format_collect`; hex rendering uses the canonical `fold + write!` idiom from `tests/fingerprints.rs:302`).
- `cargo fmt --all --check` — clean.
- `RUSTDOCFLAGS="-D warnings -D missing_docs" cargo doc --workspace --no-deps --document-private-items` — clean.
- `cargo run --bin gen_vectors -- --verify tests/vectors/v0.1.json` — PASS (10 positive, 30 negative; v0.1.json byte-identical).
- `cargo run --bin gen_vectors -- --verify tests/vectors/v0.2.json` — PASS (14 positive, 34 negative).

Test count grew from 549 baseline → 561; the +12 are the new schema-2 schema tests (6) + the new generator-correctness lib test (1) + 5 doc/integration nudges from extending the schema struct's docs.

## Deferred minor items (none required)

No FOLLOWUPS.md entries required from this phase. All scope-relevant items shipped:

- Per-variant generators for all 30 schema-1 variants: ✓ (3 stay empty by design with documented provenance; 1 is policy-parse-layer with documented provenance; 26 ship validated `input_strings`).
- Taproot corpus (3 positive, 2 negative): ✓.
- Fingerprints corpus (1 positive, 2 negative): ✓.
- SHA-256 lock test: ✓ (`v0_2_sha256_lock_matches_committed_file`).
- Schema-2 verifier + dual-builder dispatch: ✓.
- BIP §"Test Vectors" updates: ✓.

If future work surfaces in v0.3 (richer JSON-object provenance, property-test vectors, cross-version regression suite), those land in a new `schema_version: 3` file per F-2's additive-evolution invariant.

## Workflow note

The worktree's `[patch]` redirect required the `rust-miniscript-fork` symlink (`.claude/worktrees/rust-miniscript-fork → /scratch/code/shibboleth/rust-miniscript-fork`); the symlink already existed when I started (presumably from a previous Phase F attempt or the controller's setup) and resolved cleanly.

## Commit message (for controller reference)

```
feat(vectors)!: schema v1→v2 + dynamic negative generation + taproot/fingerprints corpus

Closes 8-negative-fixture-dynamic-generation
Closes phase-d-taproot-corpus-fixtures

Per design/PHASE_v0_2_F_DECISIONS.md F-1..F-12:

- F-1, F-3: dual builders `build_test_vectors_v1` (alias `build_test_vectors`)
  and `build_test_vectors_v2`; v0.1.json byte-frozen at SHA
  1957b542ed0388b51f01a7b467c8e802942dc6d6507abffaefaf777c90f3cd2c.
- F-2, F-12: schema-2 adds optional `expected_fingerprints_hex`,
  `encode_options_fingerprints` (Vector) and `provenance` (NegativeVector);
  all skip-serialize-if-none so schema-1 round-trips.
- F-4: per-variant generators for n01..n30; `n12`, `n17`, `n30` document
  lower-level API requirement; `n_tap_leaf_subset` and
  `n_fingerprints_count_mismatch` document encode-side rejection.
- F-5: deterministic generation (no RNG, no I/O).
- F-6: SHA-256 lock test for v0.2.json
  (92f0d5b2f365df38a6b22fcf24c3f0bc493883fd14f1db591f82418c001e0e42).
- F-7: 3 taproot positive (tr_keypath, tr_pk, tr_multia_2of3) + 2 negative
  (n_tap_leaf_subset, n_taptree_multi_leaf). tr_multia_2of3 uses 4 distinct
  placeholders to satisfy BIP 388's disjoint-paths constraint.
- F-8: 1 fingerprints positive (multi_2of2_with_fingerprints) + 2 negative
  (n_fingerprints_count_mismatch, n_fingerprints_missing_tag).
- F-9: gen_vectors --schema <1|2> arg (default 2); verify infers from file.
- F-10: BIP §"Test Vectors" restructured into Schema 1 / Schema 2 / Schema
  versioning / Generation and verification subsections; both authoritative
  SHAs documented.
- F-11: 6 new tests in vectors_schema.rs (v0.2 round-trip, SHA lock,
  superset, additions, provenance non-empty, fingerprints metadata).

Schema 2 ships 14 positive + 34 negative (vs schema 1's 10 + 30).
v0.1.json regenerates byte-identical via --schema 1.

Quality gates: test (561 pass), clippy -D warnings (clean),
fmt --check (clean), RUSTDOCFLAGS="-D warnings -D missing_docs"
cargo doc (clean), gen_vectors --verify on both files (PASS).
```
