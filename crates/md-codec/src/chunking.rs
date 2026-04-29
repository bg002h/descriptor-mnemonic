//! Chunk header types, byte codec, and assembly/reassembly for MD multi-string chunking.
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
//! the enum makes the invariant "chunk-set-id/count/index are set ↔ type=Chunked"
//! a compile-time guarantee rather than a runtime check that every consumer
//! must repeat.  Exhaustive pattern-matching at call sites is a feature, not a
//! burden.

use std::collections::HashMap;

use bitcoin::hashes::{Hash, sha256};

use crate::error::{Error, Result};
use crate::policy_id::ChunkSetId;

/// Maximum canonical bytecode length supported by any v0 chunking plan.
///
/// Equals `32 × 53 − 4 = 1692`: 32 Long chunks × 53 bytes per fragment,
/// minus the 4-byte cross-chunk hash appended before splitting.
pub const MAX_BYTECODE_LEN: usize = 32 * 53 - 4; // 1692

/// Maximum number of chunks supported by the v0 format.
///
/// The chunk-count field is a 5-bit unsigned value, giving a maximum of 32.
pub const MAX_CHUNK_COUNT: u8 = 32;

/// Version byte for format version 0.
const VERSION_0: u8 = 0x00;
/// Type byte for a single-string (non-chunked) card.
const TYPE_SINGLE: u8 = 0x00;
/// Type byte for a chunked card.
const TYPE_CHUNKED: u8 = 0x01;
/// Byte length of a SingleString header.
const SINGLE_HEADER_LEN: usize = 2;
/// Byte length of a Chunked header.
const CHUNKED_HEADER_LEN: usize = 7;

/// Header prepended to each chunk's fragment bytes.
///
/// Wire format (canonical byte-aligned form, before codex32 5-bit packing):
/// - `SingleString`: `[version: u8, type=0: u8]` = 2 bytes
/// - `Chunked`:      `[version: u8, type=1: u8, policy_id_be: [u8; 3], count: u8, index: u8]`
///   = 7 bytes; the `chunk_set_id` 20-bit value is stored big-endian with the top
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
        /// 20-bit chunk-set identifier shared by all chunks of a given wallet.
        chunk_set_id: ChunkSetId,
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
                chunk_set_id,
                count,
                index,
            } => {
                let w = chunk_set_id.as_u32();
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
    /// - [`Error::ReservedChunkSetIdBitsSet`] — top 4 bits of chunk-set-id are set.
    /// - [`Error::InvalidChunkCount`] — count is `0` or `> 32`.
    /// - [`Error::InvalidChunkIndex`] — `index >= count`.
    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, usize)> {
        // Need at least 2 bytes for version + type.
        if bytes.len() < SINGLE_HEADER_LEN {
            return Err(Error::ChunkHeaderTruncated {
                have: bytes.len(),
                need: SINGLE_HEADER_LEN,
            });
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
                    return Err(Error::ChunkHeaderTruncated {
                        have: bytes.len(),
                        need: CHUNKED_HEADER_LEN,
                    });
                }

                // Chunk-set-id: 3 bytes, top 4 bits of first byte must be zero.
                let hi = bytes[2];
                if hi & 0xF0 != 0 {
                    return Err(Error::ReservedChunkSetIdBitsSet);
                }
                let w = ((hi as u32) << 16) | ((bytes[3] as u32) << 8) | (bytes[4] as u32);
                // Belt-and-suspenders: the high-bit check above ensures w <= MAX.
                let chunk_set_id = ChunkSetId::new(w);

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
                        chunk_set_id,
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

// ---------------------------------------------------------------------------
// ChunkCode
// ---------------------------------------------------------------------------

/// Selects which BCH code size a chunk's encoding uses.
///
/// Codes from BIP 93 (codex32). Regular code has a 13-char checksum;
/// long code has a 15-char checksum. Tradeoff: long code carries more
/// payload per chunk but at higher transcription burden.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkCode {
    /// Regular codex32 code: 13-character checksum, 48-byte single-string capacity.
    Regular,
    /// Long codex32 code: 15-character checksum, 56-byte single-string capacity.
    Long,
}

impl ChunkCode {
    /// Single-string maximum bytecode payload (no cross-chunk hash overhead).
    pub const fn single_string_capacity(self) -> usize {
        match self {
            Self::Regular => 48,
            Self::Long => 56,
        }
    }

    /// Per-chunk fragment capacity (used when chunking).
    pub const fn fragment_capacity(self) -> usize {
        match self {
            Self::Regular => 45,
            Self::Long => 53,
        }
    }
}

impl From<ChunkCode> for crate::BchCode {
    /// Convert a [`ChunkCode`] to its [`crate::BchCode`] equivalent.
    ///
    /// Both enums have parallel `Regular` and `Long` variants. This `From`
    /// impl lets encode-pipeline code write `let bch: BchCode = code.into()`
    /// instead of a manual `match`. The helper `chunk_code_to_bch_code` in
    /// `encode.rs` is now redundant and could be removed in a follow-up.
    fn from(code: ChunkCode) -> Self {
        match code {
            ChunkCode::Regular => crate::BchCode::Regular,
            ChunkCode::Long => crate::BchCode::Long,
        }
    }
}

// ---------------------------------------------------------------------------
// ChunkingMode
// ---------------------------------------------------------------------------

/// Chunking-mode selector. Replaces the previous `force_chunked: bool`
/// parameter on [`chunking_decision`] for self-documenting call sites.
///
/// The selector is also exposed via [`crate::EncodeOptions::chunking_mode`];
/// the [`crate::EncodeOptions::with_force_chunking`] builder method translates
/// `bool → ChunkingMode` for source-compatibility with v0.1.1 callers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkingMode {
    /// Single-string when bytecode fits; chunked otherwise.
    Auto,
    /// Force chunked encoding even when single-string would fit.
    ForceChunked,
}

impl Default for ChunkingMode {
    /// Default is [`ChunkingMode::Auto`] — single-string when bytecode fits.
    fn default() -> Self {
        Self::Auto
    }
}

// ---------------------------------------------------------------------------
// ChunkingPlan
// ---------------------------------------------------------------------------

/// Result of the chunking decision: how the bytecode will be encoded.
///
/// Produced by [`chunking_decision`]. Tells the encoder whether to emit one
/// codex32 string or split across multiple chunks.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkingPlan {
    /// Encode as a single string. The bytecode plus the chunk header is
    /// short enough to fit one codex32 string at the chosen code size.
    SingleString {
        /// The BCH code size to use.
        code: ChunkCode,
    },

    /// Encode as `count` chunks of at most `fragment_size` bytes each at
    /// the chosen code size. The cross-chunk SHA-256[0..4] hash is appended
    /// to the bytecode before splitting.
    ///
    /// `fragment_size` is the **maximum** per-chunk fragment; the last chunk
    /// may be shorter. The actual division is performed by `chunk_bytes`
    /// (Task 4-E).
    Chunked {
        /// The BCH code size to use for every chunk.
        code: ChunkCode,
        /// Maximum bytes per chunk fragment (≤ `code.fragment_capacity()`).
        fragment_size: usize,
        /// Total number of chunks (1–32; in practice ≥ 2 unless [`ChunkingMode::ForceChunked`]).
        count: usize,
    },
}

