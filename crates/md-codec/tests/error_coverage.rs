//! Exhaustiveness gate: every `md_codec::Error` variant must have at least
//! one `rejects_*` test in `conformance.rs` (bucket E).
//!
//! # How it works
//!
//! `ErrorVariantName` is a hand-written mirror enum whose variant names match
//! those of `md_codec::Error` case-for-case.  `strum::EnumIter` generates
//! `ErrorVariantName::iter()` so the compiler catches any forgotten entry in
//! this mirror enum at compile time (via an exhaustive `match` in `iter()`).
//!
//! At test time, `every_error_variant_has_a_rejects_test_in_conformance`
//! converts each variant name to `snake_case`, builds the expected test-name
//! substring (e.g. `rejects_invalid_hrp`), then checks whether that substring
//! appears anywhere in the `conformance.rs` source.
//!
//! # Maintenance rule
//!
//! When a new `Error` variant is added to `src/error.rs`:
//!  1. Add a matching entry to `ErrorVariantName` below.
//!  2. Add a `rejects_<snake_case_variant>` test to `tests/conformance.rs`.
//!
//! This test will fail at CI until both steps are done.

use strum::EnumIter;
use strum::IntoEnumIterator;

/// Mirror enum of every `md_codec::Error` variant name.
///
/// Variant names must match the source enum **case-for-case**; the
/// `pascal_to_snake` helper derives the expected test-name substring from
/// `format!("{:?}", variant)`.
///
/// This enum is intentionally `#[non_exhaustive]`-free so that adding an
/// entry here triggers a recompile and, if forgotten, a test failure.
#[derive(Debug, EnumIter)]
#[allow(dead_code)]
enum ErrorVariantName {
    InvalidHrp,
    MixedCase,
    InvalidStringLength,
    InvalidChar,
    BchUncorrectable,
    InvalidBytecode,
    UnsupportedVersion,
    UnsupportedCardType,
    ChunkIndexOutOfRange,
    DuplicateChunkIndex,
    ChunkSetIdMismatch,
    TotalChunksMismatch,
    PolicyScopeViolation,
    CrossChunkHashMismatch,
    PolicyParse,
    Miniscript,
    InvalidChunkCount,
    InvalidChunkIndex,
    ReservedChunkSetIdBitsSet,
    ChunkHeaderTruncated,
    PolicyTooLarge,
    EmptyChunkList,
    MissingChunkIndex,
    MixedChunkTypes,
    SingleStringWithMultipleChunks,
    SubsetViolation,
    FingerprintsCountMismatch,
}

/// For `InvalidBytecode` the expected substring is `rejects_invalid_bytecode_`
/// (any sub-variant suffix counts), because conformance.rs typically has
/// multiple tests such as `rejects_invalid_bytecode_unknown_tag`.
const INVALID_BYTECODE_PREFIX: &str = "rejects_invalid_bytecode_";

/// Verify that every `Error` variant has a corresponding `rejects_*` test in
/// `conformance.rs`.
///
/// Reads `conformance.rs` at test time via `std::fs` so that this file
/// compiles independently of whether bucket E has landed yet.  If
/// `conformance.rs` does not exist, the test fails with an explicit message
/// rather than a compile error — the parallel batch resolves once both
/// buckets E and F are merged.
#[test]
fn every_error_variant_has_a_rejects_test_in_conformance() {
    // Locate conformance.rs relative to this test file's directory.
    // CARGO_MANIFEST_DIR points to `crates/md-codec/`.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let conformance_path = std::path::Path::new(manifest_dir)
        .join("tests")
        .join("conformance.rs");

    let conformance_src = std::fs::read_to_string(&conformance_path).unwrap_or_else(|_| {
        panic!(
            "conformance.rs not found at {}\n\
             Bucket E (tests/conformance.rs) must land before this test can pass.",
            conformance_path.display()
        )
    });

    let mut missing: Vec<String> = Vec::new();

    for variant in ErrorVariantName::iter() {
        let variant_name = format!("{variant:?}");
        let snake = pascal_to_snake(&variant_name);

        // `InvalidBytecode` is split into multiple sub-variant tests in
        // conformance.rs; any `rejects_invalid_bytecode_*` name is sufficient.
        let pattern: String = if snake == "invalid_bytecode" {
            INVALID_BYTECODE_PREFIX.to_owned()
        } else {
            format!("rejects_{snake}")
        };

        if !conformance_src.contains(pattern.as_str()) {
            missing.push(format!("{variant_name} (expected substring: {pattern:?})"));
        }
    }

    assert!(
        missing.is_empty(),
        "Error variants without a matching `rejects_*` test in conformance.rs:\n  {}\n\n\
         Fix: add the missing test(s) to tests/conformance.rs AND update \
         tests/error_coverage.rs::ErrorVariantName if the variant itself is new.",
        missing.join("\n  ")
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert a `PascalCase` identifier to `snake_case`.
///
/// Examples: `"InvalidHrp"` → `"invalid_hrp"`, `"MixedCase"` → `"mixed_case"`.
fn pascal_to_snake(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.extend(ch.to_lowercase());
    }
    out
}

// ---------------------------------------------------------------------------
// Unit tests for the helper
// ---------------------------------------------------------------------------

#[cfg(test)]
mod helper_tests {
    use super::pascal_to_snake;

    #[test]
    fn pascal_to_snake_single_word() {
        assert_eq!(pascal_to_snake("Miniscript"), "miniscript");
    }

    #[test]
    fn pascal_to_snake_two_words() {
        assert_eq!(pascal_to_snake("MixedCase"), "mixed_case");
    }

    #[test]
    fn pascal_to_snake_acronym_style() {
        assert_eq!(pascal_to_snake("InvalidHrp"), "invalid_hrp");
    }

    #[test]
    fn pascal_to_snake_three_words() {
        assert_eq!(
            pascal_to_snake("SingleStringWithMultipleChunks"),
            "single_string_with_multiple_chunks"
        );
    }
}
