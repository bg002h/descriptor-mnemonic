//! v0.11-specific error variants.

use thiserror::Error;

/// Errors produced by v0.11 wire-format codec components.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum V11Error {
    /// The bit stream was exhausted at the given bit `position`.
    #[error("bit stream exhausted at bit {position}")]
    BitStreamExhausted {
        /// Bit offset at which exhaustion was detected.
        position: usize,
    },

    /// A read of `requested` bits was attempted but only `available` bits remained.
    #[error("attempted to read {requested} bits with only {available} bits remaining")]
    BitStreamTruncated {
        /// Number of bits the caller requested.
        requested: usize,
        /// Number of bits actually available in the stream.
        available: usize,
    },

    /// Header bit 3 (reserved) was set; v0.11 requires bit 3 = 0.
    #[error("reserved header bit (bit 3) set; v0.11 requires bit 3 = 0")]
    ReservedHeaderBitSet,

    /// Wire-format version field doesn't match a supported version.
    #[error("unsupported wire-format version: got {got}")]
    UnsupportedVersion {
        /// Version value parsed from bits 0..2.
        got: u8,
    },

    /// Path depth exceeds MAX_PATH_COMPONENTS (15).
    #[error("path depth {got} exceeds maximum {max}")]
    PathDepthExceeded {
        /// Actual depth of the path.
        got: usize,
        /// Maximum allowed depth (15).
        max: usize,
    },
}
