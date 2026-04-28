//! Options passed to the top-level [`crate::encode()`] and [`crate::decode()`] functions.

use bitcoin::bip32::{DerivationPath, Fingerprint};

use crate::chunking::ChunkingMode;
use crate::wallet_id::WalletIdSeed;

// ---------------------------------------------------------------------------
// EncodeOptions
// ---------------------------------------------------------------------------

/// Options controlling the [`crate::encode()`] pipeline.
///
/// All fields default to "natural" behavior:
/// - `chunking_mode = ChunkingMode::Auto`: single-string is preferred when bytecode fits.
/// - `force_long_code = false`: regular BCH code is preferred when it fits.
/// - `wallet_id_seed = None`: chunk-header `wallet_id` is content-derived.
/// - `shared_path = None`: encoder picks the shared path per the
///   `WalletPolicy::to_bytecode` precedence chain (see that method's
///   rustdoc).
///
/// Marked `#[non_exhaustive]` so future v0.2+ knobs (e.g. fingerprints)
/// can be added without breaking external callers. Within this crate,
/// construct with `..Default::default()` and override only the fields you
/// need; downstream callers must use the same pattern.
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EncodeOptions {
    /// How chunking is selected: [`ChunkingMode::Auto`] picks single-string
    /// when bytecode fits; [`ChunkingMode::ForceChunked`] forces a chunked
    /// encoding even when single-string would fit.
    ///
    /// Use case for `ForceChunked`: ergonomic chunk size when you'd rather
    /// engrave 2 short strings than 1 long one (per BIP §"Chunking" line 438).
    /// Setting this on a small input produces a 1-chunk Chunked card (with
    /// the 7-byte chunk-header overhead) instead of a SingleString card with
    /// the 2-byte header.
    ///
    /// Replaces the v0.1 `force_chunking: bool` field. The
    /// [`EncodeOptions::with_force_chunking`] builder method still accepts a
    /// `bool` for source-compatibility.
    pub chunking_mode: ChunkingMode,
    /// Force the long BCH code (15-char checksum) even when the regular code
    /// (13-char checksum) fits.
    ///
    /// The long code carries more payload per chunk at the cost of two extra
    /// transcribed characters per string. Most often paired with
    /// `chunking_mode = ChunkingMode::ForceChunked` to test long-code behavior
    /// on small inputs.
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
    /// Override the shared derivation path used in the bytecode's path
    /// declaration. When `Some(path)`, this takes precedence over both
    /// `WalletPolicy.decoded_shared_path` (populated by from_bytecode) and
    /// `WalletPolicy.shared_path()` (real-key origin path). When `None`,
    /// the encoder falls back to the existing precedence chain.
    ///
    /// See [`crate::WalletPolicy::to_bytecode`] for the full precedence
    /// rule.
    pub shared_path: Option<DerivationPath>,
    /// Optional master-key fingerprints to embed in a bytecode fingerprints
    /// block (BIP §"Fingerprints block"). Indexed by placeholder position:
    /// `fingerprints[i]` is the master-key fingerprint for placeholder `@i`.
    ///
    /// When `Some(fps)`, the encoder emits header byte `0x04` (bit 2 = 1)
    /// and writes `[Tag::Fingerprints (0x35)][count = fps.len() as u8][4*count bytes]`
    /// immediately after the path declaration. The encoder validates that
    /// `fps.len()` equals the policy's placeholder count and returns
    /// [`crate::Error::FingerprintsCountMismatch`] otherwise.
    ///
    /// When `None` (the default), the encoder emits header byte `0x00` and
    /// no fingerprints block, preserving v0.1 wire output.
    ///
    /// # Privacy
    ///
    /// Fingerprints leak which seeds match which `@i` placeholders. The
    /// fingerprints block is **optional** — only set this field if the
    /// recovery flow benefits from the disclosure (e.g., a multisig recovery
    /// tool that needs to match seeds to placeholder positions before
    /// deriving). Recovery tools SHOULD warn before encoding fingerprints,
    /// especially for solo-user single-seed wallets where the leak is
    /// unnecessary.
    pub fingerprints: Option<Vec<Fingerprint>>,
}

impl EncodeOptions {
    /// Force chunked encoding even when bytecode fits in a single string.
    ///
    /// `bool` shim retained for source-compatibility with v0.1.1 callers:
    /// `true` selects [`ChunkingMode::ForceChunked`], `false` selects
    /// [`ChunkingMode::Auto`]. See [`EncodeOptions::chunking_mode`] for full
    /// semantics.
    pub fn with_force_chunking(mut self, force: bool) -> Self {
        self.chunking_mode = if force {
            ChunkingMode::ForceChunked
        } else {
            ChunkingMode::Auto
        };
        self
    }

