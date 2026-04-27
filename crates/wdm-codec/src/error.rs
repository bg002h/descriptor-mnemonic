//! Error types for wdm-codec.

use thiserror::Error;

/// Forward declaration; the authoritative implementation lives in
/// `wallet_id.rs` once Phase 4 of the implementation plan lands.
///
/// TODO P4: move `ChunkWalletId` to `crate::wallet_id`, make the inner
/// `u32` private, and gate construction through `ChunkWalletId::new(bits: u32)`
/// which enforces `bits <= MAX = (1 << 20) - 1`. Once moved, replace the
/// definition here with `use crate::wallet_id::ChunkWalletId;` and update
/// the re-export in `lib.rs`. The `pub(crate) u32` field below is a
/// temporary convenience for v0.1 scaffolding only — do not let chunking.rs
/// rely on it.
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

    /// String contains a character that is not in the bech32 alphabet.
    #[error("invalid character '{ch}' at position {position} (not in bech32 alphabet)")]
    InvalidChar {
        /// The invalid character encountered.
        ch: char,
        /// Zero-based character index within the data part (after the `"wdm1"` separator).
        position: usize,
    },

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

/// Kind of bytecode parse error, used inside [`Error::InvalidBytecode`].
#[non_exhaustive]
#[derive(Debug, Error, Clone, PartialEq, Eq)]
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

    /// The header byte had one or more reserved bits set to a non-zero value.
    ///
    /// `byte` is the raw header byte that was rejected; `mask` is the set of
    /// bits that are reserved-MUST-be-zero (e.g. `0x0B` for v0: bits 3, 1, 0).
    #[error("reserved bits set in header byte {byte:#04x} (reserved mask: {mask:#04x})")]
    ReservedBitsSet {
        /// The raw header byte that contained non-zero reserved bits.
        byte: u8,
        /// Bitmask of the reserved bits (those that must be zero).
        mask: u8,
    },

    /// Reconstructed miniscript fragment failed type-check during
    /// `Wsh::new(...)` or equivalent. Wraps the upstream miniscript
    /// error message; carried as `String` to insulate from upstream
    /// `miniscript::Error` churn.
    #[error("miniscript type check failed: {0}")]
    TypeCheckFailed(String),

    /// LEB128 child encoding decoded to a value outside the valid BIP32 range.
    ///
    /// BIP32 child indices are in `0..=2^31-1` for both normal and hardened
    /// forms. The wire encoding maps child index `c` to `2c` (normal) or
    /// `2c + 1` (hardened), so the maximum legal encoded value is
    /// `2*(2^31 - 1) + 1 = 2^32 - 1 = 0xFFFF_FFFF`. Any decoded value above
    /// that is rejected here.
    #[error(
        "invalid path component: encoded value {encoded} exceeds maximum BIP32 child encoding (2*(2^31-1)+1)"
    )]
    InvalidPathComponent {
        /// The raw LEB128-decoded value that exceeded the valid range.
        encoded: u64,
    },

    /// A tag byte was valid but not the tag expected at this position.
    ///
    /// For example, `decode_declaration` expects `Tag::SharedPath` (0x33) as
    /// the first byte; if it reads a different defined tag, this variant is
    /// returned. If the byte does not correspond to any defined tag at all,
    /// [`BytecodeErrorKind::UnknownTag`] is returned instead.
    #[error("unexpected tag: expected {expected:#04x}, got {got:#04x}")]
    UnexpectedTag {
        /// The tag byte value that was expected at this position.
        expected: u8,
        /// The tag byte value that was actually read.
        got: u8,
    },
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
    fn miniscript_error_can_be_wrapped_explicitly() {
        // The blanket From<miniscript::Error> impl was removed (Issue 3 from
        // the Phase 2 decision review); call sites that need to wrap a
        // miniscript error now construct Error::Miniscript explicitly.
        let parse_result = "not_a_valid_descriptor".parse::<miniscript::descriptor::Descriptor<miniscript::descriptor::DescriptorPublicKey>>();
        let ms_err = parse_result.expect_err("intentionally invalid descriptor");
        let e = Error::Miniscript(ms_err.to_string());
        assert!(matches!(e, Error::Miniscript(_)));
        let s = e.to_string();
        assert!(s.starts_with("miniscript:"), "got: {s}");
    }

    #[test]
    fn type_check_failed_variant_displays() {
        let e = Error::InvalidBytecode {
            offset: 7,
            kind: BytecodeErrorKind::TypeCheckFailed("Bdu type required".to_string()),
        };
        let s = e.to_string();
        assert!(s.contains("offset 7"), "got: {s}");
        assert!(s.contains("miniscript type check failed"), "got: {s}");
    }

    #[test]
    fn bytecode_error_kind_display() {
        let k = BytecodeErrorKind::UnknownTag(0xFF);
        assert_eq!(k.to_string(), "unknown tag 0xff");
    }
}
