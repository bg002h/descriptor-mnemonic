//! Round-trip + leaf-index propagation + parser-roundtrip tests for v0.5
//! multi-leaf TapTree.
//!
//! Covers, per `design/SPEC_v0_5_multi_leaf_taptree.md` §5:
//! - RT1-RT4: end-to-end policy-encode → decode round-trips for the multi-leaf
//!   T3, T4, T6 fixtures, plus a defense-against-symbug bytecode-distinctness
//!   check between left-heavy T4 and right-heavy T5.
//! - LI1-LI3: `decode_report.tap_leaves` populates `leaf_index` in DFS
//!   pre-order, plumbs `leaf_index` into `Error::SubsetViolation`,
//!   and treats single-leaf `tr(KEY, leaf)` as `leaf_index = 0`.
//! - PR1-PR2: parser-roundtrip equivalence — re-parsing the recovered
//!   policy's canonical string yields a structurally-identical descriptor.

use md_codec::bytecode::tag::Tag;
use md_codec::{
    DecodeOptions, DecodeResult, EncodeOptions, Error, WalletPolicy, decode, decode_bytecode,
    encode,
};

// @-template form policy strings — preferred for round-trip equivalence
// because the BIP 388 wallet-policy parser handles `multi_a` inside
// `{...}` cleanly only in template form (the concrete-key inlined form
// hits a parser quirk for the same shape; tracked as an upstream
// rust-miniscript issue but transparent to this codec).
//
// These match the v0.5 corpus fixtures in `crates/md-codec/src/vectors.rs`
// (T3-T6).
const T3_POLICY: &str = "tr(@0/**,{pk(@1/**),pk(@2/**)})";
const T4_POLICY: &str = "tr(@0/**,{pk(@1/**),{pk(@2/**),pk(@3/**)}})";
const T5_POLICY: &str = "tr(@0/**,{{pk(@1/**),pk(@2/**)},pk(@3/**)})";
const T6_POLICY: &str = "tr(@0/**,{pk(@1/**),multi_a(2,@2/**,@3/**)})";
const SINGLE_LEAF_POLICY: &str = "tr(@0/**,pk(@1/**))";

fn encode_then_decode(policy_str: &str) -> Result<DecodeResult, Error> {
    let policy: WalletPolicy = policy_str.parse()?;
    let backup = encode(&policy, &EncodeOptions::default())?;
    let strings: Vec<&str> = backup.chunks.iter().map(|c| c.raw.as_str()).collect();
    decode(&strings, &DecodeOptions::new())
}

#[test]
fn roundtrip_two_leaf_symmetric() {
    // RT1: T3 — `tr(@0, {pk(@1), pk(@2)})`.
    let decoded = encode_then_decode(T3_POLICY).expect("T3 round-trip succeeds");
    let original: WalletPolicy = T3_POLICY.parse().unwrap();
    assert_eq!(
        decoded.policy.to_canonical_string(),
        original.to_canonical_string(),
        "T3 round-trip must be canonical-string-stable"
    );
}

#[test]
fn roundtrip_three_leaf_asymmetric() {
    // RT2: T4 — `tr(@0, {pk(@1), {pk(@2), pk(@3)}})` (left-heavy).
    let decoded = encode_then_decode(T4_POLICY).expect("T4 round-trip succeeds");
    let original: WalletPolicy = T4_POLICY.parse().unwrap();
    assert_eq!(
        decoded.policy.to_canonical_string(),
        original.to_canonical_string(),
        "T4 round-trip must be canonical-string-stable"
    );
}

#[test]
fn roundtrip_multi_leaf_with_multi() {
    // RT3: T6 — `tr(@0, {pk(@1), multi_a(2, @2, @3)})`.
    let decoded = encode_then_decode(T6_POLICY).expect("T6 round-trip succeeds");
    let original: WalletPolicy = T6_POLICY.parse().unwrap();
    assert_eq!(
        decoded.policy.to_canonical_string(),
        original.to_canonical_string(),
        "T6 round-trip must be canonical-string-stable"
    );
}

