//! Corpus round-trip tests for the WDM codec (Tasks 6.2-6.13).
//!
//! Each test encodes a canonical BIP 388 wallet policy string to WDM format
//! and decodes it back, asserting structural equality with the original.
//!
//! Policy sources:
//! - C1-C5: `design/CORPUS.md` sections C1-C5
//! - E10, E12-E14: `design/CORPUS.md` Real-world examples (E-series)
//! - Coldcard: representative shape (see `corpus_coldcard_bip388_export` comment)
//!
//! Tasks 6.12 and 6.13 iterate over the shared [`CORPUS_POLICIES`] fixture array.

mod common;

use wdm_codec::{DecodeOptions, EncodeOptions, WalletPolicy, decode, encode};

// ---------------------------------------------------------------------------
// Shared fixture array (used by 6.12 idempotency and 6.13 HRP-lowercase)
// ---------------------------------------------------------------------------

/// All corpus policy strings from C1-C5, E10, E12-E14, and the Coldcard entry.
/// Ordered as they appear in `design/CORPUS.md`.
const CORPUS_POLICIES: &[&str] = &[
    // C1 - Single-key (section C1)
    "wsh(pk(@0/**))",
    // C2 - 2-of-3 sortedmulti (section C2)
    "wsh(sortedmulti(2,@0/**,@1/**,@2/**))",
    // C3 - 2-of-3 with timelock fallback (section C3)
    "wsh(or_d(multi(2,@0/**,@1/**),and_v(v:older(52560),pk(@2/**))))",
    // C4 - 6-key inheritance miniscript (section C4)
    "wsh(andor(pk(@0/**),after(1200000),or_i(and_v(v:pkh(@1/**),and_v(v:pkh(@2/**),and_v(v:pkh(@3/**),older(4032)))),and_v(v:pkh(@4/**),and_v(v:pkh(@5/**),older(32768))))))",
    // C5 - 5-of-9 thresh with 2-key timelock recovery (section C5)
    "wsh(or_d(thresh(5,pk(@0/**),s:pk(@1/**),s:pk(@2/**),s:pk(@3/**),s:pk(@4/**),s:pk(@5/**),s:pk(@6/**),s:pk(@7/**),s:pk(@8/**)),and_v(v:older(105120),multi(2,@9/**,@10/**))))",
    // E10 - Liana "Simple Inheritance" single-key + 1-year recovery
    "wsh(or_d(pk(@0/**),and_v(v:pk(@1/**),older(52560))))",
    // E12 - Liana "Expanding Multisig" 2-of-2 + recovery key
    "wsh(or_d(multi(2,@0/**,@1/**),and_v(v:older(52560),pk(@2/**))))",
    // E13 - HTLC with sha256 preimage
    // Hash = SHA-256("hello world") = b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9
    "wsh(andor(pk(@0/**),sha256(b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9),and_v(v:pk(@1/**),older(144))))",
    // E14 - Decaying multisig 3-of-3 to 2-of-3 with 6 distinct keys
    "wsh(or_d(multi(3,@0/**,@1/**,@2/**),and_v(v:older(52560),multi(2,@3/**,@4/**,@5/**))))",
    // Coldcard - representative BIP 388 export shape (2-of-3 sortedmulti, same shape as C2)
    // Coldcard Mk4 exports multisig wallets using wsh(sortedmulti(...)) per
    // https://coldcard.com/docs/multisig and public Sparrow wallet import examples.
    "wsh(sortedmulti(2,@0/**,@1/**,@2/**))",
];

// ---------------------------------------------------------------------------
// C1 - Single-key (Task 6.2)
// ---------------------------------------------------------------------------

#[test]
fn corpus_c1_single_key() {
    common::round_trip_assert("wsh(pk(@0/**))");
}

// ---------------------------------------------------------------------------
// C2 - 2-of-3 sortedmulti (Task 6.3)
// ---------------------------------------------------------------------------

#[test]
fn corpus_c2_2of3_sortedmulti() {
    common::round_trip_assert("wsh(sortedmulti(2,@0/**,@1/**,@2/**))");
}

// ---------------------------------------------------------------------------
// C3 - 2-of-3 with timelock recovery (Task 6.4)
// ---------------------------------------------------------------------------

#[test]
fn corpus_c3_2of3_with_timelock_recovery() {
    common::round_trip_assert("wsh(or_d(multi(2,@0/**,@1/**),and_v(v:older(52560),pk(@2/**))))");
}

// ---------------------------------------------------------------------------
// C4 - 6-key inheritance miniscript (Task 6.5)
// Source: design/CORPUS.md section C4
// ---------------------------------------------------------------------------

#[test]
fn corpus_c4_6key_inheritance() {
    common::round_trip_assert(concat!(
        "wsh(andor(pk(@0/**),after(1200000),or_i(",
        "and_v(v:pkh(@1/**),and_v(v:pkh(@2/**),and_v(v:pkh(@3/**),older(4032)))),",
        "and_v(v:pkh(@4/**),and_v(v:pkh(@5/**),older(32768))))))",
    ));
}

