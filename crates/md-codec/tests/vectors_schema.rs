//! Task 8.7 — TestVectorFile schema integration tests.
//!
//! These tests exercise the in-memory schema round-trip and basic invariants.
//! They do NOT depend on the existence of `tests/vectors/v0.1.json` (the
//! controller generates and commits that file separately in Tasks 8.5 and 8.6).

use md_codec::TestVectorFile;

/// Task 8.7-a — [`TestVectorFile`] round-trips cleanly through JSON.
#[test]
fn build_test_vectors_round_trips_through_serde() {
    let vectors = md_codec::vectors::build_test_vectors();
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
    let v1 = md_codec::vectors::build_test_vectors();
    let v2 = md_codec::vectors::build_test_vectors();
    assert_eq!(
        v1, v2,
        "build_test_vectors() must produce identical output on every call"
    );
}

/// Task 8.7-c — Corpus vector count and negative vector floor match expectations.
///
/// Schema-1 (build_test_vectors / v1):
///   C1-C5 + E10 + E12 + E13 + E14 + Coldcard = 10 positive vectors.
///   At least 18 negative scenarios (conformance.rs minimum).
///
/// Schema-2 (build_test_vectors_v2):
///     10 corpus + 8 taproot (v0.5: T1, T2, tr_multia_2of3, T3-T7) + 1 fingerprints
///     + 5 v0.4 default + 3 v0.4 fingerprints = 27 total.
///     Negatives: >= 47 (30 pre-v0.4 + 9 v0.4 Sh-matrix/top-level + 8 v0.5 N1-N8 +
///     1 v0.5 N9 — the legacy `n_taptree_multi_leaf` is replaced by N1; minimum guard = 27).
#[test]
fn build_test_vectors_has_expected_corpus_count() {
    let v1 = md_codec::vectors::build_test_vectors();

    assert_eq!(
        v1.vectors.len(),
        10,
        "expected exactly 10 positive corpus vectors in schema-1 (C1-C5, E10, E12, E13, E14, Coldcard); \
         got {}",
        v1.vectors.len()
    );

    assert!(
        v1.negative_vectors.len() >= 18,
        "expected >= 18 negative vectors in schema-1 (conformance.rs minimum); got {}",
        v1.negative_vectors.len()
    );

    let v2 = md_codec::vectors::build_test_vectors_v2();

    assert_eq!(
        v2.vectors.len(),
        47,
        "expected exactly 47 positive corpus vectors in schema-2 \
         (v0.10 added o1/o2/o3 — OriginPaths block coverage; was 44 in v0.9); \
         got {} — if this fails, update the expected count in tests/vectors_schema.rs",
        v2.vectors.len()
    );

    assert!(
        v2.negative_vectors.len() >= 27,
        "expected >= 27 negative vectors in schema-2 \
         (30 pre-v0.4 + 9 v0.4 Sh-matrix additions = 39 total; \
         minimum is 27 to guard against regression); got {}",
        v2.negative_vectors.len()
    );
}

/// Extra — the JSON produced by two independent serialize calls must be byte-identical.
///
/// Confirms that `serde_json::to_string_pretty` is deterministic (no HashMap
/// iteration non-determinism in the schema types).
#[test]
fn json_output_is_byte_identical_across_calls() {
    let v = md_codec::vectors::build_test_vectors();
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
    let v = md_codec::vectors::build_test_vectors();
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
            vec.expected_policy_id_words.len(),
            12,
            "vector {:?} expected_policy_id_words must have exactly 12 words",
            vec.id
        );
        // All chunk strings must start with the MD HRP.
        for chunk in &vec.expected_chunks {
            assert!(
                chunk.starts_with("md1"),
                "vector {:?} chunk {:?} must start with 'md1'",
                vec.id,
                chunk
            );
        }
    }
}