#[test]
fn t4_left_heavy_and_t5_right_heavy_emit_distinct_bytecodes() {
    // RT4: defense against accidental symmetric-bug. T4 (depth 1/2/2) and T5
    // (depth 2/2/1) must encode to different bytecode.
    let p4: WalletPolicy = T4_POLICY.parse().unwrap();
    let p5: WalletPolicy = T5_POLICY.parse().unwrap();
    let bc4 = p4
        .to_bytecode(&EncodeOptions::default())
        .expect("T4 encode succeeds");
    let bc5 = p5
        .to_bytecode(&EncodeOptions::default())
        .expect("T5 encode succeeds");
    assert_ne!(
        bc4, bc5,
        "left-heavy T4 vs right-heavy T5 must produce different bytecode"
    );
}

#[test]
fn decode_report_populates_leaf_index_dfs_preorder() {
    // LI1: T4 has leaves at depth 1, 2, 2 (DFS pre-order), so
    // decode_report.tap_leaves[i].leaf_index == i and depths match the tree
    // shape `{pk(@1), {pk(@2), pk(@3)}}`.
    let decoded = encode_then_decode(T4_POLICY).expect("T4 round-trip succeeds");
    let leaves = &decoded.report.tap_leaves;
    assert_eq!(leaves.len(), 3, "T4 has exactly 3 leaves");
    assert_eq!(leaves[0].leaf_index, 0, "leaf 0 in DFS pre-order");
    assert_eq!(leaves[0].depth, 1, "leaf 0 (left child) at depth 1");
    assert_eq!(leaves[1].leaf_index, 1, "leaf 1 in DFS pre-order");
    assert_eq!(
        leaves[1].depth, 2,
        "leaf 1 (left of right subtree) at depth 2"
    );
    assert_eq!(leaves[2].leaf_index, 2, "leaf 2 in DFS pre-order");
    assert_eq!(
        leaves[2].depth, 2,
        "leaf 2 (right of right subtree) at depth 2"
    );
}

// LI2 — REMOVED in v0.6. The decode-side `SubsetViolation { leaf_index }`
// attribution was a Layer-3 concern; v0.6 strip-Layer-3 made the decoder
// reject out-of-context tap-leaf inner tags via a structural diagnostic
// (`BytecodeErrorKind::TagInvalidContext { tag, context: "tap-leaf-inner" }`)
// instead of `SubsetViolation`. The leaf-index attribution moves to
// md-signer-compat (v0.7+) which calls `validate_tap_leaf_subset` per leaf.

#[test]
fn single_leaf_tr_uses_leaf_index_zero() {
    // LI3: T2 — single-leaf `tr(KEY, pk(KEY))` produces exactly one
    // tap_leaves entry with leaf_index=0 and depth=0 (single-leaf
    // canonicalisation).
    let decoded = encode_then_decode(SINGLE_LEAF_POLICY).expect("single-leaf round-trip succeeds");
    assert_eq!(decoded.report.tap_leaves.len(), 1);
    assert_eq!(decoded.report.tap_leaves[0].leaf_index, 0);
    assert_eq!(decoded.report.tap_leaves[0].depth, 0);
}

#[test]
fn parser_roundtrip_t4() {
    // PR1: re-parsing the recovered policy's canonical string yields a
    // structurally-identical descriptor.
    let original: WalletPolicy = T4_POLICY.parse().unwrap();
    let decoded = encode_then_decode(T4_POLICY).expect("T4 round-trip succeeds");
    let recovered_canonical = decoded.policy.to_canonical_string();
    let reparsed: WalletPolicy = recovered_canonical
        .parse()
        .expect("recovered canonical string must parse cleanly");
    assert_eq!(
        original.to_canonical_string(),
        reparsed.to_canonical_string(),
        "T4 parser-roundtrip must preserve canonical form"
    );
}

#[test]
fn parser_roundtrip_t6_with_multi() {
    // PR2: same as PR1 for T6 (mixed pk + multi_a leaves).
    let original: WalletPolicy = T6_POLICY.parse().unwrap();
    let decoded = encode_then_decode(T6_POLICY).expect("T6 round-trip succeeds");
    let recovered_canonical = decoded.policy.to_canonical_string();
    let reparsed: WalletPolicy = recovered_canonical
        .parse()
        .expect("recovered canonical string must parse cleanly");
    assert_eq!(
        original.to_canonical_string(),
        reparsed.to_canonical_string(),
        "T6 parser-roundtrip must preserve canonical form"
    );
}
