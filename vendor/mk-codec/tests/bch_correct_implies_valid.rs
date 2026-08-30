//! `bch_correct_ok_implies_valid_codeword` — the F4 miscorrection-class guard
//! (constellation-eval §2 #8 / SPEC_test_hardening_T2, T2-c).
//!
//! mk-codec's `bch_correct_regular` / `bch_correct_long` end with a *defensive
//! re-verify* (`src/string_layer/bch.rs:451` regular / `:504` long) that
//! rejects any proposed correction whose result is not actually a valid
//! codeword — the guard against a ≥5-error input whose syndromes happen to
//! factor as a degree-≤4 locator (a "repair" that blesses a wrong payload).
//! Deleting that guard, or relaxing the `deg > 4` cap in the decoder
//! (`src/string_layer/bch_decode.rs:566`), was previously caught by nothing.
//!
//! This file pins three properties of `bch_correct_*`:
//!   (i)   whenever it returns `Ok`, the result **verifies clean** —
//!         residue == target, recomputed by an INDEPENDENT polymod that shares
//!         only the code-DEFINITION constants (never the verify/correct path);
//!   (ii)  a successful correction applies **≤ 4** substitutions;
//!   (iii) for **≤ 4** injected errors it is UNCONDITIONAL — `Ok` AND == the
//!         original codeword (an `Err` here is a failure, not a vacuous pass,
//!         so a syndrome-window regression cannot hide behind an `Ok`-guard).
//!
//! Coverage is a bounded always-on proptest net PLUS deterministic mined KAT
//! cells (the random re-verify-guard hit rate is ~7e-6/trial — too rare to be
//! a reliable gate), each RED under a single named mutation:
//!   * mined re-verify cells (regular)  → RED under deleting the re-verify (bch.rs:451)
//!   * constructed re-verify cells (long) → RED under deleting the re-verify (bch.rs:504)
//!   * exact-5-error cap cells           → RED under relaxing `deg > 4` (bch_decode.rs:566)
//!
//! See the per-cell docs. The oracle is independent-residue == target plus the
//! injected-pattern ground truth, never `bch_verify_*` (the code's own path).

use mk_codec::string_layer::bch::{
    GEN_LONG, GEN_REGULAR, LONG_MASK, LONG_SHIFT, POLYMOD_INIT, REGULAR_MASK, REGULAR_SHIFT,
    bch_correct_long, bch_correct_regular, bch_create_checksum_long, bch_create_checksum_regular,
};
use mk_codec::{MK_LONG_CONST, MK_REGULAR_CONST};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Independent oracle: recompute the BCH residue from scratch and compare it to
// the per-HRP target constant. This deliberately does NOT call
// `bch_verify_regular` / `bch_verify_long` (the "code's own verify path" that
// `bch_correct_*` re-uses for its guard) and does NOT call `polymod_run`; it
// reimplements the BIP-93 ms32 polymod loop locally, sharing only the
// generator-polynomial / init / target DEFINITION constants. So a mutation to
// the correction pipeline cannot also silence the oracle.
// ---------------------------------------------------------------------------

/// `hrp_expand("mk")` — the 5-bit prelude fed before the data part. Pinned as a
/// literal (equivalently `[m>>5, k>>5, 0, m&31, k&31]`); the production value
/// is asserted in `bch::tests::hrp_expand_mk_matches_spec`.
const HRP_EXPAND_MK: [u8; 5] = [3, 3, 0, 13, 11];

/// Local, self-contained BIP-93 ms32 polymod over `values`, seeded with
/// `POLYMOD_INIT`. Independent of `bch::polymod_run` / `bch_verify_*`.
fn independent_polymod(values: &[u8], generator: &[u128; 5], shift: u32, mask: u128) -> u128 {
    let mut residue = POLYMOD_INIT;
    for &v in values {
        let b = residue >> shift;
        let mut next = ((residue & mask) << 5) ^ (v as u128);
        for (i, &g) in generator.iter().enumerate() {
            if (b >> i) & 1 != 0 {
                next ^= g;
            }
        }
        residue = next;
    }
    residue
}

