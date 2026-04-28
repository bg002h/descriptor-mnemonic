//! Top-level decode pipeline: one or more codex32-derived MD strings → [`WalletPolicy`].
//!
//! # Pipeline overview
//!
//! 1. **Per-string parse** — validate HRP, detect BCH code variant, reject
//!    mixed-case and invalid-length strings.
//! 2. **BCH validate + correct** — call [`decode_string`] per string; collect
//!    any auto-corrections.
//! 3. **Header parse** — decode each string's header bytes via
//!    [`ChunkHeader::from_bytes`]; reconstitute [`Chunk`] values.
//! 4. **Reassembly** — for single-string: fragment IS the bytecode; for
//!    chunked: [`reassemble_chunks`] performs all 7 BIP §"Reassembly"
//!    validations and strips the cross-chunk hash.
//! 5. **Bytecode decode** — [`WalletPolicy::from_bytecode`] parses the
//!    canonical bytecode.
//! 6. **Report** — populate [`Verifications`], determine [`Confidence`], build
//!    [`DecodeReport`] and [`DecodeResult`].
//!
//! # Error vs. report flow
//!
//! v0.1 always returns `Err(Error::*)` when any stage fails.  The
//! [`Confidence::Failed`] and [`DecodeOutcome::Failed`] variants exist for
//! future v0.3 guided-recovery paths where partial recovery can still yield a
//! low-confidence policy; they are **never produced** by this function in v0.1.
//!
//! # DecodeOptions
//!
//! `DecodeOptions::erasures` is reserved for v0.3 erasure decoding and is
//! silently ignored in v0.1.

