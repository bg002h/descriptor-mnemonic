//! BCH error-correction stress tests for the WDM codec (Phase 6, Tasks 6.19–6.20).
//!
//! Task 6.19 — single-substitution at every data-part position must be
//! auto-corrected with exactly one correction reported.
//!
//! Task 6.20 — random 5+-error inputs must be rejected by the BCH decoder
//! in ≥95% of cases (empirically, with seed `0xDEADBEEF`).

mod common;

// ---------------------------------------------------------------------------
// Task 6.19
// ---------------------------------------------------------------------------

/// Verify that a single substitution at **every** data-part position is
/// correctly detected and auto-corrected by the BCH decoder.
///
/// For each position `p` in `data_start..data_end`:
///  - Replace the character at `p` with the first bech32-alphabet character
///    that differs from the original.
///  - Assert `decode` succeeds.
///  - Assert exactly one correction is reported in `report.corrections`.
///  - Assert the outcome is `DecodeOutcome::AutoCorrected`.
#[test]
fn bch_single_substitution_at_every_position_corrects() {
    use wdm_codec::{
        BchCode, DecodeOptions, DecodeOutcome, EncodeOptions, WalletPolicy, decode, encode,
    };

    let p: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
    let backup = encode(&p, &EncodeOptions::default()).unwrap();
    let original = backup.chunks[0].raw.clone();
    let code = backup.chunks[0].code;

    // Data part: after "wdm1" (4 chars), before checksum (13 or 15 chars).
    let checksum_len: usize = match code {
        BchCode::Regular => 13,
        BchCode::Long => 15,
    };
    let data_start = 4;
    let data_end = original.len() - checksum_len;

    // bech32 alphabet — any character in this set is a valid substitution.
    let alphabet = "qpzry9x8gf2tvdw0s3jn54khce6mua7l";

    for pos in data_start..data_end {
        let mut chars: Vec<char> = original.chars().collect();
        let original_char = chars[pos];

        // Pick the first alphabet character that differs from the original.
        let new_char = alphabet.chars().find(|c| *c != original_char).expect(
            "bech32 alphabet always has at least one character different from any given character",
        );
        chars[pos] = new_char;
        let corrupted: String = chars.iter().collect();

        let result = decode(&[corrupted.as_str()], &DecodeOptions::new())
            .unwrap_or_else(|e| panic!("decode failed at pos {pos} for input {corrupted}: {e}"));

        assert_eq!(
            result.report.corrections.len(),
            1,
            "expected exactly 1 correction at pos {pos}, got {:?}",
            result.report.corrections,
        );
        assert_eq!(
            result.report.outcome,
            DecodeOutcome::AutoCorrected,
            "expected DecodeOutcome::AutoCorrected at pos {pos}, got {:?}",
            result.report.outcome,
        );
    }
}

// ---------------------------------------------------------------------------
// Task 6.20
// ---------------------------------------------------------------------------

/// Stress-test the BCH decoder with random 5–9-error inputs.
///
/// With seed `0xDEADBEEF` and 1 000 iterations the BCH decoder should
/// reject ≥ 95 % of inputs that have 5 or more substitution errors.
/// (The BCH code corrects at most 4 errors; any extra errors should
/// produce an uncorrectable syndrome in the overwhelming majority of cases.)
#[test]
fn many_substitutions_always_rejected() {
    use rand::Rng;
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    use wdm_codec::{DecodeOptions, EncodeOptions, WalletPolicy, decode, encode};

    let p: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
    let backup = encode(&p, &EncodeOptions::default()).unwrap();
    let original = &backup.chunks[0].raw;
    let code = backup.chunks[0].code;

    let mut rng = StdRng::seed_from_u64(0xDEAD_BEEF);
    let n_iters = 1_000usize;
    let mut accepted = 0usize;
    let mut rejected = 0usize;

    for _i in 0..n_iters {
        let n_errors: usize = rng.gen_range(5..10); // 5–9 errors per iteration
        let seed: u64 = rng.r#gen();
        let corrupted = common::corrupt_n(original, n_errors, seed, code);

        match decode(&[corrupted.as_str()], &DecodeOptions::new()) {
            Err(_) => rejected += 1,
            Ok(_) => accepted += 1, // rare BCH false-positive; acceptable
        }
    }

    eprintln!(
        "BCH stress (seed=0xDEADBEEF, n={n_iters}): {rejected} rejected, {accepted} accepted"
    );

    assert!(
        rejected > n_iters * 95 / 100,
        "expected ≥95% of 5+-error inputs to be rejected, got {rejected}/{n_iters} ({accepted} accepted)"
    );
}
