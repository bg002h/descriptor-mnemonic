//! v0 path dictionary: indicator ↔ derivation-path lookup.
//!
//! Maps the 13 well-known indicator bytes defined in BIP §"Path dictionary"
//! (lines 238–276) to their corresponding `DerivationPath` values and back.
//! Special indicators `0xFE` (explicit path) and `0xFF` (no-path reserved)
//! are *not* in this table; they are handled by the framing layer.

use std::sync::LazyLock;

use bitcoin::bip32::DerivationPath;

/// The 13 v0 dictionary entries, parsed once on first access.
static DICT: LazyLock<[(u8, DerivationPath); 13]> = LazyLock::new(|| {
    use std::str::FromStr;
    [
        // Mainnet
        (0x01, DerivationPath::from_str("m/44'/0'/0'").unwrap()),
        (0x02, DerivationPath::from_str("m/49'/0'/0'").unwrap()),
        (0x03, DerivationPath::from_str("m/84'/0'/0'").unwrap()),
        (0x04, DerivationPath::from_str("m/86'/0'/0'").unwrap()),
        (0x05, DerivationPath::from_str("m/48'/0'/0'/2'").unwrap()),
        (0x06, DerivationPath::from_str("m/48'/0'/0'/1'").unwrap()),
        (0x07, DerivationPath::from_str("m/87'/0'/0'").unwrap()),
        // Testnet (0x16 is reserved — intentional gap)
        (0x11, DerivationPath::from_str("m/44'/1'/0'").unwrap()),
        (0x12, DerivationPath::from_str("m/49'/1'/0'").unwrap()),
        (0x13, DerivationPath::from_str("m/84'/1'/0'").unwrap()),
        (0x14, DerivationPath::from_str("m/86'/1'/0'").unwrap()),
        (0x15, DerivationPath::from_str("m/48'/1'/0'/2'").unwrap()),
        (0x17, DerivationPath::from_str("m/87'/1'/0'").unwrap()),
    ]
});

/// Look up the derivation path for a known indicator byte.
///
/// Returns `None` for reserved, unknown, or special (`0xFE`/`0xFF`) indicators.
pub fn indicator_to_path(indicator: u8) -> Option<&'static DerivationPath> {
    DICT.iter()
        .find(|(ind, _)| *ind == indicator)
        .map(|(_, path)| path)
}

/// Look up the indicator byte for a known derivation path.
///
/// Returns `None` if the path is not in the v0 dictionary.
pub fn path_to_indicator(path: &DerivationPath) -> Option<u8> {
    DICT.iter().find(|(_, p)| p == path).map(|(ind, _)| *ind)
}

/// Serialize a derivation path into its wire form for use in a WDM path
/// declaration.
///
/// If the path has a known dictionary indicator (`path_to_indicator` returns
/// `Some(b)`), the output is exactly `[b]` — one byte.
///
/// Otherwise, the output is an explicit encoding:
/// - `0xFE` marker byte
/// - LEB128-encoded component count
/// - For each component, LEB128-encoded child number `2c` (unhardened)
///   or `2c + 1` (hardened), computed as `u64` to avoid overflow.
///
/// This function does **not** prepend `Tag::SharedPath` (0x33); that is the
/// path-declaration framing layer's responsibility.
pub fn encode_path(path: &DerivationPath) -> Vec<u8> {
    use crate::bytecode::varint::encode_u64;
    use bitcoin::bip32::ChildNumber;

    if let Some(indicator) = path_to_indicator(path) {
        return vec![indicator];
    }

    let mut out = Vec::new();
    out.push(0xFE);
    encode_u64(path.len() as u64, &mut out);
    for child in path {
        let encoded = match child {
            ChildNumber::Normal { index } => 2u64 * u64::from(*index),
            ChildNumber::Hardened { index } => 2u64 * u64::from(*index) + 1,
        };
        encode_u64(encoded, &mut out);
    }
    out
}

