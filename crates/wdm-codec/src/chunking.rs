//! Chunk header types and byte codec for WDM multi-string chunking.
//!
//! The `ChunkHeader` enum represents the two possible header shapes that prefix
//! each chunk's fragment bytes.  It is serialised to/from a **byte-aligned
//! canonical form** before the codex32 5-bit packing layer (Phase 7) wraps it
//! into a string.
//!
//! # Design decision (enum vs struct)
//!
//! `ChunkHeader` is modelled as an **enum** with `SingleString` and `Chunked`
//! variants rather than a struct with `Option`-typed fields.  The wire format
//! encodes an explicit `type` byte that determines which fields are present;
//! the enum makes the invariant "wallet-id/count/index are set ↔ type=Chunked"
//! a compile-time guarantee rather than a runtime check that every consumer
//! must repeat.  Exhaustive pattern-matching at call sites is a feature, not a
//! burden.

use crate::error::{Error, Result};
use crate::wallet_id::ChunkWalletId;

/// Version byte for format version 0.
const VERSION_0: u8 = 0x00;
/// Type byte for a single-string (non-chunked) card.
const TYPE_SINGLE: u8 = 0x00;
/// Type byte for a chunked card.
const TYPE_CHUNKED: u8 = 0x01;
/// Maximum permitted chunk count (5-bit field, value 1–32).
const MAX_CHUNK_COUNT: u8 = 32;
/// Byte length of a SingleString header.
const SINGLE_HEADER_LEN: usize = 2;
/// Byte length of a Chunked header.
const CHUNKED_HEADER_LEN: usize = 7;

/// Header prepended to each chunk's fragment bytes.
///
/// Wire format (canonical byte-aligned form, before codex32 5-bit packing):
/// - `SingleString`: `[version: u8, type=0: u8]` = 2 bytes
/// - `Chunked`:      `[version: u8, type=1: u8, wallet_id_be: [u8; 3], count: u8, index: u8]`
///   = 7 bytes; the `wallet_id` 20-bit value is stored big-endian with the top
///   4 bits of the first byte set to zero.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkHeader {
    /// A single codex32 string that carries the entire bytecode; no chunking.
    SingleString {
        /// Format version byte (currently `0`).
        version: u8,
    },
    /// One chunk in a multi-string sequence.
    Chunked {
        /// Format version byte (currently `0`).
        version: u8,
        /// 20-bit wallet identifier shared by all chunks of a given wallet.
        wallet_id: ChunkWalletId,
        /// Total number of chunks in this sequence (1–32).
        count: u8,
        /// Zero-based index of this chunk within the sequence (0..count-1).
        index: u8,
    },
}

