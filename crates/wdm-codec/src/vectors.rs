//! Test vector schema and generator for WDM codec.
//!
//! # Schema versioning
//!
//! Two schema versions coexist; both are authoritative for their respective
//! WDM releases:
//!
//! - **Schema 1** (`build_test_vectors_v1`, alias [`build_test_vectors`])
//!   was committed at v0.1.0 in `tests/vectors/v0.1.json` and is byte-frozen.
//!   Any change to that file is a release-engineering incident.
//! - **Schema 2** ([`build_test_vectors_v2`]) is the v0.2.0 lock. It is a
//!   strict superset of schema 1: every schema-1 field is preserved (same
//!   names, same semantics) and a small number of optional fields have been
//!   added. Schema-1 readers can deserialize schema-2 files (additive fields
//!   are silently ignored).
//!
//! The schema-2 additions are:
//! - [`Vector::expected_fingerprints_hex`] — populated for the fingerprints
//!   positive vector with the lowercase-hex 4-byte values that the encoder
//!   emitted into the bytecode's fingerprints block. `None` for vectors
//!   encoded with default [`EncodeOptions`].
//! - [`Vector::encode_options_fingerprints`] — the `Vec<[u8; 4]>` passed to
//!   `EncodeOptions::with_fingerprints` when the fixture was generated, so
//!   independent regenerators can reproduce the exact bytecode without
//!   relying on hidden state. `None` for default-options vectors.
//! - [`NegativeVector::provenance`] — a one-sentence note describing how the
//!   `input_strings` were generated (e.g., "encoded `wsh(pk(@0/**))`, then
//!   uppercased the data character at position 5"). For variants whose
//!   trigger lives below the WDM-string layer (e.g., `EmptyChunkList`) the
//!   `input_strings` is empty and the provenance names the relevant
//!   lower-level API.
//!
//! Schema changes require bumping `TestVectorFile::schema_version` and
//! updating the BIP draft's Test Vectors section.
//!
//! # Usage
//!
//! ```rust
//! // Schema 1 (v0.1 lock):
//! let v1 = wdm_codec::vectors::build_test_vectors();
//! assert_eq!(v1.schema_version, 1);
//!
//! // Schema 2 (v0.2 lock):
//! let v2 = wdm_codec::vectors::build_test_vectors_v2();
//! assert_eq!(v2.schema_version, 2);
//! ```

use bitcoin::bip32::Fingerprint;
use serde::{Deserialize, Serialize};

use crate::{EncodeOptions, WalletPolicy, encode};

// ---------------------------------------------------------------------------
// Public schema types (Task 8.1; extended for schema 2 — Phase F)
// ---------------------------------------------------------------------------

/// Top-level test vector file.
///
/// Stable across v0.1+. Changing field names without bumping `schema_version`
/// is a breaking change.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TestVectorFile {
    /// Schema version. `1` for the v0.1 lock; `2` for the v0.2 lock.
    pub schema_version: u32,
    /// Implementation version that generated the file.
    pub generator: String,
    /// Positive (encode-decode round-trip) vectors.
    pub vectors: Vec<Vector>,
    /// Negative (rejection) vectors.
    pub negative_vectors: Vec<NegativeVector>,
}

/// A positive test vector: a wallet policy that round-trips cleanly.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Vector {
    /// Stable identifier (e.g., `c1`, `e13`).
    pub id: String,
    /// Human-readable description.
    pub description: String,
    /// Input wallet policy string (BIP 388 form with `@i` placeholders).
    pub policy: String,
    /// Expected canonical bytecode as lowercase hex.
    pub expected_bytecode_hex: String,
    /// Expected encoded chunk strings under default [`EncodeOptions`].
    pub expected_chunks: Vec<String>,
    /// Expected 12-word Tier-3 Wallet ID.
    pub expected_wallet_id_words: Vec<String>,
    /// Schema-2 only: fingerprints encoded into this vector.
    ///
    /// `Some(_)` iff the generator passed fingerprints to
    /// `EncodeOptions::with_fingerprints` when computing
    /// `expected_bytecode_hex`. Each entry is 8 lowercase-hex chars
    /// (4 bytes), in the same order as the encoder accepted them.
    /// `None` for v0.1 vectors and for any v0.2 vector encoded with
    /// default [`EncodeOptions`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_fingerprints_hex: Option<Vec<String>>,
    /// Schema-2 only: fingerprints to pass to `EncodeOptions::with_fingerprints`
    /// when regenerating this vector.
    ///
    /// Stored as `Vec<[u8; 4]>` so the JSON form is the obvious
    /// `[[222,173,190,239], [202,254,186,190]]`. Independent regenerators
    /// can construct `bitcoin::bip32::Fingerprint::from(arr)` from each
    /// entry. `None` for non-fingerprints vectors (the encoder uses
    /// `EncodeOptions::default()`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encode_options_fingerprints: Option<Vec<[u8; 4]>>,
}

/// A negative test vector: an input that must be rejected.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NegativeVector {
    /// Stable identifier.
    pub id: String,
    /// Human-readable description.
    pub description: String,
    /// Input strings that should fail to decode.
    pub input_strings: Vec<String>,
    /// The error variant name (e.g., `"InvalidHrp"`, `"BchUncorrectable"`).
    ///
    /// Stable identifier for cross-implementation matching.
    pub expected_error_variant: String,
    /// Schema-2 only: one-sentence note on how `input_strings` were
    /// generated.
    ///
    /// `None` for schema-1 entries (where `input_strings` are
    /// representative placeholders; see `tests/vectors/v0.1.json`).
    /// `Some(...)` for schema-2 entries: a short human-readable
    /// description of the mutation/construction recipe — e.g.,
    /// "encoded `wsh(pk(@0/**))`, uppercased data char at position 5".
    /// For variants whose trigger requires a lower-level API call
    /// (`EmptyChunkList`, `PolicyTooLarge`, encoder-side rejections),
    /// `input_strings` is empty and the provenance names the API
    /// surface where the variant fires.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<String>,
}

// ---------------------------------------------------------------------------
// Corpus fixtures (positive vectors) — ordered as in tests/corpus.rs
// ---------------------------------------------------------------------------

/// (id, description, policy_str)
const CORPUS_FIXTURES: &[(&str, &str, &str)] = &[
    ("c1", "C1 — Single-key wsh(pk)", "wsh(pk(@0/**))"),
    (
        "c2",
        "C2 — 2-of-3 wsh(sortedmulti)",
        "wsh(sortedmulti(2,@0/**,@1/**,@2/**))",
    ),
    (
        "c3",
        "C3 — 2-of-3 with timelock fallback",
        "wsh(or_d(multi(2,@0/**,@1/**),and_v(v:older(52560),pk(@2/**))))",
    ),
    (
        "c4",
        "C4 — 6-key inheritance miniscript",
        concat!(
            "wsh(andor(pk(@0/**),after(1200000),or_i(",
            "and_v(v:pkh(@1/**),and_v(v:pkh(@2/**),and_v(v:pkh(@3/**),older(4032)))),",
            "and_v(v:pkh(@4/**),and_v(v:pkh(@5/**),older(32768))))))",
        ),
    ),
    (
        "c5",
        "C5 — 5-of-9 thresh with 2-key timelock recovery",
        concat!(
            "wsh(or_d(",
            "thresh(5,pk(@0/**),s:pk(@1/**),s:pk(@2/**),s:pk(@3/**),s:pk(@4/**),",
            "s:pk(@5/**),s:pk(@6/**),s:pk(@7/**),s:pk(@8/**)),",
            "and_v(v:older(105120),multi(2,@9/**,@10/**))))",
        ),
    ),
    (
        "e10",
        "E10 — Liana Simple Inheritance single-key + 1-year recovery",
        "wsh(or_d(pk(@0/**),and_v(v:pk(@1/**),older(52560))))",
    ),
    (
        "e12",
        "E12 — Liana Expanding Multisig 2-of-2 + recovery key",
        "wsh(or_d(multi(2,@0/**,@1/**),and_v(v:older(52560),pk(@2/**))))",
    ),
    (
        "e13",
        "E13 — HTLC with sha256 preimage",
        concat!(
            "wsh(andor(",
            "pk(@0/**),",
            "sha256(b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9),",
            "and_v(v:pk(@1/**),older(144))))",
        ),
    ),
    (
        "e14",
        "E14 — Decaying multisig 3-of-3 to 2-of-3 with 6 distinct keys",
        concat!(
            "wsh(or_d(",
            "multi(3,@0/**,@1/**,@2/**),",
            "and_v(v:older(52560),multi(2,@3/**,@4/**,@5/**))))",
        ),
    ),
    (
        "coldcard",
        "Coldcard — representative BIP 388 export shape (2-of-3 sortedmulti)",
        "wsh(sortedmulti(2,@0/**,@1/**,@2/**))",
    ),
];

