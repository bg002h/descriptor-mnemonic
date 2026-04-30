//! v0.11 ↔ codex32 BCH layer adapter, symbol-aligned per spec §3.1 / D7.
//!
//! Bypasses v0.x's byte-oriented `encode_string` / `decode_string` to avoid
//! adding an extra codex32 char per encoding due to byte-padding. Uses v0.x's
//! lower-level BCH primitives (`bch_create_checksum_regular`,
//! `bch_verify_regular`) which operate on `&[u8]` slices of 5-bit symbols.

use crate::v11::bitstream::{BitReader, BitWriter};
use crate::v11::error::V11Error;

/// Codex32 alphabet (BIP 173 lowercase). Each char = one 5-bit symbol.
const CODEX32_ALPHABET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// HRP for v0.11 (matches v0.x).
const HRP: &str = "md";

/// Regular-BCH checksum length, in 5-bit symbols.
pub(crate) const REGULAR_CHECKSUM_SYMBOLS: usize = 13;

/// Pack `bit_count` bits from `payload_bytes` into 5-bit symbols. Pads the
/// final symbol with zeros if `bit_count` is not a multiple of 5. Returns
/// `ceil(bit_count / 5)` symbols. Each output u8 contains a 5-bit value.
fn bits_to_symbols(payload_bytes: &[u8], bit_count: usize) -> Result<Vec<u8>, V11Error> {
    let symbol_count = (bit_count + 4) / 5;
    let mut r = BitReader::with_bit_limit(payload_bytes, bit_count);
    let mut symbols = Vec::with_capacity(symbol_count);
    for _ in 0..symbol_count {
        let take = r.remaining_bits().min(5);
        let val = if take == 0 { 0 } else { r.read_bits(take)? as u8 };
        // Left-justify within 5 bits if final symbol is short. (For decoder
        // round-trip purposes the spec defines bit-packing MSB-first into
        // 5-bit symbols, so zero-padding the LOW bits of the final symbol is
        // the canonical form.)
        let symbol = (val << (5 - take as u32)) & 0x1F;
        symbols.push(symbol);
    }
    Ok(symbols)
}

/// Convert a stream of 5-bit symbols back into byte-padded bytes (MSB-first).
fn symbols_to_bytes(symbols: &[u8]) -> Vec<u8> {
    let mut w = BitWriter::new();
    for &s in symbols {
        w.write_bits((s & 0x1F) as u64, 5);
    }
    w.into_bytes()
}

fn symbol_to_char(s: u8) -> char {
    CODEX32_ALPHABET[(s & 0x1F) as usize] as char
}

fn char_to_symbol(c: char) -> Option<u8> {
    let lc = c.to_ascii_lowercase() as u8;
    CODEX32_ALPHABET.iter().position(|&b| b == lc).map(|i| i as u8)
}

/// Wrap a v0.11 payload bit stream (byte-padded with exact `bit_count`)
/// into a complete codex32 md1 string with HRP and BCH checksum, symbol-aligned.
pub fn wrap_payload(payload_bytes: &[u8], bit_count: usize) -> Result<String, V11Error> {
    let data_symbols = bits_to_symbols(payload_bytes, bit_count)?;
    // v0.x exposes `bch_create_checksum_regular(hrp: &str, data: &[u8]) -> [u8; 13]`.
    let checksum: [u8; 13] = crate::encoding::bch_create_checksum_regular(HRP, &data_symbols);

    let mut s = String::with_capacity(HRP.len() + 1 + data_symbols.len() + REGULAR_CHECKSUM_SYMBOLS);
    s.push_str(HRP);
    s.push('1'); // BIP 173-style HRP separator
    for sym in &data_symbols {
        s.push(symbol_to_char(*sym));
    }
    for sym in checksum.iter() {
        s.push(symbol_to_char(*sym));
    }
    Ok(s)
}

