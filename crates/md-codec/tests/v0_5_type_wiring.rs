//! Type-wiring tests for v0.5 multi-leaf TapTree admission.
//! These tests pin the new public surface so future refactors can't
//! accidentally remove fields.
//!
//! `Error::SubsetViolation` is `#[non_exhaustive]` (so external
//! callers can't construct it directly), but downstream destructure
//! patterns must still see the `leaf_index` field. We therefore obtain
//! a real instance via the public encode API (which is the canonical
//! way external callers will encounter the variant) and verify its
//! shape via destructure.

use md_codec::Error;

/// Trigger a `SubsetViolation` via the v0.6 opt-in validator
/// `validate_tap_leaf_subset`. (`to_bytecode` itself is scope-agnostic
/// post-v0.6-strip and no longer rejects out-of-subset tap-leaf
/// operators by default.) The leaf miniscript here uses `sha256(...)`,
/// out of the historical Coldcard tap-leaf subset.
fn trigger_tap_leaf_subset_violation() -> Error {
    use md_codec::bytecode::encode::validate_tap_leaf_subset;
    use miniscript::{Miniscript, Tap};

    let leaf_str = "and_v(v:sha256(b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9),c:pk_k([6738736c/44'/0'/0']xpub6Br37sWxruYfT8ASpCjVHKGwgdnYFEn98DwiN76i2oyY6fgH1LAPmmDcF46xjxJr22gw4jmVjTE2E3URMnRPEPYyo1zoPSUba563ESMXCeb/<0;1>/*))";
    let leaf_ms: Miniscript<miniscript::DescriptorPublicKey, Tap> =
        leaf_str.parse().expect("tap-leaf miniscript parses");
    validate_tap_leaf_subset(&leaf_ms, Some(0))
        .expect_err("validator should reject out-of-subset operator")
}

#[test]
fn tap_leaf_subset_violation_has_leaf_index_field() {
    let err = trigger_tap_leaf_subset_violation();
    match err {
        Error::SubsetViolation {
            operator,
            leaf_index,
            ..
        } => {
            assert!(
                operator.contains("sha256"),
                "expected operator name to mention sha256, got {operator:?}"
            );
            // Single-leaf encode path supplies Some(0); verify the field
            // is present and populated.
            assert_eq!(leaf_index, Some(0));
        }
        other => panic!("expected SubsetViolation, got {other:?}"),
    }
}

#[test]
fn tap_leaf_subset_violation_destructure_with_leaf_index_pattern() {
    let err = trigger_tap_leaf_subset_violation();
    // Pin the field name `leaf_index` so future renames break this test.
    if let Error::SubsetViolation {
        leaf_index: Some(_),
        ..
    } = err
    {
        // shape pinned
    } else {
        panic!("expected SubsetViolation with Some(leaf_index)");
    }
}

use miniscript::Tap;
use miniscript::descriptor::DescriptorPublicKey;
use std::str::FromStr;

#[test]
fn validate_tap_leaf_subset_takes_leaf_index_arg() {
    let ms = miniscript::Miniscript::<DescriptorPublicKey, Tap>::from_str(
        "pk([deadbeef/0'/0'/0']xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz/<0;1>/*)"
    ).expect("valid tap miniscript");

    // Should accept new signature: (ms, leaf_index)
    let result = md_codec::bytecode::encode::validate_tap_leaf_subset(&ms, Some(7));
    assert!(result.is_ok());
}

use md_codec::{DecodeReport, TapLeafReport};
use std::sync::Arc;

#[test]
fn decode_report_has_tap_leaves_field() {
    // Smoke test: pin that DecodeReport has a `tap_leaves` field and that
    // it can be observed empty. We use a real decoded report obtained via
    // the public API since DecodeReport is `#[non_exhaustive]` and cannot
    // be constructed directly from outside the crate.
    use md_codec::{DecodeOptions, EncodeOptions, WalletPolicy, encode};
    let policy: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
    let backup = encode(&policy, &EncodeOptions::default()).expect("encode succeeds");
    // backup.chunks holds one or more EncodedChunk; collect raw strings.
    let strings: Vec<&str> = backup.chunks.iter().map(|c| c.raw.as_str()).collect();
    let decoded = md_codec::decode(&strings, &DecodeOptions::new()).expect("decode succeeds");
    // Non-tr top-level → tap_leaves empty.
    assert_eq!(decoded.report.tap_leaves.len(), 0);
    // Pin the type via type ascription.
    let _: &Vec<TapLeafReport> = &decoded.report.tap_leaves;
    // Pin DecodeReport remains importable.
    let _ = std::any::type_name::<DecodeReport>();
}

