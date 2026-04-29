//! Chunk-set-identity types used across MD chunking.
//!
//! This module provides four types:
//! - [`PolicyId`] — 16-byte Tier-3 Policy ID (first 16 bytes of SHA-256 of
//!   canonical bytecode).
//! - [`PolicyIdWords`] — the 12 BIP-39 words derived deterministically from a
//!   `PolicyId`.
//! - [`ChunkSetId`] — the 20-bit chunk-header field derived from a
//!   `PolicyId` by taking its first 20 bits.
//! - [`ChunkSetIdSeed`] — optional 4-byte seed to override the chunk-header
//!   `chunk_set_id` field during encoding.

use bitcoin::hashes::{Hash, sha256};
use std::fmt;

// ---------------------------------------------------------------------------
// PolicyId
// ---------------------------------------------------------------------------

/// A 16-byte Policy ID formed by taking the first 16 bytes of the SHA-256
/// hash of the wallet's canonical MD bytecode (the "Tier-3" Policy ID).
///
/// The full 128 bits serve as a collision-resistant identifier for the wallet;
/// the BIP-39 encoding ([`PolicyIdWords`]) gives a human-friendly 12-word
/// form, and [`ChunkSetId`] extracts the 20 most-significant bits for use
/// in chunk headers.
///
/// # Two-PolicyId story
///
/// MD uses **two distinct chunk-set identifiers** with different override
/// semantics. This `PolicyId` is the **content-derived** Tier-3 identifier,
/// always equal to `SHA-256(canonical_bytecode)[0..16]`. It is **never**
/// affected by [`ChunkSetIdSeed`] or [`crate::EncodeOptions::chunk_set_id_seed`].
/// In contrast, the 20-bit [`ChunkSetId`] embedded in chunk headers can be
/// overridden by [`ChunkSetIdSeed`] for deterministic test-vector generation.
///
/// The relationship is:
///
/// ```text
/// default ChunkSetId  =  PolicyId.truncate()       // first 20 bits of SHA-256
/// override ChunkSetId =  ChunkSetIdSeed.truncate()   // top 20 bits of seed
/// ```
///
/// A user holding only the 12-word [`PolicyIdWords`] form of this `PolicyId`
/// can verify which seed corresponds to which `@i` placeholder in their
/// recovered wallet policy. See `IMPLEMENTATION_PLAN_v0.1.md` §4
/// "Policy ID semantics" and the BIP draft §"Chunk-set identifier".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PolicyId([u8; 16]);

impl PolicyId {
    /// Construct a `PolicyId` from a raw 16-byte array.
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
    /// use md_codec::PolicyId;
    /// let id = PolicyId::from([0xAB; 16]);
    /// let bytes: &[u8; 16] = id.as_bytes();
    /// assert_eq!(bytes[0], 0xAB);
    /// assert_eq!(bytes.len(), 16);
    /// ```
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// Encode the policy ID as a space-separated list of 12 BIP-39 words.
    ///
    /// The words are derived deterministically from the 128-bit value using
    /// the standard BIP-39 algorithm for 128-bit entropy.  The same
    /// `PolicyId` always produces the same `PolicyIdWords`.
    pub fn to_words(&self) -> PolicyIdWords {
        // 16 bytes = 128-bit entropy, which is always a valid BIP-39 input
        // (standard lengths are 16/20/24/28/32 bytes).
        let mnemonic = bip39::Mnemonic::from_entropy(&self.0)
            .expect("128-bit entropy is always a valid BIP-39 mnemonic input");

        // Collect the 12 words into a fixed-size array.
        let mut words: [String; 12] = Default::default();
        for (slot, word) in words.iter_mut().zip(mnemonic.words()) {
            *slot = word.to_string();
        }
        PolicyIdWords(words)
    }

    /// Extract the first 20 bits of the policy ID for use in chunk headers.
    ///
    /// Bit-packing convention (big-endian / MSB-first):
    /// ```text
    /// result = (byte[0] as u32) << 12
    ///        | (byte[1] as u32) <<  4
    ///        | (byte[2] as u32) >>  4   ← top nibble of byte[2] only
    /// ```
    /// This preserves the significance ordering of the underlying SHA-256
    /// output; the top 20 bits of the 128-bit value appear as the 20
    /// least-significant bits of the returned `ChunkSetId`.
    pub fn truncate(&self) -> ChunkSetId {
        let b = &self.0;
        let bits = ((b[0] as u32) << 12) | ((b[1] as u32) << 4) | ((b[2] as u32) >> 4);
        // bits is at most 0xF_FFFF because the upper 12 bits of the u32 are
        // always zero (we shift b[0] by 12, so max contribution is 0xFF << 12
        // = 0x000F_F000, plus 0xFF << 4 = 0x0000_0FF0, plus 0x0F = 0x0000_000F
        // → max = 0x000F_FFFF = ChunkSetId::MAX).
        ChunkSetId(bits)
    }
}

