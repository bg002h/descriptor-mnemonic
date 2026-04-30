//! Chunk header per spec §9.3.
//!
//! Encodes the 37-bit chunked wire-format header:
//! 3-bit version + 1-bit chunked-flag (= 1) + 1-bit reserved (= 0) +
//! 20-bit chunk-set-id + 6-bit count-minus-1 + 6-bit index.

use crate::bitstream::{BitReader, BitWriter};
use crate::error::Error;

/// Wire header for a single chunk in a chunked v0.11 payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkHeader {
    /// Wire-format version (3 bits).
    pub version: u8,
    /// 20-bit chunk-set identifier shared by all chunks in a set.
    pub chunk_set_id: u32,
    /// Total number of chunks in the set; valid range `1..=64`.
    pub count: u8,
    /// Zero-based index of this chunk within the set; must be `< count`.
    pub index: u8,
}

impl ChunkHeader {
    /// Encode the chunk header into `w` as 37 bits.
    ///
    /// Returns an error if `count`, `index`, or `chunk_set_id` are out of range.
    pub fn write(&self, w: &mut BitWriter) -> Result<(), Error> {
        if !(1..=64).contains(&(self.count as u32)) {
            return Err(Error::ChunkCountOutOfRange { count: self.count });
        }
        if self.index >= self.count {
            return Err(Error::ChunkIndexOutOfRange { index: self.index, count: self.count });
        }
        if self.chunk_set_id >= (1 << 20) {
            return Err(Error::ChunkSetIdOutOfRange { id: self.chunk_set_id });
        }
        w.write_bits(u64::from(self.version & 0b111), 3);
        w.write_bits(1, 1); // chunked = 1
        w.write_bits(0, 1); // reserved
        w.write_bits(u64::from(self.chunk_set_id), 20);
        w.write_bits((self.count - 1) as u64, 6); // count-1 offset
        w.write_bits(u64::from(self.index), 6);
        Ok(())
    }

    /// Decode a chunk header (37 bits) from `r`.
    ///
    /// Returns [`Error::ChunkHeaderChunkedFlagMissing`] if the chunked-flag
    /// bit is not set.
    pub fn read(r: &mut BitReader) -> Result<Self, Error> {
        let version = r.read_bits(3)? as u8;
        let chunked = r.read_bits(1)? != 0;
        if !chunked {
            return Err(Error::ChunkHeaderChunkedFlagMissing);
        }
        let _reserved = r.read_bits(1)?;
        let chunk_set_id = r.read_bits(20)? as u32;
        let count = (r.read_bits(6)? + 1) as u8;
        let index = r.read_bits(6)? as u8;
        Ok(Self { version, chunk_set_id, count, index })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_header_round_trip() {
        let h = ChunkHeader { version: 0, chunk_set_id: 0xABCDE, count: 3, index: 1 };
        let mut w = BitWriter::new();
        h.write(&mut w).unwrap();
        // 3 + 1 + 1 + 20 + 6 + 6 = 37 bits
        assert_eq!(w.bit_len(), 37);
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(ChunkHeader::read(&mut r).unwrap(), h);
    }

    #[test]
    fn chunk_header_count_64_round_trip() {
        let h = ChunkHeader { version: 0, chunk_set_id: 0, count: 64, index: 63 };
        let mut w = BitWriter::new();
        h.write(&mut w).unwrap();
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(ChunkHeader::read(&mut r).unwrap(), h);
    }

    #[test]
    fn chunk_header_count_zero_rejected() {
        let h = ChunkHeader { version: 0, chunk_set_id: 0, count: 0, index: 0 };
        let mut w = BitWriter::new();
        assert!(matches!(
            h.write(&mut w),
            Err(Error::ChunkCountOutOfRange { count: 0 })
        ));
    }
}

use crate::identity::Md1EncodingId;

/// Derive the 20-bit chunk-set-id from a [`Md1EncodingId`] by taking the
/// top 20 bits of the underlying 16-byte hash, MSB-first.
///
/// The chunk-set-id groups chunks belonging to the same encoded payload.
/// Returned value is in the range `0..=0xFFFFF`.
pub fn derive_chunk_set_id(id: &Md1EncodingId) -> u32 {
    // First 20 bits of Md1EncodingId[0..16], MSB-first.
    let bytes = id.as_bytes();
    ((bytes[0] as u32) << 12) | ((bytes[1] as u32) << 4) | ((bytes[2] as u32) >> 4)
}

#[cfg(test)]
mod chunk_set_id_tests {
    use super::*;