// ---------------------------------------------------------------------------
// chunking_decision
// ---------------------------------------------------------------------------

/// Decide how to encode `bytecode_len` bytes of canonical bytecode.
///
/// # Selection rules
///
/// 1. If `mode` is [`ChunkingMode::Auto`] and `bytecode_len ≤ 48` (regular
///    single-string capacity), return `SingleString { Regular }`.
/// 2. Else if `mode` is [`ChunkingMode::Auto`] and `bytecode_len ≤ 56` (long
///    single-string capacity), return `SingleString { Long }`.
/// 3. Otherwise (chunked path): the byte stream is `bytecode_len + 4` (the
///    4-byte cross-chunk SHA-256 hash is appended before splitting).
///    - Try **Regular** first: `count = ⌈(bytecode_len + 4) / 45⌉`.
///      If `count ≤ 32`, return `Chunked { Regular, 45, count }`.
///    - Else try **Long**: `count = ⌈(bytecode_len + 4) / 53⌉`.
///      If `count ≤ 32`, return `Chunked { Long, 53, count }`.
///    - Else return [`Error::PolicyTooLarge`] with `max_supported =`
///      [`MAX_BYTECODE_LEN`] (= 32 × 53 − 4 = 1692).
///
/// The [`ChunkingMode::ForceChunked`] mode (BIP line 438) lets encoders chunk
/// even small bytecodes, e.g. to fit on physical media. When forced, the
/// single-string checks in steps 1–2 are skipped; selection within the
/// chunked path is unchanged (Regular preferred, Long fallback).
///
/// Note: when [`ChunkingMode::ForceChunked`] is used, this function still
/// prefers Regular over Long (matching the [`ChunkingMode::Auto`] behavior);
/// the BIP is silent on this preference.
///
/// ## Notes
///
/// Note: when `EncodeOptions::force_long_code` is set, the top-level
/// `encode()` function post-processes the returned plan to swap Regular
/// → Long. See `crates/md-codec/src/encode.rs::encode` Stage 3.
///
/// # Errors
///
/// Returns [`Error::PolicyTooLarge`] when `bytecode_len` exceeds [`MAX_BYTECODE_LEN`].
pub fn chunking_decision(bytecode_len: usize, mode: ChunkingMode) -> Result<ChunkingPlan> {
    // Steps 1 & 2: single-string path (skipped when ForceChunked).
    // Exhaustive match so a future ChunkingMode variant forces a compile-time
    // decision here rather than silently falling through to the chunked path.
    match mode {
        ChunkingMode::Auto => {
            if bytecode_len <= ChunkCode::Regular.single_string_capacity() {
                return Ok(ChunkingPlan::SingleString {
                    code: ChunkCode::Regular,
                });
            }
            if bytecode_len <= ChunkCode::Long.single_string_capacity() {
                return Ok(ChunkingPlan::SingleString {
                    code: ChunkCode::Long,
                });
            }
        }
        ChunkingMode::ForceChunked => {}
    }

    // Steps 3 & 4: chunked path.
    // The cross-chunk hash adds 4 bytes to the byte stream before splitting.
    let stream_len = bytecode_len + 4;

    // Step 3: try Regular.
    let regular_cap = ChunkCode::Regular.fragment_capacity(); // 45
    let regular_count = stream_len.div_ceil(regular_cap);
    if regular_count <= MAX_CHUNK_COUNT as usize {
        return Ok(ChunkingPlan::Chunked {
            code: ChunkCode::Regular,
            fragment_size: regular_cap,
            count: regular_count,
        });
    }

    // Step 4: try Long.
    let long_cap = ChunkCode::Long.fragment_capacity(); // 53
    let long_count = stream_len.div_ceil(long_cap);
    if long_count <= MAX_CHUNK_COUNT as usize {
        return Ok(ChunkingPlan::Chunked {
            code: ChunkCode::Long,
            fragment_size: long_cap,
            count: long_count,
        });
    }

    // Step 5: too large.
    Err(Error::PolicyTooLarge {
        bytecode_len,
        max_supported: MAX_BYTECODE_LEN,
    })
}

// ---------------------------------------------------------------------------
// Chunk
// ---------------------------------------------------------------------------

/// One assembled chunk: a parsed header plus its raw fragment bytes.
///
/// The wire form is `header.to_bytes() ++ fragment` (header first, then
/// fragment).  For `SingleString` this is 2 header bytes + fragment; for
/// `Chunked` this is 7 header bytes + fragment.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    /// The parsed chunk header (version, type, chunk-set-id, count, index).
    pub header: ChunkHeader,
    /// The raw fragment payload bytes for this chunk.
    pub fragment: Vec<u8>,
}

impl Chunk {
    /// Serialize to a contiguous byte buffer: `header_bytes ++ fragment_bytes`.
    ///
    /// For a `SingleString` chunk the result is `[ver, type=0] ++ fragment`;
    /// for a `Chunked` chunk it is `[ver, type=1, wid_hi, wid_mid, wid_lo,
    /// count, index] ++ fragment`.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = self.header.to_bytes();
        out.extend_from_slice(&self.fragment);
        out
    }

    /// Construct a `Chunk` from a header and fragment.
    ///
    /// No validation is performed on the header or fragment; the caller is
    /// responsible for ensuring the header is well-formed (e.g. as produced by
    /// [`ChunkHeader::from_bytes`]) and that the fragment length is consistent
    /// with the intended encoding plan.
    pub fn new(header: ChunkHeader, fragment: Vec<u8>) -> Self {
        Self { header, fragment }
    }

    /// Parse a `Chunk` from the start of `bytes`.
    ///
    /// Returns the parsed `Chunk` and the total number of bytes consumed
    /// (header length + fragment length).  The caller is responsible for
    /// determining where the fragment ends (i.e. the total byte buffer must
    /// contain exactly one complete chunk, or the caller must slice
    /// appropriately).
    ///
    /// # Errors
    ///
    /// Propagates all errors from [`ChunkHeader::from_bytes`].  If header
    /// parsing succeeds, the remainder of `bytes` is taken as the fragment.
    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, usize)> {
        let (header, header_len) = ChunkHeader::from_bytes(bytes)?;
        let fragment = bytes[header_len..].to_vec();
        let consumed = header_len + fragment.len();
        Ok((Chunk { header, fragment }, consumed))
    }
}

// ---------------------------------------------------------------------------
// EncodedChunk
// ---------------------------------------------------------------------------

