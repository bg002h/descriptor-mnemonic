//! Wallet-identity types used across WDM chunking.
//!
//! This module provides four types:
//! - [`WalletId`] — 16-byte Tier-3 Wallet ID (first 16 bytes of SHA-256 of
//!   canonical bytecode).
//! - [`WalletIdWords`] — the 12 BIP-39 words derived deterministically from a
//!   `WalletId`.
//! - [`ChunkWalletId`] — the 20-bit chunk-header field derived from a
//!   `WalletId` by taking its first 20 bits.
//! - [`WalletIdSeed`] — optional 4-byte seed to override the chunk-header
//!   `wallet_id` field during encoding.

use bitcoin::hashes::{Hash, sha256};
use std::fmt;

// ---------------------------------------------------------------------------
// WalletId
// ---------------------------------------------------------------------------

/// A 16-byte Wallet ID formed by taking the first 16 bytes of the SHA-256
/// hash of the wallet's canonical WDM bytecode (the "Tier-3" Wallet ID).
///
/// The full 128 bits serve as a collision-resistant identifier for the wallet;
/// the BIP-39 encoding ([`WalletIdWords`]) gives a human-friendly 12-word
/// form, and [`ChunkWalletId`] extracts the 20 most-significant bits for use
/// in chunk headers.
///
/// # Two-WalletId story
///
/// WDM uses **two distinct wallet identifiers** with different override
/// semantics. This `WalletId` is the **content-derived** Tier-3 identifier,
/// always equal to `SHA-256(canonical_bytecode)[0..16]`. It is **never**
/// affected by [`WalletIdSeed`] or [`crate::EncodeOptions::wallet_id_seed`].
/// In contrast, the 20-bit [`ChunkWalletId`] embedded in chunk headers can be
/// overridden by [`WalletIdSeed`] for deterministic test-vector generation.
///
/// The relationship is:
///
/// ```text
/// default ChunkWalletId  =  WalletId.truncate()       // first 20 bits of SHA-256
/// override ChunkWalletId =  WalletIdSeed.truncate()   // top 20 bits of seed
/// ```
///
/// A user holding only the 12-word [`WalletIdWords`] form of this `WalletId`
/// can verify which seed corresponds to which `@i` placeholder in their
/// recovered wallet policy. See `IMPLEMENTATION_PLAN_v0.1.md` §4
/// "Wallet ID semantics" and the BIP draft §"Wallet identifier".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WalletId([u8; 16]);