    #[test]
    fn derive_chunk_set_id_deterministic() {
        let mut bytes = [0u8; 16];
        bytes[0] = 0xab;
        bytes[1] = 0xcd;
        bytes[2] = 0xe1;
        bytes[3] = 0x23;
        let id = Md1EncodingId::new(bytes);
        let csid_a = derive_chunk_set_id(&id);
        let csid_b = derive_chunk_set_id(&id);
        assert_eq!(csid_a, csid_b);
    }

    #[test]
    fn derive_chunk_set_id_msb_first_extraction() {
        // bytes[0]=0xAB, [1]=0xCD, [2]=0xEF: top 20 bits = 0xABCDE
        let mut bytes = [0u8; 16];
        bytes[0] = 0xAB;
        bytes[1] = 0xCD;
        bytes[2] = 0xEF;
        let id = Md1EncodingId::new(bytes);
        assert_eq!(derive_chunk_set_id(&id), 0xABCDE);
    }
}

use crate::encode::Descriptor;

/// Threshold (in payload bits) above which chunking is required. Derived from
/// codex32 long-form's 75-symbol data limit (per BIP 93): 75 data symbols × 5
/// bits = 375 bits total, of which the trailing 13 symbols (see
/// `codex32::REGULAR_CHECKSUM_SYMBOLS`) are checksum.
/// Encoders attempt single-string emit first; if the codex32 wrapping reports
/// "too long for long form", split into N chunks.
pub const SINGLE_STRING_PAYLOAD_BIT_LIMIT: usize = 75 * 5;

/// Split a [`Descriptor`] into N codex32 md1 strings, each carrying a chunk
/// header and a slice of the canonical payload.
///
/// Algorithm:
/// 1. Encode the full payload (`encode_payload`).
/// 2. Compute [`crate::identity::Md1EncodingId`]; derive `ChunkSetId`.
/// 3. Choose chunk count N such that each chunk fits in codex32 long form
///    after adding the 37-bit chunk header.
/// 4. Split the payload into N approximately-equal byte-boundary slices.
/// 5. For each chunk i: prepend chunk header (37 bits), wrap via codex32 with
///    the chunked-flag bit set, emit md1 string.
///
/// Note: `bytes_per_chunk` could be 0 if `payload_bytes` were empty, but the
/// encoder validates `n ≥ 1` so the payload is always non-empty.
pub fn split(d: &Descriptor) -> Result<Vec<String>, Error> {
    use crate::bitstream::BitWriter;
    use crate::encode::encode_payload;
    use crate::identity::compute_md1_encoding_id;

    let (payload_bytes, _payload_bits) = encode_payload(d)?;

    // Compute ChunkSetId from full-encoding hash.
    let md1_id = compute_md1_encoding_id(d)?;
    let chunk_set_id = derive_chunk_set_id(&md1_id);

    // Choose chunk count from payload byte count (≤7 bits of trailing
    // codex32-padding are tolerated by the reassembled-stream TLV-rollback).
    let payload_bit_count_for_sizing = payload_bytes.len() * 8;
    let chunks_needed = payload_bit_count_for_sizing.div_ceil(SINGLE_STRING_PAYLOAD_BIT_LIMIT);
    if chunks_needed > 64 {
        return Err(Error::ChunkCountExceedsMax { needed: chunks_needed });
    }
    let count: u8 = if chunks_needed == 0 { 1 } else { chunks_needed as u8 };

    // Split payload into `count` byte-boundary slices.
    let bytes_per_chunk = payload_bytes.len().div_ceil(count as usize);

    let mut chunks = Vec::with_capacity(count as usize);
    for index in 0..count {
        let start_byte = (index as usize) * bytes_per_chunk;
        let end_byte = ((index as usize + 1) * bytes_per_chunk).min(payload_bytes.len());
        let chunk_payload_bytes = &payload_bytes[start_byte..end_byte];

        // Build per-chunk wire: 37-bit chunk header + chunk-payload bytes
        // (full 8 bits per byte, no further fractional content). Chunk's
        // exact bit count = 37 + 8 × |chunk_payload_bytes|.
        let header = ChunkHeader {
            version: 0,
            chunk_set_id,
            count,
            index,
        };
        let mut w = BitWriter::new();
        header.write(&mut w)?;
        for byte in chunk_payload_bytes {
            w.write_bits(u64::from(*byte), 8);
        }
        let chunk_bit_count = 37 + 8 * chunk_payload_bytes.len();
        let bytes = w.into_bytes();
        let s = crate::codex32::wrap_payload(&bytes, chunk_bit_count)?;
        chunks.push(s);
    }
    Ok(chunks)
}

use crate::decode::decode_payload;

/// Reassemble a [`Descriptor`] from N md1 codex32 strings.
///
/// Algorithm:
/// 1. Unwrap each string via the codex32 layer (verifies BCH per chunk).
/// 2. Parse the 37-bit chunk header from each.
/// 3. Validate consistency: same version, chunk_set_id, count.
/// 4. Sort by index; verify `0..count-1` with no gaps.
/// 5. Concatenate per-chunk payload bytes.
/// 6. Decode the reassembled payload via [`decode_payload`].
/// 7. Verify the reassembled payload's derived chunk-set-id matches the
///    chunk-set-id present in every chunk header (cross-chunk integrity).
pub fn reassemble(strings: &[&str]) -> Result<Descriptor, Error> {
    use crate::bitstream::BitReader;
    use crate::codex32::unwrap_string;
    use crate::identity::compute_md1_encoding_id;

    if strings.is_empty() {
        return Err(Error::ChunkSetEmpty);
    }

    // Unwrap each, parse 37-bit chunk header, then read whole payload bytes.
    // Use the symbol-aligned bit count returned by `unwrap_string` (NOT
    // `bytes.len() * 8`, which would over-estimate by up to 7 bits and break
    // round-trip for chunks where symbol-padding plus byte-padding crosses a
    // byte boundary — e.g. N=3, N=8, etc.).
    let mut parsed: Vec<(ChunkHeader, Vec<u8>)> = Vec::with_capacity(strings.len());
    for s in strings {
        let (bytes, symbol_aligned_bit_count) = unwrap_string(s)?;
        let mut r = BitReader::with_bit_limit(&bytes, symbol_aligned_bit_count);
        let header = ChunkHeader::read(&mut r)?;
        // Per encoder contract: chunk wire is exactly 37 + 8N bits. The
        // symbol-aligned bit count is `ceil((37+8N)/5) * 5`, which is in
        // [37+8N, 37+8N+4]. So `(symbol_aligned_bit_count - 37) / 8`
        // (floor) recovers exactly N.
        let payload_byte_count = (symbol_aligned_bit_count - 37) / 8;
        let mut chunk_payload_bytes = Vec::with_capacity(payload_byte_count);
        for _ in 0..payload_byte_count {
            let v = r.read_bits(8)? as u8;
            chunk_payload_bytes.push(v);
        }
        // Trailing ≤4 symbol-padding bits remain in r; discard.
        parsed.push((header, chunk_payload_bytes));
    }

    // Validate consistency.
    let (h0, _) = &parsed[0];
    let expected_count = h0.count;
    let expected_csid = h0.chunk_set_id;
    let expected_version = h0.version;
    for (h, _) in &parsed {
        if h.count != expected_count
            || h.chunk_set_id != expected_csid
            || h.version != expected_version
        {
            return Err(Error::ChunkSetInconsistent);
        }
    }
    if parsed.len() != expected_count as usize {
        return Err(Error::ChunkSetIncomplete {
            got: parsed.len(),
            expected: expected_count as usize,
        });
    }

    // Sort by index; verify 0..count-1 with no gaps.
    parsed.sort_by_key(|(h, _)| h.index);
    for (i, (h, _)) in parsed.iter().enumerate() {
        if h.index as usize != i {
            return Err(Error::ChunkIndexGap {
                expected: i as u8,
                got: h.index,
            });
        }
    }

    // Concatenate chunk payload bytes.
    let mut full_bytes = Vec::new();
    for (_, chunk_bytes) in &parsed {
        full_bytes.extend_from_slice(chunk_bytes);
    }

    // Decode payload. bit_len = bytes.len() * 8; TLV-rollback handles trailing padding.
    let descriptor = decode_payload(&full_bytes, full_bytes.len() * 8)?;

    // Cross-chunk integrity check.
    let md1_id = compute_md1_encoding_id(&descriptor)?;
    let derived_csid = derive_chunk_set_id(&md1_id);
    if derived_csid != expected_csid {
        return Err(Error::ChunkSetIdMismatch {
            expected: expected_csid,
            derived: derived_csid,
        });
    }

    Ok(descriptor)
}