/// One chunk of a chunked Template Card backup, ready to engrave.
///
/// `raw` is the codex32-derived string (e.g. `md10x...`) including HRP,
/// type byte, header fields, fragment, and BCH checksum. `chunk_index`
/// and `total_chunks` are extracted from the parsed header for caller
/// convenience. `code` indicates whether the BCH checksum is regular
/// (13 chars) or long (15 chars).
///
/// For type=0 (single-string) backups, `chunk_index = 0` and
/// `total_chunks = 1`.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedChunk {
    /// The full codex32 string (HRP + data + checksum), ready to engrave.
    pub raw: String,
    /// Zero-based index of this chunk within the sequence.
    pub chunk_index: u8,
    /// Total number of chunks in this sequence.
    pub total_chunks: u8,
    /// Whether the BCH checksum uses the regular (13 char) or long (15 char) code.
    pub code: crate::BchCode,
}

// ---------------------------------------------------------------------------
// Correction
// ---------------------------------------------------------------------------

/// One BCH error correction applied during decode.
///
/// Reported in `DecodeReport.corrections` so callers can surface
/// "we fixed your transcription error at chunk 1 char 17" to users.
///
/// `original` is the character the user transcribed; `corrected`
/// is what the BCH decoder computed. `char_position` is 0-indexed
/// within the chunk's data part (after the HRP+separator).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Correction {
    /// Zero-based index of the chunk where the correction was applied.
    pub chunk_index: u8,
    /// 0-indexed position within the chunk's data part (after HRP+separator).
    pub char_position: usize,
    /// The character the user transcribed (erroneous input).
    pub original: char,
    /// The character the BCH decoder computed (corrected value).
    pub corrected: char,
}

// ---------------------------------------------------------------------------
// chunk_bytes
// ---------------------------------------------------------------------------

/// Split canonical bytecode into chunks per the given plan.
///
/// # Behaviour
///
/// - **`SingleString` plan**: verifies the bytecode fits within the code's
///   single-string capacity, then returns a single [`Chunk`] whose fragment
///   is the bytecode verbatim.
/// - **`Chunked` plan**: computes a 4-byte cross-chunk SHA-256 integrity hash
///   (`SHA-256(bytecode)[0..4]`), appends it to the bytecode to form the
///   *stream*, splits the stream into `count` fragments of at most
///   `fragment_size` bytes each (the last fragment may be shorter), and wraps
///   each in a [`Chunk`] with the appropriate header fields.
///
/// All chunked chunks share the same `chunk_set_id`, `count`, and incremented
/// `index` (0-based).
///
/// # Errors
///
/// Returns [`Error::PolicyTooLarge`] if the bytecode (plus 4-byte hash for
/// chunked plans) exceeds the plan's total capacity.  This is a defensive
/// check; callers who used [`chunking_decision`] to obtain the plan will not
/// hit this error unless they pass a longer bytecode than the one used for the
/// decision.
pub fn chunk_bytes(
    canonical_bytecode: &[u8],
    plan: ChunkingPlan,
    chunk_set_id: ChunkSetId,
) -> Result<Vec<Chunk>> {
    match plan {
        ChunkingPlan::SingleString { code } => {
            if canonical_bytecode.len() > code.single_string_capacity() {
                return Err(Error::PolicyTooLarge {
                    bytecode_len: canonical_bytecode.len(),
                    max_supported: code.single_string_capacity(),
                });
            }
            Ok(vec![Chunk {
                header: ChunkHeader::SingleString { version: 0 },
                fragment: canonical_bytecode.to_vec(),
            }])
        }
        ChunkingPlan::Chunked {
            fragment_size,
            count,
            ..
        } => {
            // Compute the 4-byte cross-chunk integrity hash.
            let hash = sha256::Hash::hash(canonical_bytecode);
            let hash_bytes = &hash.as_byte_array()[..4];

            // Build the stream: bytecode ++ 4-byte hash.
            let mut stream = canonical_bytecode.to_vec();
            stream.extend_from_slice(hash_bytes);

            // Defensive capacity check: report plan-specific bytecode capacity.
            // Plan total capacity is count * fragment_size bytes of stream,
            // which holds bytecode + 4-byte hash, so max bytecode = capacity - 4.
            let total_capacity = count * fragment_size;
            if stream.len() > total_capacity {
                let plan_bytecode_capacity = total_capacity - 4;
                return Err(Error::PolicyTooLarge {
                    bytecode_len: canonical_bytecode.len(),
                    max_supported: plan_bytecode_capacity,
                });
            }

            let count_u8: u8 = count
                .try_into()
                .expect("plan validated count <= MAX_CHUNK_COUNT (32)");

            let mut chunks = Vec::with_capacity(count);
            for i in 0..count {
                let start = i * fragment_size;
                let end = ((i + 1) * fragment_size).min(stream.len());
                let fragment = stream[start..end].to_vec();
                let index_u8: u8 = i.try_into().expect("plan validated count <= 32, so i < 32");
                let header = ChunkHeader::Chunked {
                    version: 0,
                    chunk_set_id,
                    count: count_u8,
                    index: index_u8,
                };
                chunks.push(Chunk { header, fragment });
            }

            debug_assert_eq!(
                chunks.iter().map(|c| c.fragment.len()).sum::<usize>(),
                stream.len(),
                "fragment bytes must equal stream length"
            );

            Ok(chunks)
        }
    }
}

// ---------------------------------------------------------------------------
// reassemble_chunks
// ---------------------------------------------------------------------------

