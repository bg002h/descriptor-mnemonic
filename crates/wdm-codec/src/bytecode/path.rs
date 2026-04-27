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

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

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

    /// A path not in the dictionary encodes with the 0xFE explicit marker.
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

        // Decode component count from LEB128 at byte 1.
        let (count, cnt_len) =
            crate::bytecode::varint::decode_u64(&encoded[1..]).expect("valid LEB128 count");
        assert_eq!(count, 3, "m/44'/0'/1' has 3 components");

        // Verify each encoded child number: hardened c → 2c+1.
        // 44' → 89, 0' → 1, 1' → 3
        let expected_raw: &[u64] = &[89, 1, 3];
        let mut pos = 1 + cnt_len;
        for &expected in expected_raw {
            let (val, len) =
                crate::bytecode::varint::decode_u64(&encoded[pos..]).expect("valid LEB128 child");
            assert_eq!(val, expected, "child number mismatch");
            pos += len;
        }
        assert_eq!(pos, encoded.len(), "no trailing bytes");
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
