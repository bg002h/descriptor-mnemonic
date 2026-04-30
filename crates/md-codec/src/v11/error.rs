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

    /// Key count n out of range; v0.11 requires 1 ≤ n ≤ 32.
    #[error("key count {n} out of range; v0.11 requires 1 ≤ n ≤ 32")]
    KeyCountOutOfRange {
        /// Actual key count provided.
        n: u8,
    },

    /// Divergent path count doesn't match key count.
    #[error("divergent path count {got} does not match key count {n}")]
    DivergentPathCountMismatch {
        /// Expected key count.
        n: u8,
        /// Actual path count provided.
        got: usize,
    },

    /// Multipath alt-count out of range; v0.11 requires 2 ≤ count ≤ 9.
    #[error("multipath alt-count {got} out of range; v0.11 requires 2 ≤ count ≤ 9")]
    AltCountOutOfRange {
        /// Provided alt-count.
        got: usize,
    },

    /// Unknown primary tag value (0x00..0x1F unrecognized).
    #[error("unknown primary tag value 0x{0:02x}")]
    UnknownPrimaryTag(u8),

    /// Unknown extension tag value (after 0x1F primary prefix).
    #[error("unknown extension tag value 0x{0:02x}")]
    UnknownExtensionTag(u8),

    /// Threshold k out of range; v0.11 requires 1 ≤ k ≤ 32.
    #[error("threshold k={k} out of range; v0.11 requires 1 ≤ k ≤ 32")]
    ThresholdOutOfRange {
        /// Provided k value.
        k: u8,
    },

    /// Variable-arity child count out of range.
    #[error("child count {count} out of range; v0.11 requires 1 ≤ count ≤ 32")]
    ChildCountOutOfRange {
        /// Provided child count.
        count: usize,
    },

    /// k > n in k-of-n threshold/multisig.
    #[error("threshold k={k} exceeds child count n={n}; require k ≤ n")]
    KGreaterThanN {
        /// Threshold k.
        k: u8,
        /// Child count n.
        n: usize,
    },

    /// TLV ordering violation: a TLV tag was followed by a smaller-or-equal tag.
    #[error("TLV ordering violation: tag 0x{prev:02x} followed by 0x{current:02x}; require ascending")]
    TlvOrderingViolation {
        /// Previous tag value.
        prev: u8,
        /// Current tag value.
        current: u8,
    },

    /// Placeholder index in TLV entry exceeds key count n.
    #[error("placeholder index {idx} out of range; require idx < n={n}")]
    PlaceholderIndexOutOfRange {
        /// Provided index.
        idx: u8,
        /// Key count n.
        n: u8,
    },

    /// Per-`@N` override entries within a TLV must be in ascending `@N`-index order.
    #[error("override ordering violation: @{prev} followed by @{current}; require ascending")]
    OverrideOrderViolation {
        /// Previous index.
        prev: u8,
        /// Current index.
        current: u8,
    },

    /// TLV entry has zero entries; encoder MUST omit empty TLVs per spec §7.5.
    #[error("TLV entry tag 0x{tag:02x} has empty payload; encoder MUST omit empty TLVs")]
    EmptyTlvEntry {
        /// Tag of the empty entry.
        tag: u8,
    },

    /// Placeholder @i was not referenced anywhere in the tree (BIP 388 well-formedness).
    #[error("placeholder @{idx} not referenced in tree; n={n}")]
    PlaceholderNotReferenced {
        /// The unreferenced placeholder index.
        idx: u8,
        /// Key count.
        n: u8,
    },

    /// First-occurrence ordering violated (BIP 388 well-formedness).
    #[error("placeholder first-occurrence ordering violated: expected first={expected_first}, got first={got_first}")]
    PlaceholderFirstOccurrenceOutOfOrder {
        /// Expected placeholder index in canonical first-occurrence position.
        expected_first: u8,
        /// Actual placeholder index encountered first.
        got_first: u8,
    },

    /// All multipaths in a template must share the same alt-count.
    #[error("multipath alt-count mismatch: expected {expected}, got {got}")]
    MultipathAltCountMismatch {
        /// Expected alt-count.
        expected: usize,
        /// Mismatched alt-count.
        got: usize,
    },

    /// Tap-script-tree leaf has a tag that is forbidden per spec §6.3.1.
    #[error("forbidden tap-script-tree leaf tag: {tag}")]
    ForbiddenTapTreeLeaf {
        /// The forbidden tag (debug-formatted name).
        tag: String,
    },

    /// Chunk count out of range; v0.11 requires 1 ≤ count ≤ 64.
    #[error("chunk count {count} out of range; v0.11 requires 1 ≤ count ≤ 64")]
    ChunkCountOutOfRange {
        /// Provided count.
        count: u8,
    },

    /// Chunk index ≥ count; require index < count.
    #[error("chunk index {index} ≥ count {count}")]
    ChunkIndexOutOfRange {
        /// Provided index.
        index: u8,
        /// Provided count.
        count: u8,
    },

    /// Chunk-set-id exceeds 20-bit range.
    #[error("chunk-set-id 0x{id:x} exceeds 20-bit range")]
    ChunkSetIdOutOfRange {
        /// Provided ID.
        id: u32,
    },

    /// Chunk header missing chunked-flag (bit 3 must be 1).
    #[error("chunk header chunked-flag missing; bit 3 must be 1 for chunk headers")]
    ChunkHeaderChunkedFlagMissing,
}
