//! Error types for md-codec.
//!
//! [`enum@Error`] is the single error type returned by every fallible operation in
//! the public API. The variants are organized by which pipeline stage
//! produces them, so the [`crate::decode()`] documentation lists which
//! variants can fire from which stage. See each variant's rustdoc for the
//! WHEN (which call returns this) and the CORRECTIVE ACTION a caller should
//! take.

use thiserror::Error;

// `ChunkSetId` is defined in `policy_id` (the module owns both Tier-3
// `PolicyId` and the chunk-domain `ChunkSetId`) and re-exported here so
// that `Error` variants can reference it without a cross-module path.
pub use crate::policy_id::ChunkSetId;

/// Every error md-codec can return.
///
/// Marked `#[non_exhaustive]` so v0.2+ can add variants (e.g. for taproot,
/// foreign xpubs, BIP 393 recovery annotations) without breaking exhaustive
/// `match` consumers. Match with a `_` arm to remain forward-compatible.
///
/// # Variants by pipeline stage
///
/// Stage 1 (per-string parse): [`Error::InvalidHrp`], [`Error::MixedCase`],
/// [`Error::InvalidStringLength`], [`Error::InvalidChar`].
///
/// Stage 2 (BCH validate/correct): [`Error::BchUncorrectable`].
///
/// Stage 3 (header parse): [`Error::ChunkHeaderTruncated`],
/// [`Error::UnsupportedVersion`], [`Error::UnsupportedCardType`],
/// [`Error::ReservedChunkSetIdBitsSet`], [`Error::InvalidChunkCount`],
/// [`Error::InvalidChunkIndex`].
///
/// Stage 4 (reassembly): [`Error::EmptyChunkList`],
/// [`Error::MixedChunkTypes`], [`Error::SingleStringWithMultipleChunks`],
/// [`Error::ChunkSetIdMismatch`], [`Error::TotalChunksMismatch`],
/// [`Error::ChunkIndexOutOfRange`], [`Error::DuplicateChunkIndex`],
/// [`Error::MissingChunkIndex`], [`Error::CrossChunkHashMismatch`].
///
/// Stage 5 (bytecode parse): [`Error::InvalidBytecode`],
/// [`Error::PolicyScopeViolation`], [`Error::SubsetViolation`],
/// [`Error::FingerprintsCountMismatch`].
///
/// Encode-side: [`Error::PolicyTooLarge`], [`Error::PolicyParse`],
/// [`Error::Miniscript`], [`Error::FingerprintsCountMismatch`].
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum Error {
    /// HRP did not match the expected `"md"`.
    ///
    /// Returned by [`crate::decode_string`] / [`crate::decode()`]. The user
    /// transcribed a non-MD bech32 string (e.g. a Bitcoin address). Caller
    /// should reject the input and ask the user for an `md1…` string.
    #[error("invalid HRP: expected 'md', got '{0}'")]
    InvalidHrp(String),

    /// Bech32 string contained mixed-case characters.
    ///
    /// BIP 173 forbids mixed-case bech32 strings to avoid HRP confusion.
    /// Caller should normalize the input to a single case (e.g.
    /// `s.to_lowercase()`) before retrying — most engraved cards are
    /// upper-case for legibility.
    #[error("invalid case: bech32 strings must be all-lowercase or all-uppercase")]
    MixedCase,

    /// Total string length is invalid (e.g., the reserved 94 or 95 char range).
    ///
    /// The 94–95-char range is reserved-invalid in BIP 93 codex32 to avoid
    /// ambiguity between regular and long codes. Caller should re-read the
    /// physical card and verify the character count is in {≤93, 96..=108}.
    #[error("invalid string length: {0}")]
    InvalidStringLength(usize),

    /// String contains a character that is not in the bech32 alphabet.
    ///
    /// The bech32 alphabet excludes `b`, `i`, `o`, and `1` (separator) to
    /// reduce transcription errors. Caller should show `position` to the
    /// user and ask them to re-check the engraved character; common
    /// confusions are `b↔6`, `i↔1`, `o↔0`.
    #[error("invalid character '{ch}' at position {position} (not in bech32 alphabet)")]
    InvalidChar {
        /// The invalid character encountered.
        ch: char,
        /// Zero-based character index within the data part (after the `"md1"` separator).
        position: usize,
    },

    /// BCH error correction failed: too many corrupted characters.
    ///
    /// v0.1 corrects at most 1 substitution; v0.2 will reach the spec
    /// promised 4-error correction via Berlekamp-Massey decoding. Caller
    /// should ask the user to re-transcribe the chunk; if multiple chunks
    /// fail, the engraved card may be too damaged for recovery without
    /// erasure hints (deferred to v0.3).
    #[error("BCH decode failed: too many errors to correct")]
    BchUncorrectable,

    /// Bytecode parse failed at a specific offset.
    ///
    /// Returned by [`crate::WalletPolicy::from_bytecode`] (and therefore
    /// by [`crate::decode()`] after reassembly). The `kind` field carries
    /// the specific reason; see [`BytecodeErrorKind`] for the full catalog.
    /// Generally indicates either a transcription error that BCH could not
    /// catch (statistically rare) or an attacker-crafted input.
    #[error("invalid bytecode at offset {offset}: {kind}")]
    InvalidBytecode {
        /// Byte offset within the canonical bytecode where the parse failed.
        offset: usize,
        /// Specific kind of bytecode error.
        kind: BytecodeErrorKind,
    },

    /// Format version nibble is not supported by this implementation.
    ///
    /// v0.1 accepts only version `0`. A non-zero version means the card
    /// was engraved by a later MD version (v0.2+) that this implementation
    /// does not yet understand. Caller should ask the user to use a newer
    /// decoder.
    #[error("unsupported format version: {0}")]
    UnsupportedVersion(u8),

    /// Card type byte is not in the supported set.
    ///
    /// v0.1 defines two card types: `0x00` (single-string) and `0x01`
    /// (chunked). Other values are reserved for future extensions and
    /// will surface here on the v0.1 decoder.
    #[error("unsupported card type: {0}")]
    UnsupportedCardType(u8),

    /// Chunk index is out of range for the declared total (`index >= total`).
    ///
    /// Returned during reassembly when a chunk's header reports an index
    /// that doesn't fit its declared `total`. Indicates a corrupted chunk
    /// header that BCH could not catch. Caller should ask the user to
    /// re-verify the affected chunk.
    #[error("chunk index {index} out of range (total chunks: {total})")]
    ChunkIndexOutOfRange {
        /// The reported chunk index.
        index: u8,
        /// The declared total chunk count.
        total: u8,
    },

    /// A chunk index appears more than once during reassembly.
    ///
    /// The user supplied two chunks with the same `chunk_index`. Most often
    /// the user accidentally duplicated one chunk on input. Caller should
    /// deduplicate the input list and retry.
    #[error("duplicate chunk index: {0}")]
    DuplicateChunkIndex(u8),

    /// Two chunks reported different chunk-set identifiers.
    ///
    /// The user mixed chunks from two different wallets in one decode call.
    /// Compare the `expected` and `got` 20-bit fields against the Tier-3
    /// [`crate::PolicyId`] truncations to identify which chunk is foreign,
    /// then ask the user to retry with a single wallet's chunks.
    #[error("chunk-set identifier mismatch across chunks: expected {expected:?}, got {got:?}")]
    ChunkSetIdMismatch {
        /// The expected (first-seen) chunk-set identifier.
        expected: ChunkSetId,
        /// The mismatched value seen on a later chunk.
        got: ChunkSetId,
    },

    /// Two chunks reported different total chunk counts.
    ///
    /// Indicates either a corrupted chunk-header byte (BCH didn't catch
    /// it because the corruption flipped a checksum-symbol-equivalent bit)
    /// or chunks from two different chunked backups mixed together. Caller
    /// should re-verify the affected chunk against the original media.
    #[error("total-chunks mismatch across chunks: expected {expected}, got {got}")]
    TotalChunksMismatch {
        /// The expected (first-seen) total.
        expected: u8,
        /// The mismatched value seen on a later chunk.
        got: u8,
    },

    /// Policy violates MD's encoding scope.
    ///
    /// Originally framed against the v0.1 implementation subset: `wsh()`
    /// segwit-v0 top-level, all keys placeholder-referenced, all `@i`
    /// placeholders share one derivation path. Subsequent versions
    /// expanded the admitted shapes (taproot in v0.2; multi-leaf TapTree
    /// in v0.5), but this variant remains the structural-rejection
    /// catch-all for top-level shapes MD does not encode.
    ///
    /// Also returned by `policy_to_bytecode` (v0.7+, `compiler` feature)
    /// when the policy compiler emits a shape `WalletPolicy::from_descriptor`
    /// rejects.
    ///
    /// Caller should display the embedded message to the user.
    #[error("policy violates MD encoding scope: {0}")]
    PolicyScopeViolation(String),

    /// Cross-chunk integrity hash did not match the reassembled bytecode.
    ///
    /// The 4-byte trailing hash appended to the chunk stream is
    /// `SHA-256(canonical_bytecode)[0..4]`. A mismatch indicates either
    /// a fragment-byte transcription error that BCH did not catch
    /// (statistically rare given the per-chunk ECC) or a chunk reordering
    /// bug. Caller should ask the user to re-verify each chunk character
    /// by character.
    #[error("cross-chunk hash mismatch")]
    CrossChunkHashMismatch,

    /// Chunk count field is invalid (must be 1–32).
    ///
    /// The chunk-count byte must be in `1..=32`. `0` and `>32` are both
    /// invalid; v0.1 caps chunked backups at 32 chunks (the chunk-index
    /// field is 5 bits + 1 implicit). Indicates a corrupted chunk header.
    #[error("invalid chunk count: {0} (must be 1–32)")]
    InvalidChunkCount(u8),

    /// Chunk index is out of range for the declared count (`index >= count`).
    ///
    /// Returned by [`crate::ChunkHeader::from_bytes`] when a single chunk's
    /// header is internally inconsistent. Distinct from
    /// [`Error::ChunkIndexOutOfRange`], which compares against the
    /// first-seen `total` across multiple chunks.
    #[error("invalid chunk index: {index} >= count {count}")]
    InvalidChunkIndex {
        /// The chunk index that was rejected.
        index: u8,
        /// The declared total chunk count.
        count: u8,
    },

    /// The three chunk-set-id bytes in a chunk header had the reserved top 4 bits set.
    ///
    /// The chunk-set-id field is 20 bits wide; the top 4 bits of the three-byte
    /// big-endian encoding must be zero. Any non-zero high nibble in the
    /// first byte triggers this error and indicates a corrupted chunk header.
    #[error("reserved chunk-set-id bits set: top 4 bits of chunk-set-id field must be zero")]
    ReservedChunkSetIdBitsSet,

    /// The chunk header bytes were truncated (too short to contain a complete header).
    ///
    /// SingleString headers are 2 bytes; Chunked headers are 7 bytes. A
    /// short input indicates either a truncated string at the codex32
    /// layer (probably caught earlier as [`Error::InvalidStringLength`])
    /// or, for synthetic byte-level inputs, a malformed payload.
    #[error("chunk header truncated: have {have} bytes, need {need}")]
    ChunkHeaderTruncated {
        /// Number of bytes actually available.
        have: usize,
        /// Minimum number of bytes required for a complete header.
        need: usize,
    },

    /// Bytecode is too large for any v0 chunking plan.
    ///
    /// v0.1 supports up to 32 long chunks × 53 fragment bytes − 4 hash
    /// bytes = 1692 bytes of canonical bytecode. Returned by
    /// [`crate::chunking_decision`] / [`crate::encode()`]. Caller should
    /// reject the policy as too complex for engraving and consider
    /// splitting the wallet across multiple cards (a v0.2 feature).
    #[error(
        "policy too large: {bytecode_len} bytes exceeds maximum {max_supported} for v0 chunking"
    )]
    PolicyTooLarge {
        /// The bytecode length that was rejected.
        bytecode_len: usize,
        /// The maximum supported bytecode length (= 32 * 53 − 4 = 1692).
        max_supported: usize,
    },

    /// `reassemble_chunks` was called with an empty chunk list.
    ///
    /// Returned by [`crate::reassemble_chunks`] / [`crate::decode()`] when
    /// the input slice is empty. Caller must supply at least one chunk.
    #[error("reassemble_chunks called with an empty chunk list")]
    EmptyChunkList,

    /// An expected chunk index was absent from the chunk list during reassembly.
    ///
    /// In a chunked backup with declared `count`, every index in `0..count`
    /// must be present exactly once. The reported index is the lowest one
    /// missing. Caller should ask the user to locate the missing chunk
    /// (most often: missed a card, transcribed only some chunks).
    #[error("missing chunk index {0} during reassembly")]
    MissingChunkIndex(u8),

    /// The chunk list contained both SingleString and Chunked variants.
    ///
    /// A backup is either a single SingleString chunk OR a set of Chunked
    /// chunks; the two cannot coexist in one decode call. Caller mixed
    /// chunks from different backups and should retry with one set.
    #[error("mixed chunk types: chunk list must be all SingleString or all Chunked")]
    MixedChunkTypes,

    /// A SingleString chunk appeared in a multi-chunk list (length > 1).
    ///
    /// A SingleString backup has exactly one chunk by construction; passing
    /// multiple is a caller mistake. Caller should pass `&[only_chunk]`.
    #[error("single-string chunk appeared in a multi-chunk list")]
    SingleStringWithMultipleChunks,

    /// Policy parse error from the BIP 388 string form.
    ///
    /// Returned by [`std::str::FromStr`] on [`crate::WalletPolicy`]. The
    /// embedded string is the upstream miniscript parser error and should
    /// be displayed to the user.
    #[error("policy parse error: {0}")]
    PolicyParse(String),

    /// Wraps an upstream miniscript error as a string.
    ///
    /// Used for errors that originate from the `miniscript` crate but don't
    /// fit any of the more specific MD variants. The string form insulates
    /// our public API from upstream `miniscript::Error` churn.
    #[error("miniscript: {0}")]
    Miniscript(String),

    /// A taproot leaf miniscript used an operator outside the BIP §"Taproot
    /// tree" per-leaf subset (`pk_k`, `pk_h`, `multi_a`, `or_d`, `and_v`,
    /// `older` plus the safe `c:` / `v:` wrappers required to express them).
    ///
    /// Returned by both the encoder (when emitting a `Descriptor::Tr` whose
    /// leaf miniscript contains a forbidden operator) and the decoder (when
    /// parsing a tap-leaf bytecode stream that decodes to a forbidden
    /// operator). The `operator` field names the rejected fragment so the
    /// caller can show the user a precise diagnostic. See
    /// `design/PHASE_v0_2_D_DECISIONS.md` D-2.
    ///
    /// `leaf_index` carries the DFS pre-order index of the offending leaf
    /// when known (multi-leaf decode paths, multi-leaf encode paths, and
    /// single-leaf paths populate it as `Some(idx)`; legacy paths that
    /// don't yet plumb the index pass `None`).
    #[error("tap-leaf subset violation: operator '{operator}' not in Coldcard subset")]
    #[non_exhaustive]
    SubsetViolation {
        /// The miniscript operator name (e.g. `"sha256"`, `"thresh"`,
        /// `"or_b"`) that violated the subset.
        operator: String,
        /// DFS pre-order index of the offending leaf within the tap tree,
        /// when known. `None` for paths that do not plumb the index.
        leaf_index: Option<usize>,
    },

    /// Fingerprints-block count mismatched the policy's placeholder count.
    ///
    /// BIP §"Fingerprints block" requires the block's count byte to equal
    /// `max(@i in template) + 1` (i.e. one fingerprint per placeholder index).
    /// This variant fires from both directions:
    ///
    /// - **Encoder**: `EncodeOptions::fingerprints` was set to a `Vec<Fingerprint>`
    ///   whose length does not match the policy's placeholder count.
    /// - **Decoder**: the bytecode declared a fingerprints block whose count byte
    ///   did not equal the placeholder count derived from the parsed template.
    ///
    /// Caller should surface a "your fingerprints list has the wrong number of
    /// entries; the policy declares N placeholders" diagnostic. See
    /// `design/PHASE_v0_2_E_DECISIONS.md` E-5.
    #[error("fingerprints count mismatch: expected {expected} (one per placeholder), got {got}")]
    FingerprintsCountMismatch {
        /// The expected fingerprint count (placeholder count of the policy).
        expected: usize,
        /// The actual fingerprint count provided (encoder) or read from the
        /// bytecode (decoder).
        got: usize,
    },

    /// The OriginPaths bytecode count doesn't match the tree's actual
    /// placeholder count after parse.
    ///
    /// NEW in v0.10. Surfaces as a semantic, policy-layer error rather
    /// than a structural bytecode-layer error: the bytecode parsed cleanly
    /// (count byte ≤ 32, each path ≤ MAX_PATH_COMPONENTS), but the
    /// declared count did not equal the placeholder count derived from
    /// the parsed template.
    #[error(
        "OriginPaths count mismatch: tree has {expected} placeholders, OriginPaths declares {got}"
    )]
    OriginPathsCountMismatch {
        /// The expected count (placeholder count of the policy template).
        expected: usize,
        /// The count declared in the OriginPaths block.
        got: usize,
    },

    /// An explicit-form path declaration exceeded `MAX_PATH_COMPONENTS = 10`.
    ///
    /// NEW in v0.10. Applies to both `Tag::SharedPath` and
    /// `Tag::OriginPaths`. The cap mirrors BIP 388's policy-template
    /// limit on per-key path-component counts.
    #[error("path component count {got} exceeds maximum {max}")]
    PathComponentCountExceeded {
        /// The component count that was rejected.
        got: usize,
        /// The maximum allowed component count (`MAX_PATH_COMPONENTS = 10`).
        max: usize,
    },
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

    /// A tag byte is valid in some context but not allowed in this context.
    ///
    /// For example, a top-level descriptor tag (`Tag::Wsh`) appearing where
    /// a tap-leaf inner is expected. Distinct from
    /// [`BytecodeErrorKind::UnknownTag`] (no Tag exists for that byte) and
    /// [`Error::PolicyScopeViolation`] (top-level admit-set decision).
    ///
    /// Introduced in v0.6 alongside the strip-Layer-3 change so the decoder
    /// catch-all in `decode_tap_terminal` can produce a structural diagnostic
    /// rather than the now-removed `SubsetViolation`.
    ///
    /// [`Error::PolicyScopeViolation`]: super::Error::PolicyScopeViolation
    #[error("tag {tag:#04x} is invalid in context {context}")]
    TagInvalidContext {
        /// The tag byte that was structurally invalid in this context.
        tag: u8,
        /// Human-readable context name (e.g., "tap-leaf-inner", "wsh-inner").
        context: &'static str,
    },

    /// The Stage-3 5-bit→byte conversion of a BCH-validated payload failed
    /// because the payload's bit length is not a multiple of 8. This can
    /// happen for hostile inputs whose Long-code data part has 93 5-bit
    /// symbols (= 465 bits, leaving 1 bit of trailing pad) and whose final
    /// 5-bit symbol carries a non-zero low bit despite passing the BCH
    /// validate stage. Conformant encoders always pad to the byte boundary
    /// with zero bits per BIP §"Payload" "padding enabled on the encode side;
    /// reversed on decode" — the decode-side reverse is this rejection.
    ///
    /// Discovered + fixed in v0.2.2 via the v0.2.1 full-code-audit
    /// (`design/agent-reports/v0-2-1-full-code-audit.md`); the prior
    /// implementation panicked with `expect()` on this code path.
    #[error("malformed payload padding: 5-bit data does not byte-align")]
    MalformedPayloadPadding,

    /// The OriginPaths count byte is structurally invalid (zero or exceeds
    /// the BIP 388 placeholder cap of 32).
    ///
    /// NEW in v0.10. Surfaces as a structural, bytecode-layer error
    /// because it can be detected from the OriginPaths header bytes
    /// alone, before any tree-side comparison. A semantic mismatch
    /// between the count and the parsed template's placeholder count is
    /// reported via [`Error::OriginPathsCountMismatch`] instead.
    ///
    /// [`Error::OriginPathsCountMismatch`]: super::Error::OriginPathsCountMismatch
    #[error("OriginPaths count {count} is out of range (must be 1..={max})")]
    OriginPathsCountTooLarge {
        /// The structurally invalid count byte.
        count: u8,
        /// The maximum allowed count (32, mirroring the BIP 388 placeholder cap).
        max: u8,
    },
}

/// Result type used throughout md-codec.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_matches_thiserror_format() {
        let e = Error::InvalidHrp("btc".to_string());
        assert_eq!(e.to_string(), "invalid HRP: expected 'md', got 'btc'");
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
