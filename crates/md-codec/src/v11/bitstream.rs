//! Bit-aligned reader and writer.
//!
//! Per spec §4.6: bits are packed MSB-first into bytes. The first bit of the
//! payload occupies the most-significant bit of the first byte. The final byte
//! is zero-padded if needed.

/// MSB-first bit packer.
pub struct BitWriter {
    /// Backing byte buffer; the last byte is the in-progress byte.
    bytes: Vec<u8>,
    /// Bit offset within the last byte, in `0..8`. Zero means no in-progress byte.
    bit_position: usize,
}

impl BitWriter {
    /// Create an empty `BitWriter`.
    pub fn new() -> Self {
        Self { bytes: Vec::new(), bit_position: 0 }
    }

    /// Write `count` bits from `value` (LSB-aligned in `value`) into the
    /// stream MSB-first. Bits beyond `count` in `value` are ignored.
    pub fn write_bits(&mut self, value: u64, count: usize) {
        if count == 0 {
            return;
        }
        debug_assert!(count <= 64, "write_bits count must be ≤ 64");

        // Mask `value` to the requested bit count.
        let masked = if count == 64 { value } else { value & ((1u64 << count) - 1) };

        // Iterate from MSB to LSB of the requested value.
        let mut remaining = count;
        while remaining > 0 {
            // Ensure there's a current byte to write into.
            if self.bit_position == 0 {
                self.bytes.push(0);
            }
            let last = self.bytes.last_mut().unwrap();

            // How many bits free in the current byte (from bit_position MSB-side)?
            let free_in_byte = 8 - self.bit_position;
            let chunk = remaining.min(free_in_byte);

            // Pull `chunk` bits from the top of the masked value.
            let shift = (remaining - chunk) as u32;
            let bits = ((masked >> shift) & ((1u64 << chunk) - 1)) as u8;

            // Place bits into the byte at the correct offset (MSB-first).
            let byte_shift = (free_in_byte - chunk) as u32;
            *last |= bits << byte_shift;

            self.bit_position += chunk;
            if self.bit_position == 8 {
                self.bit_position = 0;
            }
            remaining -= chunk;
        }
    }

    /// Total number of bits written.
    pub fn bit_len(&self) -> usize {
        if self.bit_position == 0 {
            self.bytes.len() * 8
        } else {
            (self.bytes.len() - 1) * 8 + self.bit_position
        }
    }

    /// Consume self and produce the byte stream (final byte zero-padded).
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_5_bits_msb_first() {
        let mut w = BitWriter::new();
        w.write_bits(0b10110, 5);
        // 0b10110_000 = 0xb0 in MSB-first packing of just 5 bits with zero
        // padding on the low 3 bits.
        assert_eq!(w.into_bytes(), vec![0b1011_0000]);
    }

    #[test]
    fn write_two_5_bit_values_packs_into_one_and_a_bit() {
        let mut w = BitWriter::new();
        w.write_bits(0b11111, 5);
        w.write_bits(0b00001, 5);
        // first 5: 11111___, then 00001 occupies bits 5..0 of the next
        // 5-bit slot. Combined: 11111_000_01 = 11111000_01000000 = 0xf8 0x40
        assert_eq!(w.into_bytes(), vec![0b1111_1000, 0b0100_0000]);
    }

    #[test]
    fn write_8_bits_is_one_byte() {
        let mut w = BitWriter::new();
        w.write_bits(0xab, 8);
        assert_eq!(w.into_bytes(), vec![0xab]);
    }

    #[test]
    fn write_zero_bits_is_noop() {
        let mut w = BitWriter::new();
        w.write_bits(0xff, 0);
        assert_eq!(w.bit_len(), 0);
        assert_eq!(w.into_bytes(), Vec::<u8>::new());
    }
}