// ---------------------------------------------------------------------------
// C5 - 5-of-9 thresh with 2-key timelock recovery (Task 6.6)
// Source: design/CORPUS.md section C5 (uses thresh() + s: swap wrappers on 8 keys)
// ---------------------------------------------------------------------------

#[test]
fn corpus_c5_5of9_with_2key_recovery() {
    common::round_trip_assert(concat!(
        "wsh(or_d(",
        "thresh(5,pk(@0/**),s:pk(@1/**),s:pk(@2/**),s:pk(@3/**),s:pk(@4/**),",
        "s:pk(@5/**),s:pk(@6/**),s:pk(@7/**),s:pk(@8/**)),",
        "and_v(v:older(105120),multi(2,@9/**,@10/**))))",
    ));
}

// ---------------------------------------------------------------------------
// E10 - Liana "Simple Inheritance" (Task 6.7)
// Source: design/CORPUS.md section E10 / BitBox blog / Liana production template
// ---------------------------------------------------------------------------

#[test]
fn corpus_e10_liana_simple_inheritance() {
    common::round_trip_assert("wsh(or_d(pk(@0/**),and_v(v:pk(@1/**),older(52560))))");
}

// ---------------------------------------------------------------------------
// E12 - Liana "Expanding Multisig" (Task 6.8)
// Source: design/CORPUS.md section E12 / Liana production template
// ---------------------------------------------------------------------------

#[test]
fn corpus_e12_liana_expanding_multisig() {
    common::round_trip_assert("wsh(or_d(multi(2,@0/**,@1/**),and_v(v:older(52560),pk(@2/**))))");
}

// ---------------------------------------------------------------------------
// E13 - HTLC with sha256 preimage (Task 6.9)
// Source: design/CORPUS.md section E13 / sipa miniscript site / BOLT #3
// Hash = SHA-256("hello world") per the upstream PR test pin:
//   b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9
// ---------------------------------------------------------------------------

#[test]
fn corpus_e13_htlc_with_sha256() {
    common::round_trip_assert(concat!(
        "wsh(andor(",
        "pk(@0/**),",
        "sha256(b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9),",
        "and_v(v:pk(@1/**),older(144))))",
    ));
}

// ---------------------------------------------------------------------------
// E14 - Decaying multisig 3-of-3 to 2-of-3 with 6 distinct keys (Task 6.10)
// Source: design/CORPUS.md section E14
// BIP 388-compliant: 6 distinct key indices (not a subset reuse of the same keys)
// ---------------------------------------------------------------------------

#[test]
fn corpus_e14_decaying_multisig_6_keys() {
    common::round_trip_assert(concat!(
        "wsh(or_d(",
        "multi(3,@0/**,@1/**,@2/**),",
        "and_v(v:older(52560),multi(2,@3/**,@4/**,@5/**))))",
    ));
}

// ---------------------------------------------------------------------------
// Coldcard - BIP 388 export (Task 6.11)
//
// Coldcard Mk4 exports multisig wallets in wsh(sortedmulti(k,...)) form with
// BIP 48 derivation paths (m/48'/0'/0'/2' for native segwit).  The canonical
// BIP 388 policy string shape below matches the format described at
// https://coldcard.com/docs/multisig and visible in numerous public Sparrow
// wallet import threads on Bitcoin Stack Exchange.
//
// The Coldcard-exported policy and the C2 corpus entry have identical shape;
// this separate test documents the production-wallet provenance and ensures
// the format covers real hardware-wallet exports.
// ---------------------------------------------------------------------------

#[test]
fn corpus_coldcard_bip388_export() {
    // Standard Coldcard 2-of-3 native-segwit multisig export shape.
    common::round_trip_assert("wsh(sortedmulti(2,@0/**,@1/**,@2/**))");
}

// ---------------------------------------------------------------------------
// encode-decode-encode idempotency (Task 6.12)
//
// For each corpus policy, verify the full encode -> decode -> encode pipeline
// is FIRST-pass byte-stable for template-only policies:
//
// Invariant 1: chunk count is preserved across both encode-decode cycles.
//
// Invariant 2: raw-byte equality between the FIRST and SECOND encodes.
//   The Phase A `6a-bytecode-roundtrip-path-mismatch` fix gives `WalletPolicy`
//   a `decoded_shared_path` field populated by `from_bytecode` and consulted
//   by `to_bytecode`, so the re-encode reproduces the same path declaration
//   verbatim instead of leaking the dummy-key origin path (`m/44'/0'/0'`).
//
// Invariant 3: structural idempotency -- the template (canonical string) is
// preserved after a second decode cycle:
//   decode(encode(p)).canonical == decode(encode(decode(encode(p)))).canonical
//
// Invariant 4: second-pass determinism -- encoding the decoded policy twice
// from the same decoded WalletPolicy gives byte-identical output (was the
// strongest invariant pre-Phase-A; now redundant with Invariant 2 but kept
// as a regression guard against any future encoder non-determinism).
// ---------------------------------------------------------------------------

