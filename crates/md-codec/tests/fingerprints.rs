//! Phase E (v0.2) — Fingerprints block tests.
//!
//! Coverage per `design/PHASE_v0_2_E_DECISIONS.md`:
//! - Round-trip with fingerprints (header byte 0x04 + Tag::Fingerprints block)
//! - Round-trip without fingerprints (header byte 0x00, no block) — pinning
//!   that v0.1 wire output is preserved when the caller does not opt in
//! - Encoder rejection on count mismatch (E-3)
//! - Decoder rejection on missing Tag::Fingerprints (E-4)
//! - Decoder rejection on count mismatch in bytecode (E-4)
//! - Decoder rejection on truncation mid-block (E-4)
//! - Decoder rejection on truncation before count byte (E-4)
//!
//! See also `tests/conformance.rs::rejects_fingerprints_count_mismatch` for the
//! exhaustiveness-gated rejection assertion.

use bitcoin::bip32::Fingerprint;

use md_codec::{
    BytecodeErrorKind, DecodeOptions, EncodeOptions, Error, WalletPolicy, decode, encode,
};

// ---------------------------------------------------------------------------
// Round-trip tests
// ---------------------------------------------------------------------------

/// Round-trip positive: encode a 2-key wsh-multisig with two fingerprints,
/// decode through the full pipeline, and assert both the encoded
/// `MdBackup.fingerprints` and the decoded `DecodeResult.fingerprints` carry
/// the same two values.
#[test]
fn round_trip_with_fingerprints_two_keys() {
    let policy: WalletPolicy = "wsh(multi(2,@0/**,@1/**))"
        .parse()
        .expect("policy must parse");
    let fps = vec![
        Fingerprint::from([0xde, 0xad, 0xbe, 0xef]),
        Fingerprint::from([0xca, 0xfe, 0xba, 0xbe]),
    ];

    // Encode: MdBackup.fingerprints carries the supplied fingerprints.
    let opts = EncodeOptions::default().with_fingerprints(fps.clone());
    let backup = encode(&policy, &opts).expect("encode must succeed");
    assert_eq!(
        backup.fingerprints.as_deref(),
        Some(fps.as_slice()),
        "MdBackup.fingerprints must reflect EncodeOptions.fingerprints"
    );

    // The on-wire bytecode header MUST set bit 2 (header byte 0x04).
    let bytecode = policy.to_bytecode(&opts).expect("to_bytecode must succeed");
    assert_eq!(
        bytecode[0], 0x04,
        "header byte must be 0x04 when fingerprints are present"
    );
    // Immediately after path declaration (Tag::SharedPath + indicator)
    // must be Tag::Fingerprints (0x35), then count byte 2, then 8 bytes.
    // Path declaration for a template-only policy default-falls-back to
    // BIP 84 mainnet: [Tag::SharedPath=0x33][indicator=0x03], 2 bytes.
    assert_eq!(bytecode[1], 0x33, "byte[1] must be Tag::SharedPath");
    assert_eq!(bytecode[2], 0x03, "byte[2] must be BIP 84 indicator");
    assert_eq!(
        bytecode[3], 0x35,
        "byte[3] must be Tag::Fingerprints right after the path declaration"
    );
    assert_eq!(bytecode[4], 0x02, "byte[4] must be count = 2");
    assert_eq!(
        &bytecode[5..9],
        &[0xde, 0xad, 0xbe, 0xef],
        "bytes[5..9] must be fps[0]"
    );
    assert_eq!(
        &bytecode[9..13],
        &[0xca, 0xfe, 0xba, 0xbe],
        "bytes[9..13] must be fps[1]"
    );

    // Decode: DecodeResult.fingerprints carries the same values.
    let raws: Vec<&str> = backup.chunks.iter().map(|c| c.raw.as_str()).collect();
    let result = decode(&raws, &DecodeOptions::new()).expect("decode must succeed");
    assert_eq!(
        result.fingerprints.as_deref(),
        Some(fps.as_slice()),
        "DecodeResult.fingerprints must round-trip the encoded values"
    );
    // The recovered policy itself must still match.
    assert_eq!(
        result.policy.to_canonical_string(),
        policy.to_canonical_string()
    );
}

