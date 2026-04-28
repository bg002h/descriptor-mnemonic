//! Comprehensive coverage of the v0.2 BCH 4-error correction layer
//! (`p1-bch-4-error-correction`).
//!
//! Categorisation:
//!
//! 1. **Clean input** — 0 errors, identity result.
//! 2. **1 error at every position class** — beginning / middle / end of
//!    the data region and the checksum region. Regular and Long.
//! 3. **2 errors** at representative position pairs — both data, both
//!    checksum, mixed.
//! 4. **3 errors** at representative position triples.
//! 5. **4 errors** at representative position quadruples — full BCH
//!    `t = 4` capacity.
//! 6. **5 errors** — uncorrectable; must return `BchUncorrectable`.
//! 7. **Round-trip property tests** — `100` random `n`-error patterns
//!    each for `n ∈ {1, 2, 3, 4}`, seed `0xDEAD_BEEF`, must all
//!    round-trip; recovered word must equal the original.
//!
//! All vectors use `bch_correct_regular` / `bch_correct_long` directly
//! (working in the encoding-layer 5-bit symbol space, not WDM strings)
//! so the tests exercise the BCH decoder in isolation from the higher
//! decode-pipeline layers (chunking, header parsing, bytecode).

use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;

use md_codec::encoding::{
    HRP, bch_correct_long, bch_correct_regular, bch_create_checksum_long,
    bch_create_checksum_regular,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const REGULAR_DATA_LEN: usize = 30; // arbitrary in [1, 80]; full data+checksum = 43
const REGULAR_CHECKSUM_LEN: usize = 13;
const LONG_DATA_LEN: usize = 85; // arbitrary in [81, 93]; full data+checksum = 100
const LONG_CHECKSUM_LEN: usize = 15;

fn build_regular_codeword() -> Vec<u8> {
    let data: Vec<u8> = (0..REGULAR_DATA_LEN as u8).map(|i| i & 0x1F).collect();
    let checksum = bch_create_checksum_regular(HRP, &data);
    let mut codeword = data;
    codeword.extend_from_slice(&checksum);
    assert_eq!(codeword.len(), REGULAR_DATA_LEN + REGULAR_CHECKSUM_LEN);
    codeword
}

fn build_long_codeword() -> Vec<u8> {
    let data: Vec<u8> = (0..LONG_DATA_LEN as u8).map(|i| i & 0x1F).collect();
    let checksum = bch_create_checksum_long(HRP, &data);
    let mut codeword = data;
    codeword.extend_from_slice(&checksum);
    assert_eq!(codeword.len(), LONG_DATA_LEN + LONG_CHECKSUM_LEN);
    codeword
}

/// Inject a non-zero error symbol at each of `positions` and confirm
/// the BCH decoder recovers the original.
fn assert_corrects_regular(positions: &[usize], magnitudes: &[u8]) {
    assert_eq!(positions.len(), magnitudes.len());
    let original = build_regular_codeword();
    let mut corrupted = original.clone();
    for (&p, &m) in positions.iter().zip(magnitudes) {
        assert!(p < corrupted.len());
        assert!(m != 0 && m < 32);
        corrupted[p] ^= m;
    }
    let result = bch_correct_regular(HRP, &corrupted).unwrap_or_else(|e| {
        panic!(
            "regular: failed to correct {} errors at {:?}: {}",
            positions.len(),
            positions,
            e
        )
    });
    assert_eq!(result.corrections_applied, positions.len());
    let mut sorted = positions.to_vec();
    sorted.sort_unstable();
    let mut got = result.corrected_positions.clone();
    got.sort_unstable();
    assert_eq!(got, sorted, "wrong positions reported");
    assert_eq!(result.data, original, "wrong data after correction");
}

fn assert_corrects_long(positions: &[usize], magnitudes: &[u8]) {
    assert_eq!(positions.len(), magnitudes.len());
    let original = build_long_codeword();
    let mut corrupted = original.clone();
    for (&p, &m) in positions.iter().zip(magnitudes) {
        assert!(p < corrupted.len());
        assert!(m != 0 && m < 32);
        corrupted[p] ^= m;
    }
    let result = bch_correct_long(HRP, &corrupted).unwrap_or_else(|e| {
        panic!(
            "long: failed to correct {} errors at {:?}: {}",
            positions.len(),
            positions,
            e
        )
    });
    assert_eq!(result.corrections_applied, positions.len());
    let mut sorted = positions.to_vec();
    sorted.sort_unstable();
    let mut got = result.corrected_positions.clone();
    got.sort_unstable();
    assert_eq!(got, sorted, "wrong positions reported");
    assert_eq!(result.data, original, "wrong data after correction");
}

// ---------------------------------------------------------------------------
// 1. Clean input
// ---------------------------------------------------------------------------

#[test]
fn clean_regular_input_zero_corrections() {
    let codeword = build_regular_codeword();
    let r = bch_correct_regular(HRP, &codeword).unwrap();
    assert_eq!(r.corrections_applied, 0);
    assert!(r.corrected_positions.is_empty());
    assert_eq!(r.data, codeword);
}

#[test]
fn clean_long_input_zero_corrections() {
    let codeword = build_long_codeword();
    let r = bch_correct_long(HRP, &codeword).unwrap();
    assert_eq!(r.corrections_applied, 0);
    assert!(r.corrected_positions.is_empty());
    assert_eq!(r.data, codeword);
}

// ---------------------------------------------------------------------------
// 2. 1 error at every position class
// ---------------------------------------------------------------------------

#[test]
fn one_error_data_region_beginning_regular() {
    assert_corrects_regular(&[0], &[7]);
}

#[test]
fn one_error_data_region_middle_regular() {
    assert_corrects_regular(&[REGULAR_DATA_LEN / 2], &[19]);
}

#[test]
fn one_error_data_region_end_regular() {
    assert_corrects_regular(&[REGULAR_DATA_LEN - 1], &[31]);
}

#[test]
fn one_error_checksum_region_beginning_regular() {
    assert_corrects_regular(&[REGULAR_DATA_LEN], &[1]);
}

#[test]
fn one_error_checksum_region_middle_regular() {
    assert_corrects_regular(&[REGULAR_DATA_LEN + REGULAR_CHECKSUM_LEN / 2], &[14]);
}

#[test]
fn one_error_checksum_region_end_regular() {
    assert_corrects_regular(&[REGULAR_DATA_LEN + REGULAR_CHECKSUM_LEN - 1], &[27]);
}

#[test]
fn one_error_data_region_beginning_long() {
    assert_corrects_long(&[0], &[7]);
}

#[test]
fn one_error_data_region_middle_long() {
    assert_corrects_long(&[LONG_DATA_LEN / 2], &[19]);
}

#[test]
fn one_error_data_region_end_long() {
    assert_corrects_long(&[LONG_DATA_LEN - 1], &[31]);
}

#[test]
fn one_error_checksum_region_beginning_long() {
    assert_corrects_long(&[LONG_DATA_LEN], &[1]);
}

#[test]
fn one_error_checksum_region_middle_long() {
    assert_corrects_long(&[LONG_DATA_LEN + LONG_CHECKSUM_LEN / 2], &[14]);
}

#[test]
fn one_error_checksum_region_end_long() {
    assert_corrects_long(&[LONG_DATA_LEN + LONG_CHECKSUM_LEN - 1], &[27]);
}

// ---------------------------------------------------------------------------
// 3. 2 errors at representative position pairs
// ---------------------------------------------------------------------------

#[test]
fn two_errors_both_data_regular() {
    assert_corrects_regular(&[2, 17], &[5, 22]);
}

#[test]
fn two_errors_both_checksum_regular() {
    assert_corrects_regular(&[REGULAR_DATA_LEN + 1, REGULAR_DATA_LEN + 11], &[3, 9]);
}

#[test]
fn two_errors_mixed_regular() {
    assert_corrects_regular(&[5, REGULAR_DATA_LEN + 4], &[11, 28]);
}

#[test]
fn two_errors_both_data_long() {
    assert_corrects_long(&[10, 70], &[5, 22]);
}

#[test]
fn two_errors_both_checksum_long() {
    assert_corrects_long(&[LONG_DATA_LEN + 1, LONG_DATA_LEN + 13], &[3, 9]);
}

#[test]
fn two_errors_mixed_long() {
    assert_corrects_long(&[20, LONG_DATA_LEN + 7], &[11, 28]);
}

// ---------------------------------------------------------------------------
// 4. 3 errors at representative position triples
// ---------------------------------------------------------------------------

#[test]
fn three_errors_data_only_regular() {
    assert_corrects_regular(&[1, 13, 27], &[4, 18, 30]);
}

#[test]
fn three_errors_checksum_only_regular() {
    assert_corrects_regular(
        &[
            REGULAR_DATA_LEN,
            REGULAR_DATA_LEN + 6,
            REGULAR_DATA_LEN + 12,
        ],
        &[2, 16, 25],
    );
}

#[test]
fn three_errors_mixed_regular() {
    assert_corrects_regular(&[3, 22, REGULAR_DATA_LEN + 9], &[6, 12, 24]);
}

#[test]
fn three_errors_data_only_long() {
    assert_corrects_long(&[2, 40, 80], &[4, 18, 30]);
}

#[test]
fn three_errors_checksum_only_long() {
    assert_corrects_long(
        &[LONG_DATA_LEN, LONG_DATA_LEN + 7, LONG_DATA_LEN + 14],
        &[2, 16, 25],
    );
}

#[test]
fn three_errors_mixed_long() {
    assert_corrects_long(&[10, 60, LONG_DATA_LEN + 5], &[6, 12, 24]);
}

// ---------------------------------------------------------------------------
// 5. 4 errors — full t = 4 BCH capacity
// ---------------------------------------------------------------------------

#[test]
fn four_errors_data_only_regular() {
    assert_corrects_regular(&[0, 9, 18, 27], &[1, 11, 21, 31]);
}

#[test]
fn four_errors_checksum_only_regular() {
    assert_corrects_regular(
        &[
            REGULAR_DATA_LEN,
            REGULAR_DATA_LEN + 4,
            REGULAR_DATA_LEN + 8,
            REGULAR_DATA_LEN + 12,
        ],
        &[1, 11, 21, 31],
    );
}

#[test]
fn four_errors_mixed_regular() {
    assert_corrects_regular(&[2, 15, 28, REGULAR_DATA_LEN + 5], &[1, 11, 21, 31]);
}

#[test]
fn four_errors_data_only_long() {
    assert_corrects_long(&[0, 25, 50, 75], &[1, 11, 21, 31]);
}

#[test]
fn four_errors_checksum_only_long() {
    assert_corrects_long(
        &[
            LONG_DATA_LEN,
            LONG_DATA_LEN + 5,
            LONG_DATA_LEN + 10,
            LONG_DATA_LEN + 14,
        ],
        &[1, 11, 21, 31],
    );
}

#[test]
fn four_errors_mixed_long() {
    assert_corrects_long(&[5, 35, 65, LONG_DATA_LEN + 8], &[1, 11, 21, 31]);
}

// ---------------------------------------------------------------------------
// 6. 5 errors — uncorrectable
// ---------------------------------------------------------------------------

#[test]
fn five_errors_uncorrectable_regular() {
    let original = build_regular_codeword();
    let mut corrupted = original.clone();
    let positions: [usize; 5] = [0, 8, 16, 24, REGULAR_DATA_LEN + 6];
    let mags: [u8; 5] = [1, 2, 3, 4, 5];
    for (&p, &m) in positions.iter().zip(&mags) {
        corrupted[p] ^= m;
    }
    let result = bch_correct_regular(HRP, &corrupted);
    assert!(
        matches!(result, Err(md_codec::Error::BchUncorrectable)),
        "5-error regular input should be uncorrectable, got {:?}",
        result.map(|r| r.corrections_applied)
    );
}

#[test]
fn five_errors_uncorrectable_long() {
    let original = build_long_codeword();
    let mut corrupted = original.clone();
    let positions: [usize; 5] = [0, 20, 40, 60, LONG_DATA_LEN + 8];
    let mags: [u8; 5] = [1, 2, 3, 4, 5];
    for (&p, &m) in positions.iter().zip(&mags) {
        corrupted[p] ^= m;
    }
    let result = bch_correct_long(HRP, &corrupted);
    assert!(
        matches!(result, Err(md_codec::Error::BchUncorrectable)),
        "5-error long input should be uncorrectable, got {:?}",
        result.map(|r| r.corrections_applied)
    );
}

// ---------------------------------------------------------------------------
// 7. Round-trip property tests
// ---------------------------------------------------------------------------

/// Apply the same Fisher-Yates-style corruption used by
/// `tests/common::corrupt_n` but operating on raw 5-bit symbol arrays
/// (this test layer doesn't pass through the WDM string layer).
fn corrupt_random_n(codeword: &mut [u8], n: usize, rng: &mut StdRng) -> Vec<usize> {
    assert!(n <= codeword.len());
    let mut indices: Vec<usize> = (0..codeword.len()).collect();
    let mut chosen = Vec::with_capacity(n);
    for i in 0..n {
        let remaining = codeword.len() - i;
        let j = rng.random_range(0..remaining);
        chosen.push(indices[j]);
        indices.swap(j, remaining - 1);
    }
    for &p in &chosen {
        // Pick a non-zero error magnitude (1..=31).
        let m: u8 = rng.random_range(1..32);
        codeword[p] ^= m;
    }
    chosen.sort_unstable();
    chosen
}

fn property_test_regular(n_errors: usize, iterations: usize) {
    let mut rng = StdRng::seed_from_u64(0xDEAD_BEEF);
    let original = build_regular_codeword();
    for iter in 0..iterations {
        let mut corrupted = original.clone();
        let injected = corrupt_random_n(&mut corrupted, n_errors, &mut rng);
        let r = bch_correct_regular(HRP, &corrupted).unwrap_or_else(|e| {
            panic!(
                "regular: iter {} with {} errors at {:?} failed: {}",
                iter, n_errors, injected, e
            )
        });
        assert_eq!(
            r.corrections_applied, n_errors,
            "iter {}: wrong correction count",
            iter
        );
        assert_eq!(
            r.data, original,
            "iter {}: wrong recovered codeword (positions {:?})",
            iter, injected
        );
    }
}

fn property_test_long(n_errors: usize, iterations: usize) {
    let mut rng = StdRng::seed_from_u64(0xDEAD_BEEF);
    let original = build_long_codeword();
    for iter in 0..iterations {
        let mut corrupted = original.clone();
        let injected = corrupt_random_n(&mut corrupted, n_errors, &mut rng);
        let r = bch_correct_long(HRP, &corrupted).unwrap_or_else(|e| {
            panic!(
                "long: iter {} with {} errors at {:?} failed: {}",
                iter, n_errors, injected, e
            )
        });
        assert_eq!(
            r.corrections_applied, n_errors,
            "iter {}: wrong correction count",
            iter
        );
        assert_eq!(
            r.data, original,
            "iter {}: wrong recovered codeword (positions {:?})",
            iter, injected
        );
    }
}

#[test]
fn property_one_error_round_trip_regular() {
    property_test_regular(1, 100);
}

#[test]
fn property_two_errors_round_trip_regular() {
    property_test_regular(2, 100);
}

#[test]
fn property_three_errors_round_trip_regular() {
    property_test_regular(3, 100);
}

#[test]
fn property_four_errors_round_trip_regular() {
    property_test_regular(4, 100);
}

#[test]
fn property_one_error_round_trip_long() {
    property_test_long(1, 100);
}

#[test]
fn property_two_errors_round_trip_long() {
    property_test_long(2, 100);
}

#[test]
fn property_three_errors_round_trip_long() {
    property_test_long(3, 100);
}

#[test]
fn property_four_errors_round_trip_long() {
    property_test_long(4, 100);
}
