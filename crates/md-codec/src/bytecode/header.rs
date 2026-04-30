//! Bytecode header byte for canonical MD bytecode.
//!
//! The header is the first byte of the bytecode stream.
//! Bit layout (per BIP §"Bytecode header"):
//!
//! ```text
//! Bits 7–4: Bytecode version (v0 = 0x0).
//! Bit  3:   OriginPaths flag. 1 if per-`@N` origin paths block present, 0 otherwise.
//! Bit  2:   Fingerprints flag. 1 if fingerprints block present, 0 otherwise.
//! Bits 1–0: Reserved. MUST be 0 in v0.
//! ```
//!
//! Valid v0 values are exactly `0x00`, `0x04`, `0x08`, and `0x0C`
//! (any combination of the two flags).
//!
//! Bit 3 was reserved in v0.0–v0.9 and reclaimed in v0.10 as the
//! OriginPaths flag.

use crate::Error;
use crate::error::BytecodeErrorKind;

/// Mask of reserved bits that MUST be zero in a v0 header byte.
///
/// Bits 1 and 0 are reserved: `0b0000_0011` = `0x03`. Bit 3 was reserved
/// in v0.0–v0.9 and reclaimed in v0.10 as the OriginPaths flag.
const RESERVED_MASK: u8 = 0x03;

/// Mask for the fingerprints flag (bit 2).
const FINGERPRINTS_BIT: u8 = 0x04;

/// Mask for the OriginPaths flag (bit 3). NEW in v0.10.
const ORIGIN_PATHS_BIT: u8 = 0x08;

/// The first byte of canonical MD bytecode, encoding the format version,
/// the fingerprints-block presence flag, and (since v0.10) the
/// OriginPaths-block presence flag.
///
/// Marked `#[non_exhaustive]` so that v1+ fields can be added without a
/// breaking change (see `design/PHASE_2_DECISIONS.md` D-3).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BytecodeHeader {
    /// 4-bit version nibble (0 for v0).
    version: u8,
    /// True iff the fingerprints block is present in the bytecode stream.
    fingerprints: bool,
    /// True iff the OriginPaths block is present in the bytecode stream.
    /// NEW in v0.10.
    origin_paths: bool,
}

impl BytecodeHeader {
    /// Parse a header from a single byte.
    ///
    /// Returns:
    /// - `Err(Error::UnsupportedVersion(nibble))` if the version nibble is not 0.
    ///   The version check is performed **before** the reserved-bit check so that
    ///   a byte like `0x14` (version 1 + fingerprints) reports unsupported-version
    ///   rather than reserved-bits-set. Decoders MUST reject unknown versions.
    /// - `Err(Error::InvalidBytecode { offset: 0, kind: BytecodeErrorKind::ReservedBitsSet { .. } })`
    ///   if any reserved bit is non-zero.
    /// - `Ok(BytecodeHeader)` otherwise.
    pub fn from_byte(b: u8) -> Result<BytecodeHeader, crate::Error> {
        // Version check first — an unknown version takes priority over reserved
        // bit violations because future versions may legitimately redefine those
        // bits.
        let version = b >> 4;
        if version != 0 {
            return Err(Error::UnsupportedVersion(version));
        }

        let reserved = b & RESERVED_MASK;
        if reserved != 0 {
            return Err(Error::InvalidBytecode {
                offset: 0,
                kind: BytecodeErrorKind::ReservedBitsSet {
                    byte: b,
                    mask: RESERVED_MASK,
                },
            });
        }

        Ok(BytecodeHeader {
            version,
            fingerprints: (b & FINGERPRINTS_BIT) != 0,
            origin_paths: (b & ORIGIN_PATHS_BIT) != 0,
        })
    }

    /// Serialize the header back to a single byte.
    ///
    /// `from_byte(h.as_byte()) == Ok(h)` for any valid header `h`.
    pub const fn as_byte(self) -> u8 {
        let mut b = self.version << 4;
        if self.fingerprints {
            b |= FINGERPRINTS_BIT;
        }
        if self.origin_paths {
            b |= ORIGIN_PATHS_BIT;
        }
        b
    }

    /// Returns the 4-bit format version (0 for v0).
    pub fn version(self) -> u8 {
        self.version
    }

    /// Returns `true` iff the fingerprints block is present in the bytecode stream.
    pub fn fingerprints(self) -> bool {
        self.fingerprints
    }

    /// Returns `true` iff the OriginPaths block is present in the bytecode stream.
    /// NEW in v0.10.
    pub fn origin_paths(self) -> bool {
        self.origin_paths
    }