/// Extra — all negative vectors have non-empty id and expected_error_variant.
#[test]
fn negative_vectors_are_well_formed() {
    let v = md_codec::vectors::build_test_vectors();
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
    let regenerated = md_codec::vectors::build_test_vectors_v1();

    // Compare field-by-field; skip generator (version string may differ between runs).
    assert_eq!(
        committed.schema_version, regenerated.schema_version,
        "schema_version mismatch in committed file"
    );
    assert_eq!(
        committed.vectors, regenerated.vectors,
        "positive vectors mismatch in committed file; re-run gen_vectors --output --schema 1 to update"
    );
    assert_eq!(
        committed.negative_vectors, regenerated.negative_vectors,
        "negative vectors mismatch in committed file; re-run gen_vectors --output --schema 1 to update"
    );
}

// ---------------------------------------------------------------------------
// Schema-2 tests (Phase F — F-6, F-11)
// ---------------------------------------------------------------------------

/// Phase F — schema-2 file (`tests/vectors/v0.2.json`) round-trips through
/// `build_test_vectors_v2()` byte-identical for the typed comparison
/// (skipping `generator`).
///
/// Mirrors `committed_json_matches_regenerated_if_present` but for the
/// schema-2 lock. The file lives at `tests/vectors/v0.2.json`.
#[test]
fn committed_v0_2_json_matches_regenerated_if_present() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors/v0.2.json");

    if !path.exists() {
        // File not yet committed; controller generates it in Phase F.
        return;
    }

    let contents = std::fs::read_to_string(&path).expect("failed to read committed v0.2.json");
    let committed: TestVectorFile =
        serde_json::from_str(&contents).expect("failed to parse committed v0.2.json");
    let regenerated = md_codec::vectors::build_test_vectors_v2();

    assert_eq!(
        committed.schema_version, regenerated.schema_version,
        "schema_version mismatch in committed v0.2.json"
    );
    assert_eq!(
        committed.schema_version, 2,
        "v0.2.json must carry schema_version = 2"
    );
    assert_eq!(
        committed.vectors, regenerated.vectors,
        "positive vectors mismatch in committed v0.2.json; re-run gen_vectors --output --schema 2 to update"
    );
    assert_eq!(
        committed.negative_vectors, regenerated.negative_vectors,
        "negative vectors mismatch in committed v0.2.json; re-run gen_vectors --output --schema 2 to update"
    );
}

/// Phase F (F-6) — pin the SHA-256 of `tests/vectors/v0.2.json` so accidental
/// edits surface as a test failure.
///
/// If you regenerate v0.2.json, update the constant below to match the new
/// hash. The intent of this test is to prevent silent drift between
/// `build_test_vectors_v2()` and the committed file (especially across
/// `serde_json` formatting changes); not to prevent intentional
/// regenerations.
#[test]
fn v0_2_sha256_lock_matches_committed_file() {
    use bitcoin::hashes::{Hash, sha256};

    /// Lockfile SHA-256 (lowercase hex). Update when v0.2.json is
    /// intentionally regenerated.
    const V0_2_SHA256: &str = "6d843809274b44fd5b75755c132edb3784b2461f96c71961905b6c6497b7cfcd";

    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors/v0.2.json");
    if !path.exists() {
        // Not yet committed; skip rather than fail.
        return;
    }

    let bytes = std::fs::read(&path).expect("failed to read v0.2.json");
    let hash = sha256::Hash::hash(&bytes);
    let actual: String =
        hash.as_byte_array()
            .iter()
            .fold(String::with_capacity(64), |mut acc, b| {
                use std::fmt::Write;
                write!(acc, "{b:02x}").unwrap();
                acc
            });

    assert_eq!(
        actual, V0_2_SHA256,
        "v0.2.json SHA-256 drifted; if this is an intentional regeneration, update the V0_2_SHA256 constant in tests/vectors_schema.rs"
    );
}