/// Reassemble a list of parsed [`Chunk`]s into the original canonical bytecode.
///
/// # Validation steps (BIP §"Reassembly")
///
/// 1. Chunk list must be non-empty.
/// 2. All chunks must be the same type (no mixing of `SingleString` and `Chunked`).
/// 3. For `SingleString`: list must contain exactly one chunk; its fragment IS
///    the bytecode.
/// 4. For `Chunked` chunks:
///    a. All chunks share the same `chunk_set_id`.
///    b. All chunks declare the same `count`.
///    c. No duplicate `index` values.
///    d. No missing indexes in `0..count`.
///    e. Each index is in range (`0 ≤ index < count`).
///    f. The trailing 4-byte cross-chunk SHA-256 hash must match `SHA-256(bytecode)[0..4]`.
///
/// Chunks may be supplied in any order; they are sorted by index internally.
///
/// # Errors
///
/// Returns one of the following on failure:
/// [`Error::EmptyChunkList`], [`Error::MixedChunkTypes`],
/// [`Error::SingleStringWithMultipleChunks`], [`Error::ChunkSetIdMismatch`],
/// [`Error::TotalChunksMismatch`], [`Error::ChunkIndexOutOfRange`],
/// [`Error::DuplicateChunkIndex`], [`Error::MissingChunkIndex`],
/// [`Error::CrossChunkHashMismatch`].
pub fn reassemble_chunks(chunks: Vec<Chunk>) -> Result<Vec<u8>> {
    if chunks.is_empty() {
        return Err(Error::EmptyChunkList);
    }

    // Peek at the first header to determine the chunk type (without consuming).
    let is_single = matches!(chunks[0].header, ChunkHeader::SingleString { .. });

    if is_single {
        // Validate that ALL chunks are SingleString.
        if chunks
            .iter()
            .any(|c| !matches!(c.header, ChunkHeader::SingleString { .. }))
        {
            return Err(Error::MixedChunkTypes);
        }
        if chunks.len() > 1 {
            return Err(Error::SingleStringWithMultipleChunks);
        }
        // The fragment IS the canonical bytecode — move it out.
        return Ok(chunks.into_iter().next().unwrap().fragment);
    }

    // Chunked path: extract expected chunk_set_id and count from first chunk.
    let (expected_chunk_set_id, expected_count) = match &chunks[0].header {
        ChunkHeader::Chunked {
            chunk_set_id,
            count,
            ..
        } => (*chunk_set_id, *count),
        ChunkHeader::SingleString { .. } => unreachable!("handled above"),
    };

    // Build an index-keyed map, validating and moving each chunk's fragment.
    let mut by_index: HashMap<u8, Vec<u8>> = HashMap::new();
    for chunk in chunks.into_iter() {
        let Chunk { header, fragment } = chunk;
        match header {
            ChunkHeader::SingleString { .. } => {
                return Err(Error::MixedChunkTypes);
            }
            ChunkHeader::Chunked {
                chunk_set_id,
                count,
                index,
                ..
            } => {
                if chunk_set_id != expected_chunk_set_id {
                    return Err(Error::ChunkSetIdMismatch {
                        expected: expected_chunk_set_id,
                        got: chunk_set_id,
                    });
                }
                if count != expected_count {
                    return Err(Error::TotalChunksMismatch {
                        expected: expected_count,
                        got: count,
                    });
                }
                if index >= expected_count {
                    return Err(Error::ChunkIndexOutOfRange {
                        index,
                        total: expected_count,
                    });
                }
                if by_index.insert(index, fragment).is_some() {
                    return Err(Error::DuplicateChunkIndex(index));
                }
            }
        }
    }

    // Verify no indexes are missing (report the lowest missing index).
    for i in 0..expected_count {
        if !by_index.contains_key(&i) {
            return Err(Error::MissingChunkIndex(i));
        }
    }

    // Move fragments out in order into the stream (no clone — HashMap::remove moves).
    let mut stream = Vec::new();
    for i in 0..expected_count {
        stream.append(&mut by_index.remove(&i).expect("checked above"));
    }

    // Split off the trailing 4-byte cross-chunk hash.
    if stream.len() < 4 {
        return Err(Error::CrossChunkHashMismatch);
    }
    let split_at = stream.len() - 4;
    let bytecode = stream[..split_at].to_vec();
    let claimed_hash = &stream[split_at..];
    let computed = sha256::Hash::hash(&bytecode);
    let computed_hash = &computed.as_byte_array()[..4];

    if claimed_hash != computed_hash {
        return Err(Error::CrossChunkHashMismatch);
    }

    Ok(bytecode)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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
            chunk_set_id: ChunkSetId::new(0),
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
    fn chunked_round_trip_max_chunk_set_id() {
        // ChunkSetId::MAX = 0xF_FFFF; encodes as [0x0F, 0xFF, 0xFF].
        let hdr = ChunkHeader::Chunked {
            version: 0,
            chunk_set_id: ChunkSetId::new(ChunkSetId::MAX),
            count: 4,
            index: 0,
        };
        let bytes = hdr.to_bytes();
        // chunk_set_id bytes: [(0xFFFFF >> 16)=0x0F, (0xFFFFF >> 8) & 0xFF=0xFF, 0xFF & 0xFF=0xFF]
        assert_eq!(bytes[2..5], [0x0F, 0xFF, 0xFF]);
        let (decoded, consumed) = ChunkHeader::from_bytes(&bytes).unwrap();
        assert_eq!(decoded, hdr);
        assert_eq!(consumed, 7);
    }

    #[test]
    fn chunked_round_trip_max_count_and_index() {
        let hdr = ChunkHeader::Chunked {
            version: 0,
            chunk_set_id: ChunkSetId::new(0x1234),
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
                chunk_set_id: ChunkSetId::new(0),
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
                chunk_set_id: ChunkSetId::new(0),
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
        // [ver=0, type=1, csid=0x00,0x00,0x00, count=0, index=0]
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
    fn reject_policy_id_top_bits_set() {
        // chunk_set_id first byte = 0x10 → bit 20 set (top nibble non-zero).
        // Construct raw bytes without going through ChunkSetId::new (which panics).
        let bytes = [0x00u8, 0x01, 0x10, 0x00, 0x00, 0x01, 0x00];
        let err = ChunkHeader::from_bytes(&bytes).unwrap_err();
        assert!(
            matches!(err, Error::ReservedChunkSetIdBitsSet),
            "expected ReservedChunkSetIdBitsSet, got {err:?}"
        );
    }

    #[test]
    fn reject_truncated_input_single() {
        // Only 1 byte — too short for the 2-byte SingleString header.
        let err = ChunkHeader::from_bytes(&[0x00]).unwrap_err();
        assert!(
            matches!(err, Error::ChunkHeaderTruncated { have: 1, need: 2 }),
            "expected ChunkHeaderTruncated {{ have: 1, need: 2 }}, got {err:?}"
        );
    }

    #[test]
    fn reject_truncated_input_empty() {
        let err = ChunkHeader::from_bytes(&[]).unwrap_err();
        assert!(
            matches!(err, Error::ChunkHeaderTruncated { have: 0, need: 2 }),
            "expected ChunkHeaderTruncated {{ have: 0, need: 2 }}, got {err:?}"
        );
    }

    #[test]
    fn reject_truncated_chunked_header() {
        // type=1 but only 3 bytes — too short for the 7-byte Chunked header.
        let err = ChunkHeader::from_bytes(&[0x00, 0x01, 0x00]).unwrap_err();
        assert!(
            matches!(err, Error::ChunkHeaderTruncated { have: 3, need: 7 }),
            "expected ChunkHeaderTruncated {{ have: 3, need: 7 }}, got {err:?}"
        );
    }

    #[test]
    fn reject_truncated_chunked_two_bytes() {
        // [0x00, 0x01]: version + type=Chunked, but no chunk_set_id/count/index.
        let err = ChunkHeader::from_bytes(&[0x00, 0x01]).unwrap_err();
        assert!(
            matches!(err, Error::ChunkHeaderTruncated { have: 2, need: 7 }),
            "expected ChunkHeaderTruncated {{ have: 2, need: 7 }}, got {err:?}"
        );
    }

    #[test]
    fn reject_truncated_chunked_five_bytes() {
        // [0x00, 0x01, 0x00, 0x00, 0x00]: version + type + 3 chunk_set_id bytes, but no count/index.
        let err = ChunkHeader::from_bytes(&[0x00, 0x01, 0x00, 0x00, 0x00]).unwrap_err();
        assert!(
            matches!(err, Error::ChunkHeaderTruncated { have: 5, need: 7 }),
            "expected ChunkHeaderTruncated {{ have: 5, need: 7 }}, got {err:?}"
        );
    }

    // -----------------------------------------------------------------------
    // ChunkCode capacity constants
    // -----------------------------------------------------------------------

    #[test]
    fn chunk_code_capacity_constants() {
        assert_eq!(ChunkCode::Regular.single_string_capacity(), 48);
        assert_eq!(ChunkCode::Long.single_string_capacity(), 56);
        assert_eq!(ChunkCode::Regular.fragment_capacity(), 45);
        assert_eq!(ChunkCode::Long.fragment_capacity(), 53);
    }

    // -----------------------------------------------------------------------
    // chunking_decision tests
    // -----------------------------------------------------------------------

    #[test]
    fn chunking_decision_zero_byte_input() {
        // 0-byte bytecode, unforced → SingleString { Regular } (fits within 48-byte capacity).
        let plan = chunking_decision(0, ChunkingMode::Auto).unwrap();
        assert_eq!(
            plan,
            ChunkingPlan::SingleString {
                code: ChunkCode::Regular
            }
        );

        // 0-byte bytecode, forced → Chunked { Regular, 45, 1 }:
        // stream = 0 + 4 = 4; count = ceil(4/45) = 1.
        let plan_forced = chunking_decision(0, ChunkingMode::ForceChunked).unwrap();
        assert_eq!(
            plan_forced,
            ChunkingPlan::Chunked {
                code: ChunkCode::Regular,
                fragment_size: 45,
                count: 1,
            }
        );
    }

    #[test]
    fn single_string_long_explicit_49_bytes() {
        // 49 bytes exceeds Regular capacity (48) but fits Long (56) → SingleString { Long }.
        let plan = chunking_decision(49, ChunkingMode::Auto).unwrap();
        assert_eq!(
            plan,
            ChunkingPlan::SingleString {
                code: ChunkCode::Long
            }
        );
    }

    #[test]
    fn single_string_short_input() {
        // Small bytecode → SingleString { Regular }.
        let plan = chunking_decision(10, ChunkingMode::Auto).unwrap();
        assert_eq!(
            plan,
            ChunkingPlan::SingleString {
                code: ChunkCode::Regular
            }
        );
    }

    #[test]
    fn single_string_regular_at_boundary() {
        // At the regular capacity boundary: 48 → Regular; 49 → falls through.
        let at = chunking_decision(48, ChunkingMode::Auto).unwrap();
        assert_eq!(
            at,
            ChunkingPlan::SingleString {
                code: ChunkCode::Regular
            }
        );

        // 49 bytes is over regular single-string but still fits long single-string.
        let over = chunking_decision(49, ChunkingMode::Auto).unwrap();
        assert_eq!(
            over,
            ChunkingPlan::SingleString {
                code: ChunkCode::Long
            },
            "49 bytes should return SingleString {{ Long }}"
        );
    }

    #[test]
    fn single_string_long_at_boundary() {
        // At the long capacity boundary: 56 → Long; 57 → chunked path.
        let at = chunking_decision(56, ChunkingMode::Auto).unwrap();
        assert_eq!(
            at,
            ChunkingPlan::SingleString {
                code: ChunkCode::Long
            }
        );

        // 57 bytes exceeds both single-string capacities → must be a Chunked plan.
        let over = chunking_decision(57, ChunkingMode::Auto).unwrap();
        assert!(
            matches!(over, ChunkingPlan::Chunked { .. }),
            "57 bytes should return Chunked, got {over:?}"
        );
    }

    #[test]
    fn chunking_mode_force_chunked_skips_single_string() {
        // ChunkingMode::ForceChunked with a short bytecode → Chunked, not SingleString.
        // count = ceil((10 + 4) / 45) = ceil(14/45) = 1.
        let plan = chunking_decision(10, ChunkingMode::ForceChunked).unwrap();
        assert_eq!(
            plan,
            ChunkingPlan::Chunked {
                code: ChunkCode::Regular,
                fragment_size: 45,
                count: 1,
            }
        );
    }

    #[test]
    fn chunked_regular_minimal() {
        // 57 bytes is just over long single-string capacity.
        // stream = 57 + 4 = 61; count = ceil(61/45) = 2.
        let plan = chunking_decision(57, ChunkingMode::Auto).unwrap();
        assert_eq!(
            plan,
            ChunkingPlan::Chunked {
                code: ChunkCode::Regular,
                fragment_size: 45,
                count: 2,
            }
        );
    }

    #[test]
    fn chunked_regular_at_max_chunks() {
        // Exactly 32 regular chunks: stream = 32 * 45 = 1440, bytecode_len = 1436.
        // count = ceil(1440/45) = 32.
        let plan = chunking_decision(1436, ChunkingMode::Auto).unwrap();
        assert_eq!(
            plan,
            ChunkingPlan::Chunked {
                code: ChunkCode::Regular,
                fragment_size: 45,
                count: 32,
            }
        );
    }

    #[test]
    fn chunked_falls_through_to_long_at_regular_overflow() {
        // 1437 + 4 = 1441 > 1440 (32*45), so regular needs 33 chunks → overflow.
        // long: count = ceil(1441/53) = ceil(1441/53) = 28.
        // 1441 / 53 = 27.188... → ceil = 28.
        let plan = chunking_decision(1437, ChunkingMode::Auto).unwrap();
        assert_eq!(
            plan,
            ChunkingPlan::Chunked {
                code: ChunkCode::Long,
                fragment_size: 53,
                count: 28,
            }
        );
    }

    #[test]
    fn chunked_long_at_max_chunks() {
        // Exactly 32 long chunks: stream = 32 * 53 = 1696, bytecode_len = 1692.
        // count = ceil(1696/53) = 32.
        let plan = chunking_decision(1692, ChunkingMode::Auto).unwrap();
        assert_eq!(
            plan,
            ChunkingPlan::Chunked {
                code: ChunkCode::Long,
                fragment_size: 53,
                count: 32,
            }
        );
    }

    #[test]
    fn reject_too_large() {
        // 1693 bytes: stream = 1697 > 1696 (32*53). Must return PolicyTooLarge.
        let err = chunking_decision(1693, ChunkingMode::Auto).unwrap_err();
        assert!(
            matches!(
                err,
                Error::PolicyTooLarge {
                    bytecode_len: 1693,
                    max_supported: 1692,
                }
            ),
            "expected PolicyTooLarge {{ bytecode_len: 1693, max_supported: 1692 }}, got {err:?}"
        );
    }

    #[test]
    fn chunking_mode_force_chunked_at_max() {
        // ChunkingMode::ForceChunked at bytecode_len=1692 → same long-32 plan.
        let plan = chunking_decision(1692, ChunkingMode::ForceChunked).unwrap();
        assert_eq!(
            plan,
            ChunkingPlan::Chunked {
                code: ChunkCode::Long,
                fragment_size: 53,
                count: 32,
            }
        );
    }

    #[test]
    fn chunking_mode_force_chunked_too_large() {
        // ChunkingMode::ForceChunked at bytecode_len=1693 → PolicyTooLarge.
        let err = chunking_decision(1693, ChunkingMode::ForceChunked).unwrap_err();
        assert!(
            matches!(
                err,
                Error::PolicyTooLarge {
                    bytecode_len: 1693,
                    max_supported: MAX_BYTECODE_LEN,
                }
            ),
            "expected PolicyTooLarge {{ bytecode_len: 1693, max_supported: {MAX_BYTECODE_LEN} }}, got {err:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Chunk::to_bytes / from_bytes tests
    // -----------------------------------------------------------------------

    #[test]
    fn chunk_to_bytes_single_string() {
        let chunk = Chunk {
            header: ChunkHeader::SingleString { version: 0 },
            fragment: vec![0x05, 0x33],
        };
        assert_eq!(chunk.to_bytes(), [0x00, 0x00, 0x05, 0x33]);
    }

    #[test]
    fn chunk_to_bytes_chunked() {
        // chunk_set_id = 0x00001 (stored as [0x00, 0x00, 0x01]), count=3, index=1.
        let chunk = Chunk {
            header: ChunkHeader::Chunked {
                version: 0,
                chunk_set_id: ChunkSetId::new(0x00001),
                count: 3,
                index: 1,
            },
            fragment: vec![0xAA, 0xBB],
        };
        // Expected: [ver=0, type=1, wid_hi=0x00, wid_mid=0x00, wid_lo=0x01, count=3, index=1, 0xAA, 0xBB]
        assert_eq!(
            chunk.to_bytes(),
            [0x00, 0x01, 0x00, 0x00, 0x01, 0x03, 0x01, 0xAA, 0xBB]
        );
    }

    #[test]
    fn chunk_round_trip_single_string() {
        let original = Chunk {
            header: ChunkHeader::SingleString { version: 0 },
            fragment: vec![0x01, 0x02, 0x03],
        };
        let bytes = original.to_bytes();
        let (decoded, consumed) = Chunk::from_bytes(&bytes).unwrap();
        assert_eq!(decoded, original);
        assert_eq!(consumed, bytes.len());
    }

    #[test]
    fn chunk_round_trip_chunked() {
        let original = Chunk {
            header: ChunkHeader::Chunked {
                version: 0,
                chunk_set_id: ChunkSetId::new(0xABCDE),
                count: 5,
                index: 2,
            },
            fragment: vec![0x10, 0x20, 0x30, 0x40],
        };
        let bytes = original.to_bytes();
        let (decoded, consumed) = Chunk::from_bytes(&bytes).unwrap();
        assert_eq!(decoded, original);
        assert_eq!(consumed, bytes.len());
    }

    #[test]
    fn chunk_from_bytes_consumed_count() {
        // SingleString: 2-byte header + 3-byte fragment = 5 consumed.
        let chunk = Chunk {
            header: ChunkHeader::SingleString { version: 0 },
            fragment: vec![0xAA, 0xBB, 0xCC],
        };
        let bytes = chunk.to_bytes();
        let (_, consumed) = Chunk::from_bytes(&bytes).unwrap();
        assert_eq!(consumed, 5); // 2 header + 3 fragment

        // Chunked: 7-byte header + 2-byte fragment = 9 consumed.
        let chunk2 = Chunk {
            header: ChunkHeader::Chunked {
                version: 0,
                chunk_set_id: ChunkSetId::new(0),
                count: 1,
                index: 0,
            },
            fragment: vec![0x01, 0x02],
        };
        let bytes2 = chunk2.to_bytes();
        let (_, consumed2) = Chunk::from_bytes(&bytes2).unwrap();
        assert_eq!(consumed2, 9); // 7 header + 2 fragment
    }

    // -----------------------------------------------------------------------
    // chunk_bytes tests
    // -----------------------------------------------------------------------

    fn test_chunk_set_id() -> ChunkSetId {
        ChunkSetId::new(0x12345)
    }

    #[test]
    fn chunk_bytes_single_string() {
        let bytecode = vec![0x01u8, 0x02, 0x03];
        let plan = ChunkingPlan::SingleString {
            code: ChunkCode::Regular,
        };
        let chunks = chunk_bytes(&bytecode, plan, test_chunk_set_id()).unwrap();
        assert_eq!(chunks.len(), 1);
        assert!(matches!(
            chunks[0].header,
            ChunkHeader::SingleString { version: 0 }
        ));
        assert_eq!(chunks[0].fragment, bytecode);
    }

    #[test]
    fn chunk_bytes_chunked_typical() {
        // 100-byte input, plan = Chunked { Regular, 45, 3 }.
        // stream = 100 + 4 = 104 bytes; split into 45, 45, 14.
        let bytecode: Vec<u8> = (0u8..100).collect();
        let plan = ChunkingPlan::Chunked {
            code: ChunkCode::Regular,
            fragment_size: 45,
            count: 3,
        };
        let csid = test_chunk_set_id();
        let chunks = chunk_bytes(&bytecode, plan, csid).unwrap();
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].fragment.len(), 45);
        assert_eq!(chunks[1].fragment.len(), 45);
        assert_eq!(chunks[2].fragment.len(), 14); // 104 - 90
        for (i, chunk) in chunks.iter().enumerate() {
            match chunk.header {
                ChunkHeader::Chunked {
                    chunk_set_id,
                    count,
                    index,
                    ..
                } => {
                    assert_eq!(chunk_set_id, csid);
                    assert_eq!(count, 3);
                    assert_eq!(index, i as u8);
                }
                _ => panic!("expected Chunked header"),
            }
        }
    }

    #[test]
    fn chunk_bytes_appends_cross_chunk_hash() {
        // Verify that the last 4 bytes of the concatenated stream equal
        // SHA-256(bytecode)[0..4].
        let bytecode: Vec<u8> = (0u8..50).collect();
        let plan = ChunkingPlan::Chunked {
            code: ChunkCode::Regular,
            fragment_size: 45,
            count: 2,
        };
        let chunks = chunk_bytes(&bytecode, plan, test_chunk_set_id()).unwrap();

        // Concatenate all fragments.
        let mut stream = Vec::new();
        for c in &chunks {
            stream.extend_from_slice(&c.fragment);
        }

        // The last 4 bytes must be SHA-256(bytecode)[0..4].
        let hash = sha256::Hash::hash(&bytecode);
        let expected_hash = &hash.as_byte_array()[..4];
        let claimed = &stream[stream.len() - 4..];
        assert_eq!(claimed, expected_hash);
    }

    #[test]
    fn chunk_bytes_chunking_mode_force_chunked_minimal() {
        // 5-byte input, plan = Chunked { Regular, 45, 1 }.
        // stream = 5 + 4 = 9 bytes; single fragment of 9 bytes.
        let bytecode = vec![0x01u8, 0x02, 0x03, 0x04, 0x05];
        let plan = ChunkingPlan::Chunked {
            code: ChunkCode::Regular,
            fragment_size: 45,
            count: 1,
        };
        let chunks = chunk_bytes(&bytecode, plan, test_chunk_set_id()).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].fragment.len(), 9); // 5 bytes + 4-byte hash
    }

    #[test]
    fn chunk_bytes_too_large_for_plan() {
        // 60 bytes won't fit a SingleString { Regular } plan (capacity = 48).
        let bytecode: Vec<u8> = vec![0u8; 60];
        let plan = ChunkingPlan::SingleString {
            code: ChunkCode::Regular,
        };
        let err = chunk_bytes(&bytecode, plan, test_chunk_set_id()).unwrap_err();
        assert!(
            matches!(
                err,
                Error::PolicyTooLarge {
                    bytecode_len: 60,
                    ..
                }
            ),
            "expected PolicyTooLarge, got {err:?}"
        );
    }

    // -----------------------------------------------------------------------
    // reassemble_chunks tests
    // -----------------------------------------------------------------------

    #[test]
    fn reassemble_single_string() {
        let bytecode = vec![0x01u8, 0x02, 0x03];
        let plan = ChunkingPlan::SingleString {
            code: ChunkCode::Regular,
        };
        let chunks = chunk_bytes(&bytecode, plan, test_chunk_set_id()).unwrap();
        let recovered = reassemble_chunks(chunks).unwrap();
        assert_eq!(recovered, bytecode);
    }

    #[test]
    fn reassemble_chunked_typical() {
        let bytecode: Vec<u8> = (0u8..100).collect();
        let plan = ChunkingPlan::Chunked {
            code: ChunkCode::Regular,
            fragment_size: 45,
            count: 3,
        };
        let chunks = chunk_bytes(&bytecode, plan, test_chunk_set_id()).unwrap();
        let recovered = reassemble_chunks(chunks).unwrap();
        assert_eq!(recovered, bytecode);
    }

    #[test]
    fn reassemble_chunked_out_of_order() {
        let bytecode: Vec<u8> = (0u8..100).collect();
        let plan = ChunkingPlan::Chunked {
            code: ChunkCode::Regular,
            fragment_size: 45,
            count: 3,
        };
        let mut chunks = chunk_bytes(&bytecode, plan, test_chunk_set_id()).unwrap();
        chunks.reverse(); // put in reverse order
        let recovered = reassemble_chunks(chunks).unwrap();
        assert_eq!(recovered, bytecode);
    }

    #[test]
    fn reassemble_empty_input() {
        let err = reassemble_chunks(vec![]).unwrap_err();
        assert!(
            matches!(err, Error::EmptyChunkList),
            "expected EmptyChunkList, got {err:?}"
        );
    }

    #[test]
    fn reassemble_single_string_with_multiple() {
        let ss_chunk = Chunk {
            header: ChunkHeader::SingleString { version: 0 },
            fragment: vec![0x01, 0x02],
        };
        let err = reassemble_chunks(vec![ss_chunk.clone(), ss_chunk]).unwrap_err();
        assert!(
            matches!(err, Error::SingleStringWithMultipleChunks),
            "expected SingleStringWithMultipleChunks, got {err:?}"
        );
    }

    #[test]
    fn reassemble_mixed_types() {
        let ss = Chunk {
            header: ChunkHeader::SingleString { version: 0 },
            fragment: vec![0x01],
        };
        let chunked = Chunk {
            header: ChunkHeader::Chunked {
                version: 0,
                chunk_set_id: ChunkSetId::new(0),
                count: 2,
                index: 0,
            },
            fragment: vec![0x02],
        };
        // First chunk is SingleString, second is Chunked → MixedChunkTypes.
        let err = reassemble_chunks(vec![ss, chunked]).unwrap_err();
        assert!(
            matches!(err, Error::MixedChunkTypes),
            "expected MixedChunkTypes, got {err:?}"
        );
    }

    #[test]
    fn reassemble_chunk_set_id_mismatch() {
        let csid_a = ChunkSetId::new(0xAAAAA);
        let csid_b = ChunkSetId::new(0xBBBBB);
        let c0 = Chunk {
            header: ChunkHeader::Chunked {
                version: 0,
                chunk_set_id: csid_a,
                count: 2,
                index: 0,
            },
            fragment: vec![0x01, 0x02, 0x03, 0x04, 0x05],
        };
        let c1 = Chunk {
            header: ChunkHeader::Chunked {
                version: 0,
                chunk_set_id: csid_b,
                count: 2,
                index: 1,
            },
            fragment: vec![0x06, 0x07, 0x08, 0x09],
        };
        let err = reassemble_chunks(vec![c0, c1]).unwrap_err();
        assert!(
            matches!(
                err,
                Error::ChunkSetIdMismatch {
                    expected,
                    got,
                } if expected == csid_a && got == csid_b
            ),
            "expected ChunkSetIdMismatch, got {err:?}"
        );
    }

    #[test]
    fn reassemble_count_mismatch() {
        let csid = test_chunk_set_id();
        let c0 = Chunk {
            header: ChunkHeader::Chunked {
                version: 0,
                chunk_set_id: csid,
                count: 2,
                index: 0,
            },
            fragment: vec![0x01],
        };
        let c1 = Chunk {
            header: ChunkHeader::Chunked {
                version: 0,
                chunk_set_id: csid,
                count: 3, // mismatch: claims 3 chunks, first said 2
                index: 1,
            },
            fragment: vec![0x02],
        };
        let err = reassemble_chunks(vec![c0, c1]).unwrap_err();
        assert!(
            matches!(
                err,
                Error::TotalChunksMismatch {
                    expected: 2,
                    got: 3,
                }
            ),
            "expected TotalChunksMismatch, got {err:?}"
        );
    }

    #[test]
    fn reassemble_duplicate_index() {
        let csid = test_chunk_set_id();
        let c0 = Chunk {
            header: ChunkHeader::Chunked {
                version: 0,
                chunk_set_id: csid,
                count: 2,
                index: 0,
            },
            fragment: vec![0x01],
        };
        let c1 = Chunk {
            header: ChunkHeader::Chunked {
                version: 0,
                chunk_set_id: csid,
                count: 2,
                index: 0, // duplicate
            },
            fragment: vec![0x02],
        };
        let err = reassemble_chunks(vec![c0, c1]).unwrap_err();
        assert!(
            matches!(err, Error::DuplicateChunkIndex(0)),
            "expected DuplicateChunkIndex(0), got {err:?}"
        );
    }

    #[test]
    fn reassemble_missing_index() {
        // count = 3, but only indices 0 and 2 are present; index 1 is missing.
        let csid = test_chunk_set_id();
        let c0 = Chunk {
            header: ChunkHeader::Chunked {
                version: 0,
                chunk_set_id: csid,
                count: 3,
                index: 0,
            },
            fragment: vec![0x01],
        };
        let c2 = Chunk {
            header: ChunkHeader::Chunked {
                version: 0,
                chunk_set_id: csid,
                count: 3,
                index: 2,
            },
            fragment: vec![0x03],
        };
        let err = reassemble_chunks(vec![c0, c2]).unwrap_err();
        assert!(
            matches!(err, Error::MissingChunkIndex(1)),
            "expected MissingChunkIndex(1), got {err:?}"
        );
    }

    #[test]
    fn reassemble_index_out_of_range() {
        // Construct a Chunk manually with index >= count.
        // ChunkHeader::from_bytes would reject this, but reassemble_chunks
        // must also catch it when chunks are crafted directly.
        let csid = test_chunk_set_id();
        // count=2 but index=2 (out of range for count=2).
        // We can't use ChunkHeader directly with index >= count since from_bytes
        // validates that; we build the struct directly.
        let c_bad = Chunk {
            header: ChunkHeader::Chunked {
                version: 0,
                chunk_set_id: csid,
                count: 2,
                index: 5, // out of range: 5 >= 2
            },
            fragment: vec![0x01],
        };
        let err = reassemble_chunks(vec![c_bad]).unwrap_err();
        assert!(
            matches!(err, Error::ChunkIndexOutOfRange { index: 5, total: 2 }),
            "expected ChunkIndexOutOfRange, got {err:?}"
        );
    }

    #[test]
    fn reassemble_cross_chunk_hash_mismatch() {
        // Build a valid chunked encoding, then corrupt one byte in the last
        // fragment and verify that reassemble detects the hash mismatch.
        let bytecode: Vec<u8> = (0u8..50).collect();
        let plan = ChunkingPlan::Chunked {
            code: ChunkCode::Regular,
            fragment_size: 45,
            count: 2,
        };
        let mut chunks = chunk_bytes(&bytecode, plan, test_chunk_set_id()).unwrap();
        // Corrupt the first byte of the last fragment.
        let last = chunks.last_mut().unwrap();
        last.fragment[0] ^= 0xFF;

        let err = reassemble_chunks(chunks).unwrap_err();
        assert!(
            matches!(err, Error::CrossChunkHashMismatch),
            "expected CrossChunkHashMismatch, got {err:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Chunk::new constructor tests (Item 3)
    // -----------------------------------------------------------------------

    #[test]
    fn chunk_new_constructor_matches_struct_literal() {
        let header = ChunkHeader::SingleString { version: 0 };
        let fragment = vec![0x05, 0x33];
        let from_new = Chunk::new(header, fragment.clone());
        let from_lit = Chunk {
            header: ChunkHeader::SingleString { version: 0 },
            fragment,
        };
        assert_eq!(from_new, from_lit);
    }

    // -----------------------------------------------------------------------
    // chunk_bytes capacity error tests (Item 5)
    // -----------------------------------------------------------------------

    #[test]
    fn chunk_bytes_too_large_for_single_string_plan_reports_plan_capacity() {
        // 60 bytes won't fit a SingleString { Regular } plan (capacity = 48).
        let bytecode = vec![0u8; 60];
        let plan = ChunkingPlan::SingleString {
            code: ChunkCode::Regular,
        };
        let err = chunk_bytes(&bytecode, plan, test_chunk_set_id()).unwrap_err();
        assert!(
            matches!(
                err,
                Error::PolicyTooLarge {
                    bytecode_len: 60,
                    max_supported: 48,
                }
            ),
            "expected PolicyTooLarge {{ bytecode_len: 60, max_supported: 48 }}, got {err:?}"
        );
    }

    #[test]
    fn chunk_bytes_too_large_for_chunked_plan_reports_plan_capacity() {
        // Chunked { Regular, fragment_size=45, count=2 } → stream capacity = 2*45 = 90 bytes,
        // so max bytecode = 90 - 4 = 86 bytes. Pass 200 bytes → Err with max_supported: 86.
        let bytecode = vec![0u8; 200];
        let plan = ChunkingPlan::Chunked {
            code: ChunkCode::Regular,
            fragment_size: 45,
            count: 2,
        };
        let err = chunk_bytes(&bytecode, plan, test_chunk_set_id()).unwrap_err();
        assert!(
            matches!(
                err,
                Error::PolicyTooLarge {
                    bytecode_len: 200,
                    max_supported: 86,
                }
            ),
            "expected PolicyTooLarge {{ bytecode_len: 200, max_supported: 86 }}, got {err:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Hash-byte corruption test (Item 6)
    // -----------------------------------------------------------------------

    #[test]
    fn reassemble_cross_chunk_hash_mismatch_with_corrupted_hash_byte() {
        // Stream layout: bytecode (50 bytes) ++ hash (4 bytes) = 54 bytes.
        // fragment[0] = stream[0..45] (45 bytes, all payload).
        // fragment[1] = stream[45..54] (9 bytes: 5 payload + 4 hash).
        // Corrupting fragment[1][last] = stream[53] corrupts the last hash byte.
        let bytecode = vec![0xCCu8; 50];
        let plan = ChunkingPlan::Chunked {
            code: ChunkCode::Regular,
            fragment_size: 45,
            count: 2,
        };
        let chunk_set_id = ChunkSetId::new(0xABC);
        let mut chunks = chunk_bytes(&bytecode, plan, chunk_set_id).expect("encode");

        // Corrupt the LAST byte of the last fragment (a hash byte).
        let last = chunks.last_mut().unwrap();
        let last_idx = last.fragment.len() - 1;
        last.fragment[last_idx] ^= 0xFF;

        let err = reassemble_chunks(chunks).expect_err("expected hash mismatch");
        assert!(
            matches!(err, Error::CrossChunkHashMismatch),
            "expected CrossChunkHashMismatch, got {err:?}"
        );
    }

    // --- EncodedChunk ---

    #[test]
    fn encoded_chunk_round_trip_via_struct_construction() {
        let chunk = EncodedChunk {
            raw: "md10xtest".to_string(),
            chunk_index: 0,
            total_chunks: 1,
            code: crate::BchCode::Regular,
        };
        assert_eq!(chunk.raw, "md10xtest");
        assert_eq!(chunk.chunk_index, 0);
        assert_eq!(chunk.total_chunks, 1);
        assert_eq!(chunk.code, crate::BchCode::Regular);
    }

    // --- Correction ---

    #[test]
    fn correction_struct_construction() {
        let correction = Correction {
            chunk_index: 1,
            char_position: 17,
            original: 'a',
            corrected: 'z',
        };
        assert_eq!(correction.chunk_index, 1);
        assert_eq!(correction.char_position, 17);
        assert_eq!(correction.original, 'a');
        assert_eq!(correction.corrected, 'z');
    }

    // -----------------------------------------------------------------------
    // From<ChunkCode> for BchCode
    // -----------------------------------------------------------------------

    #[test]
    fn chunk_code_converts_to_bch_code() {
        use crate::BchCode;
        assert_eq!(BchCode::from(ChunkCode::Regular), BchCode::Regular);
        assert_eq!(BchCode::from(ChunkCode::Long), BchCode::Long);
    }
}
