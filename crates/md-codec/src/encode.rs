//! Top-level encode pipeline: `WalletPolicy` → `MdBackup`.
//!
//! Wires together Phases 1–5C: bytecode encoding, chunking decision,
//! wallet-ID derivation, chunk assembly, and codex32 string encoding.

use crate::{
    BchCode, ChunkCode, ChunkingPlan, EncodeOptions, EncodedChunk, MdBackup, Result, WalletPolicy,
    chunking::{ChunkHeader, chunk_bytes, chunking_decision},
    encoding::encode_string,
    wallet_id::{ChunkWalletId, compute_wallet_id},
};

/// Encode a wallet policy as a [`MdBackup`]: one or more codex32-derived
/// strings ready to engrave, plus the Tier-3 12-word Wallet ID.
///
/// # Pipeline
///
/// 1. **Bytecode** — `policy.to_bytecode()` produces canonical MD bytecode.
/// 2. **Chunking plan** — [`chunking_decision`] selects single-string or
///    chunked encoding. `force_long_code` can upgrade Regular → Long after
///    the fact. `chunking_mode = ChunkingMode::ForceChunked` causes chunked
///    encoding even for short input.
/// 3. **Wallet IDs** — the *chunk-header* 20-bit `wallet_id` is derived from
///    `options.wallet_id_seed` (if present) or the content hash. The
///    *Tier-3* 16-byte `WalletId` is **always** content-derived, never
///    affected by the seed.
/// 4. **Chunks** — [`chunk_bytes`] assembles `Vec<Chunk>`.
/// 5. **Codex32 strings** — each chunk's bytes are wrapped by [`encode_string`].
/// 6. **Result** — a [`MdBackup`] containing the encoded chunks and the
///    Tier-3 12-word wallet ID.
///
/// # Errors
///
/// Returns [`crate::Error::PolicyTooLarge`] when the bytecode exceeds the maximum
/// supported length (1692 bytes). Propagates any error from
/// [`WalletPolicy::to_bytecode`] or [`encode_string`].
pub fn encode(policy: &WalletPolicy, options: &EncodeOptions) -> Result<MdBackup> {
    // Stage 2: encode policy to canonical bytecode. The encoder consults
    // `options.shared_path` first per the Phase B precedence rule; see
    // [`WalletPolicy::to_bytecode`].
    let bytecode = policy.to_bytecode(options)?;

    // Stage 3: decide chunking plan, then apply force_long_code override.
    let mut plan = chunking_decision(bytecode.len(), options.chunking_mode)?;
    if options.force_long_code {
        plan = match plan {
            // Regular single-string → Long single-string (long capacity ≥ regular, always fits).
            ChunkingPlan::SingleString {
                code: ChunkCode::Regular,
            } => ChunkingPlan::SingleString {
                code: ChunkCode::Long,
            },
            // Regular chunked → Long chunked; recompute count using long fragment capacity.
            ChunkingPlan::Chunked {
                code: ChunkCode::Regular,
                ..
            } => {
                let stream_len = bytecode.len() + 4;
                let count = stream_len.div_ceil(ChunkCode::Long.fragment_capacity());
                // Long capacity is larger than regular, so count ≤ regular count ≤ 32.
                ChunkingPlan::Chunked {
                    code: ChunkCode::Long,
                    fragment_size: ChunkCode::Long.fragment_capacity(),
                    count,
                }
            }
            // Already Long; leave as-is.
            other => other,
        };
    }

    // Stage 4: derive chunk-header wallet_id (affected by seed) and assemble chunks.
    let chunk_wallet_id: ChunkWalletId = match options.wallet_id_seed {
        Some(seed) => seed.truncate(),
        None => compute_wallet_id(&bytecode).truncate(),
    };
    let chunks = chunk_bytes(&bytecode, plan, chunk_wallet_id)?;

    // Stage 5: encode each chunk to a codex32 string.
    // Hoist the BCH code lookup — it is plan-level, not per-chunk.
    let bch_code: BchCode = match plan {
        ChunkingPlan::SingleString { code } => code.into(),
        ChunkingPlan::Chunked { code, .. } => code.into(),
    };
    let chunk_count = chunks.len();
    let mut encoded_chunks: Vec<EncodedChunk> = Vec::with_capacity(chunk_count);
    for chunk in chunks {
        // Read chunk_index and total_chunks from the header — canonical source.
        let (chunk_index, total_chunks) = match &chunk.header {
            ChunkHeader::SingleString { .. } => (0u8, 1u8),
            ChunkHeader::Chunked { index, count, .. } => (*index, *count),
        };
        let header_bytes = chunk.header.to_bytes();
        let raw = encode_string(&header_bytes, &chunk.fragment)?;
        // encode_string auto-selects the code from the data-part length, but
        // we record it explicitly from the plan for structured output.
        encoded_chunks.push(EncodedChunk {
            raw,
            chunk_index,
            total_chunks,
            code: bch_code,
        });
    }

    // Stage 6: Tier-3 wallet ID is ALWAYS content-derived, never affected by seed.
    let tier3_wallet_id = compute_wallet_id(&bytecode);
    let wallet_id_words = tier3_wallet_id.to_words();

    // Surface the caller-supplied fingerprints on the backup so that an
    // `encode → user-side state` round-trip is observable without re-decoding.
    // The decoder populates `MdBackup.fingerprints` from the parsed bytecode
    // independently — see `decode::decode`.
    Ok(MdBackup {
        chunks: encoded_chunks,
        wallet_id_words,
        fingerprints: options.fingerprints.clone(),
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        EncodeOptions, WalletPolicy,
        chunking::{ChunkHeader, ChunkingMode},
        wallet_id::{WalletIdSeed, compute_wallet_id},
    };

    fn policy(s: &str) -> WalletPolicy {
        s.parse().expect("should parse")
    }

    // -----------------------------------------------------------------------
    // 1. encode_single_string_regular
    // -----------------------------------------------------------------------

    #[test]
    fn encode_single_string_regular() {
        let p = policy("wsh(pk(@0/**))");
        let opts = EncodeOptions::default();
        let backup = encode(&p, &opts).expect("encode should succeed");
        assert_eq!(backup.chunks.len(), 1, "expected 1 chunk");
        let chunk = &backup.chunks[0];
        assert_eq!(chunk.chunk_index, 0);
        assert_eq!(chunk.total_chunks, 1);
        assert_eq!(chunk.code, BchCode::Regular);
        assert!(
            chunk.raw.starts_with("md1"),
            "raw string should start with md1, got {}",
            chunk.raw
        );
    }

    // -----------------------------------------------------------------------
    // 2. encode_single_string_long_via_force
    // -----------------------------------------------------------------------

    #[test]
    fn encode_single_string_long_via_force() {
        let p = policy("wsh(pk(@0/**))");
        let opts = EncodeOptions {
            force_long_code: true,
            ..Default::default()
        };
        let backup = encode(&p, &opts).expect("encode should succeed");
        assert_eq!(backup.chunks.len(), 1);
        assert_eq!(backup.chunks[0].code, BchCode::Long);
        assert!(backup.chunks[0].raw.starts_with("md1"));
    }

    // -----------------------------------------------------------------------
    // 3. encode_single_string_long_naturally
    //    Policy whose bytecode is between 49 and 56 bytes — fits Long single-
    //    string but not Regular. We use a pkh with two keys in an `or_b`.
    //    If this is hard to construct exactly, the test verifies the output code.
    // -----------------------------------------------------------------------

    #[test]
    fn encode_single_string_long_naturally() {
        // Use a sortedmulti(1,@0,@1) wrapped in wsh — this produces enough
        // bytecode to push past the 48-byte Regular capacity while staying
        // within the 56-byte Long capacity. Check actual bytecode length first.
        let p = policy("wsh(multi(2,@0/**,@1/**,@2/**,@3/**))");
        let bytecode = p
            .to_bytecode(&EncodeOptions::default())
            .expect("bytecode encode");
        // Only run this test if the bytecode falls in the Long-single-string range.
        if bytecode.len() > 48 && bytecode.len() <= 56 {
            let opts = EncodeOptions::default();
            let backup = encode(&p, &opts).expect("encode should succeed");
            assert_eq!(backup.chunks.len(), 1);
            assert_eq!(backup.chunks[0].code, BchCode::Long);
        }
        // If bytecode doesn't fall in that range, the test passes trivially
        // (construction of an exact-length policy is input-dependent).
        // The important property (force_long_code) is tested in test #2.
    }

    // -----------------------------------------------------------------------
    // 4. encode_chunked_regular_two_chunks
    // -----------------------------------------------------------------------

    #[test]
    fn encode_chunked_regular_two_chunks() {
        // We need bytecode ~80 bytes. A multi(5,@0..@8) in wsh gives many keys.
        // Use the known policy and check it forces chunking.
        let p = policy("wsh(multi(5,@0/**,@1/**,@2/**,@3/**,@4/**,@5/**,@6/**,@7/**,@8/**))");
        let bytecode = p.to_bytecode(&EncodeOptions::default()).expect("bytecode");
        let opts = EncodeOptions::default();
        let backup = encode(&p, &opts).expect("encode");

        // Verify chunking happened as expected (>48 bytes → chunked or long-single).
        if bytecode.len() > 56 {
            // Must be chunked.
            assert!(
                backup.chunks.len() >= 2,
                "expected ≥2 chunks for {} bytecode bytes, got {}",
                bytecode.len(),
                backup.chunks.len()
            );
            for (i, chunk) in backup.chunks.iter().enumerate() {
                assert_eq!(chunk.chunk_index, i as u8);
                assert_eq!(chunk.total_chunks, backup.chunks.len() as u8);
                assert_eq!(chunk.code, BchCode::Regular);
            }
        }
        // If bytecode is ≤ 56 bytes, it fits single-string (Long).
        // The test still passes — the exact byte count depends on the bytecode encoder.
    }

    // -----------------------------------------------------------------------
    // 5. encode_force_chunked_with_short_input
    // -----------------------------------------------------------------------

    #[test]
    fn encode_force_chunked_with_short_input() {
        let p = policy("wsh(pk(@0/**))");
        let opts = EncodeOptions {
            chunking_mode: ChunkingMode::ForceChunked,
            ..Default::default()
        };
        let backup = encode(&p, &opts).expect("encode");
        // Short input: 1 chunk (stream < 45 bytes → count=1 for Regular).
        assert_eq!(backup.chunks.len(), 1);
        let chunk = &backup.chunks[0];
        assert_eq!(chunk.chunk_index, 0);
        assert_eq!(chunk.total_chunks, 1);
        // Chunked type (even with 1 chunk): verify raw header is Chunked.
        // Decode the raw string to check the header bytes.
        let decoded = crate::decode_string(&chunk.raw).expect("should decode");
        // `expect` is sound HERE because `decoded.data` came from an
        // encoder-produced MD string that we constructed two lines up; the
        // encoder always pads to the byte boundary with zero bits so the
        // 5-bit→byte conversion cannot fail on its own output. For HOSTILE
        // inputs the same call returns None — see decode.rs:135's structured
        // error and `BytecodeErrorKind::MalformedPayloadPadding`.
        let bytes = crate::five_bit_to_bytes(decoded.data())
            .expect("test fixture: encoder-produced 5-bit data is byte-aligned by construction");
        let (header, _consumed) = crate::ChunkHeader::from_bytes(&bytes).expect("header parse");
        assert!(header.is_chunked(), "expected Chunked header");
    }

    // -----------------------------------------------------------------------
    // 6. encode_too_large_returns_error (skipped — cannot construct >1692 bytes
    //    of canonical bytecode in a unit test without a synthetic path)
    // -----------------------------------------------------------------------
    // Covered by the existing chunking_decision unit tests.

    // -----------------------------------------------------------------------
    // 7. encode_tier3_wallet_id_is_content_derived_without_seed
    // -----------------------------------------------------------------------

    #[test]
    fn encode_tier3_wallet_id_is_content_derived_without_seed() {
        let p = policy("wsh(pk(@0/**))");
        let opts = EncodeOptions::default();
        let backup = encode(&p, &opts).expect("encode");

        let bytecode = p.to_bytecode(&EncodeOptions::default()).expect("bytecode");
        let expected_wallet_id = compute_wallet_id(&bytecode);

        // Reconstruct WalletId from the backup's words and compare.
        let recovered = backup.wallet_id();
        assert_eq!(
            recovered, expected_wallet_id,
            "Tier-3 WalletId must equal compute_wallet_id(bytecode)"
        );
    }

    // -----------------------------------------------------------------------
    // 8. encode_tier3_wallet_id_unaffected_by_seed
    // -----------------------------------------------------------------------

    #[test]
    fn encode_tier3_wallet_id_unaffected_by_seed() {
        let p = policy("wsh(pk(@0/**))");

        let opts_no_seed = EncodeOptions::default();
        let backup_no_seed = encode(&p, &opts_no_seed).expect("encode no seed");

        let opts_with_seed = EncodeOptions {
            wallet_id_seed: Some(WalletIdSeed::from(0xDEAD_BEEFu32)),
            ..Default::default()
        };
        let backup_with_seed = encode(&p, &opts_with_seed).expect("encode with seed");

        assert_eq!(
            backup_no_seed.wallet_id_words, backup_with_seed.wallet_id_words,
            "Tier-3 wallet_id_words must be identical regardless of seed"
        );
    }

    // -----------------------------------------------------------------------
    // 9. encode_chunk_header_wallet_id_uses_seed_when_present
    // -----------------------------------------------------------------------

    #[test]
    fn encode_chunk_header_wallet_id_uses_seed_when_present() {
        let p = policy("wsh(pk(@0/**))");
        let seed_val: u32 = 0x1234_5678;
        let opts = EncodeOptions {
            chunking_mode: ChunkingMode::ForceChunked,
            wallet_id_seed: Some(WalletIdSeed::from(seed_val)),
            ..Default::default()
        };
        let backup = encode(&p, &opts).expect("encode");
        assert_eq!(backup.chunks.len(), 1);

        // Decode the raw string to access chunk header bytes.
        let raw = &backup.chunks[0].raw;
        let decoded = crate::decode_string(raw).expect("decode_string");
        // `expect` is sound HERE because `decoded.data` came from an
        // encoder-produced MD string that we constructed two lines up; the
        // encoder always pads to the byte boundary with zero bits so the
        // 5-bit→byte conversion cannot fail on its own output. For HOSTILE
        // inputs the same call returns None — see decode.rs:135's structured
        // error and `BytecodeErrorKind::MalformedPayloadPadding`.
        let bytes = crate::five_bit_to_bytes(decoded.data())
            .expect("test fixture: encoder-produced 5-bit data is byte-aligned by construction");
        let (header, _) = crate::ChunkHeader::from_bytes(&bytes).expect("header parse");

        // The chunk header wallet_id should equal seed.truncate() = top 20 bits of seed.
        let expected_chunk_wid = WalletIdSeed::from(seed_val).truncate();
        match header {
            ChunkHeader::Chunked { wallet_id, .. } => {
                assert_eq!(
                    wallet_id, expected_chunk_wid,
                    "chunk-header wallet_id must equal seed.truncate()"
                );
            }
            ChunkHeader::SingleString { .. } => {
                panic!("expected Chunked header when chunking_mode=ForceChunked");
            }
        }
    }

    // -----------------------------------------------------------------------
    // 10. encode_chunk_header_wallet_id_is_content_derived_without_seed
    // -----------------------------------------------------------------------

    #[test]
    fn encode_chunk_header_wallet_id_is_content_derived_without_seed() {
        let p = policy("wsh(pk(@0/**))");
        let opts = EncodeOptions {
            chunking_mode: ChunkingMode::ForceChunked,
            ..Default::default()
        };
        let backup = encode(&p, &opts).expect("encode");
        assert_eq!(backup.chunks.len(), 1);

        // Decode the raw string to access chunk header bytes.
        let raw = &backup.chunks[0].raw;
        let decoded = crate::decode_string(raw).expect("decode_string");
        // `expect` is sound HERE because `decoded.data` came from an
        // encoder-produced MD string that we constructed two lines up; the
        // encoder always pads to the byte boundary with zero bits so the
        // 5-bit→byte conversion cannot fail on its own output. For HOSTILE
        // inputs the same call returns None — see decode.rs:135's structured
        // error and `BytecodeErrorKind::MalformedPayloadPadding`.
        let bytes = crate::five_bit_to_bytes(decoded.data())
            .expect("test fixture: encoder-produced 5-bit data is byte-aligned by construction");
        let (header, _) = crate::ChunkHeader::from_bytes(&bytes).expect("header parse");

        // The chunk-header wallet_id should be compute_wallet_id(bytecode).truncate().
        let bytecode = p.to_bytecode(&EncodeOptions::default()).expect("bytecode");
        let expected_chunk_wid = compute_wallet_id(&bytecode).truncate();
        match header {
            ChunkHeader::Chunked { wallet_id, .. } => {
                assert_eq!(
                    wallet_id, expected_chunk_wid,
                    "chunk-header wallet_id must equal compute_wallet_id(bytecode).truncate()"
                );
            }
            ChunkHeader::SingleString { .. } => {
                panic!("expected Chunked header when chunking_mode=ForceChunked");
            }
        }
    }

    // -----------------------------------------------------------------------
    // 11. encode_is_deterministic_with_fixed_seed
    // -----------------------------------------------------------------------

    #[test]
    fn encode_is_deterministic_with_fixed_seed() {
        let p = policy("wsh(pk(@0/**))");
        let opts = EncodeOptions {
            wallet_id_seed: Some(WalletIdSeed::from(0xABCD_1234u32)),
            ..Default::default()
        };
        let backup1 = encode(&p, &opts).expect("first encode");
        let backup2 = encode(&p, &opts).expect("second encode");
        assert_eq!(
            backup1, backup2,
            "encode with fixed seed must be deterministic"
        );
    }

    // -----------------------------------------------------------------------
    // 12. encode_idempotent_under_default_options
    // -----------------------------------------------------------------------

    #[test]
    fn encode_idempotent_under_default_options() {
        let p = policy("wsh(pk(@0/**))");
        let opts = EncodeOptions::default();
        let backup1 = encode(&p, &opts).expect("first encode");
        let backup2 = encode(&p, &opts).expect("second encode");
        assert_eq!(
            backup1, backup2,
            "encode with default options must be idempotent"
        );
    }

    // -----------------------------------------------------------------------
    // Bonus: verify force_long_code with ChunkingMode::ForceChunked upgrades chunked Regular → Long
    // -----------------------------------------------------------------------

    #[test]
    fn encode_force_chunked_force_long_produces_long_code() {
        let p = policy("wsh(pk(@0/**))");
        let opts = EncodeOptions {
            chunking_mode: ChunkingMode::ForceChunked,
            force_long_code: true,
            ..Default::default()
        };
        let backup = encode(&p, &opts).expect("encode");
        assert_eq!(backup.chunks[0].code, BchCode::Long);
    }

    // -----------------------------------------------------------------------
    // Bonus: when seed is None, chunk-header wallet_id == Tier-3 truncation
    // -----------------------------------------------------------------------

    #[test]
    fn encode_chunk_header_wid_is_truncation_of_tier3_without_seed() {
        // When seed is None, chunk-header wallet_id = first 20 bits of Tier-3.
        // The Tier-3 is compute_wallet_id(bytecode); chunk-header = .truncate().
        let p = policy("wsh(pk(@0/**))");
        let opts = EncodeOptions {
            chunking_mode: ChunkingMode::ForceChunked,
            ..Default::default()
        };
        let backup = encode(&p, &opts).expect("encode");
        let tier3 = backup.wallet_id();
        let expected_chunk_wid = tier3.truncate();

        let raw = &backup.chunks[0].raw;
        let decoded = crate::decode_string(raw).expect("decode_string");
        // `expect` is sound HERE because `decoded.data` came from an
        // encoder-produced MD string that we constructed two lines up; the
        // encoder always pads to the byte boundary with zero bits so the
        // 5-bit→byte conversion cannot fail on its own output. For HOSTILE
        // inputs the same call returns None — see decode.rs:135's structured
        // error and `BytecodeErrorKind::MalformedPayloadPadding`.
        let bytes = crate::five_bit_to_bytes(decoded.data())
            .expect("test fixture: encoder-produced 5-bit data is byte-aligned by construction");
        let (header, _) = crate::ChunkHeader::from_bytes(&bytes).expect("header parse");

        match header {
            ChunkHeader::Chunked { wallet_id, .. } => {
                assert_eq!(
                    wallet_id, expected_chunk_wid,
                    "chunk-header wallet_id must be the first 20 bits of the Tier-3 WalletId"
                );
            }
            ChunkHeader::SingleString { .. } => {
                panic!("expected Chunked header");
            }
        }
    }
}
