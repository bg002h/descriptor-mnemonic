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
}