    /// Force the long BCH code even when regular fits.
    /// See [`EncodeOptions::force_long_code`] for full semantics.
    pub fn with_force_long_code(mut self, force: bool) -> Self {
        self.force_long_code = force;
        self
    }

    /// Override the chunk-header `wallet_id` with this seed.
    /// See [`EncodeOptions::wallet_id_seed`] for full semantics.
    pub fn with_seed(mut self, seed: WalletIdSeed) -> Self {
        self.wallet_id_seed = Some(seed);
        self
    }

    /// Override the bytecode's shared derivation path declaration.
    /// See [`EncodeOptions::shared_path`] for full semantics and
    /// [`crate::WalletPolicy::to_bytecode`] for the precedence rule.
    pub fn with_shared_path(mut self, path: DerivationPath) -> Self {
        self.shared_path = Some(path);
        self
    }

    /// Set the master-key fingerprints to embed in a fingerprints block.
    /// See [`EncodeOptions::fingerprints`] for full semantics, including
    /// the privacy clause and validation rules.
    pub fn with_fingerprints(mut self, fps: Vec<Fingerprint>) -> Self {
        self.fingerprints = Some(fps);
        self
    }
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
        assert_eq!(opts.chunking_mode, ChunkingMode::Auto);
        assert!(!opts.force_long_code);
        assert!(opts.wallet_id_seed.is_none());
        assert!(opts.shared_path.is_none());
        assert!(opts.fingerprints.is_none());
    }

    #[test]
    fn encode_options_construct_with_seed() {
        let seed = WalletIdSeed::from(0xDEAD_BEEFu32);
        let opts = EncodeOptions {
            wallet_id_seed: Some(seed),
            ..Default::default()
        };
        assert_eq!(opts.wallet_id_seed, Some(seed));
        assert_eq!(opts.chunking_mode, ChunkingMode::Auto);
        assert!(!opts.force_long_code);
        assert!(opts.shared_path.is_none());
    }

    #[test]
    fn encode_options_builder_chain() {
        let seed = WalletIdSeed::from(0xdeadbeefu32);
        let opts = EncodeOptions::default()
            .with_force_chunking(true)
            .with_force_long_code(true)
            .with_seed(seed);
        assert_eq!(opts.chunking_mode, ChunkingMode::ForceChunked);
        assert!(opts.force_long_code);
        assert_eq!(opts.wallet_id_seed, Some(seed));
        assert!(opts.shared_path.is_none());
    }

    #[test]
    fn encode_options_builder_default_passthrough() {
        let opts = EncodeOptions::default();
        let opts = opts.with_force_chunking(false);
        assert_eq!(opts.chunking_mode, ChunkingMode::Auto);
        assert!(!opts.force_long_code);
        assert_eq!(opts.wallet_id_seed, None);
        assert!(opts.shared_path.is_none());
    }

    #[test]
    fn encode_options_with_shared_path_sets_field() {
        use std::str::FromStr;
        let custom = DerivationPath::from_str("m/48'/0'/0'/2'").unwrap();
        let opts = EncodeOptions::default().with_shared_path(custom.clone());
        assert_eq!(opts.shared_path, Some(custom));
        // Other fields remain at defaults.
        assert_eq!(opts.chunking_mode, ChunkingMode::Auto);
        assert!(!opts.force_long_code);
        assert!(opts.wallet_id_seed.is_none());
        assert!(opts.fingerprints.is_none());
    }

    #[test]
    fn encode_options_with_fingerprints_sets_field() {
        let fps = vec![
            Fingerprint::from([0xde, 0xad, 0xbe, 0xef]),
            Fingerprint::from([0xca, 0xfe, 0xba, 0xbe]),
        ];
        let opts = EncodeOptions::default().with_fingerprints(fps.clone());
        assert_eq!(opts.fingerprints.as_deref(), Some(fps.as_slice()));
        // Other fields remain at defaults.
        assert_eq!(opts.chunking_mode, ChunkingMode::Auto);
        assert!(!opts.force_long_code);
        assert!(opts.wallet_id_seed.is_none());
        assert!(opts.shared_path.is_none());
    }

    #[test]
    fn with_force_chunking_translates_bool_to_enum() {
        // True → ForceChunked
        let opts = EncodeOptions::default().with_force_chunking(true);
        assert_eq!(opts.chunking_mode, ChunkingMode::ForceChunked);

        // False → Auto (round-trip from ForceChunked)
        let opts = opts.with_force_chunking(false);
        assert_eq!(opts.chunking_mode, ChunkingMode::Auto);
    }

    #[test]
    fn chunking_mode_default_is_auto() {
        assert_eq!(ChunkingMode::default(), ChunkingMode::Auto);
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
