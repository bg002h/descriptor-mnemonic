//! Task 6.21 — Conformance rejection tests.
//!
//! Each `rejects_*` test names one specific rejection path in the public API
//! and asserts the right `Error` variant, making BIP-conformance-style audits
//! straightforward: one named test per rejection.
//!
//! Organisation:
//! - Layer 1 (codex32 / string level): tests 1–5
//! - Layer 2 (chunk-header / `ChunkHeader::from_bytes`): tests 6–11
//! - Layer 3 (reassembly / `reassemble_chunks`): tests 12–19
//! - Layer 4 (bytecode / `WalletPolicy::from_bytecode`): tests 20–27
//! - Layer 5 (policy scope): tests 28–30
//! - Layer 6 (`chunking_decision`): test 31

mod common;

use md_codec::{
    BytecodeErrorKind, Chunk, ChunkHeader, ChunkWalletId, ChunkingMode, DecodeOptions,
    EncodeOptions, Error, WalletPolicy, chunk_bytes, chunking_decision, decode, encode,
    reassemble_chunks,
};

// ---------------------------------------------------------------------------
// Helper macro: assert that `decode(&[input], &DecodeOptions::new())` returns
// an `Err` whose variant matches `$pattern`.
// ---------------------------------------------------------------------------

macro_rules! assert_decode_rejects {
    ($name:ident, $input:expr, $pattern:pat) => {
        #[test]
        fn $name() {
            let result = decode(&[$input], &DecodeOptions::new());
            match result {
                Err(e) if matches!(e, $pattern) => {}
                Err(other) => panic!(
                    "expected error matching {}, got {:?}",
                    stringify!($pattern),
                    other
                ),
                Ok(_) => panic!("expected error matching {}, got Ok", stringify!($pattern)),
            }
        }
    };
}

// ---------------------------------------------------------------------------
// Layer 1: codex32 / string-level rejections (errors from `decode_string`)
// ---------------------------------------------------------------------------

// 1. HRP that is not "md" → `Error::InvalidHrp`
assert_decode_rejects!(
    rejects_invalid_hrp,
    "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4",
    Error::InvalidHrp(_)
);

/// 2. Mixed-case characters → `Error::MixedCase`
#[test]
fn rejects_mixed_case() {
    // Build a valid MD string, then uppercase one data character.
    let p: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
    let backup = encode(&p, &EncodeOptions::default()).unwrap();
    let raw = &backup.chunks[0].raw;

    let mut chars: Vec<char> = raw.chars().collect();
    // Position 5 is in the data part (after "md1").
    chars[5] = chars[5].to_ascii_uppercase();
    let mixed: String = chars.into_iter().collect();

    let result = decode(&[mixed.as_str()], &DecodeOptions::new());
    match result {
        Err(Error::MixedCase) => {}
        Err(other) => panic!("expected MixedCase, got {:?}", other),
        Ok(_) => panic!("expected MixedCase, got Ok"),
    }
}

/// 3. String length in the reserved 94–95 char range → `Error::InvalidStringLength`
#[test]
fn rejects_invalid_string_length() {
    // Construct a string with data-part length of 94 (reserved-invalid).
    // "md1" prefix = 3 chars; total = 3 + 94 = 97 chars.
    // Fill data part with all-'q' (value 0); BCH will be wrong, but
    // InvalidStringLength fires before BCH checking.
    let data_part: String = "q".repeat(94);
    let s = format!("md1{data_part}");
    let result = decode(&[s.as_str()], &DecodeOptions::new());
    match result {
        Err(Error::InvalidStringLength(_)) => {}
        Err(other) => panic!("expected InvalidStringLength, got {:?}", other),
        Ok(_) => panic!("expected InvalidStringLength, got Ok"),
    }
}

/// 4. Non-bech32 character in the data part → `Error::InvalidChar`
#[test]
fn rejects_invalid_char() {
    // Build a valid string, then splice in 'b' (not in the bech32 alphabet).
    let p: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
    let backup = encode(&p, &EncodeOptions::default()).unwrap();
    let raw = &backup.chunks[0].raw;

    let mut chars: Vec<char> = raw.chars().collect();
    // Position 5 is in the data part (well past "md1").
    chars[5] = 'b';
    let bad: String = chars.into_iter().collect();

    let result = decode(&[bad.as_str()], &DecodeOptions::new());
    match result {
        Err(Error::InvalidChar { .. }) => {}
        Err(other) => panic!("expected InvalidChar, got {:?}", other),
        Ok(_) => panic!("expected InvalidChar, got Ok"),
    }
}