/// Independent re-verification of a regular-code `data_with_checksum`.
fn independent_verify_regular(dwc: &[u8]) -> bool {
    if dwc.len() < 13 {
        return false;
    }
    let mut input = HRP_EXPAND_MK.to_vec();
    input.extend_from_slice(dwc);
    independent_polymod(&input, &GEN_REGULAR, REGULAR_SHIFT, REGULAR_MASK) == MK_REGULAR_CONST
}

/// Independent re-verification of a long-code `data_with_checksum`.
fn independent_verify_long(dwc: &[u8]) -> bool {
    if dwc.len() < 15 {
        return false;
    }
    let mut input = HRP_EXPAND_MK.to_vec();
    input.extend_from_slice(dwc);
    independent_polymod(&input, &GEN_LONG, LONG_SHIFT, LONG_MASK) == MK_LONG_CONST
}

// ---------------------------------------------------------------------------
// Codeword construction + error injection (encode path — never the code under
// test). Injected errors are the ground-truth oracle for the ≤4 legs.
// ---------------------------------------------------------------------------

fn build_valid_regular(data: &[u8]) -> Vec<u8> {
    let mut dwc = data.to_vec();
    dwc.extend_from_slice(&bch_create_checksum_regular("mk", data));
    dwc
}

fn build_valid_long(data: &[u8]) -> Vec<u8> {
    let mut dwc = data.to_vec();
    dwc.extend_from_slice(&bch_create_checksum_long("mk", data));
    dwc
}

/// Apply `(position, xor_magnitude)` substitutions. Each magnitude is a nonzero
/// 5-bit value so every listed position genuinely changes.
fn apply(dwc: &[u8], injections: &[(usize, u8)]) -> Vec<u8> {
    let mut out = dwc.to_vec();
    for &(p, m) in injections {
        out[p] ^= m;
    }
    out
}

/// Map `(position_seed, magnitude)` proptest seeds to DISTINCT positions in
/// `0..len` with nonzero magnitudes (so the injected error weight is exactly
/// the returned length).
fn distinct_injections(seeds: &[(u16, u8)], len: usize) -> Vec<(usize, u8)> {
    let mut out: Vec<(usize, u8)> = Vec::new();
    for &(pseed, m) in seeds {
        let p = (pseed as usize) % len;
        if out.iter().all(|&(q, _)| q != p) {
            out.push((p, m));
        }
    }
    out
}

// Regular codes carry 14..=93 data-part symbols (data 1..=80 + 13 checksum);
// long codes carry 96..=108 (data 81..=93 + 15 checksum).
fn regular_data() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(0u8..32u8, 1..=80usize)
}

fn long_data() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(0u8..32u8, 81..=93usize)
}

/// Up to 8 injection seeds; magnitudes 1..=31 (nonzero 5-bit).
fn injection_seeds() -> impl Strategy<Value = Vec<(u16, u8)>> {
    prop::collection::vec((any::<u16>(), 1u8..=31u8), 0..=8usize)
}

// ---------------------------------------------------------------------------
// Oracle self-test — the independent verifier must agree with a freshly
// encoded codeword and reject a single-symbol tamper. If this is wrong every
// proptest below is meaningless, so pin it directly.
// ---------------------------------------------------------------------------

#[test]
fn independent_oracle_agrees_with_encode() {
    let data: Vec<u8> = (0u8..20).map(|i| i % 32).collect();

    let reg = build_valid_regular(&data);
    assert!(
        independent_verify_regular(&reg),
        "fresh regular codeword must verify"
    );
    let mut tampered = reg.clone();
    tampered[7] ^= 1;
    assert!(
        !independent_verify_regular(&tampered),
        "1-symbol tamper must fail verify"
    );

    let long_data: Vec<u8> = (0u8..85).map(|i| i % 32).collect();
    let lng = build_valid_long(&long_data);
    assert!(
        independent_verify_long(&lng),
        "fresh long codeword must verify"
    );
    let mut tampered_l = lng.clone();
    tampered_l[3] ^= 4;
    assert!(
        !independent_verify_long(&tampered_l),
        "1-symbol tamper must fail verify"
    );
}