impl fmt::Display for PolicyId {
    /// Formats the policy ID as 32 lowercase hexadecimal characters with no
    /// separator (e.g. `"ab12cd34ef56789012345678abcdef01"`).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::LowerHex for PolicyId {
    /// Same output as [`std::fmt::Display`] — 32 lowercase hex digits, no separator.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl AsRef<[u8]> for PolicyId {
    /// Returns a reference to the underlying 16-byte array.
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; 16]> for PolicyId {
    /// Construct a `PolicyId` from a raw 16-byte array.
    fn from(bytes: [u8; 16]) -> Self {
        Self::new(bytes)
    }
}

// ---------------------------------------------------------------------------
// compute_policy_id
// ---------------------------------------------------------------------------

/// Compute a 16-byte [`PolicyId`] by truncating the SHA-256 of canonical
/// bytecode.
///
/// # Algorithm
///
/// ```text
/// SHA-256(canonical_bytecode)[0..16]
/// ```
///
/// The first 16 bytes of the 32-byte SHA-256 digest are used directly as the
/// `PolicyId` (128 bits).  This is the Tier-3 Policy ID defined in the MD
/// spec (IMPLEMENTATION_PLAN §3, line 106).
///
/// The relationship to the chunk-header 20-bit field is:
/// ```text
/// ChunkSetId = PolicyId::truncate() = first 20 bits of SHA-256(bytecode)
/// ```
/// i.e. the `PolicyId` and the chunk-header field share the same SHA-256 hash;
/// the chunk-header ([`PolicyId::truncate`]) simply keeps fewer bits.
///
/// # Phase note
///
/// This is the bytes-level primitive.  Phase 5 will add a
/// `WalletPolicy`-aware wrapper (`compute_policy_id_for_policy`) that
/// canonicalizes a `WalletPolicy` to bytecode and then calls this function.
///
/// # Example
///
/// ```
/// # use md_codec::policy_id::compute_policy_id;
/// let id = compute_policy_id(b"");
/// // SHA-256("") = e3b0c44298fc1c149afbf4c8996fb924 27ae41e4649b934ca495991b7852b855
/// //              └─────── first 16 bytes ────────┘
/// assert_eq!(id.to_string(), "e3b0c44298fc1c149afbf4c8996fb924");
/// ```
pub fn compute_policy_id(canonical_bytecode: &[u8]) -> PolicyId {
    let digest = sha256::Hash::hash(canonical_bytecode);
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest.as_byte_array()[..16]);
    PolicyId::from(bytes)
}

/// Compute a [`PolicyId`] for a `WalletPolicy` by first encoding it to
/// canonical MD bytecode, then applying [`compute_policy_id`].
///
/// This is the `WalletPolicy`-aware wrapper specified in Task 5-B.
/// The name `compute_policy_id_for_policy` is used (rather than an overload
/// of `compute_policy_id`) because Rust does not support function overloading.
/// See PHASE_5_DECISIONS.md D-9.
pub fn compute_policy_id_for_policy(
    policy: &crate::WalletPolicy,
) -> Result<PolicyId, crate::Error> {
    let bytecode = policy.to_bytecode(&crate::EncodeOptions::default())?;
    Ok(compute_policy_id(&bytecode))
}

// ---------------------------------------------------------------------------
// WalletInstanceId (v0.8+)
// ---------------------------------------------------------------------------

