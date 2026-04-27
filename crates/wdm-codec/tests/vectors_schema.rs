//! Task 8.7 — TestVectorFile schema integration tests.
//!
//! These tests exercise the in-memory schema round-trip and basic invariants.
//! They do NOT depend on the existence of `tests/vectors/v0.1.json` (the
//! controller generates and commits that file separately in Tasks 8.5 and 8.6).

use wdm_codec::TestVectorFile;

/// Task 8.7-a — [`TestVectorFile`] round-trips cleanly through JSON.
#[test]
fn build_test_vectors_round_trips_through_serde() {
    let vectors = wdm_codec::vectors::build_test_vectors();
    let json =
        serde_json::to_string_pretty(&vectors).expect("TestVectorFile must serialize to JSON");
    let parsed: TestVectorFile =
        serde_json::from_str(&json).expect("TestVectorFile must deserialize from JSON");
    assert_eq!(
        vectors, parsed,
        "TestVectorFile must round-trip through JSON without data loss"
    );
}

/// Task 8.7-b — Two invocations of [`build_test_vectors`] must be structurally equal.
///
/// This guards against any non-deterministic ordering (e.g., iteration over a HashMap).
#[test]
fn build_test_vectors_is_deterministic() {
    let v1 = wdm_codec::vectors::build_test_vectors();
    let v2 = wdm_codec::vectors::build_test_vectors();
    assert_eq!(
        v1, v2,
        "build_test_vectors() must produce identical output on every call"
    );
}

/// Task 8.7-c — Corpus vector count and negative vector floor match expectations.
///
/// C1-C5 + E10 + E12 + E13 + E14 + Coldcard = 10 positive vectors.
/// At least 18 negative scenarios (conformance.rs minimum).
#[test]
fn build_test_vectors_has_expected_corpus_count() {
    let v = wdm_codec::vectors::build_test_vectors();

    assert_eq!(
        v.vectors.len(),
        10,
        "expected exactly 10 positive corpus vectors (C1-C5, E10, E12, E13, E14, Coldcard); \
         got {}",
        v.vectors.len()
    );

    assert!(
        v.negative_vectors.len() >= 18,
        "expected >= 18 negative vectors (conformance.rs minimum); got {}",
        v.negative_vectors.len()
    );
}

/// Extra — the JSON produced by two independent serialize calls must be byte-identical.
///
/// Confirms that `serde_json::to_string_pretty` is deterministic (no HashMap
/// iteration non-determinism in the schema types).
#[test]
fn json_output_is_byte_identical_across_calls() {
    let v = wdm_codec::vectors::build_test_vectors();
    let json1 = serde_json::to_string_pretty(&v).expect("serialize 1");
    let json2 = serde_json::to_string_pretty(&v).expect("serialize 2");
    assert_eq!(
        json1, json2,
        "JSON output must be byte-identical across two serde_json::to_string_pretty calls"
    );
}

/// Extra — all positive vectors have non-empty id, policy, bytecode_hex, and chunks.
#[test]
fn positive_vectors_are_well_formed() {
    let v = wdm_codec::vectors::build_test_vectors();
    for vec in &v.vectors {
        assert!(!vec.id.is_empty(), "vector id must not be empty");
        assert!(
            !vec.policy.is_empty(),
            "vector {:?} policy must not be empty",
            vec.id
        );
        assert!(
            !vec.expected_bytecode_hex.is_empty(),
            "vector {:?} expected_bytecode_hex must not be empty",
            vec.id
        );
        assert!(
            vec.expected_bytecode_hex
                .chars()
                .all(|c| c.is_ascii_hexdigit()),
            "vector {:?} expected_bytecode_hex must be lowercase hex; got {:?}",
            vec.id,
            vec.expected_bytecode_hex
        );
        assert!(
            !vec.expected_chunks.is_empty(),
            "vector {:?} must have at least one chunk",
            vec.id
        );
        assert_eq!(
            vec.expected_wallet_id_words.len(),
            12,
            "vector {:?} expected_wallet_id_words must have exactly 12 words",
            vec.id
        );
        // All chunk strings must start with the WDM HRP.
        for chunk in &vec.expected_chunks {
            assert!(
                chunk.starts_with("wdm1"),
                "vector {:?} chunk {:?} must start with 'wdm1'",
                vec.id,
                chunk
            );
        }
    }
}

/// Extra — all negative vectors have non-empty id and expected_error_variant.
#[test]
fn negative_vectors_are_well_formed() {
    let v = wdm_codec::vectors::build_test_vectors();
    for nv in &v.negative_vectors {
        assert!(!nv.id.is_empty(), "negative vector id must not be empty");
        assert!(
            !nv.expected_error_variant.is_empty(),
            "negative vector {:?} expected_error_variant must not be empty",
            nv.id
        );
    }
}

/// Extra — if the committed JSON file exists at the canonical path, verify it matches.
///
/// This test is skipped (not failed) if the file does not yet exist.
/// The controller generates it in Task 8.6; this guard provides ongoing CI protection
/// once it exists.
#[test]
fn committed_json_matches_regenerated_if_present() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors/v0.1.json");

    if !path.exists() {
        // File not yet committed (Task 8.6 pending) — skip.
        return;
    }

    let contents = std::fs::read_to_string(&path).expect("failed to read committed vectors file");
    let committed: TestVectorFile =
        serde_json::from_str(&contents).expect("failed to parse committed vectors JSON");
    let regenerated = wdm_codec::vectors::build_test_vectors();

    // Compare field-by-field; skip generator (version string may differ between runs).
    assert_eq!(
        committed.schema_version, regenerated.schema_version,
        "schema_version mismatch in committed file"
    );
    assert_eq!(
        committed.vectors, regenerated.vectors,
        "positive vectors mismatch in committed file; re-run gen_vectors --output to update"
    );
    assert_eq!(
        committed.negative_vectors, regenerated.negative_vectors,
        "negative vectors mismatch in committed file; re-run gen_vectors --output to update"
    );
}
