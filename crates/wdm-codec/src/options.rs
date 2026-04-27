//! Options passed to the top-level [`crate::encode()`] and [`crate::decode()`] functions.

use crate::wallet_id::WalletIdSeed;

// ---------------------------------------------------------------------------
// EncodeOptions
// ---------------------------------------------------------------------------

/// Options controlling the [`crate::encode()`] pipeline.
///
/// All fields default to "natural" behavior:
/// - `force_chunking = false`: single-string is preferred when bytecode fits.
/// - `force_long_code = false`: regular BCH code is preferred when it fits.
/// - `wallet_id_seed = None`: chunk-header `wallet_id` is content-derived.
///
/// Marked `#[non_exhaustive]` so future v0.2+ knobs (e.g. shared-path
/// override) can be added without breaking external callers. Within this
/// crate, construct with `..Default::default()` and override only the
/// fields you need; downstream callers must use the same pattern.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EncodeOptions {
    /// Force chunked encoding even when bytecode fits in a single string.
    ///
    /// Use case: ergonomic chunk size when you'd rather engrave 2 short
    /// strings than 1 long one (per BIP §"Chunking" line 438). Setting this
    /// on a small input produces a 1-chunk Chunked card (with the 7-byte
    /// chunk-header overhead) instead of a SingleString card with the 2-byte
    /// header.
    pub force_chunking: bool,
    /// Force the long BCH code (15-char checksum) even when the regular code
    /// (13-char checksum) fits.
    ///
    /// The long code carries more payload per chunk at the cost of two extra
    /// transcribed characters per string. Most often paired with
    /// `force_chunking = true` to test long-code behavior on small inputs.
    pub force_long_code: bool,
    /// Override the chunk-header [`crate::ChunkWalletId`] with this seed instead of
    /// using the first 20 bits of the content-derived SHA-256.
    ///
    /// The Tier-3 16-byte [`crate::WalletId`] is **unaffected** by this
    /// option (per `IMPLEMENTATION_PLAN_v0.1.md` §4 "Wallet ID semantics"
    /// and the BIP draft §"Wallet identifier"). Used for deterministic
    /// test-vector generation; production encoders should leave this `None`
    /// so the chunk-header bits remain predictable from the Tier-3 mnemonic.
    /// See [`WalletIdSeed`] for the full rationale and footgun warning.
    pub wallet_id_seed: Option<WalletIdSeed>,
}

// ---------------------------------------------------------------------------
// DecodeOptions
// ---------------------------------------------------------------------------

/// Options controlling the [`crate::decode()`] pipeline.
///
/// v0.1 has no public knobs; the type exists so v0.2+ can add builder
/// methods without breaking existing call sites. Construct with
/// [`DecodeOptions::new`] (or `DecodeOptions::default()`).
///
/// # Reserved internal fields
///
/// The struct holds a private `erasures: Vec<(usize, usize)>` field reserved
/// for v0.3 guided-recovery erasure decoding (where the user reports "I
/// can't read these characters" and the decoder uses BCH ECC to fill them
/// in beyond the substitution-only correction limit). v0.1 callers cannot
/// populate this list, and the v0.1 [`crate::decode()`] function silently
/// ignores it.
///
/// # Stability
///
/// **Not** marked `#[non_exhaustive]` because all fields are private —
/// callers can never construct via struct literal regardless, so adding
/// private fields in future versions stays a non-breaking change.
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
