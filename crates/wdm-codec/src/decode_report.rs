//! Result types for the top-level `decode()` function.
//!
//! Pure data — no algorithms here. The decoder in `decode.rs` (Task 5-E)
//! populates these types.

use bitcoin::bip32::Fingerprint;

use crate::WalletPolicy;
use crate::chunking::Correction;

// ---------------------------------------------------------------------------
// DecodeOutcome
// ---------------------------------------------------------------------------

/// One of three decode outcomes.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeOutcome {
    /// All chunks parsed cleanly with zero BCH corrections needed.
    Clean,
    /// At least one chunk had ≤4 substitution errors that the BCH ECC
    /// auto-corrected. `DecodeReport.corrections` lists each fix.
    AutoCorrected,
    /// Decode failed at some stage. `DecodeReport.confidence == Failed`.
    Failed,
}

// ---------------------------------------------------------------------------
// Confidence
// ---------------------------------------------------------------------------

/// Confidence level of the recovered policy.
///
/// Per BIP §"Recovery confidence calibration":
/// - **Confirmed**: zero BCH corrections; cross-chunk hash matched
///   first try; structural verifications all true.
/// - **High**: BCH auto-corrected ≤4 substitutions per chunk; all
///   verifications still true.
/// - **Probabilistic**: structure-aided guided recovery succeeded
///   (v0.3 path; not produced in v0.1).
/// - **Failed**: decode did not produce a usable policy.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence {
    /// Zero BCH corrections; all verifications passed.
    Confirmed,
    /// BCH auto-corrected ≤4 substitutions per chunk; all verifications
    /// still true.
    High,
    /// Structure-aided guided recovery succeeded (v0.3 path; not produced
    /// in v0.1).
    Probabilistic,
    /// Decode did not produce a usable policy.
    Failed,
}

// ---------------------------------------------------------------------------
// Verifications
// ---------------------------------------------------------------------------

/// Per-stage verification flags.
///
/// NOT `#[non_exhaustive]` — callers SHOULD match exhaustively on these
/// per the BIP's recovery requirements (so adding a field is a deliberate
/// breaking change that consumers see at the type level).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Verifications {
    /// SHA-256(canonical_bytecode)[0..4] matched the trailing hash after
    /// chunk reassembly. `true` by convention for single-string backups
    /// (no cross-chunk hash exists to verify against).
    pub cross_chunk_hash_ok: bool,
    /// All chunks declared the same `wallet_id`. Trivially `true` for
    /// single-string backups.
    pub wallet_id_consistent: bool,
    /// All chunks declared the same `total_chunks`. Trivially `true` for
    /// single-string backups.
    pub total_chunks_consistent: bool,
    /// The reassembled bytecode parsed without leftover bytes, unknown
    /// tags, or other structural errors.
    pub bytecode_well_formed: bool,
    /// The bytecode header version is supported by this implementation.
    pub version_supported: bool,
}

// ---------------------------------------------------------------------------
// DecodeReport
// ---------------------------------------------------------------------------

/// Full diagnostic report from one [`crate::decode()`] call.
///
/// Every successful decode produces this report alongside the recovered
/// [`WalletPolicy`]. The report tells the caller WHAT happened during decode
/// (any BCH corrections, which structural checks passed) and HOW MUCH to
/// trust the result (the [`Confidence`] field).
///
/// # When to consult each field
///
/// - **Show [`Self::confidence`] to the user** before they rely on the
///   recovered policy. `Confirmed` is "as good as the original engraving";
///   `High` means BCH had to fix transcription typos but everything still
///   verifies.
/// - **Show [`Self::corrections`] when non-empty**: the user transcribed
///   one or more chunks slightly wrong; surfacing "we fixed your card 1
///   character 17 from `q` to `p`" lets them double-check the original
///   media.
/// - **Use [`Self::verifications`] for forensic analysis** when something
///   went wrong; these are pre-flight checks that all return `true` on a
///   successful v0.1 decode but expose which stage validated each property.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeReport {
    /// High-level outcome of the decode attempt.
    pub outcome: DecodeOutcome,
    /// List of BCH corrections applied (empty for `Clean` decodes).
    pub corrections: Vec<Correction>,
    /// Per-stage verification flags.
    pub verifications: Verifications,
    /// Confidence level of the recovered policy.
    pub confidence: Confidence,
}

