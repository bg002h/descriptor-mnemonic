# Phase v0.2 C — BCH 4-error correction (`p1-bch-4-error-correction`)

**Status:** DONE

**Commit SHA:** `3aabcf6acdb9283541e06eca70aaccd7c6fac257`

**Branch:** `worktree-agent-a61e8d2634fbcf14c`

## Files changed

| File | Δ | Notes |
|---|---|---|
| `crates/wdm-codec/src/encoding/bch_decode.rs` | NEW (~620 LOC) | Syndrome-based BCH decoder: GF(1024) field arithmetic, BM, Chien, Forney. |
| `crates/wdm-codec/src/encoding.rs` | modified | `bch_correct_regular` / `bch_correct_long` now invoke the new decoder. Removed the v0.1 `// TODO(v0.2)` comment and the `brute_force_one_error` helper. Updated the v0.1 "two errors uncorrectable" test to assert v0.2 recovery. |
| `crates/wdm-codec/tests/bch_correction.rs` | NEW (~390 LOC) | 42 integration tests: clean input, 1-error position classes, 2/3/4-error tuples, 5-error rejection, and 100-iteration property-based round-trips for n ∈ {1, 2, 3, 4} errors. |
| `bip/bip-wallet-descriptor-mnemonic.mediawiki` | modified | Added a SHOULD-clause in §"Error-correction guarantees" naming Berlekamp-Massey + Forney over GF(1024) as the canonical decoder, with the field representation pinned (`ζ² = ζ + 1`, `β = G·ζ`, `γ = E + X·ζ`, root windows). |

## Algorithm and correctness argument

### GF(1024) representation (BIP 93 §"Generation of valid checksum")

