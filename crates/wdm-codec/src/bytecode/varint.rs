//! LEB128 unsigned variable-length integer encoding for u64.
//!
//! Used by the canonical bytecode for path child numbers, threshold counts,
//! timelock values, and length prefixes. Standard LEB128 (DWARF / WebAssembly):
//! 7 bits of payload per byte, top bit = continuation flag.

/// Encode an unsigned `u64` as LEB128 bytes, appending to `out`.
///
/// Each output byte carries 7 bits of payload in the low bits; the high bit
/// is `1` for all bytes except the last. Encoding `0` produces `[0]`.
pub fn encode_u64(mut value: u64, out: &mut Vec<u8>) {
    loop {
        let byte = (value & 0x7F) as u8;
        value >>= 7;
        if value == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

/// Decode an unsigned LEB128 value from a byte slice.
///
/// Returns `Some((value, bytes_consumed))` on success, `None` if the input
/// is truncated (last byte still has continuation flag) or would overflow
/// `u64` (more than 10 LEB128 bytes, or the 10th byte has bits set above
/// position 0 in its low 7 payload bits).
pub fn decode_u64(bytes: &[u8]) -> Option<(u64, usize)> {
    let mut value: u64 = 0;
    let mut shift: u32 = 0;
    for (i, &b) in bytes.iter().enumerate() {
        if shift >= 64 {
            // We've already shifted in 64 bits worth and the previous byte
            // had the continuation flag set; any further input overflows u64.
            return None;
        }
        let chunk = (b & 0x7F) as u64;
        // Detect overflow: chunk left-shifted by `shift` must fit in u64.
        let shifted = chunk.checked_shl(shift)?;
        // Detect overflow when shift = 63: a payload byte with anything
        // above bit 0 in its 7-bit chunk would overflow.
        if shift == 63 && chunk > 1 {
            return None;
        }
        value |= shifted;
        if b & 0x80 == 0 {
            return Some((value, i + 1));
        }
        shift += 7;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_zero() {
        let mut buf = Vec::new();
        encode_u64(0, &mut buf);
        assert_eq!(buf, vec![0]);
        assert_eq!(decode_u64(&buf), Some((0, 1)));
    }

    #[test]
    fn encode_decode_127() {
        // 127 = 0b0111_1111, fits in one byte (top bit clear).
        let mut buf = Vec::new();
        encode_u64(127, &mut buf);
        assert_eq!(buf, vec![0x7F]);
        assert_eq!(decode_u64(&buf), Some((127, 1)));
    }

    #[test]
    fn encode_decode_128() {
        // 128 = 0b1000_0000, requires two bytes: low 7 bits = 0 with
        // continuation flag, then high bit = 1.
        let mut buf = Vec::new();
        encode_u64(128, &mut buf);
        assert_eq!(buf, vec![0x80, 0x01]);
        assert_eq!(decode_u64(&buf), Some((128, 2)));
    }

    #[test]
    fn encode_decode_known_timelocks() {
        // 1_200_000 (block height): 3 LEB128 bytes.
        let mut buf = Vec::new();
        encode_u64(1_200_000, &mut buf);
        assert_eq!(buf.len(), 3);
        assert_eq!(decode_u64(&buf), Some((1_200_000, 3)));

        // 4_032 blocks (~28 days): 2 bytes.
        let mut buf = Vec::new();
        encode_u64(4_032, &mut buf);
        assert_eq!(buf.len(), 2);
        assert_eq!(decode_u64(&buf), Some((4_032, 2)));

        // 52_560 blocks (1 year): 3 bytes.
        let mut buf = Vec::new();
        encode_u64(52_560, &mut buf);
        assert_eq!(buf.len(), 3);
        assert_eq!(decode_u64(&buf), Some((52_560, 3)));
    }

    #[test]
    fn encode_decode_u64_max() {
        // Boundary: u64::MAX should round-trip.
        let mut buf = Vec::new();
        encode_u64(u64::MAX, &mut buf);
        assert_eq!(decode_u64(&buf), Some((u64::MAX, buf.len())));
    }

    #[test]
    fn decode_rejects_truncated() {
        // 0x80 with no continuation byte.
        assert_eq!(decode_u64(&[0x80]), None);
        // Multi-byte truncation.
        assert_eq!(decode_u64(&[0x80, 0x80]), None);
        // Empty input.
        assert_eq!(decode_u64(&[]), None);
    }

    #[test]
    fn decode_rejects_overflow() {
        // 11 continuation bytes plus a terminator: too many bytes for u64.
        let too_long: Vec<u8> = vec![0xFF; 10].into_iter().chain(std::iter::once(0x01)).collect();
        assert_eq!(decode_u64(&too_long), None);
        // 10 bytes where the last one has bits beyond what fits in u64
        // (10th byte is the highest-order; must be ≤ 0x01 to fit).
        let overflow: Vec<u8> = vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x02];
        assert_eq!(decode_u64(&overflow), None);
    }

    #[test]
    fn decode_returns_correct_byte_count_with_trailing_data() {
        // Input has one full LEB128 followed by extra bytes; decode_u64
        // should report exactly the consumed length.
        let buf: Vec<u8> = vec![0x80, 0x01, 0xAA, 0xBB];
        assert_eq!(decode_u64(&buf), Some((128, 2)));
    }
}
