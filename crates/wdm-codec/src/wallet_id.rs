//! Wallet-identity types used across WDM chunking.
//!
//! This module provides three types:
//! - [`WalletId`] — 16-byte Tier-3 Wallet ID (first 16 bytes of SHA-256 of
//!   canonical bytecode).
//! - [`WalletIdWords`] — the 12 BIP-39 words derived deterministically from a
//!   `WalletId`.
//! - [`ChunkWalletId`] — the 20-bit chunk-header field derived from a
//!   `WalletId` by taking its first 20 bits.

use std::fmt;

// ---------------------------------------------------------------------------
// WalletId
// ---------------------------------------------------------------------------

/// A 16-byte Wallet ID formed by taking the first 16 bytes of the SHA-256
/// hash of the wallet's canonical WDM bytecode (Tier-3 Wallet ID).
///
/// The full 128 bits serve as a collision-resistant identifier for the wallet;
/// the BIP-39 encoding ([`WalletIdWords`]) gives a human-friendly 12-word
/// form, and [`ChunkWalletId`] extracts the 20 most-significant bits for use
/// in chunk headers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WalletId([u8; 16]);

impl WalletId {
    /// Construct a `WalletId` from a raw 16-byte array.
    pub fn new(bytes: [u8; 16]) -> Self {
        Self(bytes)
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
    /// Same output as [`Display`] — 32 lowercase hex digits, no separator.
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
        Self(bytes)
    }
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
            s,
            "abababababababababababababababababab"[..32].to_string(),
            "expected 32 lowercase hex chars"
        );
        // Verify the exact string independently.
        assert_eq!(s, "abababababababababababababababababab"[..32]);
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
}