/// 16-byte identifier for a specific wallet *instance* (template plus a
/// concrete cosigner-xpub set), as opposed to a [`PolicyId`] which
/// hashes the BIP 388 template only.
///
/// Two distinct wallets that share an identical policy template
/// (same multisig shape, same shared path, **different** cosigner sets)
/// share a `PolicyId` but have **different** `WalletInstanceId`s. The
/// `WalletInstanceId` is the cryptographic identifier that
/// distinguishes wallet instances; the `PolicyId` is a template-level
/// indexing aid.
///
/// Defined in v0.8 alongside the `WalletId` → `PolicyId` rename. See
/// the BIP draft §"Wallet Instance ID" for the canonical definition
/// and `design/FOLLOWUPS.md` `chunk-set-id-is-really-template-id` for
/// the rationale.
///
/// `WalletInstanceId` is **not** carried by any physical card or wire
/// structure — it is a recovery-time derivation. Tools that have the
/// policy card (template) plus the cosigner xpubs (whether from
/// engraved mk1 cards, a digital descriptor backup, or the wallet
/// itself) compute it on demand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WalletInstanceId([u8; 16]);

impl WalletInstanceId {
    /// Return the 16 bytes as a slice.
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

impl fmt::Display for WalletInstanceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in self.0.iter() {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}

impl fmt::LowerHex for WalletInstanceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl AsRef<[u8]> for WalletInstanceId {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; 16]> for WalletInstanceId {
    fn from(bytes: [u8; 16]) -> Self {
        WalletInstanceId(bytes)
    }
}

/// Compute the [`WalletInstanceId`] for a wallet given its canonical
/// MD bytecode plus the cosigner-xpub set, in placeholder-index order.
///
/// `xpubs[i]` is the BIP 32 [`bitcoin::bip32::Xpub`] that maps to the
/// `@i` placeholder in the policy template. The serialization is
/// canonical: each xpub is encoded in its full 78-byte BIP 32 form
/// (`Xpub::encode`), and the bytes are concatenated in placeholder
/// order (`@0`, then `@1`, then `@2`, ...).
///
/// Returns `SHA-256(canonical_bytecode || encode(xpubs[0]) || encode(xpubs[1]) || ...)[0..16]`.
///
/// # Examples
///
/// ```
/// use md_codec::compute_wallet_instance_id;
/// use std::str::FromStr;
///
/// let bytecode = b"\x00\x34\x03\x05\x08\x02\x02\x33\x00\x33\x01"; // wsh(multi(2,@0,@1)) v0.8 layout
/// let xpub_a = bitcoin::bip32::Xpub::from_str(
///     "xpub6Br37sWxruYfT8ASpCjVHKGwgdnYFEn98DwiN76i2oyY6fgH1LAPmmDcF46xjxJr22gw4jmVjTE2E3URMnRPEPYyo1zoPSUba563ESMXCeb",
/// ).unwrap();
/// let xpub_b = bitcoin::bip32::Xpub::from_str(
///     "xpub6FC1fXFP1GXLX5TKtcjHGT4q89SDRehkQLtbKJ2PzWcvbBHtyDsJPLtpLtkGqYNYZdVVAjRQ5kug9CsapegmmeRutpP7PW4u4wVF9JfkDhw",
/// ).unwrap();
/// let id = compute_wallet_instance_id(bytecode, &[xpub_a, xpub_b]);
/// assert_eq!(id.as_bytes().len(), 16);
/// ```
pub fn compute_wallet_instance_id(
    canonical_bytecode: &[u8],
    xpubs: &[bitcoin::bip32::Xpub],
) -> WalletInstanceId {
    use bitcoin::hashes::HashEngine;
    let mut engine = sha256::Hash::engine();
    engine.input(canonical_bytecode);
    for xpub in xpubs {
        engine.input(&xpub.encode());
    }
    let digest = sha256::Hash::from_engine(engine);
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest.as_byte_array()[..16]);
    WalletInstanceId::from(bytes)
}

// ---------------------------------------------------------------------------
// PolicyIdWords
// ---------------------------------------------------------------------------

/// The 12 BIP-39 words that encode a [`PolicyId`].
///
/// Derived deterministically via [`PolicyId::to_words`].  The words are
/// all-lowercase English BIP-39 vocabulary, space-joined when displayed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyIdWords([String; 12]);

impl PolicyIdWords {
    /// Borrow the underlying 12-word array.
    pub fn as_slice(&self) -> &[String; 12] {
        &self.0
    }
}

impl fmt::Display for PolicyIdWords {
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

impl IntoIterator for PolicyIdWords {
    type Item = String;
    type IntoIter = std::array::IntoIter<String, 12>;

