//! TLV section per spec §3.7.

use crate::bitstream::{BitReader, BitWriter};
use crate::error::Error;
use crate::use_site_path::UseSitePath;
use crate::varint::{read_varint, write_varint};

/// TLV tag for use-site-path overrides (per-`@N` divergent path declarations).
pub const TLV_USE_SITE_PATH_OVERRIDES: u8 = 0x00;
/// TLV tag for per-`@N` xpub fingerprints (4 bytes each).
pub const TLV_FINGERPRINTS: u8 = 0x01;
/// Reserved TLV tag for v0.12 xpub payloads.
pub const TLV_XPUBS_RESERVED_V0_12: u8 = 0x02;

/// Decoded TLV section: optional UseSitePathOverrides, optional Fingerprints,
/// and any unknown TLVs preserved verbatim for forward-compatible round-trip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlvSection {
    /// Per-`@N` use-site path overrides, if present.
    pub use_site_path_overrides: Option<Vec<(u8, UseSitePath)>>,
    /// Per-`@N` xpub fingerprints (4 bytes each), if present.
    pub fingerprints: Option<Vec<(u8, [u8; 4])>>,
    /// Raw payload of unknown TLVs, keyed by tag, for forward-compat round-trip.
    /// Decoders preserve unknown TLVs verbatim through re-encoding.
    pub unknown: Vec<(u8, Vec<u8>, usize)>,
}

impl TlvSection {
    /// Create an empty TLV section (no entries).
    pub fn new_empty() -> Self {
        Self {
            use_site_path_overrides: None,
            fingerprints: None,
            unknown: Vec::new(),
        }
    }

    /// Returns true if no TLV entries are present.
    pub fn is_empty(&self) -> bool {
        self.use_site_path_overrides.is_none()
            && self.fingerprints.is_none()
            && self.unknown.is_empty()
    }

    /// Encode this TLV section onto `w`. Entries are emitted in ascending tag order.
    /// `key_index_width` is the bit-width of the per-`@N` placeholder index field.
    pub fn write(&self, w: &mut BitWriter, key_index_width: u8) -> Result<(), Error> {
        // Collect entries, sort by tag.
        let mut entries: Vec<(u8, Vec<u8>, usize)> = Vec::new();
        if let Some(overrides) = &self.use_site_path_overrides {
            let mut sub = BitWriter::new();
            for (idx, path) in overrides {
                sub.write_bits(u64::from(*idx), key_index_width as usize);
                path.write(&mut sub)?;
            }
            let bit_len = sub.bit_len();
            entries.push((TLV_USE_SITE_PATH_OVERRIDES, sub.into_bytes(), bit_len));
        }
        if let Some(fps) = &self.fingerprints {
            let mut sub = BitWriter::new();
            for (idx, fp) in fps {
                sub.write_bits(u64::from(*idx), key_index_width as usize);
                for b in fp {
                    sub.write_bits(u64::from(*b), 8);
                }
            }
            let bit_len = sub.bit_len();
            entries.push((TLV_FINGERPRINTS, sub.into_bytes(), bit_len));
        }
        for (tag, payload, bit_len) in &self.unknown {
            entries.push((*tag, payload.clone(), *bit_len));
        }
        entries.sort_by_key(|(t, _, _)| *t);

        for (tag, payload, bit_len) in entries {
            w.write_bits(u64::from(tag), 5);
            write_varint(w, bit_len as u32);
            // Re-emit payload bits MSB-first.
            let mut sub_reader = BitReader::new(&payload);
            let mut remaining = bit_len;
            while remaining > 0 {
                let chunk = remaining.min(8);
                let bits = sub_reader.read_bits(chunk)?;
                w.write_bits(bits, chunk);
                remaining -= chunk;
            }
        }
        Ok(())
    }

    /// Decode a TLV section from `r`, consuming all remaining bits.
    /// `key_index_width` is the bit-width of placeholder indices; `n` is the key count.
    pub fn read(r: &mut BitReader, key_index_width: u8, n: u8) -> Result<Self, Error> {
        let mut section = Self::new_empty();
        let mut last_tag: Option<u8> = None;
        loop {
            // Save position so we can roll back if this would-be TLV is
            // actually trailing codex32-padding (≤7 bits of zeros).
            let entry_start = r.save_position();
            if r.remaining_bits() < 5 {
                break;  // not enough bits for even a tag — clean end-of-stream
            }
            // Try to parse a complete TLV entry. Any failure (truncated read,
            // ordering violation, empty-entry-by-spec, length exceeds remaining)
            // is treated as "trailing padding" if we can rollback cleanly. If
            // rollback would consume <8 bits (consistent with codex32 padding)
            // we accept it; otherwise the error propagates as a real malformed
            // input.
            let parse_result: Result<(), Error> = (|| {
                let tag = r.read_bits(5)? as u8;
                // Ordering check is INSIDE the closure so violations at end-of-
                // stream (where padding bits form a phantom tag=0 after a real
                // tag≥1 entry) become rollback-eligible.
                if let Some(prev) = last_tag {
                    if tag <= prev {
                        return Err(Error::TlvOrderingViolation { prev, current: tag });
                    }
                }
                let bit_len = read_varint(r)? as usize;
                if bit_len > r.remaining_bits() {
                    return Err(Error::TlvLengthExceedsRemaining {
                        length: bit_len,
                        remaining: r.remaining_bits(),
                    });
                }
                // Reject zero-length TLVs uniformly. Encoder MUST omit empty
                // TLVs per spec §7.5; a zero-length entry at the end of stream
                // is treated as padding via the rollback path.
                if bit_len == 0 {
                    return Err(Error::EmptyTlvEntry { tag });
                }
                match tag {
                    TLV_USE_SITE_PATH_OVERRIDES => {
                        let entry = read_use_site_overrides(r, bit_len, key_index_width, n)?;
                        section.use_site_path_overrides = Some(entry);
                    }
                    TLV_FINGERPRINTS => {
                        let entry = read_fingerprints(r, bit_len, key_index_width, n)?;
                        section.fingerprints = Some(entry);
                    }
                    _ => {
                        // Unknown — buffer and skip per D6 forward-compat.
                        let mut sub = BitWriter::new();
                        let mut remaining = bit_len;
                        while remaining > 0 {
                            let chunk = remaining.min(8);
                            let bits = r.read_bits(chunk)?;
                            sub.write_bits(bits, chunk);
                            remaining -= chunk;
                        }
                        let payload = sub.into_bytes();
                        section.unknown.push((tag, payload, bit_len));
                    }
                }
                last_tag = Some(tag);
                Ok(())
            })();

            match parse_result {
                Ok(()) => continue,
                Err(e) => {
                    // Decide: rollback-as-padding or propagate error.
                    // Rollback is acceptable iff the bits we'd be discarding
                    // are ≤7 (consistent with codex32 padding boundary).
                    r.restore_position(entry_start);
                    let remaining_at_entry_start = r.remaining_bits();
                    // Padding tolerance: ≤7 bits of trailing zeros after the
                    // last real TLV (or after the tree if no TLVs were emitted).
                    if remaining_at_entry_start <= 7 {
                        break;
                    }
                    // More than 7 bits remained but the parse still failed —
                    // this is genuinely malformed input. Propagate.
                    return Err(e);
                }
            }
        }
        Ok(section)
    }
}