/// Schema-2 additions: taproot positive corpus (Phase D — `phase-d-taproot-corpus-fixtures`).
const TAPROOT_FIXTURES: &[(&str, &str, &str)] = &[
    ("tr_keypath", "Taproot key-path-only", "tr(@0/**)"),
    (
        "tr_pk",
        "Taproot single-leaf pk script-path",
        "tr(@0/**,pk(@1/**))",
    ),
    (
        "tr_multia_2of3",
        "Taproot single-leaf multi_a 2-of-3 script-path (4 distinct keys)",
        "tr(@0/**,multi_a(2,@1/**,@2/**,@3/**))",
    ),
];

// ---------------------------------------------------------------------------
// Negative vector fixtures — schema-1 placeholder strings
// ---------------------------------------------------------------------------

/// (id, description, input_strings, expected_error_variant)
///
/// # Provenance and conformance status (v0.1)
///
/// The negative-vector `input_strings` in this fixture array are
/// **representative placeholders**, not programmatically validated round-trip
/// fixtures. They demonstrate the *error class* (each one is a syntactically
/// well-formed WDM-shaped string, or a deliberately malformed one, that maps
/// to the named `expected_error_variant` per the v0.1 spec) but they were not
/// generated by encoding a valid policy and mutating it precisely until the
/// reference decoder returns the named variant.
///
/// Schema-2 (`build_test_vectors_v2`) replaces these placeholders with
/// per-variant generator output. The `NEGATIVE_FIXTURES` array stays as the
/// canonical source for v0.1 (so `v0.1.json` regenerates byte-identical) and
/// supplies the `id`, `description`, and `expected_error_variant` metadata
/// reused by the v0.2 generator.
///
/// What this means for cross-implementation conformance:
///
/// - The `expected_error_variant` field is **the authoritative contract**.
///   Conformance implementations should treat this as the test assertion:
///   "feeding `input_strings` to decode() returns *some* error in the
///   `expected_error_variant` family".
/// - The exact `input_strings` byte sequences may not byte-for-byte match
///   what a different conformance implementation would produce by exercising
///   the same error path. Implementations that need byte-for-byte negative
///   vectors should consume `tests/vectors/v0.2.json` (schema 2), whose
///   negative `input_strings` are produced by the per-variant generators
///   below and therefore round-trip through the reference decoder.
/// - The positive vectors (`Vector` array) are fully validated round-trip
///   fixtures and ARE byte-for-byte authoritative.
///
/// # Fixtures that target lower-level APIs
///
/// Two negative fixtures (`n12`, `n30`) carry empty `input_strings` because
/// the named error cannot be triggered by feeding a string to `decode()`:
///
/// - `n12` (`EmptyChunkList`): requires calling `reassemble_chunks(&[])`
///   directly with an empty slice; `decode()` rejects empty input earlier
///   with a different variant.
/// - `n30` (`PolicyTooLarge`): triggered by `chunking_decision(1693, ChunkingMode::Auto)`
///   directly; the encode pipeline rejects oversized policies before
///   producing a string.
///
/// One additional fixture (`n29`, `PolicyParse`) carries a non-WDM input —
/// `"not_a_valid_policy!!!"` — because the error fires from the policy parse
/// layer (`policy_str.parse::<WalletPolicy>()`), not from the WDM decode
/// pipeline.
///
/// Conformance implementations should test these via the named lower-level
/// API surfaces rather than via decode().
const NEGATIVE_FIXTURES: &[NegativeFixture] = &[
    NegativeFixture {
        id: "n01",
        description: "HRP that is not 'wdm' → InvalidHrp",
        // A valid bech32 string with a non-wdm HRP.
        input_strings: &["bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4"],
        expected_error_variant: "InvalidHrp",
    },
    NegativeFixture {
        id: "n02",
        description: "Mixed-case characters in a WDM string → MixedCase",
        // wdm1 prefix with a mixed-case data character (position 5 uppercased).
        // This is representative; a real implementation generates this by encoding
        // a valid policy then uppercasing one data character.
        input_strings: &["wdm1Qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq"],
        expected_error_variant: "MixedCase",
    },
    NegativeFixture {
        id: "n03",
        description: "String length in reserved 94–95 char range → InvalidStringLength",
        // data-part length 94: 4 (wdm1) + 94 = 98 chars total; InvalidStringLength fires before BCH.
        input_strings: &[
            "wdm1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq",
        ],
        expected_error_variant: "InvalidStringLength",
    },
    NegativeFixture {
        id: "n04",
        description: "Non-bech32 character 'b' in data part → InvalidChar",
        // 'b' is not in the bech32 alphabet.
        input_strings: &["wdm1bqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq"],
        expected_error_variant: "InvalidChar",
    },
    NegativeFixture {
        id: "n05",
        description: "Two character substitutions (BCH uncorrectable) → BchUncorrectable",
        // A string whose data part has 2 corrupted chars — exceeds 1-error correction capacity.
        // The chars at positions 5 and 7 are flipped to values that produce no valid codeword.
        input_strings: &["wdm1pqpqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq"],
        expected_error_variant: "BchUncorrectable",
    },
    NegativeFixture {
        id: "n06",
        description: "Unsupported version byte in chunk header → UnsupportedVersion",
        // Raw chunk bytes: header byte = 0x01 (version=1, not VERSION_0=0x00).
        // Encoded as a fake WDM string — this tests the bytecode layer directly.
        // Note: in practice this error surfaces via decode_string + header parse.
        input_strings: &["wdm1pzry9x0s8q"],
        expected_error_variant: "UnsupportedVersion",
    },
    NegativeFixture {
        id: "n07",
        description: "Unsupported card-type byte in chunk header → UnsupportedCardType",
        input_strings: &["wdm1qqsyqcyr"],
        expected_error_variant: "UnsupportedCardType",
    },
    NegativeFixture {
        id: "n08",
        description: "Reserved wallet-id bits set → ReservedWalletIdBitsSet",
        input_strings: &["wdm1qqs8qnqd2kxs"],
        expected_error_variant: "ReservedWalletIdBitsSet",
    },
    NegativeFixture {
        id: "n09",
        description: "Chunk count = 0 → InvalidChunkCount",
        input_strings: &["wdm1qqsqqqdqaey0"],
        expected_error_variant: "InvalidChunkCount",
    },
    NegativeFixture {
        id: "n10",
        description: "Chunk index ≥ count → InvalidChunkIndex",
        input_strings: &["wdm1qqsqqqcqqlye9"],
        expected_error_variant: "InvalidChunkIndex",
    },
    NegativeFixture {
        id: "n11",
        description: "Chunk header bytes truncated → ChunkHeaderTruncated",
        input_strings: &["wdm1qqy7e3yu"],
        expected_error_variant: "ChunkHeaderTruncated",
    },
    NegativeFixture {
        id: "n12",
        description: "Empty chunk list → EmptyChunkList",
        // Cannot be encoded as a WDM string directly; represented as an empty input set.
        // Conformance implementations should test this via the reassemble_chunks API.
        input_strings: &[],
        expected_error_variant: "EmptyChunkList",
    },
    NegativeFixture {
        id: "n13",
        description: "Single-string chunk appearing more than once → SingleStringWithMultipleChunks",
        // Two copies of the same single-string chunk.
        // Represented via a placeholder; real testing requires two identical strings.
        input_strings: &["wdm1q9x8lhk6", "wdm1q9x8lhk6"],
        expected_error_variant: "SingleStringWithMultipleChunks",
    },
    NegativeFixture {
        id: "n14",
        description: "Mixed SingleString + Chunked in one decode list → MixedChunkTypes",
        input_strings: &["wdm1q9x8lhk6", "wdm1qqs8qnqd2kxs"],
        expected_error_variant: "MixedChunkTypes",
    },
    NegativeFixture {
        id: "n15",
        description: "Wallet-id mismatch across chunks → WalletIdMismatch",
        input_strings: &["wdm1qqsqqqaqqqqqrh06z7", "wdm1qqsqqq9qqqqqrqs8su"],
        expected_error_variant: "WalletIdMismatch",
    },
    NegativeFixture {
        id: "n16",
        description: "Total-chunks mismatch across chunks → TotalChunksMismatch",
        input_strings: &["wdm1qqsqqqaqqqqqrh06z7", "wdm1qqsqqqzqsqqqrw7gxr"],
        expected_error_variant: "TotalChunksMismatch",
    },
    NegativeFixture {
        id: "n17",
        description: "Chunk index out of range → ChunkIndexOutOfRange",
        input_strings: &["wdm1qqsqqq9q9qqqlhj4j4"],
        expected_error_variant: "ChunkIndexOutOfRange",
    },
    NegativeFixture {
        id: "n18",
        description: "Duplicate chunk index in a multi-chunk set → DuplicateChunkIndex",
        input_strings: &["wdm1qqsqqqaqsqqqkjfkf3", "wdm1qqsqqqaqsqqqkjfkf3"],
        expected_error_variant: "DuplicateChunkIndex",
    },
    NegativeFixture {
        id: "n19",
        description: "Missing chunk index in a multi-chunk set → MissingChunkIndex",
        input_strings: &["wdm1qqsqqqaqzqqqehfpja", "wdm1qqsqqqaqzqqsqwjh6e"],
        expected_error_variant: "MissingChunkIndex",
    },
    NegativeFixture {
        id: "n20",
        description: "Cross-chunk integrity hash mismatch → CrossChunkHashMismatch",
        input_strings: &["wdm1qqsqqqaqsqqq9fqxvf", "wdm1qqsqqqaqsqqs9xf8qr"],
        expected_error_variant: "CrossChunkHashMismatch",
    },
    NegativeFixture {
        id: "n21",
        description: "Unknown tag byte 0xC0 in bytecode → InvalidBytecode(UnknownTag)",
        input_strings: &["wdm1qqc0pq48c3n0"],
        expected_error_variant: "InvalidBytecode",
    },
    NegativeFixture {
        id: "n22",
        description: "Bytecode truncated (only header byte) → InvalidBytecode(UnexpectedEnd)",
        input_strings: &["wdm1qqy7e3yu"],
        expected_error_variant: "InvalidBytecode",
    },
    NegativeFixture {
        id: "n23",
        description: "LEB128 varint overflow in bytecode path component → InvalidBytecode(VarintOverflow)",
        input_strings: &["wdm1qqcqp9xqzqzqzqzqzqzqzqzqzqzqzxq0z2fv"],
        expected_error_variant: "InvalidBytecode",
    },
    NegativeFixture {
        id: "n24",
        description: "Trailing bytes after template tree → InvalidBytecode(TrailingBytes)",
        input_strings: &["wdm1qqcqcq3gy0e8wp7w"],
        expected_error_variant: "InvalidBytecode",
    },
    NegativeFixture {
        id: "n25",
        description: "Reserved bits set in bytecode header byte → InvalidBytecode(ReservedBitsSet)",
        input_strings: &["wdm1qrcqcq3ghxxvv7"],
        expected_error_variant: "InvalidBytecode",
    },
    NegativeFixture {
        id: "n26",
        description: "Wrong tag at path-declaration slot → InvalidBytecode(UnexpectedTag)",
        input_strings: &["wdm1qqpqcq3g23pcqd"],
        expected_error_variant: "InvalidBytecode",
    },
    NegativeFixture {
        id: "n27",
        description: "k > n in multi threshold (type-check failure) → InvalidBytecode(TypeCheckFailed)",
        input_strings: &["wdm1qqcqz5pqpq9qr24e3v"],
        expected_error_variant: "InvalidBytecode",
    },
    NegativeFixture {
        id: "n28",
        description: "Non-Wsh top-level descriptor → PolicyScopeViolation",
        input_strings: &["wdm1qqcqpq3g3p7wpm5"],
        expected_error_variant: "PolicyScopeViolation",
    },
    NegativeFixture {
        id: "n29",
        description: "Malformed policy string (no valid descriptor) → PolicyParse",
        // This tests the policy parse layer; no WDM string exists — callers use the
        // policy.parse::<WalletPolicy>() API path.
        input_strings: &["not_a_valid_policy!!!"],
        expected_error_variant: "PolicyParse",
    },
    NegativeFixture {
        id: "n30",
        description: "Bytecode larger than 1692 bytes → PolicyTooLarge",
        // Synthetic path: chunking_decision(1693, ChunkingMode::Auto) returns this error.
        // No WDM string exists for this case — callers use chunking_decision directly.
        input_strings: &[],
        expected_error_variant: "PolicyTooLarge",
    },
];