    /// Consumes `self` and yields the 12 BIP-39 words in order.
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

// ---------------------------------------------------------------------------
// ChunkSetId
// ---------------------------------------------------------------------------

/// A 20-bit chunk-set-identity field carried in each MD chunk header.
///
/// Derived from a [`PolicyId`] via [`PolicyId::truncate`], which extracts the
/// first 20 bits (MSB-first) of the underlying 16-byte SHA-256 prefix.
///
/// The upper 12 bits of the inner `u32` are always zero.  Construct via
/// [`ChunkSetId::new`]; direct tuple-struct access is intentionally private.
///
/// # Why 20 bits, and how it relates to [`PolicyId`]
///
/// Each chunk in a chunked MD backup carries this 20-bit field in its
/// 7-byte header so that a decoder can verify that all chunks belong to
/// the same wallet **before** any BCH-corrected fragment bytes are
/// concatenated. 20 bits gives ~1-in-1M cross-wallet collision resistance,
/// adequate for engraving misfile detection while keeping the chunk header
/// compact.
///
/// By default, `ChunkSetId == PolicyId::truncate()`, so a user who knows
/// the 12-word [`PolicyIdWords`] of their Tier-3 [`PolicyId`] can predict
/// what the chunk-header field SHOULD be and confirm it matches at decode
/// time. This binding is the crux of MD's "verify the recovery without
/// access to the original media" property.
///
/// The binding can be broken on purpose by passing a [`ChunkSetIdSeed`] in
/// [`crate::EncodeOptions::chunk_set_id_seed`]; this is used by the test-vector
/// generator to fix the chunk-header bits to a known value independent of
/// the bytecode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkSetId(u32);

impl ChunkSetId {
    /// The maximum value a `ChunkSetId` may hold: 2²⁰ − 1 = `0xF_FFFF`.
    pub const MAX: u32 = (1 << 20) - 1;

