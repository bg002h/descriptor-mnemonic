//! v0 path dictionary: indicator ↔ derivation-path lookup.
//!
//! Maps the 13 well-known indicator bytes defined in BIP §"Path dictionary"
//! (lines 238–276) to their corresponding `DerivationPath` values and back.
//! Special indicators `0xFE` (explicit path) and `0xFF` (no-path reserved)
//! are *not* in this table; they are handled by the framing layer.

use std::str::FromStr;
use std::sync::LazyLock;

use bitcoin::bip32::DerivationPath;

/// The 13 v0 dictionary entries, parsed once on first access.
static DICT: LazyLock<[(u8, DerivationPath); 14]> = LazyLock::new(|| {
    [
        // Mainnet
        (0x01, DerivationPath::from_str("m/44'/0'/0'").unwrap()),
        (0x02, DerivationPath::from_str("m/49'/0'/0'").unwrap()),
        (0x03, DerivationPath::from_str("m/84'/0'/0'").unwrap()),
        (0x04, DerivationPath::from_str("m/86'/0'/0'").unwrap()),
        (0x05, DerivationPath::from_str("m/48'/0'/0'/2'").unwrap()),
        (0x06, DerivationPath::from_str("m/48'/0'/0'/1'").unwrap()),
        (0x07, DerivationPath::from_str("m/87'/0'/0'").unwrap()),
        // Testnet
        (0x11, DerivationPath::from_str("m/44'/1'/0'").unwrap()),
        (0x12, DerivationPath::from_str("m/49'/1'/0'").unwrap()),
        (0x13, DerivationPath::from_str("m/84'/1'/0'").unwrap()),
        (0x14, DerivationPath::from_str("m/86'/1'/0'").unwrap()),
        (0x15, DerivationPath::from_str("m/48'/1'/0'/2'").unwrap()),
        (0x16, DerivationPath::from_str("m/48'/1'/0'/1'").unwrap()),
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

/// Serialize a derivation path into its wire form for use in an MD path
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
///   known indicators (`0x01`–`0x07`, `0x11`–`0x17`).
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
        let count_offset = cur.offset();
        let count_raw = cur.read_varint_u64()?;
        let count = usize::try_from(count_raw).map_err(|_| crate::Error::InvalidBytecode {
            offset: count_offset,
            kind: crate::error::BytecodeErrorKind::VarintOverflow,
        })?;
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

            // The bound check above (n <= 0xFFFF_FFFF) guarantees that
            // n >> 1 <= 0x7FFF_FFFF, which fits in u32. The expect documents
            // this proof and will crash loudly if the bound is ever weakened.
            let index = u32::try_from(n >> 1)
                .expect("bound-checked above: n <= 0xFFFF_FFFF so n>>1 <= 0x7FFF_FFFF");
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

/// Serialize a path declaration into its wire form.
///
/// A path declaration is a `Tag::SharedPath` (0x33) byte followed by the
/// `encode_path` output:
/// - 1 byte for dictionary-form paths: `[0x33, indicator]`
/// - 1 + 1 + N bytes for explicit paths: `[0x33, 0xFE, count, …components]`
///
/// This is the framing layer defined in BIP §"Path declaration" (lines 222–236).
/// The `Tag::SharedPath` byte is prepended here; `encode_path` handles the rest.
///
/// Intended to be called from the Phase 5 `encode_bytecode` wrapper, which
/// concatenates `[header_byte] ++ encode_declaration(path) ++ encode_template(…)`.
pub fn encode_declaration(path: &DerivationPath) -> Vec<u8> {
    use crate::bytecode::Tag;
    let mut out = Vec::new();
    out.push(Tag::SharedPath.as_byte());
    out.extend_from_slice(&encode_path(path));
    out
}

/// Deserialize a path declaration from a cursor-style byte stream.
///
/// Reads a `Tag::SharedPath` (0x33) tag byte, then delegates to `decode_path`
/// to consume the remainder of the declaration. The cursor is advanced past all
/// consumed bytes, leaving it positioned at the first byte of the next structure
/// (e.g., the template tree).
///
/// # Errors
///
/// - `InvalidBytecode { kind: UnexpectedEnd }` — if the stream is empty or the
///   path indicator byte is missing.
/// - `InvalidBytecode { kind: UnknownTag(b) }` — if the first byte is not a
///   defined tag at all.
/// - `InvalidBytecode { kind: UnexpectedTag { expected: 0x33, got: b } }` — if
///   the first byte is a defined tag but not `Tag::SharedPath`.
/// - Any error from `decode_path` for malformed path data.
///
/// This is `pub(crate)` because `Cursor` is `pub(crate)`. The Phase 5 framing
/// layer operates on the same cursor and calls this function directly.
pub(crate) fn decode_declaration(
    cur: &mut crate::bytecode::cursor::Cursor<'_>,
) -> Result<DerivationPath, crate::Error> {
    use crate::bytecode::Tag;
    use crate::error::BytecodeErrorKind;

    let tag_offset = cur.offset();
    let b = cur.read_byte()?;

    match Tag::from_byte(b) {
        Some(Tag::SharedPath) => {
            // Correct tag — proceed to decode the path indicator + data.
            decode_path(cur)
        }
        Some(_other) => {
            // A valid tag, but not the one expected here.
            Err(crate::Error::InvalidBytecode {
                offset: tag_offset,
                kind: BytecodeErrorKind::UnexpectedTag {
                    expected: Tag::SharedPath.as_byte(),
                    got: b,
                },
            })
        }
        None => {
            // Not a defined tag at all — reuse the existing UnknownTag variant.
            Err(crate::Error::InvalidBytecode {
                offset: tag_offset,
                kind: BytecodeErrorKind::UnknownTag(b),
            })
        }
    }
}

/// Decode a path declaration from a byte slice and return the parsed
/// [`DerivationPath`] together with the number of bytes consumed.
///
/// Slice-consuming, public-facing entry point for parsing a single path
/// declaration. Use this when you have a contiguous byte buffer and want to:
///
/// - decode a single path declaration without constructing a `Cursor`, or
/// - know how far into the buffer the declaration ends, so you can
///   continue parsing whatever follows.
///
/// # Returns
///
/// `(path, bytes_consumed)` on success. `bytes_consumed` reflects only the
/// declaration itself; trailing bytes after the declaration are not
/// inspected and not counted.
///
/// # Errors
///
/// - [`crate::Error::InvalidBytecode`] with `kind: UnknownTag(b)` if the
///   first byte is not a defined tag.
/// - [`crate::Error::InvalidBytecode`] with
///   `kind: UnexpectedTag { expected: 0x33, got: b }` if the first byte is
///   a defined tag but not `Tag::SharedPath`.
/// - Any [`crate::Error::InvalidBytecode`] variant produced by the inner
///   path decoder for malformed path data (e.g. `UnexpectedEnd`,
///   `VarintOverflow`, `InvalidPathComponent`).
pub fn decode_declaration_from_bytes(
    bytes: &[u8],
) -> Result<(DerivationPath, usize), crate::Error> {
    let mut cur = crate::bytecode::cursor::Cursor::new(bytes);
    let path = decode_declaration(&mut cur)?;
    Ok((path, cur.offset()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::Tag;
    use crate::bytecode::cursor::Cursor;
    use crate::error::BytecodeErrorKind;

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

    // ── indicator_0x16_decodes_to_bip48_testnet_p2sh_p2wsh ──────────────────

    /// Closes md-path-dictionary-0x16-gap (mk1-surfaced FOLLOWUPS, v0.9.0).
    /// Mainnet indicator 0x06 is m/48'/0'/0'/1' (BIP 48 nested-segwit
    /// P2SH-P2WSH); the testnet companion 0x16 = m/48'/1'/0'/1' was missed
    /// in v0.x. v0.9 closes the gap.
    #[test]
    fn indicator_0x16_decodes_to_bip48_testnet_p2sh_p2wsh() {
        let expected = DerivationPath::from_str("m/48'/1'/0'/1'").unwrap();
        let got = indicator_to_path(0x16).expect("0x16 must map to a path after v0.9");
        assert_eq!(*got, expected);
        assert_eq!(path_to_indicator(got), Some(0x16));
    }

    // ── decode_rejects_reserved_indicator ───────────────────────────────────

    /// Reserved and unknown indicator bytes must be rejected with UnknownTag.
    #[test]
    fn decode_rejects_reserved_indicator() {
        for &b in &[0x00u8, 0x08, 0x10, 0x18, 0xFD, 0xFF] {
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
            matches!(
                err,
                crate::Error::InvalidBytecode {
                    kind: BytecodeErrorKind::UnexpectedEnd,
                    ..
                }
            ),
            "expected UnexpectedEnd for missing count, got {err:?}"
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

    /// Fixture: all 14 (indicator, path_str) pairs.
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
        (0x16, "m/48'/1'/0'/1'"),
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
        for &ind in &[0x00u8, 0x08, 0x10, 0x18, 0xFD] {
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

    /// `path_to_indicator` must use exact-equality, not prefix-match.
    /// `m/44'/0'/0'` is indicator 0x01; `m/44'/0'/0'/0` (one extra component)
    /// must return `None`, pinning that the lookup is not a prefix test.
    #[test]
    fn path_to_indicator_rejects_prefix_extension() {
        let base = DerivationPath::from_str("m/44'/0'/0'").unwrap();
        assert_eq!(
            path_to_indicator(&base),
            Some(0x01),
            "base path m/44'/0'/0' must be in dictionary"
        );
        let extended = DerivationPath::from_str("m/44'/0'/0'/0").unwrap();
        assert!(
            path_to_indicator(&extended).is_none(),
            "m/44'/0'/0'/0 has one extra component and must not match indicator 0x01"
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

    /// 128 unhardened `0` children — the component *count* itself becomes a
    /// 2-byte LEB128 (128 = 0x80, 0x01). Encodes as:
    ///   [0xFE, 0x80, 0x01, <128 × 0x00>]
    /// Verifies that neither encode_path nor decode_path hardcodes a single-byte
    /// count. Round-trips through encode → decode and checks the byte prefix.
    #[test]
    fn decode_path_round_trip_multi_byte_component_count() {
        use bitcoin::bip32::ChildNumber;

        let components = vec![ChildNumber::Normal { index: 0 }; 128];
        let original = DerivationPath::from(components);

        let encoded = encode_path(&original);

        // Count of 128 in LEB128 = [0x80, 0x01].
        assert_eq!(encoded[0], 0xFE, "must start with explicit marker 0xFE");
        assert_eq!(encoded[1], 0x80, "count LSB: LEB128(128) byte 0 = 0x80");
        assert_eq!(encoded[2], 0x01, "count MSB: LEB128(128) byte 1 = 0x01");
        // Total length: 1 (marker) + 2 (count) + 128 (components, each 0x00) = 131.
        assert_eq!(
            encoded.len(),
            131,
            "encoded length must be 1 + 2 + 128 = 131 bytes"
        );

        let decoded = decode_all(&encoded).expect("decode must succeed for 128-component path");
        assert_eq!(
            decoded, original,
            "round-trip mismatch for 128-component path"
        );
    }

    // ── path declaration tests (Task 3.5') ──────────────────────────────────

    /// Helper: decode a declaration from a full byte slice via a cursor.
    fn decode_declaration_from_slice(bytes: &[u8]) -> Result<DerivationPath, crate::Error> {
        let mut cur = Cursor::new(bytes);
        decode_declaration(&mut cur)
    }

    /// All 13 dictionary entries round-trip through encode_declaration →
    /// decode_declaration. Output must be 2 bytes: [Tag::SharedPath, indicator].
    #[test]
    fn declaration_round_trip_dict() {
        for &(ind, _) in FIXTURE {
            let original = indicator_to_path(ind).expect("fixture entry must exist");
            let encoded = encode_declaration(original);
            assert_eq!(
                encoded.len(),
                2,
                "dictionary declaration must be 2 bytes for indicator 0x{ind:02x}"
            );
            assert_eq!(
                encoded[0],
                Tag::SharedPath.as_byte(),
                "first byte must be Tag::SharedPath"
            );
            assert_eq!(encoded[1], ind, "second byte must be the indicator");
            let decoded = decode_declaration_from_slice(&encoded)
                .unwrap_or_else(|e| panic!("decode failed for indicator 0x{ind:02x}: {e}"));
            assert_eq!(
                decoded, *original,
                "round-trip mismatch for indicator 0x{ind:02x}"
            );
        }
    }

    /// Non-dictionary paths round-trip through encode_declaration →
    /// decode_declaration. Output starts with [Tag::SharedPath, 0xFE, …].
    #[test]
    fn declaration_round_trip_explicit() {
        let cases = ["m", "m/0'", "m/100", "m/44'/0", "m/44'/0'/1'"];
        for path_str in &cases {
            let original = DerivationPath::from_str(path_str).unwrap();
            let encoded = encode_declaration(&original);
            assert_eq!(
                encoded[0],
                Tag::SharedPath.as_byte(),
                "first byte must be Tag::SharedPath"
            );
            assert_eq!(encoded[1], 0xFE, "second byte must be explicit marker 0xFE");
            let decoded = decode_declaration_from_slice(&encoded)
                .unwrap_or_else(|e| panic!("decode failed for '{path_str}': {e}"));
            assert_eq!(decoded, original, "round-trip mismatch for '{path_str}'");
        }
    }

    /// Pin the wire format: encode_declaration("m/44'/0'/0'") == [Tag::SharedPath, 0x01].
    #[test]
    fn encode_declaration_dictionary_byte_layout() {
        let path = DerivationPath::from_str("m/44'/0'/0'").unwrap();
        assert_eq!(
            encode_declaration(&path),
            vec![Tag::SharedPath.as_byte(), 0x01]
        );
    }

    /// Pin the wire format: encode_declaration("m/44'/0") == [Tag::SharedPath, 0xFE, 0x02, 0x59, 0x00].
    ///   count=2, 44 hardened → 2*44+1=89=0x59, 0 unhardened → 0x00
    #[test]
    fn encode_declaration_explicit_byte_layout() {
        let path = DerivationPath::from_str("m/44'/0").unwrap();
        assert_eq!(
            encode_declaration(&path),
            vec![Tag::SharedPath.as_byte(), 0xFE, 0x02, 0x59, 0x00]
        );
    }

    /// decode_declaration with a first byte that is a defined tag but not
    /// Tag::SharedPath must return UnexpectedTag.
    #[test]
    fn decode_declaration_rejects_wrong_tag() {
        // Tag::Wsh — a valid defined tag, but not Tag::SharedPath.
        let bytes = [Tag::Wsh.as_byte(), 0x01];
        let err = decode_declaration_from_slice(&bytes).unwrap_err();
        assert!(
            matches!(
                err,
                crate::Error::InvalidBytecode {
                    kind: BytecodeErrorKind::UnexpectedTag {
                        expected,
                        got,
                    },
                    ..
                } if expected == Tag::SharedPath.as_byte() && got == Tag::Wsh.as_byte()
            ),
            "expected UnexpectedTag {{ expected: Tag::SharedPath, got: Tag::Wsh }}, got {err:?}"
        );
    }

    /// decode_declaration with a first byte that is not any defined tag must
    /// return UnknownTag — not UnexpectedTag.
    #[test]
    fn decode_declaration_rejects_unknown_tag() {
        // 0xC0 is not a defined tag byte.
        let bytes = [0xC0u8];
        let err = decode_declaration_from_slice(&bytes).unwrap_err();
        assert!(
            matches!(
                err,
                crate::Error::InvalidBytecode {
                    kind: BytecodeErrorKind::UnknownTag(0xC0),
                    ..
                }
            ),
            "expected UnknownTag(0xC0), got {err:?}"
        );
    }

    /// After decode_declaration, the cursor must be positioned immediately after
    /// the declaration bytes. Strategy: append a sentinel byte, decode the
    /// declaration, then read the sentinel and verify it.
    #[test]
    fn decode_declaration_advances_cursor() {
        const SENTINEL: u8 = 0xAB;

        // Dictionary path: declaration is 2 bytes, sentinel at index 2.
        let path_dict = indicator_to_path(0x01).unwrap();
        let mut buf = encode_declaration(path_dict);
        buf.push(SENTINEL);
        let mut cur = Cursor::new(&buf);
        let _ = decode_declaration(&mut cur).unwrap();
        assert_eq!(
            cur.read_byte().unwrap(),
            SENTINEL,
            "cursor not advanced correctly after dictionary declaration decode"
        );

        // Explicit path: declaration is 5 bytes, sentinel at index 5.
        let path_explicit = DerivationPath::from_str("m/44'/0").unwrap();
        let mut buf = encode_declaration(&path_explicit);
        buf.push(SENTINEL);
        let mut cur = Cursor::new(&buf);
        let _ = decode_declaration(&mut cur).unwrap();
        assert_eq!(
            cur.read_byte().unwrap(),
            SENTINEL,
            "cursor not advanced correctly after explicit declaration decode"
        );
    }

    /// [Tag::SharedPath] alone (no indicator byte) must return UnexpectedEnd when
    /// decode_path tries to read the missing indicator byte.
    #[test]
    fn decode_declaration_rejects_truncated() {
        let err = decode_declaration_from_slice(&[Tag::SharedPath.as_byte()]).unwrap_err();
        assert!(
            matches!(
                err,
                crate::Error::InvalidBytecode {
                    kind: BytecodeErrorKind::UnexpectedEnd,
                    ..
                }
            ),
            "expected UnexpectedEnd for truncated declaration, got {err:?}"
        );
    }

    // ── decode_declaration_from_bytes (slice-consuming pub API) ──────────────

    /// Dictionary indicator: from_bytes returns the right path and the
    /// declaration's exact byte length (2 for dictionary form).
    #[test]
    fn from_bytes_dictionary_returns_path_and_length() {
        let path_dict = indicator_to_path(0x01).unwrap();
        let buf = encode_declaration(path_dict);
        let (decoded, consumed) = decode_declaration_from_bytes(&buf).unwrap();
        assert_eq!(&decoded, path_dict, "decoded path mismatch");
        assert_eq!(consumed, 2, "dictionary declaration should consume 2 bytes");
        assert_eq!(consumed, buf.len(), "consumed should equal buffer length");
    }

    /// Explicit (non-dictionary) declaration: from_bytes returns the right
    /// path and the exact declaration byte length.
    #[test]
    fn from_bytes_explicit_returns_path_and_length() {
        let path_explicit = DerivationPath::from_str("m/44'/0").unwrap();
        let buf = encode_declaration(&path_explicit);
        let (decoded, consumed) = decode_declaration_from_bytes(&buf).unwrap();
        assert_eq!(decoded, path_explicit, "decoded path mismatch");
        assert_eq!(
            consumed,
            buf.len(),
            "explicit declaration consumed should equal buffer length"
        );
        assert_eq!(consumed, 5, "this specific path encodes to 5 bytes");
    }

    /// Trailing bytes after the declaration must NOT be consumed; the
    /// `consumed` count reports only the declaration's own length.
    #[test]
    fn from_bytes_does_not_consume_trailing_bytes() {
        let path_dict = indicator_to_path(0x01).unwrap();
        let mut buf = encode_declaration(path_dict);
        buf.extend_from_slice(&[0xAB, 0xCD, 0xEF]);
        let (decoded, consumed) = decode_declaration_from_bytes(&buf).unwrap();
        assert_eq!(&decoded, path_dict);
        assert_eq!(consumed, 2, "trailing bytes must not affect consumed count");
        assert!(
            consumed < buf.len(),
            "consumed must be less than buf.len() when trailing bytes present"
        );
    }

    /// Errors propagate identically to the cursor-based decode_declaration.
    #[test]
    fn from_bytes_propagates_errors() {
        // Wrong tag (Tag::Wsh).
        let err = decode_declaration_from_bytes(&[Tag::Wsh.as_byte(), 0x01]).unwrap_err();
        assert!(
            matches!(
                err,
                crate::Error::InvalidBytecode {
                    kind: BytecodeErrorKind::UnexpectedTag {
                        expected,
                        got,
                    },
                    ..
                } if expected == Tag::SharedPath.as_byte() && got == Tag::Wsh.as_byte()
            ),
            "expected UnexpectedTag, got {err:?}"
        );

        // Truncated (just the tag byte).
        let err = decode_declaration_from_bytes(&[Tag::SharedPath.as_byte()]).unwrap_err();
        assert!(
            matches!(
                err,
                crate::Error::InvalidBytecode {
                    kind: BytecodeErrorKind::UnexpectedEnd,
                    ..
                }
            ),
            "expected UnexpectedEnd for truncated, got {err:?}"
        );
    }
}