// ---------------------------------------------------------------------------
// (i) + (ii) — always-on invariant net: any `Ok` verifies clean and applies
// ≤ 4 corrections, over BOTH near-codeword injections and fully-random words.
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn bch_correct_ok_implies_valid_regular(
        data in regular_data(),
        seeds in injection_seeds(),
    ) {
        let dwc = build_valid_regular(&data);
        let injections = distinct_injections(&seeds, dwc.len());
        let corrupted = apply(&dwc, &injections);
        if let Ok(r) = bch_correct_regular("mk", &corrupted) {
            prop_assert!(
                independent_verify_regular(&r.data),
                "bch_correct_regular returned Ok whose data fails independent re-verification"
            );
            prop_assert!(
                r.corrections_applied <= 4,
                "corrections_applied = {} exceeds t = 4",
                r.corrections_applied
            );
            prop_assert_eq!(r.corrections_applied, r.corrected_positions.len());
        }
    }

    #[test]
    fn bch_correct_ok_implies_valid_long(
        data in long_data(),
        seeds in injection_seeds(),
    ) {
        let dwc = build_valid_long(&data);
        let injections = distinct_injections(&seeds, dwc.len());
        let corrupted = apply(&dwc, &injections);
        if let Ok(r) = bch_correct_long("mk", &corrupted) {
            prop_assert!(
                independent_verify_long(&r.data),
                "bch_correct_long returned Ok whose data fails independent re-verification"
            );
            prop_assert!(
                r.corrections_applied <= 4,
                "corrections_applied = {} exceeds t = 4",
                r.corrections_applied
            );
            prop_assert_eq!(r.corrections_applied, r.corrected_positions.len());
        }
    }

    // Fully-random symbol vectors (not near any codeword): the same implication
    // must hold. Most inputs are `Err`; the value is that no `Ok` ever escapes
    // unverified.
    #[test]
    fn bch_correct_ok_implies_valid_regular_random(
        symbols in prop::collection::vec(0u8..32u8, 14..=93usize),
    ) {
        if let Ok(r) = bch_correct_regular("mk", &symbols) {
            prop_assert!(independent_verify_regular(&r.data));
            prop_assert!(r.corrections_applied <= 4);
        }
    }

    #[test]
    fn bch_correct_ok_implies_valid_long_random(
        symbols in prop::collection::vec(0u8..32u8, 96..=108usize),
    ) {
        if let Ok(r) = bch_correct_long("mk", &symbols) {
            prop_assert!(independent_verify_long(&r.data));
            prop_assert!(r.corrections_applied <= 4);
        }
    }
}

// ---------------------------------------------------------------------------
// (iii) — UNCONDITIONAL ≤4 leg: for ≤4 injected errors, `bch_correct_*` MUST
// return `Ok` == the original codeword. `Err` fails (not a vacuous Ok-guard),
// so a syndrome-window / position-translation regression in bch_decode.rs
// (e.g. the `k = L-1-d` map at :587, or `j_start`) turns this RED loudly.
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn bch_correct_le4_is_unconditional_regular(
        data in regular_data(),
        seeds in prop::collection::vec((any::<u16>(), 1u8..=31u8), 1..=4usize),
    ) {
        let dwc = build_valid_regular(&data);
        let injections = distinct_injections(&seeds, dwc.len());
        prop_assume!(!injections.is_empty());
        let corrupted = apply(&dwc, &injections);
        let r = bch_correct_regular("mk", &corrupted)
            .expect("≤4 injected errors must ALWAYS be correctable (BCH t = 4)");
        prop_assert_eq!(&r.data, &dwc, "≤4-error correction did not recover the original");
        prop_assert_eq!(r.corrections_applied, injections.len());
    }

    #[test]
    fn bch_correct_le4_is_unconditional_long(
        data in long_data(),
        seeds in prop::collection::vec((any::<u16>(), 1u8..=31u8), 1..=4usize),
    ) {
        let dwc = build_valid_long(&data);
        let injections = distinct_injections(&seeds, dwc.len());
        prop_assume!(!injections.is_empty());
        let corrupted = apply(&dwc, &injections);
        let r = bch_correct_long("mk", &corrupted)
            .expect("≤4 injected errors must ALWAYS be correctable (BCH t = 4)");
        prop_assert_eq!(&r.data, &dwc, "≤4-error correction did not recover the original");
        prop_assert_eq!(r.corrections_applied, injections.len());
    }
}