fn read_use_site_overrides(
    r: &mut BitReader,
    bit_len: usize,
    key_index_width: u8,
    n: u8,
) -> Result<Vec<(u8, UseSitePath)>, Error> {
    let start = r.bit_position();
    let mut entries = Vec::new();
    let mut last_idx: Option<u8> = None;
    while r.bit_position() - start < bit_len {
        let idx = r.read_bits(key_index_width as usize)? as u8;
        if idx >= n {
            return Err(Error::PlaceholderIndexOutOfRange { idx, n });
        }
        if let Some(prev) = last_idx {
            if idx <= prev {
                return Err(Error::OverrideOrderViolation { prev, current: idx });
            }
        }
        last_idx = Some(idx);
        let path = UseSitePath::read(r)?;
        entries.push((idx, path));
    }
    if entries.is_empty() {
        return Err(Error::EmptyTlvEntry { tag: TLV_USE_SITE_PATH_OVERRIDES });
    }
    Ok(entries)
}

fn read_fingerprints(
    r: &mut BitReader,
    bit_len: usize,
    key_index_width: u8,
    n: u8,
) -> Result<Vec<(u8, [u8; 4])>, Error> {
    let start = r.bit_position();
    let mut entries = Vec::new();
    let mut last_idx: Option<u8> = None;
    while r.bit_position() - start < bit_len {
        let idx = r.read_bits(key_index_width as usize)? as u8;
        if idx >= n {
            return Err(Error::PlaceholderIndexOutOfRange { idx, n });
        }
        if let Some(prev) = last_idx {
            if idx <= prev {
                return Err(Error::OverrideOrderViolation { prev, current: idx });
            }
        }
        last_idx = Some(idx);
        let mut fp = [0u8; 4];
        for byte in &mut fp {
            *byte = r.read_bits(8)? as u8;
        }
        entries.push((idx, fp));
    }
    if entries.is_empty() {
        return Err(Error::EmptyTlvEntry { tag: TLV_FINGERPRINTS });
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_tlv_section_round_trip() {
        let s = TlvSection::new_empty();
        assert!(s.is_empty());
        let mut w = BitWriter::new();
        s.write(&mut w, 2).unwrap();
        assert_eq!(w.bit_len(), 0);
    }

    #[test]
    fn use_site_path_override_round_trip() {
        let mut s = TlvSection::new_empty();
        s.use_site_path_overrides = Some(vec![(
            1u8,
            UseSitePath {
                multipath: None,
                wildcard_hardened: true,
            },
        )]);
        let mut w = BitWriter::new();
        s.write(&mut w, 2).unwrap();
        let bit_len = w.bit_len();
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        let s2 = TlvSection::read(&mut r, 2, 3).unwrap();
        assert_eq!(s2, s);
        assert_eq!(r.bit_position(), bit_len);
    }

    #[test]
    fn fingerprint_round_trip() {
        let mut s = TlvSection::new_empty();
        s.fingerprints = Some(vec![
            (0u8, [0xaa, 0xbb, 0xcc, 0xdd]),
            (2u8, [0x11, 0x22, 0x33, 0x44]),
        ]);
        let mut w = BitWriter::new();
        s.write(&mut w, 2).unwrap();
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        let s2 = TlvSection::read(&mut r, 2, 3).unwrap();
        assert_eq!(s2, s);
    }

    #[test]
    fn ascending_tag_order_enforced_in_encoder() {
        let mut s = TlvSection::new_empty();
        s.fingerprints = Some(vec![(0, [0u8; 4])]);
        s.use_site_path_overrides = Some(vec![(0, UseSitePath { multipath: None, wildcard_hardened: false })]);
        let mut w = BitWriter::new();
        s.write(&mut w, 2).unwrap();
        let bytes = w.into_bytes();
        let first_tag = (bytes[0] >> 3) & 0x1F;
        assert_eq!(first_tag, TLV_USE_SITE_PATH_OVERRIDES);
    }
}