/// Unwrap a v0.11 md1 string into (byte-padded payload bytes, symbol-aligned bit count).
///
/// The returned `symbol_aligned_bit_count = 5 × data_symbol_count`. This is
/// the EXACT bit length carried by the codex32 BCH layer (rounded up to the
/// next 5-bit boundary from the actual payload). The caller uses this as
/// `decode_payload`'s `bit_len` so the v11 decoder's TLV-rollback only sees
/// ≤4 bits of trailing zero-padding (well under the 7-bit threshold).
pub fn unwrap_string(s: &str) -> Result<(Vec<u8>, usize), V11Error> {
    // 1. Strip HRP + separator.
    let prefix = format!("{}1", HRP);
    if !s.to_ascii_lowercase().starts_with(&prefix) {
        return Err(V11Error::Codex32DecodeError(format!(
            "string does not start with HRP {prefix}"
        )));
    }
    let symbols_str = &s[prefix.len()..];

    // 2. Char-to-symbol decode (tolerate visual separators per D11).
    let mut symbols = Vec::with_capacity(symbols_str.len());
    for c in symbols_str.chars() {
        if c.is_whitespace() || c == '-' {
            continue;
        }
        let sym = char_to_symbol(c).ok_or_else(|| {
            V11Error::Codex32DecodeError(format!("character {c:?} not in codex32 alphabet"))
        })?;
        symbols.push(sym);
    }

    // 3. BCH-verify.
    if !crate::encoding::bch_verify_regular(HRP, &symbols) {
        return Err(V11Error::Codex32DecodeError(
            "BCH checksum verification failed".into(),
        ));
    }

    // 4. Strip the 13-symbol checksum.
    if symbols.len() < REGULAR_CHECKSUM_SYMBOLS {
        return Err(V11Error::Codex32DecodeError(
            "string too short for BCH checksum".into(),
        ));
    }
    let data_symbols = &symbols[..symbols.len() - REGULAR_CHECKSUM_SYMBOLS];
    let bit_count = 5 * data_symbols.len();

    // 5. Convert symbols → byte-padded bytes.
    Ok((symbols_to_bytes(data_symbols), bit_count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_unwrap_round_trip_57_bits() {
        // Synthetic 57-bit payload (mimics BIP 84 single-sig length).
        let mut w = BitWriter::new();
        w.write_bits(0xDEAD_BEEF_CAFE_BABE_u64 >> 7, 57);
        let bytes = w.into_bytes();
        let s = wrap_payload(&bytes, 57).unwrap();
        // HRP "md1" (3 chars) + 12 data symbols + 13 checksum = 28 chars.
        assert_eq!(s.len(), 28);
        assert!(s.starts_with("md1"));
        let (out_bytes, out_bits) = unwrap_string(&s).unwrap();
        // Symbol-aligned bit count = 5 * 12 = 60 (≥ 57 by ≤4 padding bits).
        assert_eq!(out_bits, 60);
        // First 7 bytes match exactly; last byte's high bits match (low bits = padding).
        assert_eq!(&out_bytes[..7], &bytes[..7]);
        assert_eq!(out_bytes[7] & 0x80, bytes[7] & 0x80);
    }

    /// Critical: covers an N-byte chunk whose round-trip would mismatch under
    /// byte-aligned `bytes.len() * 8` accounting. N=3 is the smallest such case
    /// (encoder writes 8 bytes; symbol-aligned packing produces 13 symbols which
    /// unpack to 9 bytes — but symbol_aligned_bit_count = 65 stays the right
    /// reference).
    #[test]
    fn wrap_unwrap_n3_chunk_byte_count_recovers_correctly() {
        // Chunk-format wire: 37-bit header + 8*3 = 24-bit payload = 61 bits.
        let bit_count = 37 + 24;
        let mut w = BitWriter::new();
        w.write_bits(0x1FFF_FFFF_FFFF_u64, 37); // arbitrary header bits
        w.write_bits(0x00AA_BBCC_u64, 24);
        let bytes = w.into_bytes();
        assert_eq!(bytes.len(), 8); // ceil(61/8)
        let s = wrap_payload(&bytes, bit_count).unwrap();
        let (_out_bytes, out_bits) = unwrap_string(&s).unwrap();
        // Symbol-aligned bit count = 5 * ceil(61/5) = 5 * 13 = 65.
        assert_eq!(out_bits, 65);
        // (out_bits - 37) / 8 = (65 - 37) / 8 = 3 → 3 chunk-payload bytes recovered.
        let recovered_payload_byte_count = (out_bits - 37) / 8;
        assert_eq!(recovered_payload_byte_count, 3);
    }

    #[test]
    fn unwrap_rejects_non_md_string() {
        assert!(unwrap_string("xx1qpz9r4cy7").is_err());
    }

    #[test]
    fn unwrap_tolerates_visual_separators() {
        let mut w = BitWriter::new();
        w.write_bits(0b1010, 4);
        let bytes = w.into_bytes();
        let s = wrap_payload(&bytes, 4).unwrap();
        let mut grouped = String::new();
        for (i, c) in s.chars().enumerate() {
            grouped.push(c);
            if i == 3 {
                grouped.push('-');
            }
            if i == 8 {
                grouped.push(' ');
            }
        }
        let _ = unwrap_string(&grouped).unwrap();
    }
}