/// Phase F — `build_test_vectors_v2()` returns a strict superset of
/// `build_test_vectors_v1()`'s schema-1 vectors.
///
/// Each schema-1 positive vector (matched by `id`) must appear at the head
/// of the schema-2 vectors list with byte-identical fields.
#[test]
fn schema_2_is_a_superset_of_schema_1_positive_vectors() {
    let v1 = md_codec::vectors::build_test_vectors_v1();
    let v2 = md_codec::vectors::build_test_vectors_v2();

    assert_eq!(v1.schema_version, 1);
    assert_eq!(v2.schema_version, 2);

    assert!(
        v2.vectors.len() > v1.vectors.len(),
        "schema-2 must add at least one positive vector"
    );

    for (i, v1_vec) in v1.vectors.iter().enumerate() {
        let v2_vec = &v2.vectors[i];
        assert_eq!(
            v2_vec.id, v1_vec.id,
            "schema-2 positive vector at index {i} must match schema-1 id"
        );
        assert_eq!(
            v2_vec.policy, v1_vec.policy,
            "schema-2 vector {:?} policy must equal schema-1",
            v1_vec.id
        );
        assert_eq!(
            v2_vec.expected_bytecode_hex, v1_vec.expected_bytecode_hex,
            "schema-2 vector {:?} expected_bytecode_hex must equal schema-1",
            v1_vec.id
        );
        assert_eq!(
            v2_vec.expected_chunks, v1_vec.expected_chunks,
            "schema-2 vector {:?} expected_chunks must equal schema-1",
            v1_vec.id
        );
        assert_eq!(
            v2_vec.expected_policy_id_words, v1_vec.expected_policy_id_words,
            "schema-2 vector {:?} expected_policy_id_words must equal schema-1",
            v1_vec.id
        );
        // Schema-1 vectors carry no fingerprints fields.
        assert!(v2_vec.expected_fingerprints_hex.is_none());
        assert!(v2_vec.encode_options_fingerprints.is_none());
    }
}

/// Phase F — schema-2 must contain the v0.2 corpus additions.
#[test]
fn schema_2_contains_v0_2_corpus_additions() {
    let v2 = md_codec::vectors::build_test_vectors_v2();

    let positive_ids: Vec<&str> = v2.vectors.iter().map(|v| v.id.as_str()).collect();
    for required in [
        "tr_keypath_only_md_v0_5",
        "tr_single_leaf_pk_md_v0_5",
        "tr_multia_2of3",
        "multi_2of2_with_fingerprints",
    ] {
        assert!(
            positive_ids.contains(&required),
            "schema-2 must include positive vector {required:?}; got {positive_ids:?}"
        );
    }

    let negative_ids: Vec<&str> = v2
        .negative_vectors
        .iter()
        .map(|nv| nv.id.as_str())
        .collect();
    // v0.5 SPEC §5 renamed `n_taptree_multi_leaf` (the v0.4 reservation rejection)
    // into the canonical N1-N9 negative set; `n_taptree_single_inner_under_tr`
    // (N1) is the closest semantic match (truncated multi-leaf subtree → UnexpectedEnd).
    // `n_tap_leaf_subset` was removed in v0.6 (Layer 3 strip).
    for required in [
        "n_taptree_single_inner_under_tr",
        "n_fingerprints_count_mismatch",
        "n_fingerprints_missing_tag",
    ] {
        assert!(
            negative_ids.contains(&required),
            "schema-2 must include negative vector {required:?}; got {negative_ids:?}"
        );
    }
}

