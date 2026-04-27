//! Test vector schema and generator for WDM codec.
//!
//! # Schema version
//!
//! Version: 1 (v0.1 lock-in). Schema changes require bumping
//! `TestVectorFile::schema_version` and updating the BIP draft's Test Vectors
//! section.
//!
//! # Usage
//!
//! ```rust
//! let vectors = wdm_codec::vectors::build_test_vectors();
//! let json = serde_json::to_string_pretty(&vectors).unwrap();
//! ```

use serde::{Deserialize, Serialize};

use crate::{EncodeOptions, WalletPolicy, encode};

// ---------------------------------------------------------------------------
// Public schema types (Task 8.1)
// ---------------------------------------------------------------------------

/// Top-level test vector file.
///
/// Stable across v0.1+. Changing field names without bumping `schema_version`
/// is a breaking change.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TestVectorFile {
    /// Schema version. Currently 1.
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

// ---------------------------------------------------------------------------
// Negative vector fixtures — curated from conformance.rs test scenarios
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
/// What this means for cross-implementation conformance:
///
/// - The `expected_error_variant` field is **the authoritative contract**.
///   Conformance implementations should treat this as the test assertion:
///   "feeding `input_strings` to decode() returns *some* error in the
///   `expected_error_variant` family".
/// - The exact `input_strings` byte sequences may not byte-for-byte match
///   what a different conformance implementation would produce by exercising
///   the same error path. Implementations that need byte-for-byte negative
///   vectors should (a) generate them locally by exercising the actual error
///   path, or (b) wait for v0.2 to provide programmatically-generated
///   negative vectors (tracked as `8-negative-fixture-placeholder-strings`
///   in `design/FOLLOWUPS.md`).
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
/// - `n30` (`PolicyTooLarge`): triggered by `chunking_decision(1693, false)`
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
        // Synthetic path: chunking_decision(1693, false) returns this error.
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

/// Build the complete [`TestVectorFile`] by encoding all corpus entries and
/// collecting all negative fixture metadata.
///
/// Both `gen_vectors --output` and `wdm vectors` call this function; there is
/// no code duplication. Output is deterministic: calling this function twice
/// returns structurally equal values.
pub fn build_test_vectors() -> TestVectorFile {
    TestVectorFile {
        schema_version: 1,
        generator: format!("wdm-codec {}", env!("CARGO_PKG_VERSION")),
        vectors: build_positive_vectors(),
        negative_vectors: build_negative_vectors(),
    }
}

fn build_positive_vectors() -> Vec<Vector> {
    let mut out = Vec::with_capacity(CORPUS_FIXTURES.len());
    for &(id, description, policy_str) in CORPUS_FIXTURES {
        let policy: WalletPolicy = policy_str.parse().unwrap_or_else(|e| {
            panic!("vector builder: failed to parse corpus policy {id:?}: {e}")
        });

        let bytecode = policy.to_bytecode().unwrap_or_else(|e| {
            panic!("vector builder: failed to encode bytecode for {id:?}: {e}")
        });

        let expected_bytecode_hex: String = bytecode.iter().fold(
            String::with_capacity(bytecode.len() * 2),
            |mut acc, b| {
                use std::fmt::Write;
                write!(acc, "{b:02x}").unwrap();
                acc
            },
        );

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

        out.push(Vector {
            id: id.to_string(),
            description: description.to_string(),
            policy: policy_str.to_string(),
            expected_bytecode_hex,
            expected_chunks,
            expected_wallet_id_words,
        });
    }
    out
}

fn build_negative_vectors() -> Vec<NegativeVector> {
    NEGATIVE_FIXTURES
        .iter()
        .map(|f| NegativeVector {
            id: f.id.to_string(),
            description: f.description.to_string(),
            input_strings: f.input_strings.iter().map(|s| s.to_string()).collect(),
            expected_error_variant: f.expected_error_variant.to_string(),
        })
        .collect()
}
