//! Shared test helpers for the MD codec integration-test suite.
//!
//! Used by all Phase 6 bucket files via `mod common;` at the top of each
//! test file.  Every helper in this module is `pub` so bucket files can
//! reference it without qualification issues.
//!
//! # Helpers
//!
//! - [`round_trip_assert`] — encode → decode and verify structural equality.
//! - [`assert_structural_eq`] — compare two [`WalletPolicy`] values by
//!   canonical string form.
//! - [`corrupt_n`] — introduce exactly `n` deterministic substitution errors
//!   into a MD codex32 string.
//! - [`load_vector`] — stub for Phase 8 test-vector loading (always panics
//!   in v0.1).

#![allow(dead_code)] // not every bucket uses every helper

use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;

use md_codec::{BchCode, Confidence, DecodeOptions, EncodeOptions, WalletPolicy, decode, encode};

// ---------------------------------------------------------------------------
// round_trip_assert
// ---------------------------------------------------------------------------

/// Round-trip a policy through `encode` → `decode` and assert the
/// recovered policy is structurally equal to the original.
///
/// Uses default [`EncodeOptions`] and [`DecodeOptions`]. For tests that
/// need non-default options (chunking_mode, policy_id_seed), use the
/// `encode`/`decode` API directly.
pub fn round_trip_assert(policy_str: &str) {
    let policy: WalletPolicy = policy_str.parse().unwrap_or_else(|e| {
        panic!(
            "round_trip_assert: failed to parse policy {:?}: {}",
            policy_str, e
        )
    });

    let backup = encode(&policy, &EncodeOptions::default()).unwrap_or_else(|e| {
        panic!(
            "round_trip_assert: encode failed for {:?}: {}",
            policy_str, e
        )
    });

    let raw_strings: Vec<&str> = backup.chunks.iter().map(|c| c.raw.as_str()).collect();

    let decode_result = decode(&raw_strings, &DecodeOptions::new()).unwrap_or_else(|e| {
        panic!(
            "round_trip_assert: decode failed for {:?}: {}",
            policy_str, e
        )
    });

    assert_eq!(
        decode_result.report.confidence,
        Confidence::Confirmed,
        "round_trip_assert: expected Confidence::Confirmed for {:?}, got {:?}",
        policy_str,
        decode_result.report.confidence,
    );

    assert_structural_eq(&policy, &decode_result.policy);
}

// ---------------------------------------------------------------------------
// assert_structural_eq
// ---------------------------------------------------------------------------

/// Assert two [`WalletPolicy`] values are structurally equal — same
/// canonical-string form. Compares via `to_canonical_string()` rather
/// than struct internals (which include dummy keys after a bytecode
/// round-trip).
pub fn assert_structural_eq(a: &WalletPolicy, b: &WalletPolicy) {
    assert_eq!(
        a.to_canonical_string(),
        b.to_canonical_string(),
        "assert_structural_eq: policies differ\n  left:  {}\n  right: {}",
        a.to_canonical_string(),
        b.to_canonical_string(),
    );
}

// ---------------------------------------------------------------------------
// corrupt_n
// ---------------------------------------------------------------------------

/// The bech32 alphabet in 5-bit-value order (matches [`md_codec::encoding::ALPHABET`]).
const BECH32_ALPHABET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// Introduce exactly `n` substitution errors into a codex32-derived MD
/// string `s`, deterministically driven by `seed`. Used by ECC stress tests
/// to exercise BCH error correction.
///
/// The substitutions:
/// - Are uniformly distributed over the data part (after the `md1` prefix
///   and before the checksum).
/// - Replace each chosen character with another bech32-alphabet character
///   (never the same character as the original; never an out-of-alphabet
///   character).
/// - Are reproducible: same `seed` + same `s` + same `n` → same output.
///
/// `code` specifies the BCH code variant so the function can locate the
/// checksum boundary. Pass `backup.chunks[i].code`.
///
/// Returns the corrupted string.
///
/// # Panics
///
/// Panics if `n` exceeds the data-part length (there are not enough distinct
/// positions to corrupt), or if `s` does not start with `"md1"`.
pub fn corrupt_n(s: &str, n: usize, seed: u64, code: BchCode) -> String {
    assert!(
        s.starts_with("md1"),
        "corrupt_n: string does not start with 'md1': {:?}",
        s
    );

    // HRP "md" (2 chars) + separator "1" (1 char) = 3-char prefix.
    const PREFIX_LEN: usize = 3;
    let checksum_len = match code {
        BchCode::Regular => 13usize,
        BchCode::Long => 15usize,
    };

    // data_part = s[PREFIX_LEN .. s.len() - checksum_len]
    let total_len = s.len();
    assert!(
        total_len >= PREFIX_LEN + checksum_len,
        "corrupt_n: string too short to contain prefix + checksum (len={})",
        total_len,
    );
    let data_part_len = total_len - PREFIX_LEN - checksum_len;
    assert!(
        n <= data_part_len,
        "corrupt_n: n={} exceeds data-part length={} for string {:?}",
        n,
        data_part_len,
        s,
    );

    let mut chars: Vec<char> = s.chars().collect();
    let mut rng = StdRng::seed_from_u64(seed);

    // Pick `n` distinct positions within the data part using a
    // partial Fisher-Yates shuffle over an index range.
    let mut indices: Vec<usize> = (0..data_part_len).collect();
    let mut chosen_positions: Vec<usize> = Vec::with_capacity(n);
    for i in 0..n {
        let remaining = data_part_len - i;
        let j = rng.random_range(0..remaining);
        chosen_positions.push(indices[j]);
        indices.swap(j, remaining - 1);
    }

    // For each chosen position, substitute with a different bech32 char.
    for pos in chosen_positions {
        let char_idx = PREFIX_LEN + pos; // absolute index in `chars`
        let orig_char = chars[char_idx];

        // Find the alphabet index of orig_char (lowercase).
        let orig_lower = orig_char.to_ascii_lowercase();
        let orig_idx = BECH32_ALPHABET
            .iter()
            .position(|&b| b as char == orig_lower)
            .unwrap_or_else(|| {
                panic!(
                    "corrupt_n: character {:?} at position {} is not in the bech32 alphabet",
                    orig_char, char_idx
                )
            });

        // Pick a different alphabet index uniformly from the remaining 31.
        let pick = rng.random_range(0..31usize); // 0..=30 → maps to one of 31 other chars
        let new_idx = if pick < orig_idx { pick } else { pick + 1 };
        let new_char = BECH32_ALPHABET[new_idx] as char;

        // Preserve the case of the original character.
        chars[char_idx] = if orig_char.is_ascii_uppercase() {
            new_char.to_ascii_uppercase()
        } else {
            new_char
        };
    }

    chars.into_iter().collect()
}

// ---------------------------------------------------------------------------
// load_vector
// ---------------------------------------------------------------------------

/// Placeholder for Phase 8 test-vector loading.
///
/// The signature is fixed at Phase 6 so corpus tests can reference it,
/// but the body is a [`todo!`] until Phase 8 ships the JSON schema.
/// Calling this function in v0.1 will always panic with a clear message.
pub struct Vector; // Placeholder; replaced by crate::vectors::Vector in Phase 8

/// Load a test vector from `path` and return it parsed.
///
/// v0.1 implementation: not yet implemented. Always panics.
/// Real implementation lands in Phase 8 (Task 8.1+).
pub fn load_vector(_path: &str) -> Vector {
    todo!("test-vector loading lands in Phase 8 (Task 8.1+)");
}