/// Phase 6 — schema-2 must contain the v0.4 corpus additions (S1-Cs positive + Sh-matrix negative).
#[test]
fn schema_2_contains_v0_4_corpus_additions() {
    let v2 = md_codec::vectors::build_test_vectors_v2();

    let positive_ids: Vec<&str> = v2.vectors.iter().map(|v| v.id.as_str()).collect();
    for required in [
        "s1_wpkh",
        "s2_wpkh_fingerprint",
        "s3_sh_wpkh",
        "s4_sh_wpkh_fingerprint",
        "m1_sh_wsh_sortedmulti_1of2",
        "m2_sh_wsh_sortedmulti_2of3",
        "m3_sh_wsh_sortedmulti_2of3_fingerprints",
        "cs_coldcard_sh_wsh",
    ] {
        assert!(
            positive_ids.contains(&required),
            "schema-2 must include v0.4 positive vector {required:?}; got {positive_ids:?}"
        );
    }

    let negative_ids: Vec<&str> = v2
        .negative_vectors
        .iter()
        .map(|nv| nv.id.as_str())
        .collect();
    // `n_sh_bare` and `n_top_bare` were deleted in v0.6 (Tag::Bare dropped;
    // byte 0x07 is now Tag::TapTree, covered by `n_taptree_at_top_level`).
    for required in [
        "n_sh_multi",
        "n_sh_sortedmulti",
        "n_sh_pkh",
        "n_sh_tr",
        "n_sh_inner_script",
        "n_sh_key_slot",
        "n_top_pkh",
    ] {
        assert!(
            negative_ids.contains(&required),
            "schema-2 must include v0.4 negative vector {required:?}; got {negative_ids:?}"
        );
    }

    // All new fingerprints-carrying v0.4 vectors must populate both metadata fields.
    for fingerprints_id in [
        "s2_wpkh_fingerprint",
        "s4_sh_wpkh_fingerprint",
        "m3_sh_wsh_sortedmulti_2of3_fingerprints",
    ] {
        let v = v2
            .vectors
            .iter()
            .find(|v| v.id == fingerprints_id)
            .unwrap_or_else(|| {
                panic!("v0.4 fingerprints vector {fingerprints_id:?} must be present")
            });
        assert!(
            v.expected_fingerprints_hex.is_some(),
            "v0.4 fingerprints vector {fingerprints_id:?} must carry expected_fingerprints_hex"
        );
        assert!(
            v.encode_options_fingerprints.is_some(),
            "v0.4 fingerprints vector {fingerprints_id:?} must carry encode_options_fingerprints"
        );
    }

    // All v0.4 negative vectors must carry a provenance string.
    // (`n_sh_bare`/`n_top_bare` removed in v0.6 — see comment above.)
    for neg_id in [
        "n_sh_multi",
        "n_sh_sortedmulti",
        "n_sh_pkh",
        "n_sh_tr",
        "n_sh_inner_script",
        "n_sh_key_slot",
        "n_top_pkh",
    ] {
        let nv = v2
            .negative_vectors
            .iter()
            .find(|nv| nv.id == neg_id)
            .unwrap_or_else(|| panic!("v0.4 negative vector {neg_id:?} must be present"));
        let prov = nv
            .provenance
            .as_ref()
            .unwrap_or_else(|| panic!("v0.4 negative vector {neg_id:?} must carry provenance"));
        assert!(
            !prov.trim().is_empty(),
            "v0.4 negative vector {neg_id:?} provenance must be non-empty"
        );
    }
}

/// Phase F — every schema-2 negative vector carries a non-empty
/// `provenance` field.
#[test]
fn schema_2_negative_vectors_all_have_provenance() {
    let v2 = md_codec::vectors::build_test_vectors_v2();
    for nv in &v2.negative_vectors {
        let prov = nv.provenance.as_ref().unwrap_or_else(|| {
            panic!(
                "schema-2 negative vector {:?} must carry a provenance",
                nv.id
            )
        });
        assert!(
            !prov.trim().is_empty(),
            "schema-2 negative vector {:?} provenance must be non-empty",
            nv.id
        );
    }
}

/// Phase F — the fingerprints positive vector populates both
/// `expected_fingerprints_hex` and `encode_options_fingerprints`.
#[test]
fn schema_2_fingerprints_vector_carries_metadata() {
    let v2 = md_codec::vectors::build_test_vectors_v2();
    let fp_vec = v2
        .vectors
        .iter()
        .find(|v| v.id == "multi_2of2_with_fingerprints")
        .expect("schema-2 must include multi_2of2_with_fingerprints");

    let hex = fp_vec
        .expected_fingerprints_hex
        .as_ref()
        .expect("fingerprints vector must carry expected_fingerprints_hex");
    assert_eq!(
        hex,
        &vec!["deadbeef".to_string(), "cafebabe".to_string()],
        "fingerprints hex must match the BIP §\"Fingerprints block\" example"
    );

    let raw = fp_vec
        .encode_options_fingerprints
        .as_ref()
        .expect("fingerprints vector must carry encode_options_fingerprints");
    assert_eq!(
        raw,
        &vec![[0xdeu8, 0xad, 0xbe, 0xef], [0xca, 0xfe, 0xba, 0xbe]],
        "encode_options_fingerprints must mirror expected_fingerprints_hex"
    );
}
