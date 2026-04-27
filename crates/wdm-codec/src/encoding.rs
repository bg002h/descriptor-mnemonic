//! Encoding layer: bech32 alphabet conversion and BCH error correction.
//!
//! Implements the codex32-derived (BIP 93) encoding with HRP `"wdm"`.

/// Which BCH code variant a string uses.
///
/// Determined by the total data-part length: regular for ≤93 chars,
/// long for 96–108 chars. Lengths 94–95 are reserved-invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BchCode {
    /// Regular code: BCH(93,80,8). 13-char checksum.
    Regular,
    /// Long code: BCH(108,93,8). 15-char checksum.
    Long,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bch_code_equality() {
        assert_eq!(BchCode::Regular, BchCode::Regular);
        assert_ne!(BchCode::Regular, BchCode::Long);
    }

    #[test]
    fn bch_code_can_be_hashed() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(BchCode::Regular);
        set.insert(BchCode::Long);
        set.insert(BchCode::Regular);
        assert_eq!(set.len(), 2);
    }
}