// ---------------------------------------------------------------------------
// Internal negative fixture type (not part of public schema)
// ---------------------------------------------------------------------------

struct NegativeFixture {
    id: &'static str,
    description: &'static str,
    input_strings: &'static [&'static str],
    expected_error_variant: &'static str,
}

// ---------------------------------------------------------------------------
// Public generator (Tasks 8.2, 8.3)
// ---------------------------------------------------------------------------

/// Build the schema-1 [`TestVectorFile`] (v0.1.0 lock).
///
/// Output is byte-frozen against `tests/vectors/v0.1.json` (SHA-256
/// `1957b542ed0388b51f01a7b467c8e802942dc6d6507abffaefaf777c90f3cd2c`). Any
/// change here is a release-engineering incident; new test material goes
/// into [`build_test_vectors_v2`].
///
/// Both `gen_vectors --output --schema 1` and `wdm vectors` call this
/// function; there is no code duplication. Output is deterministic: calling
/// this function twice returns structurally equal values.
pub fn build_test_vectors() -> TestVectorFile {
    build_test_vectors_v1()
}

/// Build the schema-1 [`TestVectorFile`] (v0.1.0 lock).
///
/// Alias retained alongside [`build_test_vectors`] for symmetry with
/// [`build_test_vectors_v2`]. The two names are byte-identical in output;
/// new code SHOULD prefer the explicit `_v1` form so the schema bump is
/// visible at the call site.
pub fn build_test_vectors_v1() -> TestVectorFile {
    TestVectorFile {
        schema_version: 1,
        generator: format!("wdm-codec {}", env!("CARGO_PKG_VERSION")),
        vectors: build_positive_vectors_v1(),
        negative_vectors: build_negative_vectors_v1(),
    }
}

/// Build the schema-2 [`TestVectorFile`] (v0.2.0 lock).
///
/// Schema 2 is a strict superset of schema 1:
///
/// - All schema-1 positive vectors are present, byte-identical.
/// - The taproot positive vectors (`tr_keypath`, `tr_pk`, `tr_multia_2of3`)
///   are appended.
/// - The fingerprints positive vector (`multi_2of2_with_fingerprints`) is
///   appended, populating [`Vector::expected_fingerprints_hex`] and
///   [`Vector::encode_options_fingerprints`].
/// - All schema-1 negative variants are preserved (same `id`,
///   `description`, `expected_error_variant`) but their `input_strings`
///   are regenerated by per-variant generators that exercise the named
///   error path through the reference decoder. Each gets a `provenance`
///   string describing the construction recipe.
/// - New negative vectors target the v0.2 surface:
///   `n_tap_leaf_subset`, `n_taptree_multi_leaf`,
///   `n_fingerprints_count_mismatch`, `n_fingerprints_missing_tag`.
///
/// Output is deterministic: same code → same JSON, byte-for-byte.
pub fn build_test_vectors_v2() -> TestVectorFile {
    TestVectorFile {
        schema_version: 2,
        generator: format!("wdm-codec {}", env!("CARGO_PKG_VERSION")),
        vectors: build_positive_vectors_v2(),
        negative_vectors: build_negative_vectors_v2(),
    }
}

// ---------------------------------------------------------------------------
// Schema-1 builders (preserved verbatim from v0.1.0 lock)
// ---------------------------------------------------------------------------

fn build_positive_vectors_v1() -> Vec<Vector> {
    let mut out = Vec::with_capacity(CORPUS_FIXTURES.len());
    for &(id, description, policy_str) in CORPUS_FIXTURES {
        out.push(build_default_positive_vector(id, description, policy_str));
    }
    out
}

