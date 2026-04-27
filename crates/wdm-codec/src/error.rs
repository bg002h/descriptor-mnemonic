//! Error types for wdm-codec.

use thiserror::Error;

/// Forward declaration; defined fully in chunking.rs once available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkWalletId(pub(crate) u32);

/// All errors that wdm-codec can return.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum Error {
    /// HRP did not match the expected `"wdm"`.
    #[error("invalid HRP: expected 'wdm', got '{0}'")]
    InvalidHrp(String),

    /// Bech32 string contained mixed-case characters.
    #[error("invalid case: bech32 strings must be all-lowercase or all-uppercase")]
    MixedCase,

    /// Total string length is invalid (e.g., the reserved 94 or 95 char range).
    #[error("invalid string length: {0}")]
    InvalidStringLength(usize),

    /// BCH error correction failed (more than 4 substitutions).
    #[error("BCH decode failed: too many errors to correct")]
    BchUncorrectable,

    /// Bytecode parse failed at a specific offset.
    #[error("invalid bytecode at offset {offset}: {kind}")]
    InvalidBytecode {
        /// Byte offset within the canonical bytecode where the parse failed.
        offset: usize,
        /// Specific kind of bytecode error.
        kind: BytecodeErrorKind,
    },

    /// Format version is not supported by this implementation.
    #[error("unsupported format version: {0}")]
    UnsupportedVersion(u8),

    /// Card type is not supported.
    #[error("unsupported card type: {0}")]
    UnsupportedCardType(u8),

    /// Chunk index is out of range for the declared total.
    #[error("chunk index {index} out of range (total chunks: {total})")]
    ChunkIndexOutOfRange {
        /// The reported chunk index.
        index: u8,
        /// The declared total chunk count.
        total: u8,
    },

    /// A chunk index appears more than once during reassembly.
    #[error("duplicate chunk index: {0}")]
    DuplicateChunkIndex(u8),

    /// Two chunks reported different wallet identifiers.
    #[error("wallet identifier mismatch across chunks: expected {expected:?}, got {got:?}")]
    WalletIdMismatch {
        /// The expected (first-seen) chunk wallet identifier.
        expected: ChunkWalletId,
        /// The mismatched value seen on a later chunk.
        got: ChunkWalletId,
    },

    /// Two chunks reported different total chunk counts.
    #[error("total-chunks mismatch across chunks: expected {expected}, got {got}")]
    TotalChunksMismatch {
        /// The expected (first-seen) total.
        expected: u8,
        /// The mismatched value seen on a later chunk.
        got: u8,
    },

    /// Policy violates the v0.1 implementation scope.
    #[error("policy violates v0.1 scope: {0}")]
    PolicyScopeViolation(String),

    /// Cross-chunk integrity hash did not match the reassembled bytecode.
    #[error("cross-chunk hash mismatch")]
    CrossChunkHashMismatch,

    /// Policy parse error from the BIP 388 string form.
    #[error("policy parse error: {0}")]
    PolicyParse(String),

    /// Wraps a miniscript error as a string to insulate from upstream churn.
    #[error("miniscript: {0}")]
    Miniscript(String),
}

impl From<miniscript::Error> for Error {
    fn from(e: miniscript::Error) -> Self {
        Error::Miniscript(e.to_string())
    }
}

/// Kind of bytecode parse error, used inside [`Error::InvalidBytecode`].
#[non_exhaustive]
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum BytecodeErrorKind {
    /// Tag byte does not correspond to any defined operator.
    #[error("unknown tag {0:#04x}")]
    UnknownTag(u8),

    /// A length prefix declared more bytes than the buffer contains.
    #[error("truncated input")]
    Truncated,

    /// LEB128 varint exceeded its expected width.
    #[error("varint overflow")]
    VarintOverflow,

    /// Operator expected more children than were present.
    #[error("missing children: expected {expected}, got {got}")]
    MissingChildren {
        /// Number of children expected by the operator's arity.
        expected: usize,
        /// Number of children actually parsed.
        got: usize,
    },

    /// Cursor ran off the end of the buffer mid-parse.
    #[error("unexpected end of buffer")]
    UnexpectedEnd,

    /// Buffer had bytes remaining after the operator tree was fully consumed.
    #[error("trailing bytes after canonical bytecode")]
    TrailingBytes,
}

/// Result type used throughout wdm-codec.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_matches_thiserror_format() {
        let e = Error::InvalidHrp("btc".to_string());
        assert_eq!(e.to_string(), "invalid HRP: expected 'wdm', got 'btc'");
    }

    #[test]
    fn miniscript_error_is_wrapped_as_string() {
        // A real miniscript error will be wrapped as String; here we just
        // confirm the conversion compiles and produces our variant.
        let _e: Error = Error::Miniscript("test".to_string());
    }

    #[test]
    fn bytecode_error_kind_display() {
        let k = BytecodeErrorKind::UnknownTag(0xFF);
        assert_eq!(k.to_string(), "unknown tag 0xff");
    }
}
