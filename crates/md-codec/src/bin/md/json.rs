//! Local serde-able wrapper types for `md` CLI `--json` output.
//!
//! # Why wrapper types?
//!
//! The library types involved in `--json` output (`MdBackup`, `EncodedChunk`,
//! `DecodeResult`, `DecodeReport`, `Correction`, `Verifications`,
//! `Confidence`, `DecodeOutcome`) are **deliberately not** `Serialize`:
//!
//! - `WalletPolicy` (inside `DecodeResult`) wraps a miniscript
//!   `Descriptor<DescriptorPublicKey>`, which is a third-party type without a
//!   `Serialize` impl, so a blanket derive on `WalletPolicy` is not feasible.
//! - The library is a wire-format reference implementation; forcing a serde
//!   contract on every public type would expand the library's stability
//!   surface beyond what v0.1/v0.2 commits to.
//!
//! Instead, the CLI keeps a **bin-private** mirror of just the JSON shape it
//! emits, with `From<&LibraryType> for WrapperType` conversions. Field order
//! within each wrapper is **alphabetical** to match the byte-for-byte output
//! produced by v0.1.1's hand-built `serde_json::json!{}` literals (which
//! default to `BTreeMap`-sorted keys).
//!
//! See FOLLOWUPS.md `7-serialize-derives` (resolved by Phase B Wave 2 /
//! v0.2 Bucket C) for context.

use serde::{Deserialize, Serialize};

use md_codec::{
    BchCode, Confidence, DecodeOutcome, DecodeReport, DecodeResult, EncodedChunk, MdBackup,
    Verifications, chunking::Correction,
};

// ---------------------------------------------------------------------------
// Encode JSON shape
// ---------------------------------------------------------------------------

/// Top-level `md encode --json` output.
///
/// Mirrors `MdBackup` for serialization. Fields appear in alphabetical
/// order to preserve the byte-identical output of the v0.1.1 hand-built
/// `serde_json::json!{}` literal.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct EncodeJson {
    /// One JSON object per encoded chunk.
    pub chunks: Vec<EncodedChunkJson>,
    /// 12-word BIP-39 representation of the Tier-3 Wallet ID.
    pub wallet_id_words: String,
}

/// One encoded chunk, mirroring `EncodedChunk`.
///
/// Fields appear in alphabetical order. `code` is rendered as
/// `"regular"` or `"long"` (lowercase) per v0.1.1 contract.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct EncodedChunkJson {
    /// Zero-based chunk index.
    pub chunk_index: u8,
    /// BCH code variant: `"regular"` or `"long"`.
    pub code: BchCodeJson,
    /// The full codex32 string (HRP + data + checksum).
    pub raw: String,
    /// Total number of chunks in this backup.
    pub total_chunks: u8,
}

/// Lowercase string repr of `BchCode` (`"regular"` / `"long"`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum BchCodeJson {
    /// Regular (13-char checksum) code.
    Regular,
    /// Long (15-char checksum) code.
    Long,
}

impl From<&BchCode> for BchCodeJson {
    fn from(c: &BchCode) -> Self {
        match c {
            BchCode::Regular => BchCodeJson::Regular,
            BchCode::Long => BchCodeJson::Long,
        }
    }
}

impl From<&EncodedChunk> for EncodedChunkJson {
    fn from(c: &EncodedChunk) -> Self {
        EncodedChunkJson {
            chunk_index: c.chunk_index,
            code: (&c.code).into(),
            raw: c.raw.clone(),
            total_chunks: c.total_chunks,
        }
    }
}