/// Round-trip positive (no fingerprints): the default options preserve v0.1
/// wire output — header byte is 0x00 and the decoded `DecodeResult.fingerprints`
/// is `None`.
#[test]
fn round_trip_without_fingerprints_two_keys() {
    let policy: WalletPolicy = "wsh(multi(2,@0/**,@1/**))"
        .parse()
        .expect("policy must parse");
    let opts = EncodeOptions::default();
    let backup = encode(&policy, &opts).expect("encode must succeed");
    assert!(
        backup.fingerprints.is_none(),
        "MdBackup.fingerprints must be None when not opted in"
    );

    let bytecode = policy.to_bytecode(&opts).expect("to_bytecode must succeed");
    assert_eq!(
        bytecode[0], 0x00,
        "header byte must be 0x00 when no fingerprints opt-in"
    );

    let raws: Vec<&str> = backup.chunks.iter().map(|c| c.raw.as_str()).collect();
    let result = decode(&raws, &DecodeOptions::new()).expect("decode must succeed");
    assert!(
        result.fingerprints.is_none(),
        "DecodeResult.fingerprints must be None when no block was encoded"
    );
}

// ---------------------------------------------------------------------------
// Encoder rejection
// ---------------------------------------------------------------------------

/// Encoder validates `fps.len() == placeholder_count`. The conformance test
/// in `conformance.rs` covers the canonical case; this duplicate ensures the
/// expected/got values are correct for the asymmetric "too many" direction.
#[test]
fn encoder_rejects_fingerprints_too_many() {
    let policy: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
    let opts = EncodeOptions::default().with_fingerprints(vec![
        Fingerprint::from([0x11; 4]),
        Fingerprint::from([0x22; 4]),
        Fingerprint::from([0x33; 4]),
    ]);
    let err = policy.to_bytecode(&opts).unwrap_err();
    assert!(
        matches!(
            err,
            Error::FingerprintsCountMismatch {
                expected: 1,
                got: 3
            }
        ),
        "expected FingerprintsCountMismatch {{ expected: 1, got: 3 }}, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// Decoder rejections — hand-crafted bytecode
// ---------------------------------------------------------------------------

/// Header bit 2 set, but the byte after the path declaration is NOT
/// Tag::Fingerprints — the decoder must surface
/// `InvalidBytecode { kind: UnexpectedTag { expected: 0x35, got: <byte> } }`.
#[test]
fn decoder_rejects_missing_fingerprints_tag() {
    use md_codec::bytecode::Tag;

    // Layout: [header=0x04][Tag::SharedPath=0x33][indicator=0x03][Wsh=0x05][...]
    // The Wsh tag (0x05) sits where Tag::Fingerprints (0x35) should be.
    let bytes = vec![
        0x04, // header v0, fingerprints flag set
        Tag::SharedPath.as_byte(),
        0x03,                       // BIP 84 mainnet
        Tag::Wsh.as_byte(),         // 0x05 — wrong tag here
        Tag::Check.as_byte(),       // 0x0C
        Tag::PkK.as_byte(),         // 0x1B
        Tag::Placeholder.as_byte(), // 0x32
        0x00,                       // index 0
    ];
    let err = WalletPolicy::from_bytecode(&bytes).unwrap_err();
    match err {
        Error::InvalidBytecode {
            kind:
                BytecodeErrorKind::UnexpectedTag {
                    expected: 0x35,
                    got: 0x05,
                },
            ..
        } => {}
        other => panic!("expected UnexpectedTag {{ expected: 0x35, got: 0x05 }}, got {other:?}"),
    }
}

/// Header bit 2 set, Tag::Fingerprints present, count byte = 5, but the
/// reconstructed policy has only 2 placeholders. Decoder must surface
/// `FingerprintsCountMismatch { expected: 2, got: 5 }`.
#[test]
fn decoder_rejects_fingerprints_count_mismatch() {
    use md_codec::bytecode::Tag;

    // Tree: wsh(multi(2, @0/**, @1/**))
    //   [Wsh=0x05][Multi=0x19][k=0x02][n=0x02][Placeholder=0x32][0x00]
    //   [Placeholder=0x32][0x01]
    let mut bytes: Vec<u8> = vec![
        0x04, // header v0, fingerprints flag set
        Tag::SharedPath.as_byte(),
        0x03, // BIP 84 mainnet
        Tag::Fingerprints.as_byte(),
        0x05, // count = 5 — wrong; policy has only 2 placeholders
    ];
    // 5 fingerprints worth of bytes (20 bytes) — must read past them
    // before the count-vs-template validation runs.
    bytes.extend_from_slice(&[0u8; 20]);
    // Tree: wsh(multi(2, @0, @1))
    bytes.extend_from_slice(&[
        Tag::Wsh.as_byte(),
        Tag::Multi.as_byte(),
        0x02,
        0x02,
        Tag::Placeholder.as_byte(),
        0x00,
        Tag::Placeholder.as_byte(),
        0x01,
    ]);
    let err = WalletPolicy::from_bytecode(&bytes).unwrap_err();
    match err {
        Error::FingerprintsCountMismatch {
            expected: 2,
            got: 5,
        } => {}
        other => {
            panic!("expected FingerprintsCountMismatch {{ expected: 2, got: 5 }}, got {other:?}")
        }
    }
}

/// Header bit 2 set, Tag::Fingerprints present, count = 2, but only 4 of the
/// 8 expected fingerprint bytes are present. Decoder must surface
/// `InvalidBytecode { kind: UnexpectedEnd }`.
#[test]
fn decoder_rejects_fingerprints_truncated_mid_block() {
    use md_codec::bytecode::Tag;

    let bytes: Vec<u8> = vec![
        0x04, // header v0, fingerprints flag set
        Tag::SharedPath.as_byte(),
        0x03, // BIP 84 mainnet
        Tag::Fingerprints.as_byte(),
        0x02, // count = 2 → expect 8 bytes of fingerprints
        // Only 4 of the 8 fingerprint bytes — truncated.
        0xde,
        0xad,
        0xbe,
        0xef,
    ];
    let err = WalletPolicy::from_bytecode(&bytes).unwrap_err();
    match err {
        Error::InvalidBytecode {
            kind: BytecodeErrorKind::UnexpectedEnd,
            ..
        } => {}
        other => panic!("expected InvalidBytecode {{ UnexpectedEnd }}, got {other:?}"),
    }
}

/// Header bit 2 set, Tag::Fingerprints present, but the buffer ends before the
/// count byte. Decoder must surface `InvalidBytecode { kind: UnexpectedEnd }`.
#[test]
fn decoder_rejects_fingerprints_missing_count_byte() {
    use md_codec::bytecode::Tag;

    let bytes: Vec<u8> = vec![
        0x04, // header v0, fingerprints flag set
        Tag::SharedPath.as_byte(),
        0x03, // BIP 84 mainnet
        Tag::Fingerprints.as_byte(), // 0x35
              // count byte missing
    ];
    let err = WalletPolicy::from_bytecode(&bytes).unwrap_err();
    match err {
        Error::InvalidBytecode {
            kind: BytecodeErrorKind::UnexpectedEnd,
            ..
        } => {}
        other => panic!("expected InvalidBytecode {{ UnexpectedEnd }}, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Byte-layout regression: pin the canonical bytes for the BIP example
// ---------------------------------------------------------------------------

/// Pin the exact bytecode emitted by `wsh(multi(2,@0/**,@1/**))` with
/// fingerprints `[0xdeadbeef, 0xcafebabe]` and the BIP 84 default path.
///
/// The hex string is reproduced verbatim in the BIP draft's
/// §"Fingerprints block" byte-layout example. Any bytecode-format change
/// upstream would break either this test or the BIP example, surfacing the
/// drift on CI before publication.
#[test]
fn fingerprints_block_byte_layout_matches_bip_example() {
    let policy: WalletPolicy = "wsh(multi(2,@0/**,@1/**))"
        .parse()
        .expect("policy must parse");
    let opts = EncodeOptions::default().with_fingerprints(vec![
        Fingerprint::from([0xde, 0xad, 0xbe, 0xef]),
        Fingerprint::from([0xca, 0xfe, 0xba, 0xbe]),
    ]);
    let bytes = policy.to_bytecode(&opts).expect("to_bytecode must succeed");
    let hex: String = bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut acc, b| {
            use std::fmt::Write;
            write!(acc, "{b:02x}").unwrap();
            acc
        });
    // Byte-by-byte breakdown (matches the BIP example's annotation):
    //   04        | header (v0, fingerprints bit set)
    //   33 03     | path declaration: SharedPath, BIP 84 mainnet indicator
    //   35 02     | fingerprints block: tag, count = 2
    //   deadbeef  | fps[0]
    //   cafebabe  | fps[1]
    //   05 19     | wsh, multi
    //   02 02     | k = 2, n = 2
    //   32 00     | @0
    //   32 01     | @1
    assert_eq!(
        hex, "0433033502deadbeefcafebabe0519020232003201",
        "bytecode hex must match the BIP §\"Fingerprints block\" example"
    );
}