    /// Construct a valid v0 header.
    pub const fn new_v0(fingerprints: bool, origin_paths: bool) -> Self {
        BytecodeHeader {
            version: 0,
            fingerprints,
            origin_paths,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::BytecodeErrorKind;

    // --- Happy-path parsing ---

    #[test]
    fn from_byte_0x00_no_fingerprints() {
        let h = BytecodeHeader::from_byte(0x00).expect("0x00 is valid");
        assert_eq!(h.version(), 0);
        assert!(!h.fingerprints());
    }

    #[test]
    fn from_byte_0x04_fingerprints() {
        let h = BytecodeHeader::from_byte(0x04).expect("0x04 is valid");
        assert_eq!(h.version(), 0);
        assert!(h.fingerprints());
    }

    // --- Round-trip ---

    #[test]
    fn round_trip_0x00() {
        let h = BytecodeHeader::from_byte(0x00).unwrap();
        assert_eq!(h.as_byte(), 0x00);
    }

    #[test]
    fn round_trip_0x04() {
        let h = BytecodeHeader::from_byte(0x04).unwrap();
        assert_eq!(h.as_byte(), 0x04);
    }

    // --- new_v0 constructor ---

    #[test]
    fn v0_constructor_no_fingerprints() {
        assert_eq!(BytecodeHeader::new_v0(false, false).as_byte(), 0x00);
    }

    #[test]
    fn v0_constructor_with_fingerprints() {
        assert_eq!(BytecodeHeader::new_v0(true, false).as_byte(), 0x04);
    }

    // --- Reserved-bit rejection ---
    //
    // In v0.10, only bits 1 and 0 are reserved. Bit 2 is the fingerprints
    // flag and bit 3 is the OriginPaths flag (reclaimed in v0.10); both
    // are covered by the round-trip and constructor tests above.

    #[test]
    fn reserved_bit_0_set() {
        let err = BytecodeHeader::from_byte(0x01).unwrap_err();
        assert!(
            matches!(
                err,
                Error::InvalidBytecode {
                    offset: 0,
                    kind: BytecodeErrorKind::ReservedBitsSet {
                        byte: 0x01,
                        mask: RESERVED_MASK
                    }
                }
            ),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn reserved_bit_1_set() {
        let err = BytecodeHeader::from_byte(0x02).unwrap_err();
        assert!(
            matches!(
                err,
                Error::InvalidBytecode {
                    offset: 0,
                    kind: BytecodeErrorKind::ReservedBitsSet { .. }
                }
            ),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn all_reserved_bits_set_in_v0_10() {
        // 0x03 = bits 1 and 0 set, neither flag bit set. In v0.10 these are
        // the only reserved bits (bit 3 was reclaimed as OriginPaths flag).
        let err = BytecodeHeader::from_byte(0x03).unwrap_err();
        assert!(
            matches!(
                err,
                Error::InvalidBytecode {
                    offset: 0,
                    kind: BytecodeErrorKind::ReservedBitsSet {
                        byte: 0x03,
                        mask: RESERVED_MASK
                    }
                }
            ),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn all_low_nibble_bits_set() {
        // 0x0F = all lower 4 bits set
        let err = BytecodeHeader::from_byte(0x0F).unwrap_err();
        assert!(
            matches!(
                err,
                Error::InvalidBytecode {
                    offset: 0,
                    kind: BytecodeErrorKind::ReservedBitsSet { .. }
                }
            ),
            "unexpected error: {err:?}"
        );
    }

    // --- Unknown-version rejection ---

    #[test]
    fn unknown_version_0x10() {
        let err = BytecodeHeader::from_byte(0x10).unwrap_err();
        assert!(
            matches!(err, Error::UnsupportedVersion(1)),
            "expected UnsupportedVersion(1), got {err:?}"
        );
    }

    #[test]
    fn unknown_version_0xf0() {
        let err = BytecodeHeader::from_byte(0xF0).unwrap_err();
        assert!(
            matches!(err, Error::UnsupportedVersion(15)),
            "expected UnsupportedVersion(15), got {err:?}"
        );
    }

    #[test]
    fn unknown_version_takes_priority_over_reserved_bits_0x14() {
        // 0x14 = version nibble 1, fingerprints bit set — both conditions apply,
        // but version check MUST win (see from_byte doc comment).
        let err = BytecodeHeader::from_byte(0x14).unwrap_err();
        assert!(
            matches!(err, Error::UnsupportedVersion(1)),
            "expected UnsupportedVersion(1), got {err:?}"
        );
    }

    // --- v0.10: header bit 3 reclaim for OriginPaths flag ---

    #[test]
    fn header_byte_0x08_decodes_with_origin_paths_flag_set() {
        let h = BytecodeHeader::from_byte(0x08).expect("0x08 must be valid v0.10");
        assert_eq!(h.version(), 0);
        assert!(h.origin_paths());
        assert!(!h.fingerprints());
    }

    #[test]
    fn header_byte_0x0c_decodes_with_both_flags_set() {
        let h = BytecodeHeader::from_byte(0x0C).expect("0x0C must be valid v0.10");
        assert!(h.origin_paths());
        assert!(h.fingerprints());
    }

    #[test]
    fn header_byte_0x02_rejects_with_reserved_bit_1() {
        let err = BytecodeHeader::from_byte(0x02).unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidBytecode {
                kind: BytecodeErrorKind::ReservedBitsSet {
                    byte: 0x02,
                    mask: 0x03
                },
                ..
            }
        ));
    }

    #[test]
    fn header_byte_0x01_rejects_with_reserved_bit_0() {
        let err = BytecodeHeader::from_byte(0x01).unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidBytecode {
                kind: BytecodeErrorKind::ReservedBitsSet {
                    byte: 0x01,
                    mask: 0x03
                },
                ..
            }
        ));
    }

    #[test]
    fn new_v0_signature_takes_origin_paths_bool() {
        let h = BytecodeHeader::new_v0(false, false);
        assert_eq!(h.as_byte(), 0x00);
        let h = BytecodeHeader::new_v0(true, false);
        assert_eq!(h.as_byte(), 0x04);
        let h = BytecodeHeader::new_v0(false, true);
        assert_eq!(h.as_byte(), 0x08);
        let h = BytecodeHeader::new_v0(true, true);
        assert_eq!(h.as_byte(), 0x0C);
    }
}
