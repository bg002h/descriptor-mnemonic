//! Chunk header per spec §9.3.
//!
//! Encodes the 37-bit chunked wire-format header:
//! 3-bit version + 1-bit chunked-flag (= 1) + 1-bit reserved (= 0) +
//! 20-bit chunk-set-id + 6-bit count-minus-1 + 6-bit index.

use crate::v11::bitstream::{BitReader, BitWriter};
use crate::v11::error::V11Error;

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
    pub fn write(&self, w: &mut BitWriter) -> Result<(), V11Error> {
        if !(1..=64).contains(&(self.count as u32)) {
            return Err(V11Error::ChunkCountOutOfRange { count: self.count });
        }
        if self.index >= self.count {
            return Err(V11Error::ChunkIndexOutOfRange { index: self.index, count: self.count });
        }
        if self.chunk_set_id >= (1 << 20) {
            return Err(V11Error::ChunkSetIdOutOfRange { id: self.chunk_set_id });
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
    /// Returns [`V11Error::ChunkHeaderChunkedFlagMissing`] if the chunked-flag
    /// bit is not set.
    pub fn read(r: &mut BitReader) -> Result<Self, V11Error> {
        let version = r.read_bits(3)? as u8;
        let chunked = r.read_bits(1)? != 0;
        if !chunked {
            return Err(V11Error::ChunkHeaderChunkedFlagMissing);
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
            Err(V11Error::ChunkCountOutOfRange { count: 0 })
        ));
    }
}