#[test]
fn decode_tap_subtree_helper_exists() {
    // Smoke test that the function is reachable via the crate-private path.
    // Real behavioral coverage comes from the decoder routing test
    // (`multi_leaf_two_leaf_symmetric_round_trips`, currently #[ignore]'d
    // pending Phase 4) and the multi-leaf round-trips in Phase 6.
    //
    // This test is a placeholder that compiles iff the surrounding
    // multi-leaf decoder compiles (which exercises `decode_tap_subtree`
    // indirectly via `decode_tr_inner`'s routing).
}

/// End-to-end multi-leaf round-trip test pinning the v0.5 SUCCESS state.
///
/// As of Phase 4, both encoder and decoder route multi-leaf TapTree end-to-end.
/// `tap_leaves` population still lands in Phase 5; until then the round-trip
/// itself is asserted but the report-vector contents are deferred.
#[test]
fn multi_leaf_two_leaf_symmetric_round_trips() {
    use md_codec::{DecodeOptions, EncodeOptions, WalletPolicy, encode};
    let policy_str = "tr(\
[00000000/86'/0'/0']xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz/<0;1>/*,\
{pk([11111111/86'/0'/0']xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz/<0;1>/*),\
pk([22222222/86'/0'/0']xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz/<0;1>/*)\
})";

    let policy: WalletPolicy = policy_str.parse().expect("valid policy");
    let backup = encode(&policy, &EncodeOptions::default()).expect("encode succeeds");
    let strings: Vec<&str> = backup.chunks.iter().map(|c| c.raw.as_str()).collect();
    let decoded = md_codec::decode(&strings, &DecodeOptions::new()).expect("decode succeeds");

    assert_eq!(decoded.report.tap_leaves.len(), 2);
    assert_eq!(decoded.report.tap_leaves[0].leaf_index, 0);
    assert_eq!(decoded.report.tap_leaves[0].depth, 1);
    assert_eq!(decoded.report.tap_leaves[1].leaf_index, 1);
    assert_eq!(decoded.report.tap_leaves[1].depth, 1);
}

#[test]
fn keyonly_tr_produces_empty_tap_leaves() {
    use md_codec::{DecodeOptions, EncodeOptions, WalletPolicy, encode};
    let policy_str = "tr([00000000/86'/0'/0']xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz/<0;1>/*)";
    let policy: WalletPolicy = policy_str.parse().expect("valid policy");
    let backup = encode(&policy, &EncodeOptions::default()).expect("encode succeeds");
    let strings: Vec<&str> = backup.chunks.iter().map(|c| c.raw.as_str()).collect();
    let decoded = md_codec::decode(&strings, &DecodeOptions::new()).expect("decode succeeds");
    assert_eq!(decoded.report.tap_leaves.len(), 0);
}

#[test]
fn single_leaf_tr_produces_one_tap_leaf_at_depth_zero() {
    use md_codec::{DecodeOptions, EncodeOptions, WalletPolicy, encode};
    let policy_str = "tr([00000000/86'/0'/0']xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz/<0;1>/*,pk([11111111/86'/0'/0']xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz/<0;1>/*))";
    let policy: WalletPolicy = policy_str.parse().expect("valid policy");
    let backup = encode(&policy, &EncodeOptions::default()).expect("encode succeeds");
    let strings: Vec<&str> = backup.chunks.iter().map(|c| c.raw.as_str()).collect();
    let decoded = md_codec::decode(&strings, &DecodeOptions::new()).expect("decode succeeds");
    assert_eq!(decoded.report.tap_leaves.len(), 1);
    assert_eq!(decoded.report.tap_leaves[0].leaf_index, 0);
    assert_eq!(decoded.report.tap_leaves[0].depth, 0);
}

#[test]
fn tap_leaf_report_struct_has_required_fields() {
    let ms_str = "pk([deadbeef/0'/0'/0']xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz/<0;1>/*)";
    let ms = miniscript::Miniscript::<DescriptorPublicKey, Tap>::from_str(ms_str).unwrap();
    // TapLeafReport is `#[non_exhaustive]` so external construction would
    // fail; pin the field accessors via type ascription instead.
    fn _accessor_pinning(leaf: &TapLeafReport) {
        let _: usize = leaf.leaf_index;
        let _: &Arc<miniscript::Miniscript<DescriptorPublicKey, Tap>> = &leaf.miniscript;
        let _: u8 = leaf.depth;
    }
    // Also exercise the type itself is exported with the right name.
    let _ = std::any::type_name::<TapLeafReport>();
    // Touch the ms to keep it referenced.
    let _ = ms.to_string();
}