/// Deserialize a derivation path from its wire form.
///
/// Reads a single indicator byte from `cur`, then either:
/// - Returns the corresponding dictionary path if the byte is one of the 13
///   known indicators (`0x01`–`0x07`, `0x11`–`0x15`, `0x17`).
/// - Decodes an explicit path if the byte is `0xFE`: reads a LEB128 component
///   count, then that many LEB128-encoded child numbers. Each encoded value `n`
///   maps to: hardened `n >> 1` if `n & 1 == 1`, else unhardened `n >> 1`.
///   The maximum legal encoded value per component is `2*(2^31-1)+1 =
///   0xFFFF_FFFF`; values above that are rejected with
///   `BytecodeErrorKind::InvalidPathComponent`.
/// - Rejects any other byte (reserved / `0xFF`) with
///   `BytecodeErrorKind::UnknownTag(b)`. Path indicator bytes share the same
///   "unrecognized 1-byte selector" semantics as operator tags, so reusing
///   `UnknownTag` keeps the error surface small.
///
/// This function does **not** consume or expect a `Tag::SharedPath` prefix;
/// that is the path-declaration framing layer's responsibility (Task 3.5').
// Task 3.5' will call decode_path from the path-declaration framing layer.
// Until that lands, suppress the dead-code lint so clippy stays clean.
#[allow(dead_code)]
pub(crate) fn decode_path(
    cur: &mut crate::bytecode::cursor::Cursor<'_>,
) -> Result<DerivationPath, crate::Error> {
    use crate::error::BytecodeErrorKind;
    use bitcoin::bip32::ChildNumber;

    let indicator_offset = cur.offset();
    let b = cur.read_byte()?;

    // Fast path: dictionary lookup.
    if let Some(path) = indicator_to_path(b) {
        return Ok(path.clone());
    }

    // Explicit form.
    if b == 0xFE {
        let count = cur.read_varint_u64()? as usize;
        let mut components = Vec::with_capacity(count);
        for _ in 0..count {
            let comp_offset = cur.offset();
            let n = cur.read_varint_u64()?;

            // Maximum legal encoded value: 2*(2^31-1)+1 = 0xFFFF_FFFF.
            // Anything above that cannot be expressed as a valid BIP32 child
            // index (0..=2^31-1) in either hardened or unhardened form.
            if n > 0xFFFF_FFFF_u64 {
                return Err(crate::Error::InvalidBytecode {
                    offset: comp_offset,
                    kind: BytecodeErrorKind::InvalidPathComponent { encoded: n },
                });
            }

            let index = (n >> 1) as u32;
            let child = if n & 1 == 1 {
                ChildNumber::Hardened { index }
            } else {
                ChildNumber::Normal { index }
            };
            components.push(child);
        }
        return Ok(DerivationPath::from(components));
    }

    // Reserved / unknown indicator byte.
    // Reusing UnknownTag because indicator bytes share the same semantics as
    // operator tag bytes: both are unrecognized 1-byte selectors in a
    // position where a specific set of values is required.
    Err(crate::Error::InvalidBytecode {
        offset: indicator_offset,
        kind: BytecodeErrorKind::UnknownTag(b),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::cursor::Cursor;
    use crate::error::BytecodeErrorKind;
    use std::str::FromStr;

    // ── decode_path helpers ──────────────────────────────────────────────────

    /// Decode a path from a byte slice, asserting all bytes are consumed.
    fn decode_all(bytes: &[u8]) -> Result<DerivationPath, crate::Error> {
        let mut cur = Cursor::new(bytes);
        let path = decode_path(&mut cur)?;
        // Verify the cursor consumed everything.
        assert_eq!(
            cur.offset(),
            bytes.len(),
            "cursor did not consume all bytes"
        );
        Ok(path)
    }

    // ── decode_dictionary_round_trip ─────────────────────────────────────────

    /// All 13 dictionary entries: encode_path → decode_path returns original path.
    #[test]
    fn decode_dictionary_round_trip() {
        for &(ind, _) in FIXTURE {
            let original = indicator_to_path(ind).expect("fixture entry must exist");
            let encoded = encode_path(original);
            let decoded = decode_all(&encoded)
                .unwrap_or_else(|e| panic!("decode failed for indicator 0x{ind:02x}: {e}"));
            assert_eq!(
                decoded, *original,
                "round-trip mismatch for indicator 0x{ind:02x}"
            );
        }
    }

    // ── decode_explicit_round_trip ───────────────────────────────────────────

    /// Several non-dictionary paths round-trip through encode_path → decode_path.
    #[test]
    fn decode_explicit_round_trip() {
        let cases = [
            "m",
            "m/0",
            "m/0'",
            "m/44'/0",
            "m/44'/0'/1'",
            "m/100",
            "m/2147483647'",
        ];
        for path_str in &cases {
            let original = DerivationPath::from_str(path_str).unwrap();
            let encoded = encode_path(&original);
            let decoded = decode_all(&encoded)
                .unwrap_or_else(|e| panic!("decode failed for '{path_str}': {e}"));
            assert_eq!(decoded, original, "round-trip mismatch for '{path_str}'");
        }
    }

    // ── decode_explicit_canonical_byte_sequences ─────────────────────────────

    /// Pin specific known byte sequences to their expected paths, independently
    /// of encode_path, to guard against both sides drifting together.
    #[test]
    fn decode_explicit_canonical_byte_sequences() {
        // [0xFE, 0x00] → m (empty path, 0 components)
        let empty = decode_all(&[0xFE, 0x00]).unwrap();
        assert_eq!(empty, DerivationPath::from_str("m").unwrap());

        // [0xFE, 0x01, 0x00] → m/0 (1 component, unhardened index 0)
        let m0 = decode_all(&[0xFE, 0x01, 0x00]).unwrap();
        assert_eq!(m0, DerivationPath::from_str("m/0").unwrap());

        // [0xFE, 0x02, 0x59, 0x00] → m/44'/0
        //   count=2, 44 hardened→ 2*44+1=89=0x59, 0 unhardened→0x00
        let m44h0 = decode_all(&[0xFE, 0x02, 0x59, 0x00]).unwrap();
        assert_eq!(m44h0, DerivationPath::from_str("m/44'/0").unwrap());

        // [0xFE, 0x01, 0x01] → m/0' (1 component, hardened index 0; n=1 → n&1==1 → hardened, index=0)
        let m0h = decode_all(&[0xFE, 0x01, 0x01]).unwrap();
        assert_eq!(m0h, DerivationPath::from_str("m/0'").unwrap());
    }

    // ── decode_rejects_reserved_indicator ───────────────────────────────────

    /// Reserved and unknown indicator bytes must be rejected with UnknownTag.
    #[test]
    fn decode_rejects_reserved_indicator() {
        for &b in &[0x00u8, 0x08, 0x10, 0x16, 0x18, 0xFD, 0xFF] {
            let mut cur = Cursor::new(std::slice::from_ref(&b));
            let err = decode_path(&mut cur).unwrap_err();
            assert!(
                matches!(
                    err,
                    crate::Error::InvalidBytecode {
                        kind: BytecodeErrorKind::UnknownTag(got),
                        ..
                    } if got == b
                ),
                "expected UnknownTag(0x{b:02x}), got {err:?}"
            );
        }
    }

    // ── decode_rejects_truncated_explicit ────────────────────────────────────

    /// Truncated explicit-form inputs must be rejected, not panic.
    #[test]
    fn decode_rejects_truncated_explicit() {
        // [0xFE] only — no count byte.
        let err = {
            let mut cur = Cursor::new(&[0xFE]);
            decode_path(&mut cur).unwrap_err()
        };
        assert!(
            matches!(err, crate::Error::InvalidBytecode { .. }),
            "expected InvalidBytecode for missing count, got {err:?}"
        );

        // [0xFE, 0x02, 0x00] — count says 2 components but only 1 is present.
        let err = {
            let mut cur = Cursor::new(&[0xFE, 0x02, 0x00]);
            decode_path(&mut cur).unwrap_err()
        };
        assert!(
            matches!(
                err,
                crate::Error::InvalidBytecode {
                    kind: BytecodeErrorKind::UnexpectedEnd,
                    ..
                }
            ),
            "expected UnexpectedEnd for truncated components, got {err:?}"
        );
    }

    // ── decode_rejects_invalid_child_component ───────────────────────────────

    /// A component whose encoded value > 0xFFFF_FFFF must be rejected with
    /// InvalidPathComponent. The simplest construction: encoded value 2^32
    /// (= 2 * 2^31) in LEB128 = [0x80, 0x80, 0x80, 0x80, 0x10].
    /// Full byte sequence: [0xFE, 0x01, 0x80, 0x80, 0x80, 0x80, 0x10].
    #[test]
    fn decode_rejects_invalid_child_component() {
        // 2^32 = 0x100000000, LEB128 encoding: [0x80, 0x80, 0x80, 0x80, 0x10]
        let bytes = [0xFE, 0x01, 0x80, 0x80, 0x80, 0x80, 0x10];
        let mut cur = Cursor::new(&bytes);
        let err = decode_path(&mut cur).unwrap_err();
        assert!(
            matches!(
                err,
                crate::Error::InvalidBytecode {
                    kind: BytecodeErrorKind::InvalidPathComponent {
                        encoded: 0x100000000
                    },
                    ..
                }
            ),
            "expected InvalidPathComponent {{ encoded: 0x100000000 }}, got {err:?}"
        );
    }

    // ── decode_leaves_cursor_at_correct_offset ───────────────────────────────

    /// After decoding a path, the cursor must be positioned immediately after
    /// the path bytes — not consuming extra bytes, not leaving bytes consumed.
    /// Strategy: encode a path, append a sentinel byte, decode the path, then
    /// read the sentinel and assert it matches.
    #[test]
    fn decode_leaves_cursor_at_correct_offset() {
        const SENTINEL: u8 = 0xAB;

        // Test with a dictionary path (single-byte indicator).
        let path_dict = indicator_to_path(0x01).unwrap();
        let mut encoded_dict = encode_path(path_dict);
        encoded_dict.push(SENTINEL);
        let mut cur = Cursor::new(&encoded_dict);
        let _ = decode_path(&mut cur).unwrap();
        assert_eq!(
            cur.read_byte().unwrap(),
            SENTINEL,
            "cursor not at correct offset after dictionary path decode"
        );

        // Test with an explicit path (m/44'/0, 4 bytes: [0xFE, 0x02, 0x59, 0x00]).
        let path_explicit = DerivationPath::from_str("m/44'/0").unwrap();
        let mut encoded_explicit = encode_path(&path_explicit);
        encoded_explicit.push(SENTINEL);
        let mut cur = Cursor::new(&encoded_explicit);
        let _ = decode_path(&mut cur).unwrap();
        assert_eq!(
            cur.read_byte().unwrap(),
            SENTINEL,
            "cursor not at correct offset after explicit path decode"
        );
    }

    /// Fixture: all 13 (indicator, path_str) pairs.
    const FIXTURE: &[(u8, &str)] = &[
        (0x01, "m/44'/0'/0'"),
        (0x02, "m/49'/0'/0'"),
        (0x03, "m/84'/0'/0'"),
        (0x04, "m/86'/0'/0'"),
        (0x05, "m/48'/0'/0'/2'"),
        (0x06, "m/48'/0'/0'/1'"),
        (0x07, "m/87'/0'/0'"),
        (0x11, "m/44'/1'/0'"),
        (0x12, "m/49'/1'/0'"),
        (0x13, "m/84'/1'/0'"),
        (0x14, "m/86'/1'/0'"),
        (0x15, "m/48'/1'/0'/2'"),
        (0x17, "m/87'/1'/0'"),
    ];

    #[test]
    fn dictionary_round_trip() {
        for &(ind, path_str) in FIXTURE {
            let expected = DerivationPath::from_str(path_str).unwrap();
            let got = indicator_to_path(ind)
                .unwrap_or_else(|| panic!("indicator_to_path(0x{ind:02x}) returned None"));
            assert_eq!(*got, expected, "path mismatch for indicator 0x{ind:02x}");
            assert_eq!(
                path_to_indicator(got),
                Some(ind),
                "path_to_indicator round-trip failed for 0x{ind:02x}"
            );
        }
    }

    #[test]
    fn unknown_indicator_returns_none() {
        for &ind in &[0x00u8, 0x08, 0x10, 0x16, 0x18, 0xFD] {
            assert!(
                indicator_to_path(ind).is_none(),
                "expected None for indicator 0x{ind:02x}"
            );
        }
    }

    #[test]
    fn special_indicators_return_none() {
        assert!(indicator_to_path(0xFE).is_none(), "0xFE should return None");
        assert!(indicator_to_path(0xFF).is_none(), "0xFF should return None");
    }

    #[test]
    fn unknown_path_returns_none() {
        let path = DerivationPath::from_str("m/0").unwrap();
        assert!(
            path_to_indicator(&path).is_none(),
            "m/0 is not in the dictionary"
        );
    }

    #[test]
    fn path_to_indicator_is_path_equality_not_string_equality() {
        // "h" and "'" are both valid hardened markers; DerivationPath normalises
        // them so both forms must resolve to the same indicator.
        let apostrophe = DerivationPath::from_str("m/44'/0'/0'").unwrap();
        let h_form = DerivationPath::from_str("m/44h/0h/0h").unwrap();
        assert_eq!(
            apostrophe, h_form,
            "DerivationPath should normalise hardened markers"
        );
        assert_eq!(
            path_to_indicator(&apostrophe),
            path_to_indicator(&h_form),
            "both forms must yield the same indicator"
        );
        assert_eq!(path_to_indicator(&h_form), Some(0x01));
    }

    // ── encode_path tests ────────────────────────────────────────────────────

    /// All 13 dictionary entries encode to exactly one byte (their indicator).
    #[test]
    fn encode_dictionary_entry_uses_single_byte() {
        for &(ind, _) in FIXTURE {
            let path = indicator_to_path(ind).expect("fixture entry must exist");
            let encoded = encode_path(path);
            assert_eq!(
                encoded,
                vec![ind],
                "dictionary entry 0x{ind:02x} should encode to single byte"
            );
        }
    }

    /// A path not in the dictionary uses the 0xFE explicit form and round-trips.
    #[test]
    fn encode_unknown_path_uses_explicit_form() {
        // m/44'/0'/1' is not in the dictionary (account index 1, not 0).
        let path = DerivationPath::from_str("m/44'/0'/1'").unwrap();
        assert!(
            path_to_indicator(&path).is_none(),
            "test path must not be in the dictionary"
        );
        let encoded = encode_path(&path);
        assert_eq!(encoded[0], 0xFE, "must start with explicit marker");
        assert_eq!(
            decode_all(&encoded).unwrap(),
            path,
            "encode→decode round-trip"
        );
    }

    /// m/0' (single hardened component) → [0xFE, 0x01, 0x01].
    #[test]
    fn encode_explicit_hardened_marker() {
        let path = DerivationPath::from_str("m/0'").unwrap();
        assert_eq!(encode_path(&path), vec![0xFE, 0x01, 0x01]);
    }

    /// m/0 (single unhardened component) → [0xFE, 0x01, 0x00].
    #[test]
    fn encode_explicit_unhardened() {
        let path = DerivationPath::from_str("m/0").unwrap();
        assert_eq!(encode_path(&path), vec![0xFE, 0x01, 0x00]);
    }

    /// m/44'/0 → [0xFE, 0x02, 0x59, 0x00].
    ///   count = 2 → 0x02
    ///   44 hardened → 2*44+1 = 89 = 0x59
    ///   0  unhardened → 0x00
    #[test]
    fn encode_explicit_mixed() {
        let path = DerivationPath::from_str("m/44'/0").unwrap();
        assert_eq!(encode_path(&path), vec![0xFE, 0x02, 0x59, 0x00]);
    }

    /// m (empty path) → [0xFE, 0x00]. Empty path is not in the dictionary.
    #[test]
    fn encode_explicit_empty_path() {
        let path = DerivationPath::from_str("m").unwrap();
        assert!(
            path_to_indicator(&path).is_none(),
            "empty path must not be in the dictionary"
        );
        assert_eq!(encode_path(&path), vec![0xFE, 0x00]);
    }

    /// m/100 exercises multi-byte LEB128: 2*100 = 200 = 0xC8 > 127.
    /// LEB128(200) = [0xC8, 0x01] → output [0xFE, 0x01, 0xC8, 0x01].
    #[test]
    fn encode_explicit_large_child_number() {
        let path = DerivationPath::from_str("m/100").unwrap();
        assert_eq!(encode_path(&path), vec![0xFE, 0x01, 0xC8, 0x01]);
    }

    /// m/2147483647' (max BIP32 hardened index).
    /// encoded = 2*(2^31-1)+1 = 2^32-1 = 0xFFFFFFFF.
    /// LEB128(0xFFFF_FFFF) = [0xFF, 0xFF, 0xFF, 0xFF, 0x0F].
    /// Output: [0xFE, 0x01, 0xFF, 0xFF, 0xFF, 0xFF, 0x0F].
    #[test]
    fn encode_explicit_max_child() {
        let path = DerivationPath::from_str("m/2147483647'").unwrap();
        let mut expected_leb = Vec::new();
        crate::bytecode::varint::encode_u64(0xFFFF_FFFF_u64, &mut expected_leb);
        let mut expected = vec![0xFE, 0x01];
        expected.extend_from_slice(&expected_leb);
        assert_eq!(encode_path(&path), expected);
        // Also verify the LEB128 bytes themselves for documentation.
        assert_eq!(expected_leb, vec![0xFF, 0xFF, 0xFF, 0xFF, 0x0F]);
    }
}