impl From<&MdBackup> for EncodeJson {
    fn from(b: &MdBackup) -> Self {
        EncodeJson {
            chunks: b.chunks.iter().map(EncodedChunkJson::from).collect(),
            wallet_id_words: b.wallet_id_words.to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// Decode JSON shape
// ---------------------------------------------------------------------------

/// Top-level `md decode --json` output.
///
/// Mirrors `DecodeResult` but renders `policy` as the canonical string
/// (since the library `WalletPolicy` is not `Serialize`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DecodeJson {
    /// Canonical-string form of the recovered wallet policy.
    pub policy: String,
    /// Diagnostic report.
    pub report: DecodeReportJson,
}

/// Mirrors `DecodeReport`. Fields in alphabetical order.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DecodeReportJson {
    /// Confidence level (Debug-formatted variant name).
    pub confidence: String,
    /// BCH corrections applied (one entry per corrected character).
    pub corrections: Vec<CorrectionJson>,
    /// High-level decode outcome (Debug-formatted variant name).
    pub outcome: String,
    /// Per-stage verification flags.
    pub verifications: VerificationsJson,
}

/// Mirrors `Correction`. Fields in alphabetical order.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct CorrectionJson {
    /// 0-indexed position within the chunk's data part (after HRP+separator).
    pub char_position: usize,
    /// Zero-based chunk index where the correction was applied.
    pub chunk_index: u8,
    /// The character the BCH decoder computed (corrected).
    pub corrected: String,
    /// The character the user transcribed (erroneous).
    pub original: String,
}

/// Mirrors `Verifications`. Fields in alphabetical order.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct VerificationsJson {
    /// Reassembled bytecode parsed without leftover bytes / unknown tags.
    pub bytecode_well_formed: bool,
    /// SHA-256(canonical_bytecode)[0..4] matched after chunk reassembly.
    pub cross_chunk_hash_ok: bool,
    /// All chunks declared the same `total_chunks`.
    pub total_chunks_consistent: bool,
    /// The bytecode header version is supported by this implementation.
    pub version_supported: bool,
    /// All chunks declared the same `wallet_id`.
    pub wallet_id_consistent: bool,
}

impl From<&Correction> for CorrectionJson {
    fn from(c: &Correction) -> Self {
        CorrectionJson {
            char_position: c.char_position,
            chunk_index: c.chunk_index,
            corrected: c.corrected.to_string(),
            original: c.original.to_string(),
        }
    }
}

impl From<&Verifications> for VerificationsJson {
    fn from(v: &Verifications) -> Self {
        VerificationsJson {
            bytecode_well_formed: v.bytecode_well_formed,
            cross_chunk_hash_ok: v.cross_chunk_hash_ok,
            total_chunks_consistent: v.total_chunks_consistent,
            version_supported: v.version_supported,
            wallet_id_consistent: v.wallet_id_consistent,
        }
    }
}

/// Format a `Confidence` enum value the same way `format!("{:?}", c)`
/// did in v0.1.1's hand-built JSON literal.
fn confidence_debug(c: Confidence) -> String {
    // Use Debug — this is the v0.1.1 contract. `Confidence` does not impl
    // `Display`, and switching to a different repr would change the JSON.
    format!("{c:?}")
}

/// Format a `DecodeOutcome` enum value the same way `format!("{:?}", o)`
/// did in v0.1.1's hand-built JSON literal.
fn outcome_debug(o: DecodeOutcome) -> String {
    format!("{o:?}")
}

impl From<&DecodeReport> for DecodeReportJson {
    fn from(r: &DecodeReport) -> Self {
        DecodeReportJson {
            confidence: confidence_debug(r.confidence),
            corrections: r.corrections.iter().map(CorrectionJson::from).collect(),
            outcome: outcome_debug(r.outcome),
            verifications: VerificationsJson::from(&r.verifications),
        }
    }
}