use crate::{
    BchCode, Chunk, ChunkHeader, Confidence, DecodeOptions, DecodeOutcome, DecodeReport,
    DecodeResult, Error, Verifications, WalletPolicy,
    chunking::{Correction, reassemble_chunks},
    encoding::{decode_string, five_bit_to_bytes},
    error::BytecodeErrorKind,
};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Decode a list of codex32-derived MD backup strings into a wallet policy.
///
/// `strings` must be either:
/// - A single-element slice containing a single-string backup, or
/// - A slice of all chunks belonging to one chunked backup (in any order).
///
/// The [`DecodeOptions`] type has no public fields in v0.1; pass
/// `&DecodeOptions::new()` for default behavior.  The `erasures` field is
/// reserved for v0.3 guided recovery and is silently ignored here.
///
/// # Errors
///
/// Returns `Err(Error::*)` for any hard failure:
///
/// | Stage | Errors |
/// |-------|--------|
/// | 1 – parse | `InvalidHrp`, `MixedCase`, `InvalidStringLength`, `InvalidChar` |
/// | 2 – BCH | `BchUncorrectable` |
/// | 3 – header | `ChunkHeaderTruncated`, `UnsupportedVersion`, `UnsupportedCardType`, `ReservedWalletIdBitsSet`, `InvalidChunkCount`, `InvalidChunkIndex` |
/// | 4 – reassembly | `EmptyChunkList`, `MixedChunkTypes`, `SingleStringWithMultipleChunks`, `WalletIdMismatch`, `TotalChunksMismatch`, `ChunkIndexOutOfRange`, `DuplicateChunkIndex`, `MissingChunkIndex`, `CrossChunkHashMismatch` |
/// | 5 – bytecode | `InvalidBytecode`, `UnsupportedVersion`, `PolicyScopeViolation`, `FingerprintsCountMismatch` |
///
/// # v0.1 confidence levels produced
///
/// - `Confirmed` — zero BCH corrections, all verifications `true`.
/// - `High` — some BCH auto-corrections applied, all verifications still `true`.
/// - `Probabilistic` and `Failed` are **never produced** in v0.1.
///
/// # Note on `char_position` in corrections
///
/// Note on [`Correction.char_position`][crate::Correction::char_position]:
/// when this function reports BCH corrections in the [`DecodeReport`], each
/// `Correction.char_position` is a 0-indexed offset into the chunk's
/// data part (the chars after the `md1` HRP+separator). This matches the
/// coordinate system used by the encoding layer's `decode_string`.
pub fn decode(strings: &[&str], _options: &DecodeOptions) -> Result<DecodeResult, Error> {
    // Stage 1 + 2: per-string parse and BCH validate/correct.
    // `decode_string` handles HRP check, case check, length check, and BCH correction.
    // Collect corrections across all strings.

    let mut all_corrections: Vec<Correction> = Vec::new();

    // Stage 2 output: one (decoded_5bit_data, bch_code) per input string.
    let mut decoded_strings: Vec<(Vec<u8>, BchCode)> = Vec::with_capacity(strings.len());

    for (chunk_idx, &s) in strings.iter().enumerate() {
        let decoded = decode_string(s)?;

        // Translate any BCH corrections from DecodedString's internal positions
        // to the public Correction type. corrected_positions are 0-indexed into
        // the data_with_checksum slice (i.e. after "md1"); we map them to
        // char_position within the data part. original/corrected chars are read
        // from the alphabet.
        if decoded.corrections_applied > 0 {
            // The original string (lowercased) data part starts after "md1" (len=3).
            let s_lower = s.to_lowercase();
            let data_part_start = s_lower.rfind('1').map(|p| p + 1).unwrap_or(4);
            let data_chars: Vec<char> = s_lower[data_part_start..].chars().collect();

            for &pos in &decoded.corrected_positions {
                // `pos` is an index into the data-part 5-bit values, in the
                // same coordinate system as `data_chars` (chars after `"md1"`).
                // It may land in the data region (`pos < decoded.data.len()`)
                // OR in the checksum region — both cases are answered uniformly
                // by `DecodedString::corrected_char_at`, which retains the full
                // post-correction symbol sequence (data + checksum).
                let original_char = if pos < data_chars.len() {
                    data_chars[pos]
                } else {
                    '?'
                };
                let corrected_char = decoded.corrected_char_at(pos);
                all_corrections.push(Correction {
                    chunk_index: chunk_idx as u8,
                    char_position: pos,
                    original: original_char,
                    corrected: corrected_char,
                });
            }
        }

        decoded_strings.push((decoded.data, decoded.code));
    }

    // Stage 3: header parse — convert 5-bit data → bytes → Chunk.
    let mut chunks: Vec<Chunk> = Vec::with_capacity(decoded_strings.len());

    for (data_5bit, _code) in decoded_strings {
        // `five_bit_to_bytes` returns None when the input bit length is not a
        // multiple of 8 — i.e., a non-zero trailing pad bit after the byte
        // boundary. For ENCODER-PRODUCED inputs this is structurally impossible
        // (the encoder always pads with zero). For HOSTILE inputs, however,
        // the BCH layer will validate any input that satisfies the polymod —
        // including a Long-code data part (93 5-bit symbols = 465 bits) whose
        // final symbol carries a non-zero low bit. The audit at
        // `design/agent-reports/v0-2-1-full-code-audit.md` reproduced this
        // case (was a panic via `expect()` in v0.2.0/v0.2.1; v0.2.2 returns
        // a structured error).
        let bytes = five_bit_to_bytes(&data_5bit).ok_or(Error::InvalidBytecode {
            offset: 0,
            kind: BytecodeErrorKind::MalformedPayloadPadding,
        })?;
        let (header, header_len) = ChunkHeader::from_bytes(&bytes)?;
        let fragment = bytes[header_len..].to_vec();
        chunks.push(Chunk { header, fragment });
    }

    // Stage 4: reassembly.
    // `reassemble_chunks` handles all 7 BIP §"Reassembly" validations.
    // For single-string: returns fragment directly. For chunked: strips 4-byte hash.
    let bytecode = reassemble_chunks(chunks)?;

    // After reassembly succeeded, all cross-chunk verifications are satisfied.
    let cross_chunk_hash_ok = true;
    let wallet_id_consistent = true;
    let total_chunks_consistent = true;

    // Stage 5: bytecode decode (Phase E — also recovers the optional
    // fingerprints block when header bit 2 is set).
    let (policy, fingerprints) = WalletPolicy::from_bytecode_with_fingerprints(&bytecode)?;

    // After successful from_bytecode, the bytecode is well-formed and version is supported.
    let bytecode_well_formed = true;
    let version_supported = true;

    // Stage 6: build report.
    let verifications = Verifications {
        cross_chunk_hash_ok,
        wallet_id_consistent,
        total_chunks_consistent,
        bytecode_well_formed,
        version_supported,
    };

    let (outcome, confidence) = if all_corrections.is_empty() {
        (DecodeOutcome::Clean, Confidence::Confirmed)
    } else {
        (DecodeOutcome::AutoCorrected, Confidence::High)
    };

    let report = DecodeReport {
        outcome,
        corrections: all_corrections,
        verifications,
        confidence,
    };

    Ok(DecodeResult {
        policy,
        report,
        fingerprints,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DecodeOptions, EncodeOptions, WalletPolicy, chunking::ChunkingMode, encode::encode,
        wallet_id::WalletIdSeed,
    };

    fn policy(s: &str) -> WalletPolicy {
        s.parse().expect("policy parse")
    }

    fn default_opts() -> DecodeOptions {
        DecodeOptions::new()
    }

    fn encode_opts() -> EncodeOptions {
        EncodeOptions::default()
    }

    fn force_chunked_mode_opts() -> EncodeOptions {
        EncodeOptions::default().with_force_chunking(true)
    }

    // -----------------------------------------------------------------------
    // 1. decode_round_trip_single_string_regular
    // -----------------------------------------------------------------------

    #[test]
    fn decode_round_trip_single_string_regular() {
        let p = policy("wsh(pk(@0/**))");
        let backup = encode(&p, &encode_opts()).expect("encode");
        assert_eq!(backup.chunks.len(), 1);

        let raw = backup.chunks[0].raw.as_str();
        let result = decode(&[raw], &default_opts()).expect("decode");
        // Compare canonical forms.
        assert_eq!(
            result.policy.to_canonical_string(),
            p.to_canonical_string(),
            "decoded policy must match original"
        );
    }

    // -----------------------------------------------------------------------
    // 2. decode_round_trip_single_string_long_via_force
    // -----------------------------------------------------------------------

    #[test]
    fn decode_round_trip_single_string_long_via_force() {
        let p = policy("wsh(pk(@0/**))");
        let opts = EncodeOptions {
            force_long_code: true,
            ..Default::default()
        };
        let backup = encode(&p, &opts).expect("encode");
        assert_eq!(backup.chunks.len(), 1);
        assert_eq!(backup.chunks[0].code, BchCode::Long);

        let raw = backup.chunks[0].raw.as_str();
        let result = decode(&[raw], &default_opts()).expect("decode");
        assert_eq!(result.policy.to_canonical_string(), p.to_canonical_string());
    }

    // -----------------------------------------------------------------------
    // 3. decode_round_trip_chunked_two_chunks
    // -----------------------------------------------------------------------

    #[test]
    fn decode_round_trip_chunked_two_chunks() {
        // Use force_chunking to guarantee the Chunked encoding path is exercised
        // (Chunked header + cross-chunk hash) regardless of encoder details.
        // The sha256 terminal embeds a 32-byte hash, driving the bytecode well
        // above the Regular single-chunk fragment capacity (45 bytes), so ≥2
        // physical chunks are produced deterministically.
        let p = policy(
            "wsh(and_v(v:pk(@0/**),sha256(1111111111111111111111111111111111111111111111111111111111111111)))",
        );

        let backup = encode(&p, &force_chunked_mode_opts()).expect("encode");
        assert!(
            backup.chunks.len() >= 2,
            "expected ≥2 chunks for sha256 policy under force_chunking, got {}",
            backup.chunks.len()
        );

        let raws: Vec<&str> = backup.chunks.iter().map(|c| c.raw.as_str()).collect();
        let result = decode(&raws, &default_opts()).expect("decode");
        assert_eq!(result.policy.to_canonical_string(), p.to_canonical_string());
    }

    // -----------------------------------------------------------------------
    // 4. decode_round_trip_chunked_with_seed
    // -----------------------------------------------------------------------

    #[test]
    fn decode_round_trip_chunked_with_seed() {
        let p = policy("wsh(pk(@0/**))");
        let opts = EncodeOptions {
            chunking_mode: ChunkingMode::ForceChunked,
            wallet_id_seed: Some(WalletIdSeed::from(0xDEAD_BEEFu32)),
            ..Default::default()
        };
        let backup = encode(&p, &opts).expect("encode");

        let raws: Vec<&str> = backup.chunks.iter().map(|c| c.raw.as_str()).collect();
        let result = decode(&raws, &default_opts()).expect("decode");
        // Seed only affects chunk-header wallet_id; recovered policy must still match.
        assert_eq!(result.policy.to_canonical_string(), p.to_canonical_string());
    }

    // -----------------------------------------------------------------------
    // 5. decode_round_trip_with_sha256_terminal
    // -----------------------------------------------------------------------

    #[test]
    fn decode_round_trip_with_sha256_terminal() {
        // sha256() terminal proves the upstream patch is wired through the pipeline.
        let p = policy(
            "wsh(and_v(v:pk(@0/**),sha256(1111111111111111111111111111111111111111111111111111111111111111)))",
        );
        let backup = encode(&p, &encode_opts()).expect("encode");

        let raws: Vec<&str> = backup.chunks.iter().map(|c| c.raw.as_str()).collect();
        let result = decode(&raws, &default_opts()).expect("decode");
        assert_eq!(result.policy.to_canonical_string(), p.to_canonical_string());
    }

    // -----------------------------------------------------------------------
    // 6. decode_rejects_empty_input
    // -----------------------------------------------------------------------

    #[test]
    fn decode_rejects_empty_input() {
        let err = decode(&[], &default_opts()).expect_err("should reject empty input");
        assert!(
            matches!(err, Error::EmptyChunkList),
            "expected EmptyChunkList, got {err:?}"
        );
    }

    // -----------------------------------------------------------------------
    // 7. decode_rejects_invalid_hrp
    // -----------------------------------------------------------------------

    #[test]
    fn decode_rejects_invalid_hrp() {
        // A valid bech32 string with HRP "bc" instead of "md".
        // We encode a valid MD string and replace "md1" with "bc1q" prefix.
        // Instead, just construct a well-known Bitcoin bech32 address.
        let segwit = "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4";
        let err = decode(&[segwit], &default_opts()).expect_err("should reject invalid HRP");
        assert!(
            matches!(err, Error::InvalidHrp(_)),
            "expected InvalidHrp, got {err:?}"
        );
    }

    // -----------------------------------------------------------------------
    // 8. decode_rejects_mixed_case
    // -----------------------------------------------------------------------

    #[test]
    fn decode_rejects_mixed_case() {
        let p = policy("wsh(pk(@0/**))");
        let backup = encode(&p, &encode_opts()).expect("encode");
        let raw = &backup.chunks[0].raw;

        // Uppercase one character in the data part (after "md1").
        let mut chars: Vec<char> = raw.chars().collect();
        // Position 5 is in the data part (index 0..2 = "md1").
        chars[5] = chars[5].to_ascii_uppercase();
        let bad: String = chars.into_iter().collect();

        let err = decode(&[bad.as_str()], &default_opts()).expect_err("should reject mixed case");
        assert!(
            matches!(err, Error::MixedCase),
            "expected MixedCase, got {err:?}"
        );
    }

    // -----------------------------------------------------------------------
    // 9. decode_rejects_chunks_with_different_wallet_ids
    // -----------------------------------------------------------------------

    #[test]
    fn decode_rejects_chunks_with_different_wallet_ids() {
        // Encode two different policies with force_chunking, then try to decode
        // a chunk from each — they will have different wallet_ids.
        let p_a = policy("wsh(pk(@0/**))");
        let p_b = policy("wsh(pk(@0/**))");

        let opts_a = EncodeOptions {
            chunking_mode: ChunkingMode::ForceChunked,
            wallet_id_seed: Some(WalletIdSeed::from(0x1111_1111u32)),
            ..Default::default()
        };
        let opts_b = EncodeOptions {
            chunking_mode: ChunkingMode::ForceChunked,
            wallet_id_seed: Some(WalletIdSeed::from(0x2222_2222u32)),
            ..Default::default()
        };

        let backup_a = encode(&p_a, &opts_a).expect("encode a");
        let backup_b = encode(&p_b, &opts_b).expect("encode b");

        let raw_a = backup_a.chunks[0].raw.as_str();
        let raw_b = backup_b.chunks[0].raw.as_str();

        let err = decode(&[raw_a, raw_b], &default_opts())
            .expect_err("should reject chunks with different wallet_ids");
        assert!(
            matches!(err, Error::WalletIdMismatch { .. }),
            "expected WalletIdMismatch, got {err:?}"
        );
    }

    // -----------------------------------------------------------------------
    // 10. decode_rejects_chunks_with_duplicate_indices
    // -----------------------------------------------------------------------

    #[test]
    fn decode_rejects_chunks_with_duplicate_indices() {
        let p = policy("wsh(multi(5,@0/**,@1/**,@2/**,@3/**,@4/**,@5/**,@6/**,@7/**,@8/**))");
        let backup = encode(&p, &force_chunked_mode_opts()).expect("encode");
        let raw0 = backup.chunks[0].raw.as_str();
        // Pass chunk 0 twice → duplicate index 0.
        let err = decode(&[raw0, raw0], &default_opts())
            .expect_err("should reject duplicate chunk index");
        assert!(
            matches!(err, Error::DuplicateChunkIndex(0)),
            "expected DuplicateChunkIndex(0), got {err:?}"
        );
    }

    // -----------------------------------------------------------------------
    // 11. decode_rejects_chunks_with_missing_index
    // -----------------------------------------------------------------------

    #[test]
    fn decode_rejects_chunks_with_missing_index() {
        // We need a policy that produces ≥3 chunks so we can omit the middle one.
        // Use a large multi-sig policy and force-chunking if needed.
        // multi(9,@0..@8) in wsh should produce ~3 chunks with Regular code.
        let p = policy("wsh(multi(9,@0/**,@1/**,@2/**,@3/**,@4/**,@5/**,@6/**,@7/**,@8/**))");
        let backup = encode(&p, &encode_opts()).expect("encode");

        if backup.chunks.len() < 3 {
            // Not enough chunks; skip (the test is optional per the prompt).
            return;
        }

        // Pass chunks 0 and 2, skip chunk 1 → MissingChunkIndex(1).
        let raw0 = backup.chunks[0].raw.as_str();
        let raw2 = backup.chunks[2].raw.as_str();
        let err =
            decode(&[raw0, raw2], &default_opts()).expect_err("should reject missing chunk index");
        assert!(
            matches!(err, Error::MissingChunkIndex(1)),
            "expected MissingChunkIndex(1), got {err:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Shared helper for tests 13 and 14.
    // Encodes a small policy and decodes it, returning the DecodeResult.
    // -----------------------------------------------------------------------

    fn happy_path_decode() -> DecodeResult {
        let p = policy("wsh(pk(@0/**))");
        let backup = encode(&p, &encode_opts()).expect("encode");
        let raw = backup.chunks[0].raw.clone();
        decode(&[raw.as_str()], &default_opts()).expect("decode")
    }

    // -----------------------------------------------------------------------
    // 13. decode_report_outcome_clean_when_no_corrections
    // -----------------------------------------------------------------------

    #[test]
    fn decode_report_outcome_clean_when_no_corrections() {
        let result = happy_path_decode();
        assert_eq!(result.report.outcome, DecodeOutcome::Clean);
        assert_eq!(result.report.confidence, Confidence::Confirmed);
        assert!(
            result.report.corrections.is_empty(),
            "no corrections expected for a clean encode"
        );
    }

    // -----------------------------------------------------------------------
    // 14. decode_report_verifications_all_true_on_happy_path
    // -----------------------------------------------------------------------

    #[test]
    fn decode_report_verifications_all_true_on_happy_path() {
        let result = happy_path_decode();
        assert!(result.report.verifications.cross_chunk_hash_ok);
        assert!(result.report.verifications.wallet_id_consistent);
        assert!(result.report.verifications.total_chunks_consistent);
        assert!(result.report.verifications.bytecode_well_formed);
        assert!(result.report.verifications.version_supported);
    }

    // -----------------------------------------------------------------------
    // Bonus: chunked clean decode also yields Confirmed report
    // -----------------------------------------------------------------------

    #[test]
    fn decode_report_chunked_clean_confirmed() {
        let p = policy("wsh(multi(5,@0/**,@1/**,@2/**,@3/**,@4/**,@5/**,@6/**,@7/**,@8/**))");

        // force_chunking guarantees a multi-chunk backup regardless of encoder details.
        let backup = encode(&p, &force_chunked_mode_opts()).expect("encode");
        let raws: Vec<&str> = backup.chunks.iter().map(|c| c.raw.as_str()).collect();
        let result = decode(&raws, &default_opts()).expect("decode");

        assert_eq!(result.report.outcome, DecodeOutcome::Clean);
        assert_eq!(result.report.confidence, Confidence::Confirmed);
        assert!(result.report.corrections.is_empty());
        assert!(result.report.verifications.cross_chunk_hash_ok);
        assert!(result.report.verifications.wallet_id_consistent);
        assert!(result.report.verifications.total_chunks_consistent);
        assert!(result.report.verifications.bytecode_well_formed);
        assert!(result.report.verifications.version_supported);
    }

    // -----------------------------------------------------------------------
    // 15. decode_correction_in_data_region_reports_real_corrected_char
    // -----------------------------------------------------------------------
    //
    // Closes followup `5e-checksum-correction-fallback`: data-region path.
    //
    // BCH ECC corrections inside the *data* region (i.e. before the checksum)
    // have always reported the actual corrected character because
    // `DecodedString.data` retained the corrected 5-bit symbol at that index.
    // This test pins that historical behaviour so the parallel checksum-region
    // fix doesn't regress it.

    #[test]
    fn decode_correction_in_data_region_reports_real_corrected_char() {
        // Use the smallest valid single-string policy so we can find the
        // data-region boundary deterministically.
        let p = policy("wsh(pk(@0/**))");
        let backup = encode(&p, &encode_opts()).expect("encode");
        let raw = backup.chunks[0].raw.clone();

        // Compute the data-region boundary: chars 0..2 are "md", char 2 is
        // the '1' separator, chars 3.. are the data part. The data part is
        // [data_5bit_symbols][13-char checksum] for the regular code.
        let total_chars = raw.chars().count();
        let data_part_len = total_chars - 3; // strip "md1"
        let checksum_len = 13; // regular code
        let data_region_len = data_part_len - checksum_len;
        assert!(
            data_region_len >= 1,
            "test fixture must have a non-empty data region"
        );

        // Pick a position inside the data region (offset 2 from start of data
        // part, well clear of the checksum boundary).
        let data_part_pos = 2;
        assert!(data_part_pos < data_region_len);
        let abs_pos = 3 + data_part_pos; // position in the full string (HRP "md" + "1" = 3 chars)

        let mut chars: Vec<char> = raw.chars().collect();
        let original_char = chars[abs_pos];
        // Flip to a different bech32 alphabet character.
        let new_char = if original_char == 'q' { 'p' } else { 'q' };
        chars[abs_pos] = new_char;
        let corrupted: String = chars.into_iter().collect();

        let result = decode(&[corrupted.as_str()], &default_opts()).expect("decode");
        assert_eq!(
            result.report.corrections.len(),
            1,
            "expected exactly one correction"
        );
        let c = &result.report.corrections[0];
        assert_eq!(c.char_position, data_part_pos);
        assert_eq!(
            c.original, new_char,
            "original should be the corrupted char"
        );
        assert_eq!(
            c.corrected, original_char,
            "corrected char must equal the actual restored character (data-region)"
        );
    }

    // -----------------------------------------------------------------------
    // 16. decode_correction_in_checksum_region_reports_real_corrected_char
    // -----------------------------------------------------------------------
    //
    // Closes followup `5e-checksum-correction-fallback`.
    //
    // Before the v0.2 fix, BCH corrections that landed inside the 13-char
    // checksum region reported `Correction.corrected = 'q'` (= `ALPHABET[0]`)
    // as a placeholder because `DecodedString.data` had the checksum stripped
    // off, and the decode path had no way to look up the corrected symbol.
    // After the fix, `DecodedString::corrected_char_at` exposes the post-BCH
    // character at any data-part position, so the reported `corrected` is now
    // the actual restored character — even for corruptions inside the
    // checksum region.

    #[test]
    fn decode_correction_in_checksum_region_reports_real_corrected_char() {
        let p = policy("wsh(pk(@0/**))");
        let backup = encode(&p, &encode_opts()).expect("encode");
        let raw = backup.chunks[0].raw.clone();

        let total_chars = raw.chars().count();
        let data_part_len = total_chars - 3; // strip "md1" (HRP "md" + "1" = 3 chars)
        let checksum_len = 13;
        let data_region_len = data_part_len - checksum_len;

        // Pick a position inside the checksum region. Use the middle of the
        // 13-char checksum so we're clear of any boundary effects.
        let data_part_pos = data_region_len + (checksum_len / 2);
        let abs_pos = 3 + data_part_pos; // HRP "md" + "1" = 3 chars

        let mut chars: Vec<char> = raw.chars().collect();
        let original_char = chars[abs_pos];
        let new_char = if original_char == 'q' { 'p' } else { 'q' };
        chars[abs_pos] = new_char;
        let corrupted: String = chars.into_iter().collect();

        let result = decode(&[corrupted.as_str()], &default_opts()).expect("decode");
        assert_eq!(
            result.report.corrections.len(),
            1,
            "expected exactly one correction"
        );
        let c = &result.report.corrections[0];
        assert_eq!(c.char_position, data_part_pos);
        assert_eq!(c.original, new_char);
        assert_eq!(
            c.corrected, original_char,
            "corrected char must be the real restored character, not the legacy 'q' placeholder"
        );
        // Belt-and-braces: explicitly verify we are not reporting the historic
        // placeholder 'q' (unless 'q' happens to be the genuine original char).
        if original_char != 'q' {
            assert_ne!(
                c.corrected, 'q',
                "checksum-region correction must not fall back to ALPHABET[0]='q' placeholder"
            );
        }
    }
}