#[test]
fn corpus_encode_decode_encode_idempotency() {
    for &policy_str in CORPUS_POLICIES {
        let policy: WalletPolicy = policy_str.parse().unwrap_or_else(|e| {
            panic!(
                "idempotency: failed to parse policy {:?}: {}",
                policy_str, e
            )
        });

        // First cycle: original policy -> encode -> decode
        let backup1 = encode(&policy, &EncodeOptions::default()).unwrap_or_else(|e| {
            panic!(
                "idempotency: first encode failed for {:?}: {}",
                policy_str, e
            )
        });
        let raw_strings1: Vec<&str> = backup1.chunks.iter().map(|c| c.raw.as_str()).collect();
        let decode_result1 = decode(&raw_strings1, &DecodeOptions::new()).unwrap_or_else(|e| {
            panic!(
                "idempotency: first decode failed for {:?}: {}",
                policy_str, e
            )
        });

        // Second cycle: decoded policy -> encode -> decode
        let backup2 =
            encode(&decode_result1.policy, &EncodeOptions::default()).unwrap_or_else(|e| {
                panic!(
                    "idempotency: second encode failed for {:?}: {}",
                    policy_str, e
                )
            });
        let raw_strings2: Vec<&str> = backup2.chunks.iter().map(|c| c.raw.as_str()).collect();
        let decode_result2 = decode(&raw_strings2, &DecodeOptions::new()).unwrap_or_else(|e| {
            panic!(
                "idempotency: second decode failed for {:?}: {}",
                policy_str, e
            )
        });

        // Invariant 1: chunk count is stable.
        assert_eq!(
            backup1.chunks.len(),
            backup2.chunks.len(),
            "idempotency: chunk count changed after encode-decode-encode for {:?}",
            policy_str,
        );

        // Invariant 2: FIRST-pass raw-byte equality. With the Phase A
        // `decoded_shared_path` fix, encode -> decode -> encode is byte-stable
        // on the very first round-trip for template-only policies.
        for (i, (c1, c2)) in backup1.chunks.iter().zip(backup2.chunks.iter()).enumerate() {
            assert_eq!(
                c1.raw, c2.raw,
                concat!(
                    "idempotency: first-pass byte-equality violated for chunk[{}],",
                    " policy {:?}\n  encode1: {}\n  encode2: {}"
                ),
                i, policy_str, c1.raw, c2.raw,
            );
        }

        // Invariant 3: structural (template) form is preserved.
        common::assert_structural_eq(&decode_result1.policy, &decode_result2.policy);

        // Invariant 4: second-pass determinism -- encoding the decoded policy
        // again must give byte-identical output.
        let backup2b =
            encode(&decode_result1.policy, &EncodeOptions::default()).unwrap_or_else(|e| {
                panic!(
                    "idempotency: second encode (repeat) failed for {:?}: {}",
                    policy_str, e
                )
            });
        for (i, (c2, c2b)) in backup2
            .chunks
            .iter()
            .zip(backup2b.chunks.iter())
            .enumerate()
        {
            assert_eq!(
                c2.raw, c2b.raw,
                concat!(
                    "idempotency: second encode is not deterministic for chunk[{}]",
                    " (Invariant 4), policy {:?}\n  run1: {}\n  run2: {}"
                ),
                i, policy_str, c2.raw, c2b.raw,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// HRP-lowercase property (Task 6.13)
//
// For each corpus policy, encode and assert all chunk raw strings are
// entirely lowercase ASCII.  Per BIP "General format" / bech32 convention,
// WDM strings are all-lowercase by default.  Bech32 characters are drawn
// from the 32-character lowercase alphabet; this test guards against any
// encoder path that accidentally uppercases characters.
// ---------------------------------------------------------------------------

#[test]
fn corpus_hrp_lowercase_property() {
    for &policy_str in CORPUS_POLICIES {
        let policy: WalletPolicy = policy_str.parse().unwrap_or_else(|e| {
            panic!(
                "hrp_lowercase: failed to parse policy {:?}: {}",
                policy_str, e
            )
        });

        let backup = encode(&policy, &EncodeOptions::default())
            .unwrap_or_else(|e| panic!("hrp_lowercase: encode failed for {:?}: {}", policy_str, e));

        for (i, chunk) in backup.chunks.iter().enumerate() {
            let raw = &chunk.raw;
            assert!(
                raw.chars().all(|c| !c.is_ascii_uppercase()),
                concat!(
                    "hrp_lowercase: chunk[{}] contains uppercase character(s)",
                    " for {:?}\n  chunk: {}"
                ),
                i,
                policy_str,
                raw,
            );
        }
    }
}
