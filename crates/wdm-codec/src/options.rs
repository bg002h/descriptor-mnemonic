//! Options passed to the top-level `encode()` and `decode()` functions.

use crate::wallet_id::WalletIdSeed;

// ---------------------------------------------------------------------------
// EncodeOptions
// ---------------------------------------------------------------------------

/// Options controlling the encode pipeline.
///
/// All fields default to "natural" behavior:
/// - `force_chunking = false`: single-string is preferred when bytecode fits.
/// - `force_long_code = false`: regular BCH code is preferred when it fits.
/// - `wallet_id_seed = None`: chunk-header `wallet_id` is content-derived.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EncodeOptions {
    /// Force chunked encoding even when bytecode fits in a single string.
    ///
    /// Use case: ergonomic chunk size when you'd rather engrave 2 short
    /// strings than 1 long one (per BIP §"Chunking" line 438).
    pub force_chunking: bool,
    /// Force long BCH code even when regular fits.
    ///
    /// Pairs with `force_chunking = true` to test long-code behavior on
    /// small inputs.
    pub force_long_code: bool,
    /// Override the chunk-header `wallet_id` with this seed instead of
    /// using the first 20 bits of the content-derived SHA-256.
    ///
    /// The Tier-3 16-byte WalletId is unaffected. Used for deterministic
    /// test-vector generation.
    pub wallet_id_seed: Option<WalletIdSeed>,
}

// ---------------------------------------------------------------------------
// DecodeOptions
// ---------------------------------------------------------------------------

/// Options controlling the decode pipeline.
///
/// v0.1 has no public knobs; the type exists so v0.2+ can add builder
/// methods without breaking existing call sites. Erasure decoding is
/// supported internally for use by guided recovery (v0.3); v0.1 callers
/// do not invoke that path.
///
/// Note: NOT `#[non_exhaustive]` because all fields are private — callers
/// can never construct via struct literal regardless. Adding private fields
/// stays non-breaking.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DecodeOptions {
    // private; v0.3 will expose via with_erasure_hint
    erasures: Vec<(usize, usize)>,
}

impl DecodeOptions {
    /// Construct default decode options (no public knobs in v0.1).
    pub fn new() -> Self {
        Self::default()
    }

    /// Internal accessor for the erasure list; used by guided recovery
    /// (v0.3) and tests. Not exposed publicly.
    #[allow(dead_code)] // used in Task 5-E (decode pipeline) and guided recovery (v0.3)
    pub(crate) fn erasures(&self) -> &[(usize, usize)] {
        &self.erasures
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_options_default_is_all_off() {
        let opts = EncodeOptions::default();
        assert!(!opts.force_chunking);
        assert!(!opts.force_long_code);
        assert!(opts.wallet_id_seed.is_none());
    }

    #[test]
    fn encode_options_construct_with_seed() {
        let seed = WalletIdSeed::from(0xDEAD_BEEFu32);
        let opts = EncodeOptions {
            wallet_id_seed: Some(seed),
            ..Default::default()
        };
        assert_eq!(opts.wallet_id_seed, Some(seed));
        assert!(!opts.force_chunking);
        assert!(!opts.force_long_code);
    }

    #[test]
    fn decode_options_default_is_empty() {
        assert!(DecodeOptions::default().erasures().is_empty());
    }

    #[test]
    fn decode_options_new_matches_default() {
        assert_eq!(DecodeOptions::new(), DecodeOptions::default());
    }
}