impl ChunkHeader {
    /// Serialize to canonical byte form.
    ///
    /// Returns 2 bytes for [`ChunkHeader::SingleString`] and 7 bytes for
    /// [`ChunkHeader::Chunked`].
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            ChunkHeader::SingleString { version } => {
                vec![*version, TYPE_SINGLE]
            }
            ChunkHeader::Chunked {
                version,
                wallet_id,
                count,
                index,
            } => {
                let w = wallet_id.as_u32();
                vec![
                    *version,
                    TYPE_CHUNKED,
                    (w >> 16) as u8,
                    (w >> 8) as u8,
                    w as u8,
                    *count,
                    *index,
                ]
            }
        }
    }

    /// Parse a `ChunkHeader` from the start of `bytes`.
    ///
    /// Returns the parsed header and the number of bytes consumed (2 for
    /// `SingleString`, 7 for `Chunked`).  The caller may slice off the
    /// remainder as the fragment payload.
    ///
    /// # Errors
    ///
    /// - [`Error::ChunkHeaderTruncated`] — fewer bytes than the minimum header.
    /// - [`Error::UnsupportedVersion`] — version byte is not `0`.
    /// - [`Error::UnsupportedCardType`] — type byte is not `0` or `1`.
    /// - [`Error::InvalidWalletIdEncoding`] — top 4 bits of wallet-id are set.
    /// - [`Error::InvalidChunkCount`] — count is `0` or `> 32`.
    /// - [`Error::InvalidChunkIndex`] — `index >= count`.
    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, usize)> {
        // Need at least 2 bytes for version + type.
        if bytes.len() < SINGLE_HEADER_LEN {
            return Err(Error::ChunkHeaderTruncated);
        }

        let version = bytes[0];
        if version != VERSION_0 {
            return Err(Error::UnsupportedVersion(version));
        }

        let type_byte = bytes[1];
        match type_byte {
            TYPE_SINGLE => Ok((ChunkHeader::SingleString { version }, SINGLE_HEADER_LEN)),
            TYPE_CHUNKED => {
                // Need 7 bytes total for the chunked header.
                if bytes.len() < CHUNKED_HEADER_LEN {
                    return Err(Error::ChunkHeaderTruncated);
                }

                // Wallet-id: 3 bytes, top 4 bits of first byte must be zero.
                let hi = bytes[2];
                if hi & 0xF0 != 0 {
                    return Err(Error::InvalidWalletIdEncoding);
                }
                let w = ((hi as u32) << 16) | ((bytes[3] as u32) << 8) | (bytes[4] as u32);
                // Belt-and-suspenders: the high-bit check above ensures w <= MAX.
                let wallet_id = ChunkWalletId::new(w);

                let count = bytes[5];
                if count == 0 || count > MAX_CHUNK_COUNT {
                    return Err(Error::InvalidChunkCount(count));
                }

                let index = bytes[6];
                if index >= count {
                    return Err(Error::InvalidChunkIndex { index, count });
                }

                Ok((
                    ChunkHeader::Chunked {
                        version,
                        wallet_id,
                        count,
                        index,
                    },
                    CHUNKED_HEADER_LEN,
                ))
            }
            other => Err(Error::UnsupportedCardType(other)),
        }
    }

    /// Returns the format version byte.
    pub fn version(&self) -> u8 {
        match self {
            ChunkHeader::SingleString { version } | ChunkHeader::Chunked { version, .. } => {
                *version
            }
        }
    }

    /// Returns `true` if this header is the `Chunked` variant.
    pub fn is_chunked(&self) -> bool {
        matches!(self, ChunkHeader::Chunked { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Round-trip tests
    // -----------------------------------------------------------------------

    #[test]
    fn single_string_round_trip() {
        let hdr = ChunkHeader::SingleString { version: 0 };
        let bytes = hdr.to_bytes();
        assert_eq!(bytes, &[0x00, 0x00]);
        let (decoded, consumed) = ChunkHeader::from_bytes(&bytes).unwrap();
        assert_eq!(decoded, hdr);
        assert_eq!(consumed, 2);
    }

    #[test]
    fn chunked_round_trip_minimal() {
        let hdr = ChunkHeader::Chunked {
            version: 0,
            wallet_id: ChunkWalletId::new(0),
            count: 1,
            index: 0,
        };
        let bytes = hdr.to_bytes();
        assert_eq!(bytes, &[0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00]);
        let (decoded, consumed) = ChunkHeader::from_bytes(&bytes).unwrap();
        assert_eq!(decoded, hdr);
        assert_eq!(consumed, 7);
    }

    #[test]
    fn chunked_round_trip_max_wallet_id() {
        // ChunkWalletId::MAX = 0xF_FFFF; encodes as [0x0F, 0xFF, 0xFF].
        let hdr = ChunkHeader::Chunked {
            version: 0,
            wallet_id: ChunkWalletId::new(ChunkWalletId::MAX),
            count: 4,
            index: 0,
        };
        let bytes = hdr.to_bytes();
        // wallet_id bytes: [(0xFFFFF >> 16)=0x0F, (0xFFFFF >> 8) & 0xFF=0xFF, 0xFF & 0xFF=0xFF]
        assert_eq!(bytes[2..5], [0x0F, 0xFF, 0xFF]);
        let (decoded, consumed) = ChunkHeader::from_bytes(&bytes).unwrap();
        assert_eq!(decoded, hdr);
        assert_eq!(consumed, 7);
    }

    #[test]
    fn chunked_round_trip_max_count_and_index() {
        let hdr = ChunkHeader::Chunked {
            version: 0,
            wallet_id: ChunkWalletId::new(0x1234),
            count: 32,
            index: 31,
        };
        let bytes = hdr.to_bytes();
        let (decoded, consumed) = ChunkHeader::from_bytes(&bytes).unwrap();
        assert_eq!(decoded, hdr);
        assert_eq!(consumed, 7);
    }

    #[test]
    fn from_bytes_returns_consumed_count() {
        // SingleString: consumed = 2, remainder is the rest.
        let mut buf = vec![0x00u8, 0x00, 0xAA, 0xBB, 0xCC];
        let (_, consumed) = ChunkHeader::from_bytes(&buf).unwrap();
        assert_eq!(consumed, 2);
        assert_eq!(&buf[consumed..], &[0xAA, 0xBB, 0xCC]);

        // Chunked: consumed = 7, remainder follows.
        buf = vec![0x00, 0x01, 0x00, 0x00, 0x01, 0x02, 0x01, 0xDE, 0xAD];
        let (_, consumed) = ChunkHeader::from_bytes(&buf).unwrap();
        assert_eq!(consumed, 7);
        assert_eq!(&buf[consumed..], &[0xDE, 0xAD]);
    }

    // -----------------------------------------------------------------------
    // Accessor tests
    // -----------------------------------------------------------------------

    #[test]
    fn version_accessor() {
        assert_eq!(ChunkHeader::SingleString { version: 0 }.version(), 0);
        assert_eq!(
            ChunkHeader::Chunked {
                version: 0,
                wallet_id: ChunkWalletId::new(0),
                count: 1,
                index: 0,
            }
            .version(),
            0
        );
    }

    #[test]
    fn is_chunked_accessor() {
        assert!(!ChunkHeader::SingleString { version: 0 }.is_chunked());
        assert!(
            ChunkHeader::Chunked {
                version: 0,
                wallet_id: ChunkWalletId::new(0),
                count: 1,
                index: 0,
            }
            .is_chunked()
        );
    }

    // -----------------------------------------------------------------------
    // Rejection tests
    // -----------------------------------------------------------------------

    #[test]
    fn reject_unknown_version() {
        let bytes = [0x01u8, 0x00];
        let err = ChunkHeader::from_bytes(&bytes).unwrap_err();
        assert!(
            matches!(err, Error::UnsupportedVersion(1)),
            "expected UnsupportedVersion(1), got {err:?}"
        );
    }

    #[test]
    fn reject_unknown_type() {
        let bytes = [0x00u8, 0x02];
        let err = ChunkHeader::from_bytes(&bytes).unwrap_err();
        assert!(
            matches!(err, Error::UnsupportedCardType(2)),
            "expected UnsupportedCardType(2), got {err:?}"
        );
    }

    #[test]
    fn reject_zero_count() {
        // [ver=0, type=1, wid=0x00,0x00,0x00, count=0, index=0]
        let bytes = [0x00u8, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00];
        let err = ChunkHeader::from_bytes(&bytes).unwrap_err();
        assert!(
            matches!(err, Error::InvalidChunkCount(0)),
            "expected InvalidChunkCount(0), got {err:?}"
        );
    }

    #[test]
    fn reject_count_above_32() {
        // count = 33
        let bytes = [0x00u8, 0x01, 0x00, 0x00, 0x00, 33, 0x00];
        let err = ChunkHeader::from_bytes(&bytes).unwrap_err();
        assert!(
            matches!(err, Error::InvalidChunkCount(33)),
            "expected InvalidChunkCount(33), got {err:?}"
        );
    }

    #[test]
    fn reject_index_ge_count() {
        // count=5, index=5 (index must be 0..4)
        let bytes = [0x00u8, 0x01, 0x00, 0x00, 0x00, 5, 5];
        let err = ChunkHeader::from_bytes(&bytes).unwrap_err();
        assert!(
            matches!(err, Error::InvalidChunkIndex { index: 5, count: 5 }),
            "expected InvalidChunkIndex {{ index: 5, count: 5 }}, got {err:?}"
        );
    }

    #[test]
    fn reject_wallet_id_top_bits_set() {
        // wallet_id first byte = 0x10 → bit 20 set (top nibble non-zero).
        // Construct raw bytes without going through ChunkWalletId::new (which panics).
        let bytes = [0x00u8, 0x01, 0x10, 0x00, 0x00, 0x01, 0x00];
        let err = ChunkHeader::from_bytes(&bytes).unwrap_err();
        assert!(
            matches!(err, Error::InvalidWalletIdEncoding),
            "expected InvalidWalletIdEncoding, got {err:?}"
        );
    }

    #[test]
    fn reject_truncated_input_single() {
        // Only 1 byte — too short for the 2-byte SingleString header.
        let err = ChunkHeader::from_bytes(&[0x00]).unwrap_err();
        assert!(
            matches!(err, Error::ChunkHeaderTruncated),
            "expected ChunkHeaderTruncated, got {err:?}"
        );
    }

    #[test]
    fn reject_truncated_input_empty() {
        let err = ChunkHeader::from_bytes(&[]).unwrap_err();
        assert!(
            matches!(err, Error::ChunkHeaderTruncated),
            "expected ChunkHeaderTruncated, got {err:?}"
        );
    }

    #[test]
    fn reject_truncated_chunked_header() {
        // type=1 but only 3 bytes — too short for the 7-byte Chunked header.
        let err = ChunkHeader::from_bytes(&[0x00, 0x01, 0x00]).unwrap_err();
        assert!(
            matches!(err, Error::ChunkHeaderTruncated),
            "expected ChunkHeaderTruncated, got {err:?}"
        );
    }
}
