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
}