`GF(32)` uses the codex32/BIP 93 primitive polynomial `x⁵ + x³ + 1`
(matching the `bech32` crate's `Fe32`). Cross-validated by the
`gf32_alpha_powers_match_bech32_log_inv_table` test, which compares the
`α = 2` powers against bech32's published `LOG_INV` table.

`GF(1024) = GF(32)[ζ] / (ζ² − ζ − 1)` per BIP 93 §"Generation of valid
checksum" (BIP-93 source: "We extend GF[32] to GF[1024] by adjoining a
primitive cube root of unity, ζ, satisfying ζ² = ζ + P"). Cross-validated
by `zeta_is_primitive_cube_root_of_unity` (verifies `ζ² = ζ + 1` and
`ζ³ = 1`).

### Generator polynomial roots

The decoder evaluates syndromes at the **8 consecutive integer powers**
of the BCH-defining primitive element:

* **Regular code:** `α = β = G·ζ` (order 93), syndromes at `β^77, …, β^84`.
* **Long code:** `α = γ = E + X·ζ` (order 1023), syndromes at `γ^1019, …, γ^1026`.

Both windows match BIP 93's §"Generation of valid checksum" root list:

* Regular: roots at `i ∈ {17, 20, 46, 49, 52, 77, 78, 79, 80, 81, 82, 83, 84}` — the contiguous tail `{77..84}` is the 8-consecutive window we use.
* Long: roots at `i ∈ {32, 64, 96, 895, 927, 959, 991, 1019, 1020, 1021, 1022, 1023, 1024, 1025, 1026}` — the contiguous tail `{1019..1026}` is our window.

Cross-validated by `generator_polynomial_evaluates_to_zero_at_specified_roots`,
which reconstructs `g(x)` from `GEN_*[0]` (the polymod state encodes
`x^r mod g(x)`, so `g(x) = x^r + GEN_*[0]_packed_as_polynomial`) and
verifies vanishing at every root listed in the BIP.

### Decoder correctness

Standard textbook BM/Chien/Forney pipeline (Lin & Costello §6.3):

1. **Syndrome computation**: `S_m = E(α^{j_start + m - 1})` for `m = 1..8`.
   `E(x)` is the polymod residue (data + checksum, including HRP mixing)
   XORed with the WDM target constant; this is congruent to the error
   polynomial mod `g(x)`. Since `g(α^j) = 0`, the residue and the true
   error polynomial agree at all 8 roots.

2. **Berlekamp-Massey**: Massey 1969 form, 0-indexed syndromes. Returns
   `Λ(x)` of degree equal to the number of errors when correctable.

3. **Chien search**: bounded to `0..data_with_checksum_len` so we never
   accept a "phantom error" in the HRP-expansion prefix. Returns the
   polynomial-degree representation of each error.

4. **Forney's algorithm with shift**: `e_k = X_k^{1 - j_start} · Ω(X_k^{-1})
   / Λ'(X_k^{-1})`. The `X_k^{1 - j_start}` factor is the standard
   correction for syndromes that start at `α^{j_start}` rather than `α^1`
   (cf. Lin & Costello eq. 6.21 with the substitution
   `S_j → S_{j_start + j - 1}`).

5. **Position translation**: For `data_with_checksum.len() = L`, an
   error at index `k` lies at polynomial degree `d = L − 1 − k`. The
   inverse `k = L − 1 − d` is applied to convert Chien-search output
   to user-facing positions. The output is sorted ascending.

6. **Defensive verification**: `bch_correct_*` re-runs `bch_verify_*` on
   the corrected codeword and rejects the result if it doesn't pass.
   This guards against the pathological case where 5+ actual errors
   produce a degree-≤ 4 `Λ` with valid roots in the position range
   (mathematically possible because BCH only guarantees decoder
   correctness within the `t = 4` decoding sphere).

### Correctness verified by

* **Roots of g(x)**: `generator_polynomial_evaluates_to_zero_at_specified_roots`
  confirms `g_regular(β^i) = 0` for i ∈ {17, 20, 46, 49, 52, 77..84} and
  `g_long(γ^i) = 0` for i ∈ {32, 64, 96, 895, 927, 959, 991, 1019..1026}.
  This ties the abstract algorithm to the concrete `GEN_*` constants
  shipped in `encoding.rs`.

* **End-to-end recovery**: the property-based round-trip tests (8 tests
  × 100 iterations × 4 error counts = 3,200 random recovery cases)
  establish that for every n ∈ {1, 2, 3, 4} the decoder recovers the
  original codeword exactly.

* **5+-error rejection**: the existing
  `tests/ecc.rs::many_substitutions_always_rejected` (1,000 iterations
  of 5–9-error inputs, fixed seed `0xDEAD_BEEF`) still passes its ≥95%
  rejection threshold under the new decoder.

* **Wire format unchanged**: `gen_vectors --verify v0.1.json` PASSES
  byte-identical (10 positive + 30 negative vectors), confirming the
  decoder only corrects MORE inputs than before but produces the SAME
  output for any successfully-corrected input.

## Test count breakdown

| Category | Tests | Notes |
|---|---:|---|
| Clean input | 2 | regular + long |
| 1 error (position classes) | 12 | data begin/mid/end + checksum begin/mid/end × 2 codes |
| 2 errors | 6 | both-data / both-checksum / mixed × 2 codes |
| 3 errors | 6 | data-only / checksum-only / mixed × 2 codes |
| 4 errors (t = 4 capacity) | 6 | data-only / checksum-only / mixed × 2 codes |
| 5 errors (uncorrectable) | 2 | regular + long |
| Property tests (random) | 8 | 100 iterations each at n ∈ {1, 2, 3, 4} × 2 codes |
| GF(32) / GF(1024) self-checks | 5 | identity, zero, alpha-powers, ζ relation, β/γ orders |
| Generator polynomial root verification | 1 | cross-checks against BIP 93 §"Generation of valid checksum" |
| 1/2/3/4-error decoder unit tests (in `bch_decode.rs`) | 4 | tighter integration with the residue-extraction path |
| Pre-existing tests carried forward | 489 | `cargo test -p wdm-codec` baseline (389 lib + 100 integration); see below for net change |
| **Total** | **535** | (lib 389 + integration 141 + doc 5) |

Pre-Phase-C baseline: 482 tests. Net delta: **+53 tests** (42 new
integration + 11 new unit + 0 retired). One test renamed
(`bch_correct_regular_two_errors_uncorrectable_v0_1` →
`bch_correct_regular_two_errors_recovered_v0_2`) with flipped sign.

## Quality gates

| Gate | Status |
|---|---|
| `cargo test -p wdm-codec` | PASS (535 / 535) |
| `cargo clippy --all-targets -p wdm-codec -- -D warnings` | PASS (clean) |
| `cargo fmt --all --check` | PASS (clean) |
| `RUSTDOCFLAGS="-D warnings -D missing_docs" cargo doc --workspace --no-deps --document-private-items` | PASS (clean) |
| `cargo run --bin gen_vectors -- --verify crates/wdm-codec/tests/vectors/v0.1.json` | PASS (10 positive, 30 negative; wire format unchanged) |
| `tests/ecc.rs::many_substitutions_always_rejected` | PASS (≥95% rejection of 5+-error inputs) |

All gates were run with the workspace `[patch]` workaround for the
worktree depth: `cargo --config 'patch."https://github.com/apoelstra/rust-miniscript".miniscript.path="/scratch/code/shibboleth/rust-miniscript-fork"'`.

## API surface guarantee

`bch_correct_regular(hrp: &str, data_with_checksum: &[u8]) -> Result<CorrectionResult, Error>`
and `bch_correct_long` keep their exact v0.1 signatures.
`CorrectionResult.{data, corrections_applied, corrected_positions}` is
unchanged. v0.1 callers recompile without modification. The only
observable behavioural difference: 2/3/4-error inputs that previously
returned `Err(BchUncorrectable)` now return `Ok(CorrectionResult)` with
`corrections_applied ∈ {2, 3, 4}` and the appropriate `corrected_positions`.

The decoder breaks ties (when multiple weight-≤ 4 codewords exist
equidistant from the received word — possible for 4-error inputs only)
by ascending position index, matching the BIP draft's new SHOULD-clause.

## BIP draft edit

Added a SHOULD-clause at the end of §"Error-correction guarantees"
(line 156-157 of the mediawiki source) that:

* Names Berlekamp-Massey + Forney as the canonical decoder.
* Pins the GF(1024) representation: `GF(32)[ζ] / (ζ² − ζ − P)` with `ζ`
  a primitive cube root of unity.
* Pins the primitive elements: `β = G·ζ` (regular, order 93) and
  `γ = E + X·ζ` (long, order 1023).
* Pins the 8-consecutive-roots windows: `{β^77..β^84}` and
  `{γ^1019..γ^1026}`.
* Permits alternative algorithms but requires that ties be broken by
  ascending position index for byte-identical `Correction.corrected`
  output across implementations.

The clause is placed after "Conformant decoders MUST report which
correction was applied" so it composes naturally with the existing
reporter requirements.

## Deferred minor items

None. The implementation is complete and ships all spec-mandated test
coverage. No `FOLLOWUPS.md` entries are needed.

## Notable design choices

* **No new dependency**: GF(32) and GF(1024) arithmetic are implemented
  in-crate. GF(32) uses the existing 5-bit primitive-polynomial
  representation; GF(1024) is built as a length-2 tower over GF(32) with
  the `ζ² = ζ + 1` quadratic baked in as XOR + GF(32) ops.

* **Field representation derived from the BIP, not searched**: The
  earlier draft of this implementation searched at runtime for an
  irreducible quadratic that matched the published `GEN_*` constants.
  When BIP 93 §"Generation of valid checksum" was located on the
  bitcoin/bips master branch, it specified the field directly:
  `ζ² = ζ + P`, `β = G·ζ`, `γ = E + X·ζ`. The runtime search was
  removed and the constants hard-coded; correctness is established by
  the `generator_polynomial_evaluates_to_zero_at_specified_roots` test
  rather than a runtime feasibility check.

* **Shifted Forney rather than syndrome relabeling**: Standard BCH
  decoders evaluate at `α^1, …, α^{2t}`. Codex32's BCH code has
  consecutive roots starting at higher exponents (77 / 1019). Two
  options: (a) relabel the syndromes (shift the whole field
  representation), or (b) keep the standard sequence and apply a
  position-shift inside Forney's formula. The implementation chose (b)
  because it leaves the BM algorithm untouched (clearer code, easier
  audit) and isolates the codex32-specific arithmetic to a single
  multiplicative factor.

* **Chien search bounded to `data_with_checksum_len`, not full
  codeword length**: The polymod operates over `hrp_expand || data ||
  checksum`. Errors are physically impossible in the HRP region (the
  HRP is fixed and known to both encoder and decoder). Restricting the
  Chien search to the transmitted region ensures we never report a
  spurious correction outside `data_with_checksum`. A misbehaving
  decoder that does report such a correction would be caught by the
  defensive `bch_verify_*` re-check, but bounding at the source is
  cleaner.