impl From<&DecodeResult> for DecodeJson {
    fn from(r: &DecodeResult) -> Self {
        DecodeJson {
            policy: r.policy.to_canonical_string(),
            report: DecodeReportJson::from(&r.report),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use md_codec::{
        DecodeOptions, EncodeOptions, WalletPolicy, decode, decode_report::DecodeOutcome, encode,
    };

    #[test]
    fn bch_code_json_renders_lowercase() {
        let s = serde_json::to_string(&BchCodeJson::Regular).unwrap();
        assert_eq!(s, "\"regular\"");
        let s = serde_json::to_string(&BchCodeJson::Long).unwrap();
        assert_eq!(s, "\"long\"");
    }

    #[test]
    fn encoded_chunk_json_field_order_is_alphabetical() {
        // EncodedChunk → EncodedChunkJson construction uses the public field
        // setters; we verify the resulting JSON's key order matches v0.1.1
        // (alphabetical: chunk_index, code, raw, total_chunks).
        let chunk = EncodedChunkJson {
            chunk_index: 0,
            code: BchCodeJson::Regular,
            raw: "md1qqqq".to_string(),
            total_chunks: 1,
        };
        let s = serde_json::to_string(&chunk).unwrap();
        // The first key must be `chunk_index`, the last must be
        // `total_chunks`. We assert by index-of substring since
        // serde_json's compact form preserves declaration order.
        let pos_chunk = s.find("\"chunk_index\"").unwrap();
        let pos_code = s.find("\"code\"").unwrap();
        let pos_raw = s.find("\"raw\"").unwrap();
        let pos_total = s.find("\"total_chunks\"").unwrap();
        assert!(pos_chunk < pos_code);
        assert!(pos_code < pos_raw);
        assert!(pos_raw < pos_total);
    }

    #[test]
    fn encode_json_round_trip_via_serde() {
        // Build a real `MdBackup`, convert via `From`, serialize, then
        // deserialize back into `EncodeJson`. The wrapper must be symmetric.
        let policy: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
        let backup = encode(&policy, &EncodeOptions::default()).unwrap();
        let wrapper = EncodeJson::from(&backup);
        let s = serde_json::to_string(&wrapper).unwrap();
        let round: EncodeJson = serde_json::from_str(&s).unwrap();
        assert_eq!(wrapper, round);
        assert!(!round.chunks.is_empty());
        assert!(!round.wallet_id_words.is_empty());
    }

    #[test]
    fn decode_json_round_trip_via_serde() {
        // Build a real `DecodeResult`, convert via `From`, serialize, then
        // deserialize. The wrapper must be symmetric.
        let policy: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
        let backup = encode(&policy, &EncodeOptions::default()).unwrap();
        let chunk = &backup.chunks[0].raw;
        let result = decode(&[chunk.as_str()], &DecodeOptions::new()).unwrap();
        let wrapper = DecodeJson::from(&result);
        let s = serde_json::to_string(&wrapper).unwrap();
        let round: DecodeJson = serde_json::from_str(&s).unwrap();
        assert_eq!(wrapper, round);
    }

    #[test]
    fn confidence_debug_matches_v011_contract() {
        assert_eq!(confidence_debug(Confidence::Confirmed), "Confirmed");
        assert_eq!(confidence_debug(Confidence::High), "High");
        assert_eq!(confidence_debug(Confidence::Probabilistic), "Probabilistic");
        assert_eq!(confidence_debug(Confidence::Failed), "Failed");
    }

    #[test]
    fn outcome_debug_matches_v011_contract() {
        assert_eq!(outcome_debug(DecodeOutcome::Clean), "Clean");
        assert_eq!(outcome_debug(DecodeOutcome::AutoCorrected), "AutoCorrected");
        assert_eq!(outcome_debug(DecodeOutcome::Failed), "Failed");
    }

    #[test]
    fn verifications_from_library_type() {
        let v = Verifications {
            cross_chunk_hash_ok: true,
            wallet_id_consistent: false,
            total_chunks_consistent: true,
            bytecode_well_formed: false,
            version_supported: true,
        };
        let j = VerificationsJson::from(&v);
        assert!(j.cross_chunk_hash_ok);
        assert!(!j.wallet_id_consistent);
        assert!(j.total_chunks_consistent);
        assert!(!j.bytecode_well_formed);
        assert!(j.version_supported);
    }

    #[test]
    fn correction_from_library_type() {
        let c = Correction {
            chunk_index: 2,
            char_position: 17,
            original: 'q',
            corrected: 'p',
        };
        let j = CorrectionJson::from(&c);
        assert_eq!(j.chunk_index, 2);
        assert_eq!(j.char_position, 17);
        assert_eq!(j.original, "q");
        assert_eq!(j.corrected, "p");
    }
}