    /// Construct a `ChunkSetId` from a 20-bit value.
    ///
    /// # Panics
    ///
    /// Panics if `bits > Self::MAX` (i.e., if any of the upper 12 bits of
    /// `bits` are set).
    pub fn new(bits: u32) -> Self {
        assert!(
            bits <= Self::MAX,
            "ChunkSetId value {bits:#x} exceeds 20-bit maximum ({:#x})",
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
// ChunkSetIdSeed
// ---------------------------------------------------------------------------

/// Optional 4-byte seed that, when supplied via
/// [`crate::EncodeOptions::chunk_set_id_seed`], overrides the chunk-header
/// `chunk_set_id` field.
///
/// The Tier-3 16-byte [`PolicyId`] is *always* content-derived and is NOT
/// affected by this seed (per `IMPLEMENTATION_PLAN_v0.1.md` §4 "Policy ID
/// semantics" and the BIP draft §"Chunk-set identifier"). The seed is only used
/// to override the 20-bit [`ChunkSetId`] embedded in chunk headers.
///
/// # When to use this
///
/// Production encoders should leave [`crate::EncodeOptions::chunk_set_id_seed`]
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
/// `ChunkSetIdSeed(<redacted>)`) so log spew cannot accidentally leak
/// the seed.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkSetIdSeed([u8; 4]);

impl ChunkSetIdSeed {
    /// Get the underlying 4 bytes.
    pub fn as_bytes(&self) -> &[u8; 4] {
        &self.0
    }

    /// Return the seed as a 32-bit big-endian unsigned integer.
    pub fn as_u32(&self) -> u32 {
        u32::from_be_bytes(self.0)
    }

    /// Truncate this seed to a 20-bit [`ChunkSetId`].
    ///
    /// Takes the high 20 bits of the u32 view (matches
    /// [`PolicyId::truncate`]'s big-endian-first-20-bits convention):
    /// ```text
    /// result = self.as_u32() >> 12
    /// ```
    /// This yields the top 20 bits of the 32-bit seed.
    pub fn truncate(&self) -> ChunkSetId {
        let bits = self.as_u32() >> 12;
        ChunkSetId::new(bits)
    }
}

impl fmt::Debug for ChunkSetIdSeed {
    /// Redacts the raw bytes to prevent accidental logging of seed material.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ChunkSetIdSeed(<redacted>)")
    }
}

impl From<u32> for ChunkSetIdSeed {
    /// Construct from a `u32` using big-endian byte order (high byte first).
    fn from(n: u32) -> Self {
        Self(n.to_be_bytes())
    }
}

impl From<[u8; 4]> for ChunkSetIdSeed {
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

    // --- PolicyId display / formatting ---

    #[test]
    fn policy_id_display_is_hex() {
        let id = PolicyId::from([0xABu8; 16]);
        let s = id.to_string();
        assert_eq!(
            s, "abababababababababababababababab",
            "expected 32 lowercase hex chars"
        );
        // LowerHex should produce the same output.
        assert_eq!(format!("{:x}", id), s);
    }

    #[test]
    fn policy_id_as_ref_returns_underlying_bytes() {
        let bytes = [
            0x01u8, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
            0x0F, 0x10,
        ];
        let id = PolicyId::from(bytes);
        assert_eq!(id.as_ref(), &bytes);
    }

    // --- PolicyId::truncate ---

    #[test]
    fn policy_id_truncate_takes_first_20_bits() {
        // All-zeros: result should be 0.
        let zero = PolicyId::from([0u8; 16]);
        assert_eq!(zero.truncate().as_u32(), 0);

        // All-0xFF: all 20 bits set → 0xF_FFFF.
        let all_ff = PolicyId::from([0xFFu8; 16]);
        assert_eq!(all_ff.truncate().as_u32(), 0xF_FFFF);

        // Mixed: bytes = [0x12, 0x34, 0x56, ...]
        // bit-pack: (0x12 << 12) | (0x34 << 4) | (0x56 >> 4)
        //         = 0x12_000 | 0x340 | 0x5
        //         = 0x12345
        let mut mixed = [0u8; 16];
        mixed[0] = 0x12;
        mixed[1] = 0x34;
        mixed[2] = 0x56;
        let id = PolicyId::from(mixed);
        let expected = ((0x12u32) << 12) | ((0x34u32) << 4) | ((0x56u32) >> 4);
        assert_eq!(id.truncate().as_u32(), expected);
        assert_eq!(id.truncate().as_u32(), 0x12345);
    }

    // --- PolicyId::to_words determinism ---

    #[test]
    fn policy_id_to_words_deterministic() {
        let id = PolicyId::from([0x42u8; 16]);
        let w1 = id.to_words();
        let w2 = id.to_words();
        assert_eq!(w1, w2, "to_words must be deterministic");
    }

    #[test]
    fn policy_id_to_words_yields_12_distinct_words_for_typical_input() {
        // Use a non-repeating byte sequence to ensure 12 distinct BIP-39 words.
        // ([0xAB; 16] happens to produce only 9 distinct words; a varied input
        // avoids repeated entropy patterns that collapse into repeated words.)
        let bytes = [
            0x01u8, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54,
            0x32, 0x10,
        ];
        let id = PolicyId::from(bytes);
        let words: Vec<String> = id.to_words().into_iter().collect();
        assert_eq!(words.len(), 12, "expected 12 words");
        let unique: std::collections::HashSet<&String> = words.iter().collect();
        assert_eq!(
            unique.len(),
            12,
            "expected 12 distinct words for varied input"
        );
    }

    // --- PolicyIdWords display & iterator ---

    #[test]
    fn policy_id_words_display_is_space_joined() {
        let id = PolicyId::from([0x00u8; 16]);
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
    fn policy_id_words_intoiterator_yields_12() {
        let id = PolicyId::from([0xDEu8; 16]);
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
    fn policy_id_words_as_slice_yields_12_strings() {
        let id = PolicyId::from([0x55u8; 16]);
        let words = id.to_words();
        let slice = words.as_slice();
        assert_eq!(slice.len(), 12, "as_slice must return exactly 12 words");
        for (i, word) in slice.iter().enumerate() {
            assert!(!word.is_empty(), "word {i} is empty");
        }
        // Borrowing via as_slice does not consume `words`.
        assert_eq!(words.as_slice().len(), 12, "as_slice is re-borrowable");
    }

    // --- compute_policy_id ---

    #[test]
    fn compute_policy_id_known_input() {
        // SHA-256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        // First 16 bytes as hex: e3b0c44298fc1c149afbf4c8996fb924
        let id = compute_policy_id(b"");
        assert_eq!(
            id.to_string(),
            "e3b0c44298fc1c149afbf4c8996fb924",
            "SHA-256(\"\") truncated to 16 bytes should match known vector"
        );
    }

    #[test]
    fn compute_policy_id_deterministic() {
        // Same input must always produce the same PolicyId.
        let id1 = compute_policy_id(b"hello world");
        let id2 = compute_policy_id(b"hello world");
        assert_eq!(id1, id2, "compute_policy_id must be deterministic");
    }

    #[test]
    fn compute_policy_id_distinguishes_inputs() {
        // Different inputs must produce different WalletIds.
        let id_a = compute_policy_id(b"a");
        let id_b = compute_policy_id(b"b");
        assert_ne!(id_a, id_b, "distinct inputs must yield distinct WalletIds");
    }

    #[test]
    fn compute_policy_id_truncate_is_first_20_bits() {
        // SHA-256("") = e3b0c44298fc1c149afbf4c8996fb924...
        // First 3 bytes: e3, b0, c4
        // First 20 bits: (0xe3 << 12) | (0xb0 << 4) | (0xc4 >> 4)
        //              = 0xe3000 | 0xb00 | 0xc
        //              = 0xe3b0c
        let id = compute_policy_id(b"");
        let chunk_id = id.truncate();

        let expected: u32 = ((0xe3u32) << 12) | ((0xb0u32) << 4) | ((0xc4u32) >> 4);
        assert_eq!(expected, 0xe3b0c, "manual bit-pack sanity check");
        assert_eq!(
            chunk_id.as_u32(),
            0xe3b0c,
            "ChunkSetId must equal the first 20 bits of SHA-256(bytecode)"
        );
    }

    #[test]
    fn compute_policy_id_with_typical_bytecode_input() {
        // A plausible short canonical-bytecode prefix — must not panic and must
        // return a valid PolicyId (no specific value assertion needed here).
        let fixture: &[u8] = &[0x05, 0x33, 0x01, 0x05, 0x1B, 0x32, 0x00];
        let id = compute_policy_id(fixture);
        // 32-hex-char Display output sanity check.
        let s = id.to_string();
        assert_eq!(s.len(), 32, "PolicyId Display must be 32 hex characters");
        assert!(
            s.chars().all(|c| c.is_ascii_hexdigit()),
            "PolicyId Display must be lowercase hex: {s:?}"
        );
    }

    // --- ChunkSetId ---

    #[test]
    fn chunk_set_id_new_accepts_zero_and_max() {
        assert_eq!(ChunkSetId::new(0).as_u32(), 0);
        assert_eq!(ChunkSetId::new(ChunkSetId::MAX).as_u32(), 0xF_FFFF);
    }

    #[test]
    #[should_panic]
    fn chunk_set_id_new_panics_above_max() {
        ChunkSetId::new(0x10_0000); // MAX + 1
    }

    #[test]
    fn chunk_set_id_max_is_20_bits() {
        assert_eq!(ChunkSetId::MAX, 0xF_FFFF);
    }

    // --- ChunkSetIdSeed ---

    #[test]
    fn chunk_set_id_seed_from_u32_is_big_endian() {
        let seed = ChunkSetIdSeed::from(0x1234_5678u32);
        assert_eq!(seed.as_bytes(), &[0x12, 0x34, 0x56, 0x78]);
    }

    #[test]
    fn chunk_set_id_seed_from_bytes_round_trip() {
        let seed = ChunkSetIdSeed::from([0xab; 4]);
        assert_eq!(seed.as_bytes(), &[0xab; 4]);
    }

    #[test]
    fn chunk_set_id_seed_as_u32_round_trip() {
        let seed = ChunkSetIdSeed::from(0x1234_5678u32);
        assert_eq!(seed.as_u32(), 0x1234_5678);
    }

    #[test]
    fn chunk_set_id_seed_debug_redacts_bytes() {
        let s = format!("{:?}", ChunkSetIdSeed::from(0xDEAD_BEEFu32));
        assert_eq!(s, "ChunkSetIdSeed(<redacted>)");
        // Must not reveal any part of the raw bytes.
        assert!(!s.to_lowercase().contains("dead"));
        assert!(!s.to_lowercase().contains("beef"));
        assert!(!s.to_lowercase().contains("de"));
    }

    #[test]
    fn chunk_set_id_seed_truncate_takes_high_20_bits() {
        // 0x12345678 >> 12 = 0x12345
        let seed = ChunkSetIdSeed::from(0x1234_5678u32);
        assert_eq!(seed.truncate().as_u32(), 0x12345);
    }

    // ---- WalletInstanceId ----

    /// Two wallets with identical canonical bytecode but different
    /// xpub sets produce different `WalletInstanceId`s. This is the
    /// core property that distinguishes `WalletInstanceId` from
    /// `PolicyId` (which would collide on identical bytecode).
    #[test]
    fn wallet_instance_id_differs_when_xpubs_differ() {
        use std::str::FromStr;
        let bytecode = b"\x00\x34\x03\x05\x08\x02\x02\x33\x00\x33\x01";
        let xpub_a = bitcoin::bip32::Xpub::from_str(
            "xpub6Br37sWxruYfT8ASpCjVHKGwgdnYFEn98DwiN76i2oyY6fgH1LAPmmDcF46xjxJr22gw4jmVjTE2E3URMnRPEPYyo1zoPSUba563ESMXCeb",
        ).unwrap();
        let xpub_b = bitcoin::bip32::Xpub::from_str(
            "xpub6FC1fXFP1GXLX5TKtcjHGT4q89SDRehkQLtbKJ2PzWcvbBHtyDsJPLtpLtkGqYNYZdVVAjRQ5kug9CsapegmmeRutpP7PW4u4wVF9JfkDhw",
        ).unwrap();
        let xpub_c = bitcoin::bip32::Xpub::from_str(
            "xpub6FC1fXFP1GXQpyRFfSE1vzzySqs3Vg63bzimYLeqtNUYbzA87kMNTcuy9ubr7MmavGRjW2FRYHP4WGKjwutbf1ghgkUW9H7e3ceaPLRcVwa",
        ).unwrap();

        let id_ab = compute_wallet_instance_id(bytecode, &[xpub_a, xpub_b]);
        let id_ac = compute_wallet_instance_id(bytecode, &[xpub_a, xpub_c]);
        let id_bc = compute_wallet_instance_id(bytecode, &[xpub_b, xpub_c]);

        // Same template, different xpub sets → different instance IDs.
        assert_ne!(id_ab, id_ac);
        assert_ne!(id_ab, id_bc);
        assert_ne!(id_ac, id_bc);
    }

    /// Order matters: `[xpub_a, xpub_b]` and `[xpub_b, xpub_a]` are
    /// different wallets (different `@0` and `@1` cosigner roles)
    /// and produce different `WalletInstanceId`s. Recovery tools
    /// MUST preserve placeholder-index ordering when computing.
    #[test]
    fn wallet_instance_id_is_xpub_order_sensitive() {
        use std::str::FromStr;
        let bytecode = b"\x00\x34\x03\x05\x08\x02\x02\x33\x00\x33\x01";
        let xpub_a = bitcoin::bip32::Xpub::from_str(
            "xpub6Br37sWxruYfT8ASpCjVHKGwgdnYFEn98DwiN76i2oyY6fgH1LAPmmDcF46xjxJr22gw4jmVjTE2E3URMnRPEPYyo1zoPSUba563ESMXCeb",
        ).unwrap();
        let xpub_b = bitcoin::bip32::Xpub::from_str(
            "xpub6FC1fXFP1GXLX5TKtcjHGT4q89SDRehkQLtbKJ2PzWcvbBHtyDsJPLtpLtkGqYNYZdVVAjRQ5kug9CsapegmmeRutpP7PW4u4wVF9JfkDhw",
        ).unwrap();

        let id_ab = compute_wallet_instance_id(bytecode, &[xpub_a, xpub_b]);
        let id_ba = compute_wallet_instance_id(bytecode, &[xpub_b, xpub_a]);
        assert_ne!(id_ab, id_ba);
    }

    /// Determinism: same inputs → same output across calls.
    #[test]
    fn wallet_instance_id_is_deterministic() {
        use std::str::FromStr;
        let bytecode = b"\x00\x34\x03\x06\x33\x00";
        let xpub = bitcoin::bip32::Xpub::from_str(
            "xpub6Br37sWxruYfT8ASpCjVHKGwgdnYFEn98DwiN76i2oyY6fgH1LAPmmDcF46xjxJr22gw4jmVjTE2E3URMnRPEPYyo1zoPSUba563ESMXCeb",
        ).unwrap();
        let id1 = compute_wallet_instance_id(bytecode, &[xpub]);
        let id2 = compute_wallet_instance_id(bytecode, &[xpub]);
        assert_eq!(id1, id2);
    }
}