// ---------------------------------------------------------------------------
// Mined KAT cells — deterministic RED-proof for implication (i).
//
// Each `(data, injections)` was mined against MK's own regular constants
// (`GEN_REGULAR` / `MK_REGULAR_CONST`): 5–8 injected errors whose syndromes
// factor as a degree-≤4 locator with valid GF(32) magnitudes, so the RAW
// decoder (`decode_regular_errors`) returns a non-empty correction — but
// applying it does NOT re-verify. Under UNMUTATED source `bch_correct_regular`
// therefore returns `Err` (the re-verify guard at bch.rs:451 rejects it), so
// the `Ok` arm below is not taken and the cell passes. DELETE that re-verify
// and the same input returns `Ok` with unverifiable data → assertion (i) fires
// DETERMINISTICALLY (no reliance on the ~7e-6 random path).
// ---------------------------------------------------------------------------

/// (data, injections) — injections carry ≥5 errors (beyond BCH t = 4).
type ReverifyCell = (&'static [u8], &'static [(usize, u8)]);

const REVERIFY_REGULAR_CELLS: &[ReverifyCell] = &[
    (
        &[
            31, 14, 2, 0, 26, 31, 19, 31, 31, 28, 26, 20, 9, 23, 29, 20, 18, 13, 27, 30, 10, 23, 9,
            29, 26, 7, 2, 13, 23, 2, 4, 27, 27, 20, 29, 1, 25, 6, 13, 23, 24, 8, 25, 7, 25, 6, 19,
            9, 11, 1, 9, 25, 25, 10, 3, 11, 19, 9, 15, 20, 13, 12, 27, 30, 19, 6, 25, 7, 31, 15, 0,
            17, 11, 10, 14, 11,
        ],
        &[
            (35, 11),
            (14, 25),
            (77, 14),
            (5, 22),
            (50, 19),
            (11, 26),
            (53, 12),
            (59, 13),
        ],
    ),
    (
        &[
            31, 24, 19, 3, 20, 17, 26, 26, 29, 6, 4, 29, 15, 29, 29, 13, 0, 7, 1, 20, 7, 25, 6, 16,
            1, 29, 13, 1, 10, 25, 3, 19, 4, 22, 21, 9, 26, 21, 29, 9, 24, 23, 23, 31, 11, 11, 15,
            1, 24, 20, 29, 7, 1, 20, 15, 8, 12, 16, 24, 8, 22, 14, 5, 27, 31, 24, 29, 7, 24, 21,
            24, 14, 11, 11, 18, 20, 4,
        ],
        &[(10, 10), (52, 18), (64, 6), (85, 11), (1, 2), (34, 24)],
    ),
    (
        &[
            12, 30, 0, 0, 9, 12, 26, 27, 27, 25, 10, 24, 24, 26, 4, 23, 11, 11, 29, 21, 11, 19, 25,
            21, 16, 7, 27, 27, 31, 22, 21, 17, 0, 19, 5, 25, 20, 6, 16, 31, 20, 22, 20, 4, 3, 28,
            25, 30, 23, 16, 25, 22, 11, 3, 1, 22, 25, 29, 0, 22,
        ],
        &[(56, 10), (5, 27), (41, 11), (68, 22), (50, 2)],
    ),
    (
        &[
            17, 18, 5, 12, 31, 13, 30, 2, 27, 25, 27, 17, 0, 5, 18, 16, 27, 22, 27, 13, 25, 18, 3,
            12, 12, 27, 24, 31, 5, 0, 17, 8, 20, 30, 4,
        ],
        &[(13, 20), (31, 10), (43, 2), (7, 28), (19, 27)],
    ),
];