impl WalletId {
    /// Construct a `WalletId` from a raw 16-byte array.
    pub fn new(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Borrow the underlying 16 bytes as a fixed-size array reference.
    ///
    /// Use this when you need a typed `&[u8; 16]` (for example, to copy into
    /// another fixed-size array without a length-checked panic). The
    /// [`AsRef<[u8]>`][AsRef] impl returns a length-erased slice; `as_bytes`
    /// is the typed accessor.
    ///
    /// ```
    /// use md_codec::WalletId;
    /// let id = WalletId::from([0xAB; 16]);
    /// let bytes: &[u8; 16] = id.as_bytes();
    /// assert_eq!(bytes[0], 0xAB);
    /// assert_eq!(bytes.len(), 16);
    /// ```
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// Encode the wallet ID as a space-separated list of 12 BIP-39 words.
    ///
    /// The words are derived deterministically from the 128-bit value using
    /// the standard BIP-39 algorithm for 128-bit entropy.  The same
    /// `WalletId` always produces the same `WalletIdWords`.
    pub fn to_words(&self) -> WalletIdWords {
        // 16 bytes = 128-bit entropy, which is always a valid BIP-39 input
        // (standard lengths are 16/20/24/28/32 bytes).
        let mnemonic = bip39::Mnemonic::from_entropy(&self.0)
            .expect("128-bit entropy is always a valid BIP-39 mnemonic input");

        // Collect the 12 words into a fixed-size array.
        let mut words: [String; 12] = Default::default();
        for (slot, word) in words.iter_mut().zip(mnemonic.words()) {
            *slot = word.to_string();
        }
        WalletIdWords(words)
    }

    /// Extract the first 20 bits of the wallet ID for use in chunk headers.
    ///
    /// Bit-packing convention (big-endian / MSB-first):
    /// ```text
    /// result = (byte[0] as u32) << 12
    ///        | (byte[1] as u32) <<  4
    ///        | (byte[2] as u32) >>  4   ← top nibble of byte[2] only
    /// ```
    /// This preserves the significance ordering of the underlying SHA-256
    /// output; the top 20 bits of the 128-bit value appear as the 20
    /// least-significant bits of the returned `ChunkWalletId`.
    pub fn truncate(&self) -> ChunkWalletId {
        let b = &self.0;
        let bits = ((b[0] as u32) << 12) | ((b[1] as u32) << 4) | ((b[2] as u32) >> 4);
        // bits is at most 0xF_FFFF because the upper 12 bits of the u32 are
        // always zero (we shift b[0] by 12, so max contribution is 0xFF << 12
        // = 0x000F_F000, plus 0xFF << 4 = 0x0000_0FF0, plus 0x0F = 0x0000_000F
        // → max = 0x000F_FFFF = ChunkWalletId::MAX).
        ChunkWalletId(bits)
    }
}

impl fmt::Display for WalletId {
    /// Formats the wallet ID as 32 lowercase hexadecimal characters with no
    /// separator (e.g. `"ab12cd34ef56789012345678abcdef01"`).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::LowerHex for WalletId {
    /// Same output as [`std::fmt::Display`] — 32 lowercase hex digits, no separator.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl AsRef<[u8]> for WalletId {
    /// Returns a reference to the underlying 16-byte array.
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; 16]> for WalletId {
    /// Construct a `WalletId` from a raw 16-byte array.
    fn from(bytes: [u8; 16]) -> Self {
        Self::new(bytes)
    }
}

// ---------------------------------------------------------------------------
// compute_wallet_id
// ---------------------------------------------------------------------------

/// Compute a 16-byte [`WalletId`] by truncating the SHA-256 of canonical
/// bytecode.
///
/// # Algorithm
///
/// ```text
/// SHA-256(canonical_bytecode)[0..16]
/// ```
///
/// The first 16 bytes of the 32-byte SHA-256 digest are used directly as the
/// `WalletId` (128 bits).  This is the Tier-3 Wallet ID defined in the WDM
/// spec (IMPLEMENTATION_PLAN §3, line 106).
///
/// The relationship to the chunk-header 20-bit field is:
/// ```text
/// ChunkWalletId = WalletId::truncate() = first 20 bits of SHA-256(bytecode)
/// ```
/// i.e. the `WalletId` and the chunk-header field share the same SHA-256 hash;
/// the chunk-header ([`WalletId::truncate`]) simply keeps fewer bits.
///
/// # Phase note
///
/// This is the bytes-level primitive.  Phase 5 will add a
/// `WalletPolicy`-aware wrapper (`compute_wallet_id_for_policy`) that
/// canonicalizes a `WalletPolicy` to bytecode and then calls this function.
///
/// # Example
///
/// ```
/// # use md_codec::wallet_id::compute_wallet_id;
/// let id = compute_wallet_id(b"");
/// // SHA-256("") = e3b0c44298fc1c149afbf4c8996fb924 27ae41e4649b934ca495991b7852b855
/// //              └─────── first 16 bytes ────────┘
/// assert_eq!(id.to_string(), "e3b0c44298fc1c149afbf4c8996fb924");
/// ```
pub fn compute_wallet_id(canonical_bytecode: &[u8]) -> WalletId {
    let digest = sha256::Hash::hash(canonical_bytecode);
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest.as_byte_array()[..16]);
    WalletId::from(bytes)
}

