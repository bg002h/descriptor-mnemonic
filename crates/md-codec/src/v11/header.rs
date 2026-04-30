//! Payload header (5 bits) per spec §3.3.
//!
//!   bit 4: divergent-paths flag (0=shared origin path, 1=divergent)
//!   bit 3: reserved (MUST be 0 in v0.11; chunk header reuses this slot for chunked-flag)
//!   bits 2..0: version (3 bits; v0.11 = 0)

use crate::v11::bitstream::{BitReader, BitWriter};
use crate::v11::error::V11Error;

/// 5-bit payload header per spec §3.3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    /// Wire-format generation. v0.11 = 0; future versions increment.
    pub version: u8,
    /// Bit 4: false = shared origin path, true = divergent per-`@N` paths.
    pub divergent_paths: bool,
}

impl Header {
    /// Wire-format version constant for v0.11.
    pub const V0_11_VERSION: u8 = 0;

    /// Encode the 5-bit header into the bit stream.
    pub fn write(&self, w: &mut BitWriter) {
        // bit 3 (reserved) is always 0 in v0.11.
        let bits = (u64::from(self.divergent_paths) << 4)
            | u64::from(self.version & 0b111);
        w.write_bits(bits, 5);
    }

    /// Decode the 5-bit header from the bit stream.
    /// Rejects reserved-bit-set and unsupported-version inputs.
    pub fn read(r: &mut BitReader) -> Result<Self, V11Error> {
        let bits = r.read_bits(5)?;
        let divergent_paths = (bits >> 4) & 1 != 0;
        let reserved = (bits >> 3) & 1;
        let version = (bits & 0b111) as u8;
        if reserved != 0 {
            return Err(V11Error::ReservedHeaderBitSet);
        }
        if version != Self::V0_11_VERSION {
            return Err(V11Error::UnsupportedVersion { got: version });
        }
        Ok(Self { version, divergent_paths })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_round_trip_shared() {
        let h = Header { version: 0, divergent_paths: false };
        let mut w = BitWriter::new();
        h.write(&mut w);
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(Header::read(&mut r).unwrap(), h);
    }

    #[test]
    fn header_round_trip_divergent() {
        let h = Header { version: 0, divergent_paths: true };
        let mut w = BitWriter::new();
        h.write(&mut w);
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(Header::read(&mut r).unwrap(), h);
    }

    #[test]
    fn header_rejects_reserved_bit() {
        // Bit 3 set in raw 5-bit value 0b01000 = 0x08
        // packed at MSB of byte: 0b01000_000 = 0x40
        let bytes = vec![0x40];
        let mut r = BitReader::new(&bytes);
        assert!(matches!(
            Header::read(&mut r),
            Err(V11Error::ReservedHeaderBitSet)
        ));
    }

    #[test]
    fn header_rejects_wrong_version() {
        // Version 1 (in bits 0-2 = 0b001), other bits 0: raw 5-bit value 0b00001 = 0x01
        // packed MSB: 0b00001_000 = 0x08
        let bytes = vec![0x08];
        let mut r = BitReader::new(&bytes);
        assert!(matches!(
            Header::read(&mut r),
            Err(V11Error::UnsupportedVersion { got: 1 })
        ));
    }

    #[test]
    fn header_common_case_byte_value() {
        // Common case: version=0, reserved=0, divergent_paths=0 ⇒ 0b00000 = 0x00
        let h = Header { version: 0, divergent_paths: false };
        let mut w = BitWriter::new();
        h.write(&mut w);
        assert_eq!(w.into_bytes(), vec![0x00]);
    }
}