// ---------------------------------------------------------------------------
// DecodeResult
// ---------------------------------------------------------------------------

/// The result of a successful [`crate::decode()`]: the recovered
/// [`WalletPolicy`] plus a [`DecodeReport`].
///
/// Pair this with [`crate::WdmBackup`] from the encode side to see the
/// type-state graph: encode produces `WdmBackup`, decode produces
/// `DecodeResult`. The `WdmBackup` is the engraving-side artifact; the
/// `DecodeResult` is the recovery-side artifact.
///
/// Marked `#[non_exhaustive]` so v0.2+ can add fields (e.g. recovered
/// fingerprints or a derivation hint) without breaking pattern matching.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeResult {
    /// The recovered wallet policy.
    pub policy: WalletPolicy,
    /// Full diagnostic report for this decode.
    pub report: DecodeReport,
    /// Master-key fingerprints recovered from the bytecode's optional
    /// fingerprints block (BIP §"Fingerprints block"). `None` if the
    /// bytecode header bit 2 was 0 (no block); `Some(fps)` if the block
    /// was present and parsed successfully, with `fps[i]` corresponding
    /// to placeholder `@i`.
    ///
    /// Phase E (v0.2). Recovery tools that surface this field to users
    /// MUST flag it as privacy-sensitive — fingerprints leak which seeds
    /// match which placeholders.
    pub fingerprints: Option<Vec<Fingerprint>>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_outcome_variants_are_distinct() {
        assert_ne!(DecodeOutcome::Clean, DecodeOutcome::AutoCorrected);
        assert_ne!(DecodeOutcome::Clean, DecodeOutcome::Failed);
        assert_ne!(DecodeOutcome::AutoCorrected, DecodeOutcome::Failed);
    }

    #[test]
    fn confidence_variants_are_distinct() {
        assert_ne!(Confidence::Confirmed, Confidence::High);
        assert_ne!(Confidence::Confirmed, Confidence::Probabilistic);
        assert_ne!(Confidence::Confirmed, Confidence::Failed);
        assert_ne!(Confidence::High, Confidence::Probabilistic);
        assert_ne!(Confidence::High, Confidence::Failed);
        assert_ne!(Confidence::Probabilistic, Confidence::Failed);
    }

    #[test]
    fn verifications_default_construction() {
        let v = Verifications {
            cross_chunk_hash_ok: true,
            wallet_id_consistent: true,
            total_chunks_consistent: true,
            bytecode_well_formed: true,
            version_supported: true,
        };
        assert!(v.cross_chunk_hash_ok);
        assert!(v.wallet_id_consistent);
        assert!(v.total_chunks_consistent);
        assert!(v.bytecode_well_formed);
        assert!(v.version_supported);
    }

    #[test]
    fn decode_report_struct_construction() {
        let report = DecodeReport {
            outcome: DecodeOutcome::Clean,
            corrections: vec![],
            verifications: Verifications {
                cross_chunk_hash_ok: true,
                wallet_id_consistent: true,
                total_chunks_consistent: true,
                bytecode_well_formed: true,
                version_supported: true,
            },
            confidence: Confidence::Confirmed,
        };
        assert_eq!(report.outcome, DecodeOutcome::Clean);
        assert!(report.corrections.is_empty());
        assert_eq!(report.confidence, Confidence::Confirmed);
    }

    #[test]
    fn decode_result_struct_construction() {
        let policy: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
        let report = DecodeReport {
            outcome: DecodeOutcome::Clean,
            corrections: vec![],
            verifications: Verifications {
                cross_chunk_hash_ok: true,
                wallet_id_consistent: true,
                total_chunks_consistent: true,
                bytecode_well_formed: true,
                version_supported: true,
            },
            confidence: Confidence::Confirmed,
        };
        let result = DecodeResult {
            policy,
            report,
            fingerprints: None,
        };
        assert_eq!(result.report.outcome, DecodeOutcome::Clean);
        assert!(result.fingerprints.is_none());
    }
}