/// Compute a [`WalletId`] for a `WalletPolicy` by first encoding it to
/// canonical WDM bytecode, then applying [`compute_wallet_id`].
///
/// This is the `WalletPolicy`-aware wrapper specified in Task 5-B.
/// The name `compute_wallet_id_for_policy` is used (rather than an overload
/// of `compute_wallet_id`) because Rust does not support function overloading.
/// See PHASE_5_DECISIONS.md D-9.
pub fn compute_wallet_id_for_policy(
    policy: &crate::WalletPolicy,
) -> Result<WalletId, crate::Error> {
    let bytecode = policy.to_bytecode(&crate::EncodeOptions::default())?;
    Ok(compute_wallet_id(&bytecode))
}

// ---------------------------------------------------------------------------
// WalletIdWords
// ---------------------------------------------------------------------------

/// The 12 BIP-39 words that encode a [`WalletId`].
///
/// Derived deterministically via [`WalletId::to_words`].  The words are
/// all-lowercase English BIP-39 vocabulary, space-joined when displayed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalletIdWords([String; 12]);

impl WalletIdWords {
    /// Borrow the underlying 12-word array.
    pub fn as_slice(&self) -> &[String; 12] {
        &self.0
    }
}

impl fmt::Display for WalletIdWords {
    /// Formats the words as a single space-separated string with no leading
    /// or trailing space (e.g. `"abandon ability able about ..."`).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for word in &self.0 {
            if first {
                first = false;
            } else {
                f.write_str(" ")?;
            }
            f.write_str(word)?;
        }
        Ok(())
    }
}

impl IntoIterator for WalletIdWords {
    type Item = String;
    type IntoIter = std::array::IntoIter<String, 12>;

    /// Consumes `self` and yields the 12 BIP-39 words in order.
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

// ---------------------------------------------------------------------------
// ChunkWalletId
// ---------------------------------------------------------------------------

/// A 20-bit wallet-identity field carried in each WDM chunk header.
///
/// Derived from a [`WalletId`] via [`WalletId::truncate`], which extracts the
/// first 20 bits (MSB-first) of the underlying 16-byte SHA-256 prefix.
///
/// The upper 12 bits of the inner `u32` are always zero.  Construct via
/// [`ChunkWalletId::new`]; direct tuple-struct access is intentionally private.
///
/// # Why 20 bits, and how it relates to [`WalletId`]
///
/// Each chunk in a chunked WDM backup carries this 20-bit field in its
/// 7-byte header so that a decoder can verify that all chunks belong to
/// the same wallet **before** any BCH-corrected fragment bytes are
/// concatenated. 20 bits gives ~1-in-1M cross-wallet collision resistance,
/// adequate for engraving misfile detection while keeping the chunk header
/// compact.
///
/// By default, `ChunkWalletId == WalletId::truncate()`, so a user who knows
/// the 12-word [`WalletIdWords`] of their Tier-3 [`WalletId`] can predict
/// what the chunk-header field SHOULD be and confirm it matches at decode
/// time. This binding is the crux of WDM's "verify the recovery without
/// access to the original media" property.
///
/// The binding can be broken on purpose by passing a [`WalletIdSeed`] in
/// [`crate::EncodeOptions::wallet_id_seed`]; this is used by the test-vector
/// generator to fix the chunk-header bits to a known value independent of
/// the bytecode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkWalletId(u32);

impl ChunkWalletId {
    /// The maximum value a `ChunkWalletId` may hold: 2²⁰ − 1 = `0xF_FFFF`.
    pub const MAX: u32 = (1 << 20) - 1;

    /// Construct a `ChunkWalletId` from a 20-bit value.
    ///
    /// # Panics
    ///
    /// Panics if `bits > Self::MAX` (i.e., if any of the upper 12 bits of
    /// `bits` are set).
    pub fn new(bits: u32) -> Self {
        assert!(
            bits <= Self::MAX,
            "ChunkWalletId value {bits:#x} exceeds 20-bit maximum ({:#x})",
            Self::MAX
        );
        Self(bits)
    }

