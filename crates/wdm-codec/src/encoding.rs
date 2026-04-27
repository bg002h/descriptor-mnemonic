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

/// The bech32 32-character alphabet, in 5-bit-value order.
///
/// `q=0, p=1, z=2, r=3, y=4, 9=5, x=6, 8=7, g=8, f=9, 2=10, t=11, v=12,
///  d=13, w=14, 0=15, s=16, 3=17, j=18, n=19, 5=20, 4=21, k=22, h=23,
///  c=24, e=25, 6=26, m=27, u=28, a=29, 7=30, l=31`.
pub const ALPHABET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// Inverse lookup: char (lowercase ASCII) -> 5-bit value, or 0xFF if not in alphabet.
// Used by Task 1.3 (HRP/length validation).
#[allow(dead_code)]
const ALPHABET_INV: [u8; 128] = build_alphabet_inv();

const fn build_alphabet_inv() -> [u8; 128] {
    let mut inv = [0xFFu8; 128];
    let mut i = 0;
    while i < 32 {
        inv[ALPHABET[i] as usize] = i as u8;
        i += 1;
    }
    inv
}

/// Convert a sequence of 8-bit bytes to a sequence of 5-bit values
/// (padded with zero bits at the end if the bit count is not a multiple of 5).
pub fn bytes_to_5bit(bytes: &[u8]) -> Vec<u8> {
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    let mut out = Vec::with_capacity((bytes.len() * 8).div_ceil(5));
    for &b in bytes {
        acc = (acc << 8) | b as u32;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(((acc >> bits) & 0x1F) as u8);
        }
    }
    if bits > 0 {
        out.push(((acc << (5 - bits)) & 0x1F) as u8);
    }
    out
}

/// Convert a sequence of 5-bit values back to 8-bit bytes.
///
/// Returns `None` if any value in `values` is ≥ 32 (out of 5-bit range),
/// or if the trailing padding bits are non-zero.
pub fn five_bit_to_bytes(values: &[u8]) -> Option<Vec<u8>> {
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    let mut out = Vec::with_capacity(values.len() * 5 / 8);
    for &v in values {
        if v >= 32 {
            return None;
        }
        acc = (acc << 5) | v as u32;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push(((acc >> bits) & 0xFF) as u8);
        }
    }
    // Any remaining bits must be zero (padding).
    if bits >= 5 {
        return None;
    }
    if (acc & ((1 << bits) - 1)) != 0 {
        return None;
    }
    Some(out)
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

    #[test]
    fn alphabet_is_32_unique_chars() {
        let mut seen = std::collections::HashSet::new();
        for &c in ALPHABET {
            assert!(seen.insert(c), "duplicate char in alphabet: {}", c as char);
        }
        assert_eq!(seen.len(), 32);
    }

    #[test]
    fn bytes_to_5bit_round_trip_zero() {
        let bytes = vec![0x00];
        let fives = bytes_to_5bit(&bytes);
        assert_eq!(fives, vec![0, 0]);
        let back = five_bit_to_bytes(&fives).unwrap();
        assert_eq!(back, bytes);
    }

    #[test]
    fn bytes_to_5bit_round_trip_known_value() {
        // 0xFF = binary 11111111. Splits as 11111 (=31) and 111 (padded with 00 to 11100=28).
        let bytes = vec![0xFF];
        let fives = bytes_to_5bit(&bytes);
        assert_eq!(fives, vec![31, 28]);
    }

    #[test]
    fn bytes_to_5bit_round_trip_multibyte() {
        // 3 bytes = 24 bits → 5 five-bit groups (25 bits, 1 pad bit).
        let bytes = vec![0xDE, 0xAD, 0xBE];
        let back = five_bit_to_bytes(&bytes_to_5bit(&bytes)).unwrap();
        assert_eq!(back, bytes);
    }

    #[test]
    fn five_bit_to_bytes_rejects_nonzero_padding() {
        // Two 5-bit values = 10 bits, of which 8 form a byte and 2 are padding.
        // If padding bits are nonzero, decode must fail.
        // 31 = 11111, 1 = 00001. Last 2 bits (= 01) are nonzero padding.
        assert!(five_bit_to_bytes(&[31, 1]).is_none());
    }

    #[test]
    fn five_bit_to_bytes_rejects_value_out_of_range() {
        assert!(five_bit_to_bytes(&[32]).is_none());
    }
}