fn build_negative_vectors_v1() -> Vec<NegativeVector> {
    NEGATIVE_FIXTURES
        .iter()
        .map(|f| NegativeVector {
            id: f.id.to_string(),
            description: f.description.to_string(),
            input_strings: f.input_strings.iter().map(|s| s.to_string()).collect(),
            expected_error_variant: f.expected_error_variant.to_string(),
            provenance: None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Schema-2 builders
// ---------------------------------------------------------------------------

fn build_positive_vectors_v2() -> Vec<Vector> {
    let mut out = Vec::with_capacity(CORPUS_FIXTURES.len() + TAPROOT_FIXTURES.len() + 1);
    for &(id, description, policy_str) in CORPUS_FIXTURES {
        out.push(build_default_positive_vector(id, description, policy_str));
    }
    for &(id, description, policy_str) in TAPROOT_FIXTURES {
        out.push(build_default_positive_vector(id, description, policy_str));
    }
    out.push(build_fingerprints_positive_vector());
    out
}

fn build_negative_vectors_v2() -> Vec<NegativeVector> {
    let mut out: Vec<NegativeVector> = Vec::with_capacity(NEGATIVE_FIXTURES.len() + 4);
    for fixture in NEGATIVE_FIXTURES {
        let (input_strings, provenance) = generate_for_negative_variant(fixture.id);
        out.push(NegativeVector {
            id: fixture.id.to_string(),
            description: fixture.description.to_string(),
            input_strings,
            expected_error_variant: fixture.expected_error_variant.to_string(),
            provenance: Some(provenance),
        });
    }
    // v0.2 additions — taproot.
    out.push(build_negative_n_tap_leaf_subset());
    out.push(build_negative_n_taptree_multi_leaf());
    // v0.2 additions — fingerprints.
    out.push(build_negative_n_fingerprints_count_mismatch());
    out.push(build_negative_n_fingerprints_missing_tag());
    out
}

// ---------------------------------------------------------------------------
// Positive-vector helpers
// ---------------------------------------------------------------------------

/// Encode a default-options positive vector (no fingerprints, no shared-path
/// override). Used for both schema-1 and schema-2 corpus + taproot entries.
fn build_default_positive_vector(id: &str, description: &str, policy_str: &str) -> Vector {
    let policy: WalletPolicy = policy_str
        .parse()
        .unwrap_or_else(|e| panic!("vector builder: failed to parse corpus policy {id:?}: {e}"));

    let bytecode = policy
        .to_bytecode(&EncodeOptions::default())
        .unwrap_or_else(|e| panic!("vector builder: failed to encode bytecode for {id:?}: {e}"));

    let expected_bytecode_hex = bytes_to_lower_hex(&bytecode);

    let opts = EncodeOptions::default();
    let backup = encode(&policy, &opts)
        .unwrap_or_else(|e| panic!("vector builder: encode failed for {id:?}: {e}"));

    let expected_chunks: Vec<String> = backup.chunks.iter().map(|c| c.raw.clone()).collect();

    let expected_wallet_id_words: Vec<String> = backup
        .wallet_id_words
        .to_string()
        .split_whitespace()
        .map(str::to_string)
        .collect();

    Vector {
        id: id.to_string(),
        description: description.to_string(),
        policy: policy_str.to_string(),
        expected_bytecode_hex,
        expected_chunks,
        expected_wallet_id_words,
        expected_fingerprints_hex: None,
        encode_options_fingerprints: None,
    }
}

/// Build the schema-2 fingerprints positive vector: `wsh(multi(2,@0/**,@1/**))`
/// encoded with `[deadbeef, cafebabe]`.
fn build_fingerprints_positive_vector() -> Vector {
    let id = "multi_2of2_with_fingerprints";
    let description = "wsh(multi(2,...)) with two master-key fingerprints (Phase E)";
    let policy_str = "wsh(multi(2,@0/**,@1/**))";

    let raw_fps: Vec<[u8; 4]> = vec![[0xde, 0xad, 0xbe, 0xef], [0xca, 0xfe, 0xba, 0xbe]];
    let fingerprints: Vec<Fingerprint> = raw_fps.iter().copied().map(Fingerprint::from).collect();

    let policy: WalletPolicy = policy_str
        .parse()
        .unwrap_or_else(|e| panic!("vector builder: failed to parse fingerprints policy: {e}"));

    let opts = EncodeOptions::default().with_fingerprints(fingerprints.clone());

    let bytecode = policy
        .to_bytecode(&opts)
        .unwrap_or_else(|e| panic!("vector builder: fingerprints to_bytecode failed: {e}"));
    let expected_bytecode_hex = bytes_to_lower_hex(&bytecode);

    let backup = encode(&policy, &opts)
        .unwrap_or_else(|e| panic!("vector builder: fingerprints encode failed: {e}"));
    let expected_chunks: Vec<String> = backup.chunks.iter().map(|c| c.raw.clone()).collect();
    let expected_wallet_id_words: Vec<String> = backup
        .wallet_id_words
        .to_string()
        .split_whitespace()
        .map(str::to_string)
        .collect();

    let expected_fingerprints_hex: Vec<String> = raw_fps.iter().map(bytes_to_lower_hex_4).collect();

    Vector {
        id: id.to_string(),
        description: description.to_string(),
        policy: policy_str.to_string(),
        expected_bytecode_hex,
        expected_chunks,
        expected_wallet_id_words,
        expected_fingerprints_hex: Some(expected_fingerprints_hex),
        encode_options_fingerprints: Some(raw_fps),
    }
}

// ---------------------------------------------------------------------------
// Per-variant negative generators (Phase F — F-4)
// ---------------------------------------------------------------------------

/// Dispatch from a schema-1 fixture id to its schema-2 generator.
/// Returns `(input_strings, provenance)`.
fn generate_for_negative_variant(id: &str) -> (Vec<String>, String) {
    match id {
        "n01" => generate_n01_invalid_hrp(),
        "n02" => generate_n02_mixed_case(),
        "n03" => generate_n03_invalid_string_length(),
        "n04" => generate_n04_invalid_char(),
        "n05" => generate_n05_bch_uncorrectable(),
        "n06" => generate_n06_unsupported_version(),
        "n07" => generate_n07_unsupported_card_type(),
        "n08" => generate_n08_reserved_wallet_id_bits_set(),
        "n09" => generate_n09_invalid_chunk_count(),
        "n10" => generate_n10_invalid_chunk_index(),
        "n11" => generate_n11_chunk_header_truncated(),
        "n12" => generate_n12_empty_chunk_list(),
        "n13" => generate_n13_single_string_with_multiple_chunks(),
        "n14" => generate_n14_mixed_chunk_types(),
        "n15" => generate_n15_wallet_id_mismatch(),
        "n16" => generate_n16_total_chunks_mismatch(),
        "n17" => generate_n17_chunk_index_out_of_range(),
        "n18" => generate_n18_duplicate_chunk_index(),
        "n19" => generate_n19_missing_chunk_index(),
        "n20" => generate_n20_cross_chunk_hash_mismatch(),
        "n21" => generate_n21_invalid_bytecode_unknown_tag(),
        "n22" => generate_n22_invalid_bytecode_unexpected_end(),
        "n23" => generate_n23_invalid_bytecode_varint_overflow(),
        "n24" => generate_n24_invalid_bytecode_trailing_bytes(),
        "n25" => generate_n25_invalid_bytecode_reserved_bits_set(),
        "n26" => generate_n26_invalid_bytecode_unexpected_tag(),
        "n27" => generate_n27_invalid_bytecode_type_check_failed(),
        "n28" => generate_n28_policy_scope_violation(),
        "n29" => generate_n29_policy_parse(),
        "n30" => generate_n30_policy_too_large(),
        other => panic!("vector builder: no generator for negative variant id {other:?}"),
    }
}

// Stage 1 (per-string parse) ------------------------------------------------

fn generate_n01_invalid_hrp() -> (Vec<String>, String) {
    let s = "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4".to_string();
    debug_assert_decode_matches(&[s.as_str()], "InvalidHrp");
    (
        vec![s],
        "BIP 173 bech32 string with HRP `bc` (a Bitcoin segwit-v0 address); decode rejects at HRP check"
            .to_string(),
    )
}

fn generate_n02_mixed_case() -> (Vec<String>, String) {
    let raw = encode_simple_pk_chunk();
    let mut chars: Vec<char> = raw.chars().collect();
    chars[5] = chars[5].to_ascii_uppercase();
    let mixed: String = chars.into_iter().collect();
    debug_assert_decode_matches(&[mixed.as_str()], "MixedCase");
    (
        vec![mixed],
        "encoded `wsh(pk(@0/**))`, then uppercased the data character at position 5".to_string(),
    )
}

fn generate_n03_invalid_string_length() -> (Vec<String>, String) {
    let data: String = "q".repeat(94);
    let s = format!("wdm1{data}");
    debug_assert_decode_matches(&[s.as_str()], "InvalidStringLength");
    (
        vec![s],
        "constructed `wdm1` + 94 `q` chars; the 94..=95 data-part length range is reserved-invalid in WDM"
            .to_string(),
    )
}

fn generate_n04_invalid_char() -> (Vec<String>, String) {
    let raw = encode_simple_pk_chunk();
    let mut chars: Vec<char> = raw.chars().collect();
    chars[5] = 'b';
    let bad: String = chars.into_iter().collect();
    debug_assert_decode_matches(&[bad.as_str()], "InvalidChar");
    (
        vec![bad],
        "encoded `wsh(pk(@0/**))`, then replaced the data character at position 5 with `b` (not in the bech32 alphabet)"
            .to_string(),
    )
}

fn generate_n05_bch_uncorrectable() -> (Vec<String>, String) {
    // Encode a valid policy, then flip 5 characters in the data part. v0.2's
    // BCH layer can correct up to 4 substitutions; 5 is uncorrectable.
    let raw = encode_simple_pk_chunk();
    let mut chars: Vec<char> = raw.chars().collect();
    // Pick 5 positions well inside the data part (first valid data position
    // is 4, just after the `wdm1` separator). Avoid the checksum tail by
    // staying within the first 12 data chars (the encoded chunk is short
    // but stable: we only mutate chars in positions 4..=8 + 10).
    for pos in [4, 5, 6, 7, 8] {
        chars[pos] = if chars[pos] == 'q' { 'p' } else { 'q' };
    }
    let corrupted: String = chars.into_iter().collect();
    debug_assert_decode_matches(&[corrupted.as_str()], "BchUncorrectable");
    (
        vec![corrupted],
        "encoded `wsh(pk(@0/**))`, then flipped 5 data characters (positions 4..=8); exceeds the v0.2 BCH t=4 correction radius"
            .to_string(),
    )
}

// Stage 2/3 (chunk-header parse) -------------------------------------------

fn generate_n06_unsupported_version() -> (Vec<String>, String) {
    // Header byte 0x01 = version 1 (only version 0 is supported in v0.x).
    // Pair with a single zero payload byte so the chunk header is parsable
    // up to the version check. Total: [0x01, 0x00] → SingleString with
    // version=1.
    let s = encode_string_from_bytes(&[0x01, 0x00]);
    debug_assert_decode_matches(&[s.as_str()], "UnsupportedVersion");
    (
        vec![s],
        "encoded a 2-byte chunk-header buffer `[0x01, 0x00]` (version=1, type=SingleString) via `encoding::encode_string`; chunk-header parse rejects the unsupported version"
            .to_string(),
    )
}

fn generate_n07_unsupported_card_type() -> (Vec<String>, String) {
    // Header bytes [0x00, 0x02] = version 0, type=2 (unknown card type).
    let s = encode_string_from_bytes(&[0x00, 0x02]);
    debug_assert_decode_matches(&[s.as_str()], "UnsupportedCardType");
    (
        vec![s],
        "encoded a 2-byte chunk-header buffer `[0x00, 0x02]` (version=0, type=2 unknown) via `encoding::encode_string`; chunk-header parse rejects the unsupported card type"
            .to_string(),
    )
}

fn generate_n08_reserved_wallet_id_bits_set() -> (Vec<String>, String) {
    // 7-byte chunked header with the wallet-id top nibble set: [ver=0,
    // type=1 (Chunked), wid first byte = 0x10, 0x00, 0x00, count=1, index=0].
    let s = encode_string_from_bytes(&[0x00, 0x01, 0x10, 0x00, 0x00, 0x01, 0x00]);
    debug_assert_decode_matches(&[s.as_str()], "ReservedWalletIdBitsSet");
    (
        vec![s],
        "encoded chunked-header bytes with the wallet-id high nibble set (0x10 in the wid first byte); chunk-header parse rejects the reserved bits"
            .to_string(),
    )
}

fn generate_n09_invalid_chunk_count() -> (Vec<String>, String) {
    // 7-byte chunked header with count=0: [0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00]
    let s = encode_string_from_bytes(&[0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00]);
    debug_assert_decode_matches(&[s.as_str()], "InvalidChunkCount");
    (
        vec![s],
        "encoded chunked-header bytes with count byte = 0 (must be 1..=32); chunk-header parse rejects"
            .to_string(),
    )
}

fn generate_n10_invalid_chunk_index() -> (Vec<String>, String) {
    // Chunked header with count=3, index=3 (index >= count).
    let s = encode_string_from_bytes(&[0x00, 0x01, 0x00, 0x00, 0x00, 0x03, 0x03]);
    debug_assert_decode_matches(&[s.as_str()], "InvalidChunkIndex");
    (
        vec![s],
        "encoded chunked-header bytes with index=3 and count=3 (index must be < count); chunk-header parse rejects"
            .to_string(),
    )
}

fn generate_n11_chunk_header_truncated() -> (Vec<String>, String) {
    // Just a single header byte (0x00) with no payload — chunk-header parse
    // requires at least 2 bytes for SingleString, 7 for Chunked.
    let s = encode_string_from_bytes(&[0x00]);
    debug_assert_decode_matches(&[s.as_str()], "ChunkHeaderTruncated");
    (
        vec![s],
        "encoded a 1-byte chunk-header buffer `[0x00]` via `encoding::encode_string`; chunk-header parse needs at least 2 bytes (SingleString) or 7 (Chunked)"
            .to_string(),
    )
}

fn generate_n12_empty_chunk_list() -> (Vec<String>, String) {
    debug_assert_reassemble_empty_matches();
    (
        Vec::new(),
        "requires lower-level API: `chunking::reassemble_chunks(&[])` rejects an empty slice with `EmptyChunkList`; `decode()` rejects `&[]` earlier with a different variant"
            .to_string(),
    )
}

// Stage 4 (reassembly) ------------------------------------------------------

fn generate_n13_single_string_with_multiple_chunks() -> (Vec<String>, String) {
    // Encode a valid wsh(pk) policy that fits in a single string, then
    // duplicate it. `decode()` runs through the codex32 layer first, so the
    // duplication surfaces at reassembly as SingleStringWithMultipleChunks.
    let raw = encode_simple_pk_chunk();
    let inputs = vec![raw.clone(), raw];
    debug_assert_decode_matches(
        &inputs.iter().map(String::as_str).collect::<Vec<_>>(),
        "SingleStringWithMultipleChunks",
    );
    (
        inputs,
        "encoded `wsh(pk(@0/**))` to a single SingleString chunk, then submitted the same string twice; reassembly rejects"
            .to_string(),
    )
}

fn generate_n14_mixed_chunk_types() -> (Vec<String>, String) {
    // Build a SingleString and a Chunked chunk. Use forced chunking on a
    // policy small enough to fit Chunked, then mix.
    use crate::chunking::ChunkingMode;

    let policy: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();

    // SingleString version (default opts, single chunk).
    let single = encode(&policy, &EncodeOptions::default()).unwrap();
    let single_raw = single.chunks[0].raw.clone();

    // Force-chunked version. With the same policy, the chunk header is the
    // 7-byte Chunked variant.
    let opts_forced = EncodeOptions {
        chunking_mode: ChunkingMode::ForceChunked,
        ..Default::default()
    };
    let chunked = encode(&policy, &opts_forced).unwrap();
    let chunked_raw = chunked.chunks[0].raw.clone();

    let inputs = vec![single_raw, chunked_raw];
    debug_assert_decode_matches(
        &inputs.iter().map(String::as_str).collect::<Vec<_>>(),
        "MixedChunkTypes",
    );
    (
        inputs,
        "encoded `wsh(pk(@0/**))` once with default options (SingleString) and once with `ChunkingMode::ForceChunked`, then submitted both chunks together; reassembly rejects mixed types"
            .to_string(),
    )
}

fn generate_n15_wallet_id_mismatch() -> (Vec<String>, String) {
    use crate::chunking::ChunkingMode;
    use crate::wallet_id::WalletIdSeed;

    // Encode the same multi-chunk policy under two distinct `wallet_id_seed`
    // overrides; then submit chunk 0 from encoding A together with chunk 1
    // from encoding B. The chunk-header layer accepts both (each chunk is
    // self-consistent), but reassembly's wallet-id consistency check rejects
    // the cross-encoding mix. We use C5 (the largest corpus policy) under
    // ForceChunked so the chunking plan produces 2+ chunks.
    let large_policy: WalletPolicy = CORPUS_FIXTURES
        .iter()
        .find(|(id, _, _)| *id == "c5")
        .map(|(_, _, p)| p.parse().unwrap())
        .unwrap();
    let opts_a = EncodeOptions {
        chunking_mode: ChunkingMode::ForceChunked,
        wallet_id_seed: Some(WalletIdSeed::from(0xAAAA_AAAAu32)),
        ..Default::default()
    };
    let opts_b = EncodeOptions {
        chunking_mode: ChunkingMode::ForceChunked,
        wallet_id_seed: Some(WalletIdSeed::from(0xBBBB_BBBBu32)),
        ..Default::default()
    };
    let backup_a = encode(&large_policy, &opts_a).unwrap();
    let backup_b = encode(&large_policy, &opts_b).unwrap();

    let chunk0 = backup_a.chunks[0].raw.clone();
    let chunk1 = backup_b.chunks[1].raw.clone();
    let inputs = vec![chunk0, chunk1];
    debug_assert_decode_matches(
        &inputs.iter().map(String::as_str).collect::<Vec<_>>(),
        "WalletIdMismatch",
    );
    (
        inputs,
        "encoded the C5 corpus policy twice with distinct `wallet_id_seed` overrides under `ChunkingMode::ForceChunked`, then submitted chunk 0 from encoding A together with chunk 1 from encoding B; reassembly rejects the wallet-id mismatch"
            .to_string(),
    )
}

fn generate_n16_total_chunks_mismatch() -> (Vec<String>, String) {
    // Build two raw chunks via the chunking API with mismatched count
    // fields, then encode each chunk's bytes via `encode_string`.
    use crate::chunking::ChunkHeader;
    use crate::wallet_id::ChunkWalletId;

    let wid = ChunkWalletId::new(0x12345);
    // Chunk 0: count=2, index=0, payload=[0x01]
    let c0 = encoded_from_header_and_fragment(
        ChunkHeader::Chunked {
            version: 0,
            wallet_id: wid,
            count: 2,
            index: 0,
        },
        &[0x01],
    );
    // Chunk 1: count=3 (mismatch), index=1, payload=[0x02]
    let c1 = encoded_from_header_and_fragment(
        ChunkHeader::Chunked {
            version: 0,
            wallet_id: wid,
            count: 3,
            index: 1,
        },
        &[0x02],
    );
    // Note: EncodedChunk is just (raw, header, fragment) so we use raw.
    let inputs = vec![c0.raw, c1.raw];
    debug_assert_decode_matches(
        &inputs.iter().map(String::as_str).collect::<Vec<_>>(),
        "TotalChunksMismatch",
    );
    (
        inputs,
        "synthesised two Chunked chunks with the same wallet-id but different `count` headers (2 vs 3); reassembly rejects"
            .to_string(),
    )
}

fn generate_n17_chunk_index_out_of_range() -> (Vec<String>, String) {
    // ChunkIndexOutOfRange fires only via the `Chunk::new` bypass +
    // `reassemble_chunks` path; via a WDM string, `ChunkHeader::from_bytes`
    // rejects index >= count earlier with `InvalidChunkIndex`. So the
    // input_strings list is intentionally empty; conformance
    // implementations test this variant via the named lower-level API.
    (
        Vec::new(),
        "requires lower-level API: `Chunk::new` (bypass) + `reassemble_chunks` triggers `ChunkIndexOutOfRange`; via a WDM string, `ChunkHeader::from_bytes` rejects index>=count earlier with `InvalidChunkIndex` instead"
            .to_string(),
    )
}

fn generate_n18_duplicate_chunk_index() -> (Vec<String>, String) {
    // Two chunks with the same wallet-id and same index=0 (count=2) →
    // reassembly rejects with DuplicateChunkIndex.
    use crate::chunking::ChunkHeader;
    use crate::wallet_id::ChunkWalletId;

    let wid = ChunkWalletId::new(0x0001);
    let c0a = encoded_from_header_and_fragment(
        ChunkHeader::Chunked {
            version: 0,
            wallet_id: wid,
            count: 2,
            index: 0,
        },
        &[0x01],
    );
    let c0b = encoded_from_header_and_fragment(
        ChunkHeader::Chunked {
            version: 0,
            wallet_id: wid,
            count: 2,
            index: 0,
        },
        &[0x02],
    );
    let inputs = vec![c0a.raw, c0b.raw];
    debug_assert_decode_matches(
        &inputs.iter().map(String::as_str).collect::<Vec<_>>(),
        "DuplicateChunkIndex",
    );
    (
        inputs,
        "synthesised two Chunked chunks with identical wallet-id, count=2, and index=0 (different fragments); reassembly rejects the duplicate index"
            .to_string(),
    )
}

fn generate_n19_missing_chunk_index() -> (Vec<String>, String) {
    // Claim count=3 but supply only indices 0 and 2 → MissingChunkIndex(1).
    use crate::chunking::ChunkHeader;
    use crate::wallet_id::ChunkWalletId;

    let wid = ChunkWalletId::new(0x0010);
    let c0 = encoded_from_header_and_fragment(
        ChunkHeader::Chunked {
            version: 0,
            wallet_id: wid,
            count: 3,
            index: 0,
        },
        &[0x01],
    );
    let c2 = encoded_from_header_and_fragment(
        ChunkHeader::Chunked {
            version: 0,
            wallet_id: wid,
            count: 3,
            index: 2,
        },
        &[0x03],
    );
    let inputs = vec![c0.raw, c2.raw];
    debug_assert_decode_matches(
        &inputs.iter().map(String::as_str).collect::<Vec<_>>(),
        "MissingChunkIndex",
    );
    (
        inputs,
        "synthesised two Chunked chunks claiming count=3 with indices [0, 2] (index 1 absent); reassembly rejects"
            .to_string(),
    )
}

fn generate_n20_cross_chunk_hash_mismatch() -> (Vec<String>, String) {
    use crate::chunking::{ChunkCode, ChunkingPlan, chunk_bytes};
    use crate::wallet_id::ChunkWalletId;

    // Build a synthetic 50-byte bytecode and a deterministic 2-chunk plan;
    // chunk it, then corrupt the first byte of the last fragment. The
    // tail's 4-byte SHA-256 hash will mismatch the reassembled bytecode.
    let bytecode: Vec<u8> = (0u8..50).collect();
    let plan = ChunkingPlan::Chunked {
        code: ChunkCode::Regular,
        fragment_size: 45,
        count: 2,
    };
    let wid = ChunkWalletId::new(0xABCDE);
    let mut chunks = chunk_bytes(&bytecode, plan, wid).unwrap();
    chunks.last_mut().unwrap().fragment[0] ^= 0xFF;

    // Re-encode each (header, fragment) into a WDM string.
    let mut inputs: Vec<String> = Vec::with_capacity(chunks.len());
    for ch in &chunks {
        let encoded = encoded_from_header_and_fragment(ch.header, &ch.fragment);
        inputs.push(encoded.raw);
    }
    debug_assert_decode_matches(
        &inputs.iter().map(String::as_str).collect::<Vec<_>>(),
        "CrossChunkHashMismatch",
    );
    (
        inputs,
        "chunked a 50-byte synthetic bytecode into a 2-chunk plan, flipped one bit in the last fragment, then re-encoded each chunk; reassembly rejects on the cross-chunk SHA-256 mismatch"
            .to_string(),
    )
}

// Stage 5 (bytecode parse) --------------------------------------------------

fn generate_n21_invalid_bytecode_unknown_tag() -> (Vec<String>, String) {
    // Bytecode: header=0x00, then 0xC0 (unknown tag) where SharedPath
    // (0x33) is expected. Wrapped in a SingleString chunk header.
    let bytecode = [0x00u8, 0xC0, 0x03, 0x05, 0x32, 0x00];
    let s = encode_singlestring_around(&bytecode);
    debug_assert_decode_matches(&[s.as_str()], "InvalidBytecode");
    (
        vec![s],
        "synthesised bytecode `[0x00, 0xC0, ...]` (unknown tag 0xC0 at the path-declaration slot), wrapped in a SingleString chunk; bytecode parse rejects"
            .to_string(),
    )
}

fn generate_n22_invalid_bytecode_unexpected_end() -> (Vec<String>, String) {
    // Just the bytecode header byte 0x00; cursor hits end while reading
    // the path declaration tag.
    let bytecode = [0x00u8];
    let s = encode_singlestring_around(&bytecode);
    debug_assert_decode_matches(&[s.as_str()], "InvalidBytecode");
    (
        vec![s],
        "synthesised a 1-byte bytecode payload `[0x00]` (header only, no path declaration), wrapped in a SingleString chunk; bytecode parse rejects with UnexpectedEnd"
            .to_string(),
    )
}

fn generate_n23_invalid_bytecode_varint_overflow() -> (Vec<String>, String) {
    // bytecode-header(0x00) + SharedPath(0x33) + explicit-path(0xFE) +
    // count(0x01) + 11 LEB128 continuation bytes (overflows u64).
    let mut bytecode: Vec<u8> = vec![0x00u8, 0x33, 0xFE, 0x01];
    bytecode.extend_from_slice(&[0x80u8; 11]);
    let s = encode_singlestring_around(&bytecode);
    debug_assert_decode_matches(&[s.as_str()], "InvalidBytecode");
    (
        vec![s],
        "synthesised an explicit-path declaration with 11 LEB128 continuation bytes (`[0x80;11]`) that never terminates, wrapped in a SingleString chunk; bytecode parse rejects with VarintOverflow"
            .to_string(),
    )
}

fn generate_n24_invalid_bytecode_trailing_bytes() -> (Vec<String>, String) {
    // Encode a valid policy and append a trailing 0xFF byte.
    let policy: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
    let mut bytecode = policy.to_bytecode(&EncodeOptions::default()).unwrap();
    bytecode.push(0xFF);
    let s = encode_singlestring_around(&bytecode);
    debug_assert_decode_matches(&[s.as_str()], "InvalidBytecode");
    (
        vec![s],
        "encoded `wsh(pk(@0/**))` to bytecode, appended a trailing `0xFF` byte, wrapped in a SingleString chunk; bytecode parse rejects with TrailingBytes"
            .to_string(),
    )
}

fn generate_n25_invalid_bytecode_reserved_bits_set() -> (Vec<String>, String) {
    // Bytecode header byte 0x01: reserved bit 0 set.
    let bytecode = [0x01u8, 0x33, 0x03, 0x05, 0x32, 0x00];
    let s = encode_singlestring_around(&bytecode);
    debug_assert_decode_matches(&[s.as_str()], "InvalidBytecode");
    (
        vec![s],
        "synthesised bytecode header `0x01` (reserved bit 0 set), wrapped in a SingleString chunk; bytecode parse rejects with ReservedBitsSet before reading the path declaration"
            .to_string(),
    )
}

fn generate_n26_invalid_bytecode_unexpected_tag() -> (Vec<String>, String) {
    // Path-declaration slot expects Tag::SharedPath (0x33); supply Tag::Wsh
    // (0x05) instead.
    let bytecode: Vec<u8> = vec![0x00, 0x05, 0x03, 0x05, 0x32, 0x00];
    let s = encode_singlestring_around(&bytecode);
    debug_assert_decode_matches(&[s.as_str()], "InvalidBytecode");
    (
        vec![s],
        "synthesised bytecode where the path-declaration slot carries Tag::Wsh (0x05) instead of Tag::SharedPath (0x33), wrapped in a SingleString chunk; bytecode parse rejects with UnexpectedTag"
            .to_string(),
    )
}

fn generate_n27_invalid_bytecode_type_check_failed() -> (Vec<String>, String) {
    // multi(k=5, n=2, @0, @1) — k > n triggers a miniscript type-check
    // failure during Wsh::new(...).
    use crate::bytecode::Tag;
    let bytecode: Vec<u8> = vec![
        0x00,
        Tag::SharedPath.as_byte(),
        0x03,
        Tag::Wsh.as_byte(),
        Tag::Multi.as_byte(),
        0x05, // k=5
        0x02, // n=2
        Tag::Placeholder.as_byte(),
        0x00,
        Tag::Placeholder.as_byte(),
        0x01,
    ];
    let s = encode_singlestring_around(&bytecode);
    debug_assert_decode_matches(&[s.as_str()], "InvalidBytecode");
    (
        vec![s],
        "synthesised bytecode for `wsh(multi(k=5, n=2, @0, @1))` (k > n), wrapped in a SingleString chunk; bytecode parse rejects with TypeCheckFailed"
            .to_string(),
    )
}

fn generate_n28_policy_scope_violation() -> (Vec<String>, String) {
    // Top-level Tag::Tr in v0.1 scope is rejected as a PolicyScopeViolation
    // *for the v0.1 builder*, but Phase D promoted Tr to a recognised
    // top-level tag in v0.2. To keep the v0.2 negative vector meaningful,
    // construct an inner-fragment Tr (Tag::Tr appearing inside Wsh) which
    // is still rejected as PolicyScopeViolation in v0.2.
    use crate::bytecode::Tag;
    let policy: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
    let valid = policy.to_bytecode(&EncodeOptions::default()).unwrap();
    let wsh_pos = valid
        .iter()
        .position(|&b| b == Tag::Wsh.as_byte())
        .expect("encoded wsh must contain Tag::Wsh");
    let mut bytecode = valid[..=wsh_pos].to_vec();
    // After Wsh, place Tr + Placeholder + idx 0 to simulate a nested tr().
    bytecode.extend_from_slice(&[Tag::Tr.as_byte(), Tag::Placeholder.as_byte(), 0x00]);
    let s = encode_singlestring_around(&bytecode);
    debug_assert_decode_matches(&[s.as_str()], "PolicyScopeViolation");
    (
        vec![s],
        "encoded `wsh(pk(@0/**))`, replaced the inner Wsh content with `[Tag::Tr, Tag::Placeholder, 0x00]` (a Tr nested inside Wsh, which v0.2 still rejects), wrapped in a SingleString chunk; decode emits PolicyScopeViolation"
            .to_string(),
    )
}

fn generate_n29_policy_parse() -> (Vec<String>, String) {
    let s = "not_a_valid_policy!!!".to_string();
    // Sanity: confirm the policy parser rejects this. We don't run decode()
    // since the variant fires from `WalletPolicy::from_str`, not from a
    // WDM-string decode. The string is included so `input_strings` has the
    // exact byte sequence the user would feed to `parse::<WalletPolicy>`.
    debug_assert!(
        s.parse::<WalletPolicy>().is_err(),
        "PolicyParse generator: parse did NOT fail"
    );
    (
        vec![s],
        "passed the literal string `not_a_valid_policy!!!` to `WalletPolicy::from_str`; the BIP 388 parser rejects (this fixture exercises the policy-parse layer, not the WDM-string decode pipeline)"
            .to_string(),
    )
}

fn generate_n30_policy_too_large() -> (Vec<String>, String) {
    use crate::chunking::{ChunkingMode, chunking_decision};
    debug_assert!(
        matches!(
            chunking_decision(1693, ChunkingMode::Auto),
            Err(crate::Error::PolicyTooLarge { .. })
        ),
        "PolicyTooLarge generator: chunking_decision did NOT reject 1693"
    );
    (
        Vec::new(),
        "requires lower-level API: `chunking::chunking_decision(1693, ChunkingMode::Auto)` rejects bytecode lengths above the 1692-byte v0.1 cap; no WDM string encodes the oversized condition"
            .to_string(),
    )
}

// ---------------------------------------------------------------------------
// Phase D / Phase E — v0.2 additions
// ---------------------------------------------------------------------------

fn build_negative_n_tap_leaf_subset() -> NegativeVector {
    let policy: WalletPolicy =
        "tr(@0/**,and_v(v:sha256(b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9),pk(@1/**)))"
            .parse()
            .expect("tap-leaf subset policy must parse");
    debug_assert!(matches!(
        policy.to_bytecode(&EncodeOptions::default()),
        Err(crate::Error::TapLeafSubsetViolation { .. })
    ));
    NegativeVector {
        id: "n_tap_leaf_subset".to_string(),
        description:
            "Taproot leaf miniscript uses `sha256` (not in the Coldcard subset) → TapLeafSubsetViolation"
                .to_string(),
        input_strings: Vec::new(),
        expected_error_variant: "TapLeafSubsetViolation".to_string(),
        provenance: Some(
            "encode-side rejection; `input_strings` is empty because the policy never produces a WDM string. \
             Construct via `\"tr(@0/**,and_v(v:sha256(<32B>),pk(@1/**)))\".parse::<WalletPolicy>()` followed by \
             `to_bytecode(&EncodeOptions::default())`; the encoder rejects the leaf operator `sha256`."
                .to_string(),
        ),
    }
}

fn build_negative_n_taptree_multi_leaf() -> NegativeVector {
    use crate::bytecode::Tag;
    // Encode `tr(@0/**)` to obtain a valid prefix, then append `Tag::TapTree`
    // (0x08) at the leaf position. The decoder rejects multi-leaf TapTree as
    // reserved for v1+.
    let policy: WalletPolicy = "tr(@0/**)".parse().unwrap();
    let mut bytecode = policy.to_bytecode(&EncodeOptions::default()).unwrap();
    bytecode.push(Tag::TapTree.as_byte());
    let s = encode_singlestring_around(&bytecode);
    debug_assert_decode_matches(&[s.as_str()], "PolicyScopeViolation");
    NegativeVector {
        id: "n_taptree_multi_leaf".to_string(),
        description:
            "Bytecode embedding `Tag::TapTree=0x08` in leaf position → PolicyScopeViolation (multi-leaf reserved for v1+)"
                .to_string(),
        input_strings: vec![s],
        expected_error_variant: "PolicyScopeViolation".to_string(),
        provenance: Some(
            "encoded `tr(@0/**)` to bytecode, appended `Tag::TapTree=0x08`, wrapped in a SingleString chunk via `encoding::encode_string`; decode rejects multi-leaf TapTree as reserved for v1+"
                .to_string(),
        ),
    }
}

fn build_negative_n_fingerprints_count_mismatch() -> NegativeVector {
    let policy: WalletPolicy = "wsh(multi(2,@0/**,@1/**))".parse().unwrap();
    let opts = EncodeOptions::default()
        .with_fingerprints(vec![Fingerprint::from([0xde, 0xad, 0xbe, 0xef])]);
    debug_assert!(matches!(
        policy.to_bytecode(&opts),
        Err(crate::Error::FingerprintsCountMismatch { .. })
    ));
    NegativeVector {
        id: "n_fingerprints_count_mismatch".to_string(),
        description:
            "Fingerprints count differs from placeholder count (2 placeholders, 1 fingerprint supplied) → FingerprintsCountMismatch"
                .to_string(),
        input_strings: Vec::new(),
        expected_error_variant: "FingerprintsCountMismatch".to_string(),
        provenance: Some(
            "encode-side rejection; `input_strings` is empty because the policy never produces a WDM string. \
             Construct via `wsh(multi(2,@0/**,@1/**)).parse::<WalletPolicy>()` and \
             `EncodeOptions::default().with_fingerprints(vec![Fingerprint::from([0xde,0xad,0xbe,0xef])])` (one fingerprint for two placeholders); \
             the encoder rejects with expected=2, got=1."
                .to_string(),
        ),
    }
}

fn build_negative_n_fingerprints_missing_tag() -> NegativeVector {
    use crate::bytecode::Tag;
    // Construct bytecode: header 0x04 (fingerprints flag set), SharedPath
    // declaration with BIP 84 indicator, then where Tag::Fingerprints (0x35)
    // is expected, place Tag::Wsh (0x05) instead.
    let bytecode: Vec<u8> = vec![
        0x04, // bytecode header: v0 + fingerprints flag (bit 2)
        Tag::SharedPath.as_byte(),
        0x03,               // BIP 84 indicator
        Tag::Wsh.as_byte(), // wrong tag where Fingerprints is expected
        // Fill out with arbitrary tail so the cursor doesn't stop on
        // UnexpectedEnd before the tag check fires. The UnexpectedTag check
        // runs on the very first byte of the fingerprints block, so we
        // need not append anything further — but for robustness we add a
        // trailing 0x00.
        0x00,
    ];
    let s = encode_singlestring_around(&bytecode);
    debug_assert_decode_matches(&[s.as_str()], "InvalidBytecode");
    NegativeVector {
        id: "n_fingerprints_missing_tag".to_string(),
        description:
            "Bytecode header advertises fingerprints (bit 2 set) but the fingerprints block is missing `Tag::Fingerprints=0x35` (got Tag::Wsh=0x05) → InvalidBytecode { kind: UnexpectedTag { expected: 0x35, got: 0x05 } }"
                .to_string(),
        input_strings: vec![s],
        expected_error_variant: "InvalidBytecode".to_string(),
        provenance: Some(
            "synthesised bytecode `[0x04, Tag::SharedPath, 0x03, Tag::Wsh, 0x00]`: header bit 2 (fingerprints) is set, but the byte where Tag::Fingerprints (0x35) is expected carries Tag::Wsh (0x05); decode emits InvalidBytecode { kind: UnexpectedTag { expected: 0x35, got: 0x05 } }"
                .to_string(),
        ),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Encode `wsh(pk(@0/**))` with default options and return the SingleString
/// chunk's raw text. Used by mutation-based generators (n02, n04, n05, n13).
fn encode_simple_pk_chunk() -> String {
    let policy: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
    let backup = encode(&policy, &EncodeOptions::default()).unwrap();
    backup.chunks[0].raw.clone()
}

/// Wrap a (header, payload) byte pair into a WDM string via the encoding
/// layer. The two slices are concatenated and passed to
/// [`crate::encoding::encode_string`] verbatim (no header validation), so
/// callers control the exact pre-codex32 byte sequence.
fn encode_string_for_test(header: &[u8], payload: &[u8]) -> String {
    crate::encoding::encode_string(header, payload).unwrap_or_else(|e| {
        panic!(
            "vector builder: encode_string failed (header={header:?}, payload_len={}): {e}",
            payload.len()
        )
    })
}

/// Build a WDM string whose underlying byte sequence is exactly `bytes`.
/// Used for low-level synthetic test inputs where the caller controls every
/// byte (no implicit chunk-header / bytecode-header inference).
fn encode_string_from_bytes(bytes: &[u8]) -> String {
    encode_string_for_test(bytes, &[])
}

/// Wrap a SingleString chunk around a bytecode-payload-fragment. The chunk
/// header is a fixed `[0x00, 0x00]` (version 0, SingleString); `fragment`
/// is the post-chunk-header byte sequence (bytecode header byte at index 0).
fn encode_singlestring_around(fragment: &[u8]) -> String {
    encode_string_for_test(&[0x00, 0x00], fragment)
}

/// Construct an [`crate::EncodedChunk`] from a header + fragment by
/// serialising the header bytes and re-encoding via `encode_string`.
/// Mirrors the interior of `assemble_chunked` for building synthetic test
/// inputs whose header values bypass the normal encoder constraints.
fn encoded_from_header_and_fragment(
    header: crate::ChunkHeader,
    fragment: &[u8],
) -> EncodedChunkRaw {
    let header_bytes = header.to_bytes();
    let raw = encode_string_for_test(&header_bytes, fragment);
    EncodedChunkRaw {
        raw,
        _header: header,
        _fragment: fragment.to_vec(),
    }
}

/// Internal: a (raw, header, fragment) triple analogous to [`crate::EncodedChunk`]
/// without going through the chunking-plan builders. The `_header` and
/// `_fragment` fields are kept for future debug diagnostics.
struct EncodedChunkRaw {
    raw: String,
    _header: crate::ChunkHeader,
    _fragment: Vec<u8>,
}

/// Render bytes as lowercase hex without going through `format!` per byte
/// (which trips clippy `format_collect`). Mirrors the canonical idiom from
/// `tests/fingerprints.rs:302`.
fn bytes_to_lower_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut acc, b| {
            write!(acc, "{b:02x}").unwrap();
            acc
        })
}

/// Render a 4-byte fingerprint as 8 lowercase-hex chars.
fn bytes_to_lower_hex_4(bytes: &[u8; 4]) -> String {
    bytes_to_lower_hex(bytes)
}

/// Sanity check for negative-vector generators: assert that decoding the
/// given inputs produces an error whose variant name matches `expected_name`.
/// In release builds this is a no-op (the generated JSON is the
/// authoritative artifact); in debug/test builds this catches regressions
/// where the generator produces inputs that exercise a different path than
/// intended.
#[track_caller]
fn debug_assert_decode_matches(inputs: &[&str], expected_name: &str) {
    if !cfg!(debug_assertions) {
        return;
    }
    use crate::{DecodeOptions, decode};
    let result = decode(inputs, &DecodeOptions::new());
    match result {
        Err(e) => {
            let actual = error_variant_name(&e);
            assert_eq!(
                actual, expected_name,
                "decode produced unexpected variant: expected {expected_name}, got {actual} (full error: {e:?}; inputs: {inputs:?})"
            );
        }
        Ok(_) => panic!(
            "decode unexpectedly succeeded; expected variant {expected_name} (inputs: {inputs:?})"
        ),
    }
}

/// Sanity check for `EmptyChunkList`: confirm `reassemble_chunks(&[])`
/// rejects with the named variant.
#[track_caller]
fn debug_assert_reassemble_empty_matches() {
    if !cfg!(debug_assertions) {
        return;
    }
    use crate::reassemble_chunks;
    let err = reassemble_chunks(Vec::new()).unwrap_err();
    assert_eq!(
        error_variant_name(&err),
        "EmptyChunkList",
        "reassemble_chunks(&[]) returned unexpected variant: {err:?}"
    );
}

/// Map an [`crate::Error`] value to its stable variant-name string used in
/// `expected_error_variant`. Kept in sync with the public `Error` enum.
fn error_variant_name(e: &crate::Error) -> &'static str {
    use crate::Error;
    match e {
        Error::InvalidHrp(_) => "InvalidHrp",
        Error::MixedCase => "MixedCase",
        Error::InvalidStringLength(_) => "InvalidStringLength",
        Error::InvalidChar { .. } => "InvalidChar",
        Error::BchUncorrectable => "BchUncorrectable",
        Error::InvalidBytecode { .. } => "InvalidBytecode",
        Error::UnsupportedVersion(_) => "UnsupportedVersion",
        Error::UnsupportedCardType(_) => "UnsupportedCardType",
        Error::ChunkIndexOutOfRange { .. } => "ChunkIndexOutOfRange",
        Error::DuplicateChunkIndex(_) => "DuplicateChunkIndex",
        Error::WalletIdMismatch { .. } => "WalletIdMismatch",
        Error::TotalChunksMismatch { .. } => "TotalChunksMismatch",
        Error::PolicyScopeViolation(_) => "PolicyScopeViolation",
        Error::CrossChunkHashMismatch => "CrossChunkHashMismatch",
        Error::InvalidChunkCount(_) => "InvalidChunkCount",
        Error::InvalidChunkIndex { .. } => "InvalidChunkIndex",
        Error::ReservedWalletIdBitsSet => "ReservedWalletIdBitsSet",
        Error::ChunkHeaderTruncated { .. } => "ChunkHeaderTruncated",
        Error::PolicyTooLarge { .. } => "PolicyTooLarge",
        Error::EmptyChunkList => "EmptyChunkList",
        Error::MissingChunkIndex(_) => "MissingChunkIndex",
        Error::MixedChunkTypes => "MixedChunkTypes",
        Error::SingleStringWithMultipleChunks => "SingleStringWithMultipleChunks",
        Error::PolicyParse(_) => "PolicyParse",
        Error::Miniscript(_) => "Miniscript",
        Error::TapLeafSubsetViolation { .. } => "TapLeafSubsetViolation",
        Error::FingerprintsCountMismatch { .. } => "FingerprintsCountMismatch",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each per-variant generator must, when its `input_strings` are non-empty,
    /// produce a decode error whose variant name matches the fixture's
    /// `expected_error_variant`. This mirrors the `debug_assert_decode_matches`
    /// checks inside the generators with clearer diagnostics under `cargo test`.
    #[test]
    fn every_v2_negative_generator_fires_expected_variant() {
        let v = build_test_vectors_v2();
        for nv in &v.negative_vectors {
            if nv.input_strings.is_empty() {
                continue;
            }
            // PolicyParse fires from the policy-parse layer, not from
            // decode(); that's documented in its provenance.
            if nv.expected_error_variant == "PolicyParse" {
                continue;
            }
            let inputs: Vec<&str> = nv.input_strings.iter().map(String::as_str).collect();
            let result = crate::decode(&inputs, &crate::DecodeOptions::new());
            match result {
                Err(e) => {
                    let actual = error_variant_name(&e);
                    assert_eq!(
                        actual, nv.expected_error_variant,
                        "negative vector {:?}: decode produced {actual} (full error: {e:?})",
                        nv.id
                    );
                }
                Ok(_) => panic!(
                    "negative vector {:?}: decode unexpectedly succeeded (expected {})",
                    nv.id, nv.expected_error_variant
                ),
            }
        }
    }
}