    /// Returns the 20-bit value as a `u32` with the upper 12 bits guaranteed
    /// to be zero.
    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

// ---------------------------------------------------------------------------
// WalletIdSeed
// ---------------------------------------------------------------------------

/// Optional 4-byte seed that, when supplied via
/// [`crate::EncodeOptions::wallet_id_seed`], overrides the chunk-header
/// `wallet_id` field.
///
/// The Tier-3 16-byte [`WalletId`] is *always* content-derived and is NOT
/// affected by this seed (per `IMPLEMENTATION_PLAN_v0.1.md` §4 "Wallet ID
/// semantics" and the BIP draft §"Wallet identifier"). The seed is only used
/// to override the 20-bit [`ChunkWalletId`] embedded in chunk headers.
///
/// # When to use this
///
/// Production encoders should leave [`crate::EncodeOptions::wallet_id_seed`]
/// at `None` so that the chunk-header field is content-derived from the
/// canonical bytecode and a holder of the Tier-3 mnemonic can verify it.
///
/// Set this seed only for:
/// - **Deterministic test-vector generation** — fixing the chunk-header
///   bits to a known value across implementations.
/// - **Synthetic conformance tests** — exercising the chunk-header parser
///   with arbitrary 20-bit values without recomputing SHA-256.
///
/// Setting this seed in production breaks the "Tier-3 mnemonic predicts the
/// chunk-header bits" property and is therefore a footgun for end users.
///
/// # Debug redaction
///
/// `Debug` deliberately redacts the byte contents (printing
/// `WalletIdSeed(<redacted>)`) so log spew cannot accidentally leak
/// the seed.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct WalletIdSeed([u8; 4]);

impl WalletIdSeed {
    /// Get the underlying 4 bytes.
    pub fn as_bytes(&self) -> &[u8; 4] {
        &self.0
    }

    /// Return the seed as a 32-bit big-endian unsigned integer.
    pub fn as_u32(&self) -> u32 {
        u32::from_be_bytes(self.0)
    }