#[test]
fn mined_reverify_regular_kats_imply_valid() {
    for (idx, (data, injections)) in REVERIFY_REGULAR_CELLS.iter().enumerate() {
        assert!(
            injections.len() >= 5,
            "cell {idx}: ground truth is a >t (≥5) error pattern"
        );
        let dwc = build_valid_regular(data);
        let corrupted = apply(&dwc, injections);
        // Sanity: the injected word is genuinely off-codeword.
        assert!(
            !independent_verify_regular(&corrupted),
            "cell {idx}: corrupted must be invalid"
        );

        // Implication (i): the ONLY acceptable `Ok` is one that independently
        // verifies. Under unmutated source this is `Err` (guard rejects);
        // deleting the re-verify (bch.rs:451) makes it `Ok`-unverifiable → RED.
        if let Ok(r) = bch_correct_regular("mk", &corrupted) {
            assert!(
                independent_verify_regular(&r.data),
                "MINED (i) regular cell {idx}: bch_correct_regular returned Ok whose data fails \
                 independent re-verification — the re-verify guard at bch.rs:451 was bypassed"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// CONSTRUCTED long cells — deterministic RED-proof for implication (i) on the
// LONG re-verify guard (bch.rs:504).
//
// Unlike the regular case, a random ≥5-error long injection essentially never
// reaches the long guard (measured 0 non-verifying raw fits in 10M+ trials —
// the long code's error positions span only ~108/1023 of the γ-group, so a
// spurious ≤4 locator almost never has all roots in range). The guard is
// nonetheless REACHABLE, and these cells reach it BY CONSTRUCTION.
//
// Construction (each cell is a full 108-symbol `data_with_checksum`):
//   * `M(x) = g_long(x) / m₂(x)` (degree 13, coeffs x^0..x^13
//     `[1,20,4,17,1,20,23,23,20,1,17,4,20,1]`). The 8-syndrome window
//     {γ^1019..γ^1022, γ^0..γ^3} is Frobenius-closed (·32 mod 1023 → g_long's
//     15 roots, bch_decode.rs:62-66), so M vanishes at 7 of the 8 window
//     points and perturbs EXACTLY syndrome[0] by δ ≠ 0.
//   * Take a valid codeword `c = data ‖ bch_create_checksum_long("mk", data)`,
//     inject ONE real error, then add M at the low-degree end
//     (`r[107-d] ^= M[d]`). True error weight = 15 (≫ t = 4).
//   * The raw decoder (`decode_long_errors`) then returns a SPURIOUS single-
//     symbol ≤4 fit (the real error) that does NOT re-verify → under UNMUTATED
//     source `bch_correct_long` returns `Err` via the bch.rs:504 guard.
//
// DELETE that guard (bch.rs:504 → `if true`) and the same input returns `Ok`
// with unverifiable data → implication (i) fires on the independent oracle,
// DETERMINISTICALLY. Each cell was re-verified by execution before pinning:
// raw fit non-empty & in-range, `bch_verify_long(applied fix) == false`,
// `bch_correct_long == Err`. Cell 0 is the reviewer's §4 vector; cells 1–2
// vary the data formula, real-error position, and magnitude.
// ---------------------------------------------------------------------------

const REVERIFY_LONG_CELLS: &[&[u8]] = &[
    // data[i] = (i*7+3) mod 32 ; real error r[40] ^= 13.
    &[
        3, 10, 17, 24, 31, 6, 13, 20, 27, 2, 9, 16, 23, 30, 5, 12, 19, 26, 1, 8, 15, 22, 29, 4, 11,
        18, 25, 0, 7, 14, 21, 28, 3, 10, 17, 24, 31, 6, 13, 20, 22, 2, 9, 16, 23, 30, 5, 12, 19,
        26, 1, 8, 15, 22, 29, 4, 11, 18, 25, 0, 7, 14, 21, 28, 3, 10, 17, 24, 31, 6, 13, 20, 27, 2,
        9, 16, 23, 30, 5, 12, 19, 26, 1, 8, 15, 22, 29, 4, 11, 18, 25, 0, 7, 14, 14, 8, 15, 1, 31,
        13, 1, 20, 13, 28, 13, 2, 8, 9,
    ],
    // data[i] = (i*11+5) mod 32 ; real error r[40] ^= 30.
    &[
        5, 16, 27, 6, 17, 28, 7, 18, 29, 8, 19, 30, 9, 20, 31, 10, 21, 0, 11, 22, 1, 12, 23, 2, 13,
        24, 3, 14, 25, 4, 15, 26, 5, 16, 27, 6, 17, 28, 7, 18, 3, 8, 19, 30, 9, 20, 31, 10, 21, 0,
        11, 22, 1, 12, 23, 2, 13, 24, 3, 14, 25, 4, 15, 26, 5, 16, 27, 6, 17, 28, 7, 18, 29, 8, 19,
        30, 9, 20, 31, 10, 21, 0, 11, 22, 1, 12, 23, 2, 13, 24, 3, 14, 25, 25, 2, 12, 13, 17, 11,
        20, 7, 3, 23, 28, 6, 2, 7, 16,
    ],
    // data[i] = (i*13+7) mod 32 ; real error r[55] ^= 5.
    &[
        7, 20, 1, 14, 27, 8, 21, 2, 15, 28, 9, 22, 3, 16, 29, 10, 23, 4, 17, 30, 11, 24, 5, 18, 31,
        12, 25, 6, 19, 0, 13, 26, 7, 20, 1, 14, 27, 8, 21, 2, 15, 28, 9, 22, 3, 16, 29, 10, 23, 4,
        17, 30, 11, 24, 5, 23, 31, 12, 25, 6, 19, 0, 13, 26, 7, 20, 1, 14, 27, 8, 21, 2, 15, 28, 9,
        22, 3, 16, 29, 10, 23, 4, 17, 30, 11, 24, 5, 18, 31, 12, 25, 6, 19, 18, 24, 17, 20, 23, 14,
        26, 25, 22, 6, 6, 13, 25, 8, 17,
    ],
];

#[test]
fn mined_reverify_long_kats_imply_valid() {
    for (idx, r) in REVERIFY_LONG_CELLS.iter().enumerate() {
        assert_eq!(
            r.len(),
            108,
            "long cell {idx} must be a 108-symbol data_with_checksum"
        );
        // Ground truth: the constructed word is genuinely off-codeword.
        assert!(
            !independent_verify_long(r),
            "cell {idx}: constructed input must be off-codeword"
        );

        // Implication (i): the ONLY acceptable `Ok` is one that independently
        // verifies. Under unmutated source the bch.rs:504 guard rejects the
        // spurious in-range ≤4 fit, so this is `Err`; deleting that re-verify
        // makes it `Ok`-unverifiable and fires on the INDEPENDENT oracle → RED.
        let result = bch_correct_long("mk", r);
        if let Ok(cr) = &result {
            assert!(
                independent_verify_long(&cr.data),
                "MINED (i) long cell {idx}: bch_correct_long returned Ok whose data fails \
                 independent re-verification — the re-verify guard at bch.rs:504 was bypassed"
            );
        }
        // Current-behavior pin: the guard fires. (Under the bch.rs:504 mutation
        // the implication above fires first, on the oracle — not this pin.)
        assert!(
            result.is_err(),
            "long cell {idx}: expected the bch.rs:504 re-verify guard to reject the spurious \
             ≤4 fit, but bch_correct_long returned Ok"
        );
    }
}

// ---------------------------------------------------------------------------
// Mined exact-5-error cap cells — deterministic RED-proof for property (ii).
//
// Each `(data, injections)` injects EXACTLY 5 errors whose Berlekamp–Massey
// locator has degree 5 with 5 in-range roots and valid GF(32) magnitudes. The
// production `deg > 4` cap (`bch_decode.rs:566`) rejects the degree-5 locator,
// so under UNMUTATED source `bch_correct_regular` returns `Err` and the `Ok`
// arm is not taken. RELAX that cap (drop `|| deg > 4`) and the same input
// returns `Ok` with `corrections_applied == 5` → property (ii) fires
// DETERMINISTICALLY. (Mined by temporarily relaxing the cap; the vectors are
// pinned against the shipped constants.)
// ---------------------------------------------------------------------------

const CAP5_REGULAR_CELLS: &[ReverifyCell] = &[
    (
        &[
            1, 30, 16, 12, 6, 2, 13, 3, 23, 6, 30, 29, 28, 13, 24, 9, 6, 26, 21, 14, 10, 3, 29, 29,
            24, 14, 20, 4, 22, 6, 22, 8, 5, 0, 28, 8, 9, 14, 6, 17, 28, 30, 27, 8, 25, 12,
        ],
        &[(39, 30), (0, 30), (36, 23), (54, 13), (27, 31)],
    ),
    (
        &[
            26, 30, 6, 28, 25, 30, 4, 0, 6, 4, 30, 23, 26, 26, 16, 15, 13, 22, 10, 4, 26, 14, 0,
            25, 4, 29, 0, 6, 21, 27, 10, 19, 31, 19, 28, 25, 26, 16, 18, 3, 20, 31, 28, 10, 1, 26,
            3, 16, 29, 22, 16, 4, 13, 13, 15, 13, 30, 8, 30, 24, 30, 10, 15, 9, 4, 12, 31, 16, 2,
            17, 21, 17, 23, 6, 29, 1, 13, 19, 30,
        ],
        &[(34, 28), (52, 25), (58, 19), (88, 9), (10, 18)],
    ),
    (
        &[
            29, 10, 16, 11, 6, 27, 12, 1, 3, 16, 11, 9, 26, 30, 25, 25, 6, 12, 6, 20, 5, 13, 31,
            25, 18, 22, 5, 27, 11, 20, 4, 19, 20, 12, 11, 17, 15, 20, 19, 15, 11, 11, 0, 10, 31, 3,
            15, 14, 17, 23, 11, 16, 25, 30, 20, 10,
        ],
        &[(28, 14), (64, 16), (37, 18), (52, 11), (13, 17)],
    ),
];

#[test]
fn mined_cap5_regular_kats_bound_corrections() {
    for (idx, (data, injections)) in CAP5_REGULAR_CELLS.iter().enumerate() {
        assert_eq!(
            injections.len(),
            5,
            "cell {idx}: exact-5-error ground truth"
        );
        let dwc = build_valid_regular(data);
        let corrupted = apply(&dwc, injections);

        // Property (ii): a successful correction never applies more than t = 4.
        // Unmutated: `Err` (the deg>4 cap rejects). Relax the cap and this
        // returns `Ok` with corrections_applied == 5 → RED.
        if let Ok(r) = bch_correct_regular("mk", &corrupted) {
            assert!(
                r.corrections_applied <= 4,
                "MINED (ii) regular cell {idx}: bch_correct_regular applied {} > 4 corrections — \
                 the `deg > 4` cap at bch_decode.rs:566 was relaxed",
                r.corrections_applied
            );
        }
    }
}