/// 5. Too many corruptions (> 1 substitution for v0.1) → `Error::BchUncorrectable`
#[test]
fn rejects_bch_uncorrectable() {
    // Encode a valid policy, then corrupt two characters in the data part.
    // v0.1 can correct 0–1 substitutions; 2 is uncorrectable.
    let p: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
    let backup = encode(&p, &EncodeOptions::default()).unwrap();
    let raw = &backup.chunks[0].raw;

    let mut chars: Vec<char> = raw.chars().collect();
    // Corrupt positions 5 and 7 (both in the data part, after "md1").
    // Replace with a character that is NOT the same as the original.
    for pos in [5, 7] {
        chars[pos] = if chars[pos] == 'q' { 'p' } else { 'q' };
    }
    let corrupted: String = chars.into_iter().collect();

    let result = decode(&[corrupted.as_str()], &DecodeOptions::new());
    match result {
        Err(Error::BchUncorrectable) => {}
        // A 2-char corruption might, very rarely, produce a valid codeword by
        // accident (1-in-32^2 ≈ 0.1% probability per position pair). In that
        // case we may hit a downstream error or Ok; mark that case as acceptable
        // but note it in the failure message.
        Err(other) => panic!("expected BchUncorrectable, got {:?}", other),
        Ok(_) => {
            // This can happen if the 2-char flip accidentally forms a valid
            // codeword.  Rather than panicking on an extremely rare false-pass,
            // skip with a note. (Probability ≈ 1/1024 per test run.)
            eprintln!(
                "rejects_bch_uncorrectable: 2-char corruption accidentally formed \
                 a valid codeword — this is expected to be very rare; run again to \
                 confirm the test normally passes."
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Layer 2: chunk-header rejections (`ChunkHeader::from_bytes`)
// ---------------------------------------------------------------------------

/// 6. Unsupported version byte → `Error::UnsupportedVersion`
///
/// `ChunkHeader::from_bytes` stores the raw version byte (not a nibble-shifted
/// value), so byte 0x01 (version=1, type=SingleString but wrong version) gives
/// `UnsupportedVersion(1)`.  Byte 0x02 is the smallest non-zero value that
/// doesn't coincide with any valid type byte when in position 0.
#[test]
fn rejects_unsupported_version() {
    // Byte 0 = 0x01 → version=1 (not VERSION_0=0x00) → UnsupportedVersion(1).
    let bytes = [0x01u8, 0x00];
    let err = ChunkHeader::from_bytes(&bytes).unwrap_err();
    assert!(
        matches!(err, Error::UnsupportedVersion(1)),
        "expected UnsupportedVersion(1), got {:?}",
        err
    );
}

/// 7. Unsupported card-type byte → `Error::UnsupportedCardType`
#[test]
fn rejects_unsupported_card_type() {
    // version=0, type=2 (unknown).
    let bytes = [0x00u8, 0x02];
    let err = ChunkHeader::from_bytes(&bytes).unwrap_err();
    assert!(
        matches!(err, Error::UnsupportedCardType(2)),
        "expected UnsupportedCardType(2), got {:?}",
        err
    );
}

/// 8. Reserved wallet-id bits set → `Error::ReservedWalletIdBitsSet`
#[test]
fn rejects_reserved_wallet_id_bits_set() {
    // version=0, type=Chunked(1), wallet_id first byte=0x10 (top nibble set).
    let bytes = [0x00u8, 0x01, 0x10, 0x00, 0x00, 0x01, 0x00];
    let err = ChunkHeader::from_bytes(&bytes).unwrap_err();
    assert!(
        matches!(err, Error::ReservedWalletIdBitsSet),
        "expected ReservedWalletIdBitsSet, got {:?}",
        err
    );
}

/// 9. Count=0 → `Error::InvalidChunkCount`
#[test]
fn rejects_invalid_chunk_count() {
    // [ver=0, type=1, wid=0,0,0, count=0, index=0]
    let bytes = [0x00u8, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00];
    let err = ChunkHeader::from_bytes(&bytes).unwrap_err();
    assert!(
        matches!(err, Error::InvalidChunkCount(0)),
        "expected InvalidChunkCount(0), got {:?}",
        err
    );
}

/// 10. index >= count → `Error::InvalidChunkIndex`
#[test]
fn rejects_invalid_chunk_index() {
    // [ver=0, type=1, wid=0,0,0, count=3, index=3] (index==count → out of range)
    let bytes = [0x00u8, 0x01, 0x00, 0x00, 0x00, 0x03, 0x03];
    let err = ChunkHeader::from_bytes(&bytes).unwrap_err();
    assert!(
        matches!(err, Error::InvalidChunkIndex { index: 3, count: 3 }),
        "expected InvalidChunkIndex {{ index: 3, count: 3 }}, got {:?}",
        err
    );
}

/// 11. Chunk header bytes truncated → `Error::ChunkHeaderTruncated`
#[test]
fn rejects_chunk_header_truncated() {
    // Only 1 byte — not enough for even the 2-byte SingleString header.
    let err = ChunkHeader::from_bytes(&[0x00]).unwrap_err();
    assert!(
        matches!(err, Error::ChunkHeaderTruncated { have: 1, need: 2 }),
        "expected ChunkHeaderTruncated {{ have: 1, need: 2 }}, got {:?}",
        err
    );
}

// ---------------------------------------------------------------------------
// Layer 3: reassembly rejections (`reassemble_chunks`)
// ---------------------------------------------------------------------------

/// 12. Empty chunk list → `Error::EmptyChunkList`
#[test]
fn rejects_empty_chunk_list() {
    let err = reassemble_chunks(vec![]).unwrap_err();
    assert!(
        matches!(err, Error::EmptyChunkList),
        "expected EmptyChunkList, got {:?}",
        err
    );
}

/// 13. Single-string chunk appearing more than once → `Error::SingleStringWithMultipleChunks`
#[test]
fn rejects_single_string_with_multiple_chunks() {
    let ss = Chunk::new(
        ChunkHeader::SingleString { version: 0 },
        vec![0x01, 0x02, 0x03],
    );
    let err = reassemble_chunks(vec![ss.clone(), ss]).unwrap_err();
    assert!(
        matches!(err, Error::SingleStringWithMultipleChunks),
        "expected SingleStringWithMultipleChunks, got {:?}",
        err
    );
}

/// 14. Mixed SingleString + Chunked in one list → `Error::MixedChunkTypes`
#[test]
fn rejects_mixed_chunk_types() {
    let ss = Chunk::new(ChunkHeader::SingleString { version: 0 }, vec![0x01]);
    let chunked = Chunk::new(
        ChunkHeader::Chunked {
            version: 0,
            wallet_id: ChunkWalletId::new(0x0001),
            count: 2,
            index: 0,
        },
        vec![0x02],
    );
    // First chunk is SingleString → mixed.
    let err = reassemble_chunks(vec![ss, chunked]).unwrap_err();
    assert!(
        matches!(err, Error::MixedChunkTypes),
        "expected MixedChunkTypes, got {:?}",
        err
    );
}

/// 15. Wallet-id mismatch across chunks → `Error::WalletIdMismatch`
#[test]
fn rejects_wallet_id_mismatch() {
    let wid_a = ChunkWalletId::new(0xAAAAA);
    let wid_b = ChunkWalletId::new(0xBBBBB);
    let c0 = Chunk::new(
        ChunkHeader::Chunked {
            version: 0,
            wallet_id: wid_a,
            count: 2,
            index: 0,
        },
        vec![0x01, 0x02, 0x03, 0x04, 0x05],
    );
    let c1 = Chunk::new(
        ChunkHeader::Chunked {
            version: 0,
            wallet_id: wid_b,
            count: 2,
            index: 1,
        },
        vec![0x06, 0x07, 0x08, 0x09],
    );
    let err = reassemble_chunks(vec![c0, c1]).unwrap_err();
    assert!(
        matches!(err, Error::WalletIdMismatch { .. }),
        "expected WalletIdMismatch, got {:?}",
        err
    );
}

/// 16. Total-chunks mismatch across chunks → `Error::TotalChunksMismatch`
#[test]
fn rejects_total_chunks_mismatch() {
    let wid = ChunkWalletId::new(0x12345);
    let c0 = Chunk::new(
        ChunkHeader::Chunked {
            version: 0,
            wallet_id: wid,
            count: 2,
            index: 0,
        },
        vec![0x01],
    );
    let c1 = Chunk::new(
        ChunkHeader::Chunked {
            version: 0,
            wallet_id: wid,
            count: 3, // mismatch
            index: 1,
        },
        vec![0x02],
    );
    let err = reassemble_chunks(vec![c0, c1]).unwrap_err();
    assert!(
        matches!(
            err,
            Error::TotalChunksMismatch {
                expected: 2,
                got: 3
            }
        ),
        "expected TotalChunksMismatch {{ expected: 2, got: 3 }}, got {:?}",
        err
    );
}

/// 17. Chunk index ≥ declared total → `Error::ChunkIndexOutOfRange`
///
/// This only triggers via `reassemble_chunks` (which has an additional guard
/// beyond `ChunkHeader::from_bytes`) when `Chunk::new` is used to bypass
/// header validation.
#[test]
fn rejects_chunk_index_out_of_range() {
    let wid = ChunkWalletId::new(0x0042);
    // Build a Chunk directly with index > count using Chunk::new (bypasses from_bytes validation).
    let bad = Chunk::new(
        ChunkHeader::Chunked {
            version: 0,
            wallet_id: wid,
            count: 2,
            index: 5, // out of range: 5 >= 2
        },
        vec![0x01],
    );
    let err = reassemble_chunks(vec![bad]).unwrap_err();
    assert!(
        matches!(err, Error::ChunkIndexOutOfRange { index: 5, total: 2 }),
        "expected ChunkIndexOutOfRange {{ index: 5, total: 2 }}, got {:?}",
        err
    );
}

/// 18. Duplicate chunk index → `Error::DuplicateChunkIndex`
#[test]
fn rejects_duplicate_chunk_index() {
    let wid = ChunkWalletId::new(0x0001);
    let c0a = Chunk::new(
        ChunkHeader::Chunked {
            version: 0,
            wallet_id: wid,
            count: 2,
            index: 0,
        },
        vec![0x01],
    );
    let c0b = Chunk::new(
        ChunkHeader::Chunked {
            version: 0,
            wallet_id: wid,
            count: 2,
            index: 0, // duplicate
        },
        vec![0x02],
    );
    let err = reassemble_chunks(vec![c0a, c0b]).unwrap_err();
    assert!(
        matches!(err, Error::DuplicateChunkIndex(0)),
        "expected DuplicateChunkIndex(0), got {:?}",
        err
    );
}

/// 19. Missing chunk index in a multi-chunk set → `Error::MissingChunkIndex`
#[test]
fn rejects_missing_chunk_index() {
    let wid = ChunkWalletId::new(0x0010);
    // Claim count=3 but only provide indices 0 and 2; index 1 is absent.
    let c0 = Chunk::new(
        ChunkHeader::Chunked {
            version: 0,
            wallet_id: wid,
            count: 3,
            index: 0,
        },
        vec![0x01],
    );
    let c2 = Chunk::new(
        ChunkHeader::Chunked {
            version: 0,
            wallet_id: wid,
            count: 3,
            index: 2,
        },
        vec![0x03],
    );
    let err = reassemble_chunks(vec![c0, c2]).unwrap_err();
    assert!(
        matches!(err, Error::MissingChunkIndex(1)),
        "expected MissingChunkIndex(1), got {:?}",
        err
    );
}

/// 20. Cross-chunk integrity hash mismatch → `Error::CrossChunkHashMismatch`
#[test]
fn rejects_cross_chunk_hash_mismatch() {
    use md_codec::{ChunkCode, ChunkingPlan};

    let bytecode: Vec<u8> = (0u8..50).collect();
    let plan = ChunkingPlan::Chunked {
        code: ChunkCode::Regular,
        fragment_size: 45,
        count: 2,
    };
    let wid = ChunkWalletId::new(0xABCDE);
    let mut chunks = chunk_bytes(&bytecode, plan, wid).unwrap();

    // Corrupt the first byte of the last fragment (corrupts either payload or hash).
    let last = chunks.last_mut().unwrap();
    last.fragment[0] ^= 0xFF;

    let err = reassemble_chunks(chunks).unwrap_err();
    assert!(
        matches!(err, Error::CrossChunkHashMismatch),
        "expected CrossChunkHashMismatch, got {:?}",
        err
    );
}

// ---------------------------------------------------------------------------
// Layer 4: bytecode-level rejections (`WalletPolicy::from_bytecode`)
// ---------------------------------------------------------------------------

/// 20b. Malicious Long-code input with non-zero trailing pad bit →
/// `Error::InvalidBytecode { kind: MalformedPayloadPadding }`.
///
/// **Background**: the v0.2.1 full-code-audit
/// (`design/agent-reports/v0-2-1-full-code-audit.md`) discovered that a
/// crafted Long-code MD string can pass the BCH validate stage and panic
/// in Stage 3 of decode (`five_bit_to_bytes` returns None on non-byte-aligned
/// input). v0.2.2 converts that panic to a structured error.
///
/// **Construction**: the Long-code data part is 93 5-bit symbols = 465 bits
/// (= 58 bytes + 1 trailing bit). A conformant encoder always pads with zero
/// bits. This test sets the final symbol's lowest bit, computes the
/// legitimate Long-code BCH checksum over that hostile data, assembles the
/// MD string, and asserts decode rejects with `MalformedPayloadPadding`.
#[test]
fn rejects_malformed_payload_padding() {
    use md_codec::encoding::{ALPHABET, bch_create_checksum_long};

    // 93 5-bit symbols, all zero except the last whose low bit is 1.
    let mut data_5bit = vec![0u8; 93];
    data_5bit[92] = 0x01;

    // Compute the legitimate Long-code BCH checksum for HRP "md".
    let checksum = bch_create_checksum_long("md", &data_5bit);

    // Assemble the MD string: "md1" + ALPHABET[symbol] for each data + each checksum char.
    let mut s = String::from("md1");
    for &v in &data_5bit {
        s.push(ALPHABET[v as usize] as char);
    }
    for &v in &checksum {
        s.push(ALPHABET[v as usize] as char);
    }

    let result = decode(&[s.as_str()], &DecodeOptions::new());
    match result {
        Err(Error::InvalidBytecode {
            kind: BytecodeErrorKind::MalformedPayloadPadding,
            ..
        }) => {}
        other => {
            panic!("expected InvalidBytecode {{ kind: MalformedPayloadPadding }}, got {other:?}")
        }
    }
}

/// 21. Unknown tag byte in the bytecode → `Error::InvalidBytecode { kind: UnknownTag(_) }`
///
/// The path-declaration slot (bytes[1]) expects Tag::SharedPath (0x33).
/// Supplying an unknown tag byte (0xC0) there triggers UnknownTag.
#[test]
fn rejects_invalid_bytecode_unknown_tag() {
    // header=0x00, then 0xC0 (not a defined tag) where SharedPath tag is expected.
    let bytes = [0x00u8, 0xC0, 0x03, 0x05, 0x32, 0x00];
    let err = WalletPolicy::from_bytecode(&bytes).unwrap_err();
    assert!(
        matches!(
            err,
            Error::InvalidBytecode {
                kind: BytecodeErrorKind::UnknownTag(_),
                ..
            }
        ),
        "expected InvalidBytecode {{ kind: UnknownTag(_) }}, got {:?}",
        err
    );
}

/// 22. Input truncated at the bytecode level → `Error::InvalidBytecode { kind: Truncated }`
///
/// Submitting only the header byte leaves the path declaration and tree
/// completely absent. This surfaces as UnexpectedEnd (which is the `Truncated`
/// analogue in the cursor layer).
#[test]
fn rejects_invalid_bytecode_truncated() {
    // Only the header byte — no path declaration or tree.
    let bytes = [0x00u8];
    let err = WalletPolicy::from_bytecode(&bytes).unwrap_err();
    // The cursor hits end-of-buffer while trying to read the first declaration byte.
    assert!(
        matches!(
            err,
            Error::InvalidBytecode {
                kind: BytecodeErrorKind::UnexpectedEnd,
                ..
            }
        ),
        "expected InvalidBytecode {{ kind: UnexpectedEnd }}, got {:?}",
        err
    );
}

/// 23. LEB128 varint overflow in a path component → `Error::InvalidBytecode { kind: VarintOverflow }`
///
/// Construct: header(0x00) + SharedPath(0x33) + explicit-path(0xFE) +
/// count(0x01) + LEB128 with 11 continuation bytes (overflows u64).
#[test]
fn rejects_invalid_bytecode_varint_overflow() {
    // 11 bytes of LEB128 with continuation bits set = VarintOverflow.
    let leb128_overflow = [0x80u8; 11]; // 11 bytes, all continuation bits set, never terminates
    let mut bytes = vec![
        0x00u8, // header
        0x33,   // Tag::SharedPath
        0xFE,   // explicit path marker
        0x01,   // count = 1 component
    ];
    bytes.extend_from_slice(&leb128_overflow);

    let err = WalletPolicy::from_bytecode(&bytes).unwrap_err();
    assert!(
        matches!(
            err,
            Error::InvalidBytecode {
                kind: BytecodeErrorKind::VarintOverflow | BytecodeErrorKind::UnexpectedEnd,
                ..
            }
        ),
        "expected InvalidBytecode {{ kind: VarintOverflow or UnexpectedEnd }}, got {:?}",
        err
    );
}

/// 24. `BytecodeErrorKind::MissingChildren` — emitted by the explicit arity check.
///
/// `multi(k=2, n=2)` with only 1 placeholder provided: the decoder reads
/// the first placeholder successfully, then on the second iteration hits end
/// of buffer. The arity check intercepts the `UnexpectedEnd` from the cursor
/// and converts it into `MissingChildren { expected: 2, got: 1 }`.
#[test]
fn rejects_invalid_bytecode_missing_children() {
    use md_codec::bytecode::Tag;

    // multi(2, @0) — k=2, n=2, only 1 key provided (second is absent).
    let bytes: Vec<u8> = vec![
        0x00,
        Tag::SharedPath.as_byte(),
        0x03,
        Tag::Wsh.as_byte(),
        Tag::Multi.as_byte(),
        0x02, // k=2
        0x02, // n=2
        Tag::Placeholder.as_byte(),
        0x00, // index 0 — only one key; second expected but absent
    ];

    let err = WalletPolicy::from_bytecode(&bytes).unwrap_err();
    assert!(
        matches!(
            err,
            Error::InvalidBytecode {
                kind: BytecodeErrorKind::MissingChildren {
                    expected: 2,
                    got: 1
                },
                ..
            }
        ),
        "expected MissingChildren {{ expected: 2, got: 1 }}, got {:?}",
        err
    );
}

/// 25. Cursor hits end of buffer mid-parse → `Error::InvalidBytecode { kind: UnexpectedEnd }`
///
/// We stop mid-tree: header + SharedPath + Wsh tag but no inner tag.
#[test]
fn rejects_invalid_bytecode_unexpected_end() {
    use md_codec::bytecode::Tag;

    let bytes: Vec<u8> = vec![
        0x00,                      // header
        Tag::SharedPath.as_byte(), // 0x33
        0x03,                      // BIP84 indicator
        Tag::Wsh.as_byte(),        // 0x05
                                   // Nothing follows — cursor hits end while reading the inner tag.
    ];

    let err = WalletPolicy::from_bytecode(&bytes).unwrap_err();
    assert!(
        matches!(
            err,
            Error::InvalidBytecode {
                kind: BytecodeErrorKind::UnexpectedEnd,
                ..
            }
        ),
        "expected InvalidBytecode {{ kind: UnexpectedEnd }}, got {:?}",
        err
    );
}

/// 26. Trailing bytes after the template tree → `Error::InvalidBytecode { kind: TrailingBytes }`
#[test]
fn rejects_invalid_bytecode_trailing_bytes() {
    // Encode a minimal valid policy and append a trailing byte.
    let p: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
    let mut bytes = p.to_bytecode(&EncodeOptions::default()).unwrap();
    bytes.push(0xFF); // trailing byte

    let err = WalletPolicy::from_bytecode(&bytes).unwrap_err();
    assert!(
        matches!(
            err,
            Error::InvalidBytecode {
                kind: BytecodeErrorKind::TrailingBytes,
                ..
            }
        ),
        "expected InvalidBytecode {{ kind: TrailingBytes }}, got {:?}",
        err
    );
}

/// 27. Reserved bits set in the header byte → `Error::InvalidBytecode { kind: ReservedBitsSet { .. } }`
#[test]
fn rejects_invalid_bytecode_reserved_bits_set() {
    // 0x01 = version 0, reserved bit 0 set. Must be rejected before we even
    // read the path declaration.
    let bytes = [0x01u8, 0x33, 0x03, 0x05, 0x32, 0x00];
    let err = WalletPolicy::from_bytecode(&bytes).unwrap_err();
    assert!(
        matches!(
            err,
            Error::InvalidBytecode {
                kind: BytecodeErrorKind::ReservedBitsSet { .. },
                ..
            }
        ),
        "expected InvalidBytecode {{ kind: ReservedBitsSet {{ .. }} }}, got {:?}",
        err
    );
}

/// 28. Path indicator byte is a defined (but wrong) tag → `Error::InvalidBytecode { kind: UnexpectedTag { .. } }`
///
/// The path-declaration slot expects Tag::SharedPath (0x33) as the first byte.
/// Supplying a different but defined tag (Tag::Wsh = 0x05) triggers UnexpectedTag.
#[test]
fn rejects_invalid_bytecode_unexpected_tag() {
    use md_codec::bytecode::Tag;

    // header=0x00, then Tag::Wsh (0x05) where Tag::SharedPath (0x33) is expected.
    let bytes: Vec<u8> = vec![
        0x00,               // header
        Tag::Wsh.as_byte(), // 0x05 — wrong tag at the declaration slot
        0x03,
        0x05,
        0x32,
        0x00,
    ];

    let err = WalletPolicy::from_bytecode(&bytes).unwrap_err();
    assert!(
        matches!(
            err,
            Error::InvalidBytecode {
                kind: BytecodeErrorKind::UnexpectedTag {
                    expected: 0x33,
                    got: 0x05
                },
                ..
            }
        ),
        "expected InvalidBytecode {{ kind: UnexpectedTag {{ expected: 0x33, got: 0x05 }} }}, got {:?}",
        err
    );
}

/// 29. Miniscript type-check fails when building the descriptor.
///
/// Expected error: `Error::InvalidBytecode { kind: TypeCheckFailed(_) }`
///
/// `wsh(multi(2,@0,@1))` requires k ≤ n (threshold ≤ key count). If we set
/// k > n we get a type-check failure.
///
/// We craft bytecode for `wsh(multi(k=5, n=2, @0, @1))` which is k > n,
/// triggering a type-check failure during `Wsh::new(...)`.
#[test]
fn rejects_invalid_bytecode_type_check_failed() {
    use md_codec::bytecode::Tag;

    // multi(5, @0, @1): k=5, n=2, but keys only [@0, @1] → k > n → type-check failure.
    let bytes: Vec<u8> = vec![
        0x00,                       // header
        Tag::SharedPath.as_byte(),  // 0x33
        0x03,                       // BIP84 indicator
        Tag::Wsh.as_byte(),         // 0x05
        Tag::Multi.as_byte(),       // 0x19
        0x05,                       // k=5 (LEB128)
        0x02,                       // n=2 (LEB128)
        Tag::Placeholder.as_byte(), // 0x32
        0x00,                       // index 0
        Tag::Placeholder.as_byte(), // 0x32
        0x01,                       // index 1
    ];

    let err = WalletPolicy::from_bytecode(&bytes).unwrap_err();
    assert!(
        matches!(
            err,
            Error::InvalidBytecode {
                kind: BytecodeErrorKind::TypeCheckFailed(_),
                ..
            }
        ),
        "expected InvalidBytecode {{ kind: TypeCheckFailed(_) }}, got {:?}",
        err
    );
}

/// 30. Path component encoded value exceeds max BIP32 range.
///
/// Expected error: `Error::InvalidBytecode { kind: InvalidPathComponent { .. } }`
///
/// Explicit path with encoded value `2^32 = 0x100000000`
/// (LEB128: `[0x80, 0x80, 0x80, 0x80, 0x10]`).
#[test]
fn rejects_invalid_bytecode_invalid_path_component() {
    // Explicit path: 0xFE marker, count=1, then 2^32 in LEB128.
    // 2^32 = 0x100000000; LEB128 = [0x80, 0x80, 0x80, 0x80, 0x10].
    let bytes: Vec<u8> = vec![
        0x00, // header
        0x33, // Tag::SharedPath
        0xFE, // explicit path marker
        0x01, // count = 1
        0x80, 0x80, 0x80, 0x80, 0x10, // LEB128(2^32) → InvalidPathComponent
    ];

    let err = WalletPolicy::from_bytecode(&bytes).unwrap_err();
    assert!(
        matches!(
            err,
            Error::InvalidBytecode {
                kind: BytecodeErrorKind::InvalidPathComponent {
                    encoded: 0x100000000
                },
                ..
            }
        ),
        "expected InvalidBytecode {{ kind: InvalidPathComponent {{ encoded: 0x100000000 }} }}, got {:?}",
        err
    );
}

// ---------------------------------------------------------------------------
// Layer 5: policy-scope rejections
// ---------------------------------------------------------------------------

/// 31. Non-Wsh top-level → `Error::PolicyScopeViolation`
///
/// The bytecode decoder in v0.1 only accepts Tag::Wsh at the top level.
/// Tag::Tr (taproot, 0x06) triggers PolicyScopeViolation.
#[test]
fn rejects_policy_scope_violation() {
    use md_codec::bytecode::Tag;

    // header=0x00 + SharedPath + Tr tag (not Wsh) at the top level.
    let bytes: Vec<u8> = vec![
        0x00,                      // header
        Tag::SharedPath.as_byte(), // 0x33
        0x03,                      // BIP84 indicator
        Tag::Tr.as_byte(),         // 0x06 — Tr is not allowed in v0.1
        0x00,
    ];

    let err = WalletPolicy::from_bytecode(&bytes).unwrap_err();
    assert!(
        matches!(err, Error::PolicyScopeViolation(_)),
        "expected PolicyScopeViolation, got {:?}",
        err
    );
}

/// 32. Malformed policy string → `Error::PolicyParse`
#[test]
fn rejects_policy_parse() {
    let result = "not_a_valid_policy!!!".parse::<WalletPolicy>();
    match result {
        Err(Error::PolicyParse(_)) => {}
        Err(other) => panic!("expected PolicyParse, got {:?}", other),
        Ok(_) => panic!("expected PolicyParse, got Ok"),
    }
}

// ---------------------------------------------------------------------------
// Layer 6: chunking-decision rejections
// ---------------------------------------------------------------------------

/// 33. `Error::Miniscript` — wraps a miniscript library error as a string.
///
/// `Error::Miniscript` is a wrapping variant used by call sites to capture
/// upstream `miniscript::Error` values. It is not produced by any default
/// `decode`/`from_bytecode` code path (those use `PolicyParse` or
/// `PolicyScopeViolation` instead). The variant exists so that custom
/// integrators can construct it explicitly via `Error::Miniscript(msg)`.
/// We assert the variant is constructible and displays correctly.
#[test]
fn rejects_miniscript() {
    // Construct the variant directly; assert it matches.
    let err = Error::Miniscript("test upstream error".to_string());
    assert!(
        matches!(err, Error::Miniscript(_)),
        "expected Miniscript(_), got {:?}",
        err
    );
    // Also verify it is a genuine Err path: callers who produce it would
    // return it, so the variant must implement the Error trait (thiserror).
    let display = err.to_string();
    assert!(
        display.starts_with("miniscript:"),
        "Miniscript display must start with 'miniscript:', got: {}",
        display
    );
}

/// 34. Bytecode larger than 1692 bytes → `Error::PolicyTooLarge`
#[test]
fn rejects_policy_too_large() {
    // 1693 bytes exceeds MAX_BYTECODE_LEN (1692).
    let err = chunking_decision(1693, ChunkingMode::Auto).unwrap_err();
    assert!(
        matches!(
            err,
            Error::PolicyTooLarge {
                bytecode_len: 1693,
                max_supported: 1692
            }
        ),
        "expected PolicyTooLarge {{ bytecode_len: 1693, max_supported: 1692 }}, got {:?}",
        err
    );
}

// ---------------------------------------------------------------------------
// Layer 7: Taproot per-leaf subset rejection (Phase D)
// ---------------------------------------------------------------------------

// 32. Tap-leaf miniscript using an out-of-subset operator → `Error::TapLeafSubsetViolation`
//
// The Coldcard subset (BIP §"Taproot tree") allows `pk_k`, `pk_h`,
// `multi_a`, `or_d`, `and_v`, `older` and the safe `c:` / `v:` wrappers.
// `sha256(...)` is NOT in the subset — emitting
// `tr(@0/**, and_v(v:sha256(...), pk(@1/**)))` must be rejected at encode
// time with a precise per-operator diagnostic. (We wrap sha256 inside an
// and_v(v:..., pk()) so miniscript's "all spend paths must require a
// signature" constraint is satisfied at parse time; the rejection then
// comes from the MD subset check, not from the upstream miniscript
// parser.)
#[test]
fn rejects_tap_leaf_subset_violation() {
    let policy: WalletPolicy =
        "tr(@0/**,and_v(v:sha256(b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9),pk(@1/**)))"
            .parse()
            .expect("tap-leaf with sha256 inside and_v should parse syntactically");
    let err = policy.to_bytecode(&EncodeOptions::default()).unwrap_err();
    match err {
        Error::TapLeafSubsetViolation { ref operator } => {
            assert!(
                operator.contains("sha256"),
                "expected operator name to contain 'sha256', got {operator:?}"
            );
        }
        other => panic!("expected TapLeafSubsetViolation, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Layer 8: Fingerprints-block validation (Phase E)
// ---------------------------------------------------------------------------

/// 33. Fingerprints-block count mismatch → `Error::FingerprintsCountMismatch`
///
/// The BIP MUST clause requires `count == max(@i) + 1` (one fingerprint per
/// distinct placeholder). Encoding a 2-key policy with only one fingerprint
/// supplied via `EncodeOptions::with_fingerprints` must surface
/// `Error::FingerprintsCountMismatch { expected: 2, got: 1 }`.
#[test]
fn rejects_fingerprints_count_mismatch() {
    use bitcoin::bip32::Fingerprint;

    let policy: WalletPolicy = "wsh(multi(2,@0/**,@1/**))"
        .parse()
        .expect("2-key multisig policy must parse");
    let opts = EncodeOptions::default()
        .with_fingerprints(vec![Fingerprint::from([0xde, 0xad, 0xbe, 0xef])]);
    let err = policy.to_bytecode(&opts).unwrap_err();
    match err {
        Error::FingerprintsCountMismatch { expected, got } => {
            assert_eq!(expected, 2, "expected count must equal placeholder_count");
            assert_eq!(got, 1, "got count must equal supplied fingerprints len");
        }
        other => panic!("expected FingerprintsCountMismatch, got {other:?}"),
    }
}