    /// Truncate this seed to a 20-bit [`ChunkWalletId`].
    ///
    /// Takes the high 20 bits of the u32 view (matches
    /// [`WalletId::truncate`]'s big-endian-first-20-bits convention):
    /// ```text
    /// result = self.as_u32() >> 12
    /// ```
    /// This yields the top 20 bits of the 32-bit seed.
    pub fn truncate(&self) -> ChunkWalletId {
        let bits = self.as_u32() >> 12;
        ChunkWalletId::new(bits)
    }
}

impl fmt::Debug for WalletIdSeed {
    /// Redacts the raw bytes to prevent accidental logging of seed material.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WalletIdSeed(<redacted>)")
    }
}

impl From<u32> for WalletIdSeed {
    /// Construct from a `u32` using big-endian byte order (high byte first).
    fn from(n: u32) -> Self {
        Self(n.to_be_bytes())
    }
}

impl From<[u8; 4]> for WalletIdSeed {
    /// Construct directly from a 4-byte array.
    fn from(bytes: [u8; 4]) -> Self {
        Self(bytes)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- WalletId display / formatting ---

    #[test]
    fn wallet_id_display_is_hex() {
        let id = WalletId::from([0xABu8; 16]);
        let s = id.to_string();
        assert_eq!(
            s, "abababababababababababababababab",
            "expected 32 lowercase hex chars"
        );
        // LowerHex should produce the same output.
        assert_eq!(format!("{:x}", id), s);
    }

    #[test]
    fn wallet_id_as_ref_returns_underlying_bytes() {
        let bytes = [
            0x01u8, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
            0x0F, 0x10,
        ];
        let id = WalletId::from(bytes);
        assert_eq!(id.as_ref(), &bytes);
    }

    // --- WalletId::truncate ---

    #[test]
    fn wallet_id_truncate_takes_first_20_bits() {
        // All-zeros: result should be 0.
        let zero = WalletId::from([0u8; 16]);
        assert_eq!(zero.truncate().as_u32(), 0);

        // All-0xFF: all 20 bits set → 0xF_FFFF.
        let all_ff = WalletId::from([0xFFu8; 16]);
        assert_eq!(all_ff.truncate().as_u32(), 0xF_FFFF);

        // Mixed: bytes = [0x12, 0x34, 0x56, ...]
        // bit-pack: (0x12 << 12) | (0x34 << 4) | (0x56 >> 4)
        //         = 0x12_000 | 0x340 | 0x5
        //         = 0x12345
        let mut mixed = [0u8; 16];
        mixed[0] = 0x12;
        mixed[1] = 0x34;
        mixed[2] = 0x56;
        let id = WalletId::from(mixed);
        let expected = ((0x12u32) << 12) | ((0x34u32) << 4) | ((0x56u32) >> 4);
        assert_eq!(id.truncate().as_u32(), expected);
        assert_eq!(id.truncate().as_u32(), 0x12345);
    }

    // --- WalletId::to_words determinism ---

    #[test]
    fn wallet_id_to_words_deterministic() {
        let id = WalletId::from([0x42u8; 16]);
        let w1 = id.to_words();
        let w2 = id.to_words();
        assert_eq!(w1, w2, "to_words must be deterministic");
    }

    #[test]
    fn wallet_id_to_words_yields_12_distinct_words_for_typical_input() {
        // Use a non-repeating byte sequence to ensure 12 distinct BIP-39 words.
        // ([0xAB; 16] happens to produce only 9 distinct words; a varied input
        // avoids repeated entropy patterns that collapse into repeated words.)
        let bytes = [
            0x01u8, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54,
            0x32, 0x10,
        ];
        let id = WalletId::from(bytes);
        let words: Vec<String> = id.to_words().into_iter().collect();
        assert_eq!(words.len(), 12, "expected 12 words");
        let unique: std::collections::HashSet<&String> = words.iter().collect();
        assert_eq!(
            unique.len(),
            12,
            "expected 12 distinct words for varied input"
        );
    }

    // --- WalletIdWords display & iterator ---

    #[test]
    fn wallet_id_words_display_is_space_joined() {
        let id = WalletId::from([0x00u8; 16]);
        let words = id.to_words();
        let s = words.to_string();
        // No leading or trailing space.
        assert!(!s.starts_with(' '), "no leading space: {s:?}");
        assert!(!s.ends_with(' '), "no trailing space: {s:?}");
        // Exactly 11 internal spaces (12 words → 11 separators).
        assert_eq!(
            s.chars().filter(|&c| c == ' ').count(),
            11,
            "expected 11 spaces for 12 words: {s:?}"
        );
    }

    #[test]
    fn wallet_id_words_intoiterator_yields_12() {
        let id = WalletId::from([0xDEu8; 16]);
        let words: Vec<String> = id.to_words().into_iter().collect();
        assert_eq!(words.len(), 12);
        for (i, word) in words.iter().enumerate() {
            assert!(!word.is_empty(), "word {i} is empty");
            // BIP-39 English words are all lowercase ASCII.
            assert!(
                word.chars().all(|c| c.is_ascii_lowercase()),
                "word {i} ({word:?}) contains non-lowercase-ASCII characters"
            );
        }
    }

    #[test]
    fn wallet_id_words_as_slice_yields_12_strings() {
        let id = WalletId::from([0x55u8; 16]);
        let words = id.to_words();
        let slice = words.as_slice();
        assert_eq!(slice.len(), 12, "as_slice must return exactly 12 words");
        for (i, word) in slice.iter().enumerate() {
            assert!(!word.is_empty(), "word {i} is empty");
        }
        // Borrowing via as_slice does not consume `words`.
        assert_eq!(words.as_slice().len(), 12, "as_slice is re-borrowable");
    }

    // --- compute_wallet_id ---

    #[test]
    fn compute_wallet_id_known_input() {
        // SHA-256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        // First 16 bytes as hex: e3b0c44298fc1c149afbf4c8996fb924
        let id = compute_wallet_id(b"");
        assert_eq!(
            id.to_string(),
            "e3b0c44298fc1c149afbf4c8996fb924",
            "SHA-256(\"\") truncated to 16 bytes should match known vector"
        );
    }

    #[test]
    fn compute_wallet_id_deterministic() {
        // Same input must always produce the same WalletId.
        let id1 = compute_wallet_id(b"hello world");
        let id2 = compute_wallet_id(b"hello world");
        assert_eq!(id1, id2, "compute_wallet_id must be deterministic");
    }

    #[test]
    fn compute_wallet_id_distinguishes_inputs() {
        // Different inputs must produce different WalletIds.
        let id_a = compute_wallet_id(b"a");
        let id_b = compute_wallet_id(b"b");
        assert_ne!(id_a, id_b, "distinct inputs must yield distinct WalletIds");
    }

    #[test]
    fn compute_wallet_id_truncate_is_first_20_bits() {
        // SHA-256("") = e3b0c44298fc1c149afbf4c8996fb924...
        // First 3 bytes: e3, b0, c4
        // First 20 bits: (0xe3 << 12) | (0xb0 << 4) | (0xc4 >> 4)
        //              = 0xe3000 | 0xb00 | 0xc
        //              = 0xe3b0c
        let id = compute_wallet_id(b"");
        let chunk_id = id.truncate();

        let expected: u32 = ((0xe3u32) << 12) | ((0xb0u32) << 4) | ((0xc4u32) >> 4);
        assert_eq!(expected, 0xe3b0c, "manual bit-pack sanity check");
        assert_eq!(
            chunk_id.as_u32(),
            0xe3b0c,
            "ChunkWalletId must equal the first 20 bits of SHA-256(bytecode)"
        );
    }

    #[test]
    fn compute_wallet_id_with_typical_bytecode_input() {
        // A plausible short canonical-bytecode prefix — must not panic and must
        // return a valid WalletId (no specific value assertion needed here).
        let fixture: &[u8] = &[0x05, 0x33, 0x01, 0x05, 0x1B, 0x32, 0x00];
        let id = compute_wallet_id(fixture);
        // 32-hex-char Display output sanity check.
        let s = id.to_string();
        assert_eq!(s.len(), 32, "WalletId Display must be 32 hex characters");
        assert!(
            s.chars().all(|c| c.is_ascii_hexdigit()),
            "WalletId Display must be lowercase hex: {s:?}"
        );
    }

    // --- ChunkWalletId ---

    #[test]
    fn chunk_wallet_id_new_accepts_zero_and_max() {
        assert_eq!(ChunkWalletId::new(0).as_u32(), 0);
        assert_eq!(ChunkWalletId::new(ChunkWalletId::MAX).as_u32(), 0xF_FFFF);
    }

    #[test]
    #[should_panic]
    fn chunk_wallet_id_new_panics_above_max() {
        ChunkWalletId::new(0x10_0000); // MAX + 1
    }

    #[test]
    fn chunk_wallet_id_max_is_20_bits() {
        assert_eq!(ChunkWalletId::MAX, 0xF_FFFF);
    }

    // --- WalletIdSeed ---

    #[test]
    fn wallet_id_seed_from_u32_is_big_endian() {
        let seed = WalletIdSeed::from(0x1234_5678u32);
        assert_eq!(seed.as_bytes(), &[0x12, 0x34, 0x56, 0x78]);
    }

    #[test]
    fn wallet_id_seed_from_bytes_round_trip() {
        let seed = WalletIdSeed::from([0xab; 4]);
        assert_eq!(seed.as_bytes(), &[0xab; 4]);
    }

    #[test]
    fn wallet_id_seed_as_u32_round_trip() {
        let seed = WalletIdSeed::from(0x1234_5678u32);
        assert_eq!(seed.as_u32(), 0x1234_5678);
    }

    #[test]
    fn wallet_id_seed_debug_redacts_bytes() {
        let s = format!("{:?}", WalletIdSeed::from(0xDEAD_BEEFu32));
        assert_eq!(s, "WalletIdSeed(<redacted>)");
        // Must not reveal any part of the raw bytes.
        assert!(!s.to_lowercase().contains("dead"));
        assert!(!s.to_lowercase().contains("beef"));
        assert!(!s.to_lowercase().contains("de"));
    }

    #[test]
    fn wallet_id_seed_truncate_takes_high_20_bits() {
        // 0x12345678 >> 12 = 0x12345
        let seed = WalletIdSeed::from(0x1234_5678u32);
        assert_eq!(seed.truncate().as_u32(), 0x12345);
    }
}
