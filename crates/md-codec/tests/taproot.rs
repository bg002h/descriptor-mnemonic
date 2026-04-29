//! Phase D — Taproot single-leaf round-trip and rejection tests.
//!
//! Covers:
//! - Round-trip positives: key-path-only `tr(K)`, single-leaf `tr(K, pk(K))`,
//!   `tr(K, multi_a(...))`, and a nested `or_d`/`and_v`/`older` shape.
//! - Subset rejections: out-of-subset operators inside the leaf
//!   (`sha256` wrapped under `and_v`).
//! - Wire-format rejections: a synthesized `Tag::TapTree` (0x08) byte
//!   sequence and a nested `Tag::Tr` are rejected with the correct
//!   `PolicyScopeViolation` messages.
//!
//! See `design/PHASE_v0_2_D_DECISIONS.md` for the binding spec.

use md_codec::{EncodeOptions, Error, WalletPolicy};

/// Round-trip a policy through `to_bytecode → from_bytecode → to_bytecode`
/// and assert the second `to_bytecode` produces byte-identical output to
/// the first. Returns the bytecode on success so individual tests can
/// additionally pin specific tag bytes.
#[track_caller]
fn assert_roundtrips(policy_str: &str) -> Vec<u8> {
    let policy: WalletPolicy = policy_str
        .parse()
        .unwrap_or_else(|e| panic!("policy {policy_str:?} should parse: {e}"));
    let bytes_first = policy
        .to_bytecode(&EncodeOptions::default())
        .unwrap_or_else(|e| panic!("policy {policy_str:?} to_bytecode failed: {e}"));
    let decoded = WalletPolicy::from_bytecode(&bytes_first)
        .unwrap_or_else(|e| panic!("policy {policy_str:?} from_bytecode failed: {e}"));
    let bytes_second = decoded
        .to_bytecode(&EncodeOptions::default())
        .unwrap_or_else(|e| panic!("policy {policy_str:?} second to_bytecode failed: {e}"));
    assert_eq!(
        bytes_first, bytes_second,
        "round-trip should be byte-stable for {policy_str:?}"
    );
    bytes_first
}

// ---------------------------------------------------------------------------
// Round-trip positives
// ---------------------------------------------------------------------------

#[test]
fn taproot_key_path_only_round_trips() {
    // `tr(@0/**)` — key-spend-only taproot. Bytecode shape (with header
    // byte and shared-path declaration prefixed):
    //   [header][shared-path declaration][Tag::Tr=0x06][Tag::Placeholder=0x32][0x00]
    let bytes = assert_roundtrips("tr(@0/**)");
    // Pin the trailing 3 bytes of the operator tree.
    let n = bytes.len();
    assert_eq!(
        &bytes[n - 3..],
        &[0x06, 0x32, 0x00],
        "expected [Tr][Placeholder][idx=0] tail, got {:02x?}",
        &bytes[n - 3..]
    );
}

#[test]
fn taproot_single_leaf_pk_round_trips() {
    // `tr(@0/**, pk(@1/**))` — single-leaf with a script path that uses a
    // distinct placeholder. The leaf miniscript `pk(K)` is parsed by the
    // BIP 388 frontend as `c:pk_k(K)`, so the leaf encoding is
    //   [Tag::Check=0x0C][Tag::PkK=0x1B][Tag::Placeholder=0x32][idx=1]
    // and the full operator tree is
    //   [Tag::Tr=0x06][Tag::Placeholder=0x32][0x00]
    //   [Tag::Check=0x0C][Tag::PkK=0x1B][Tag::Placeholder=0x32][0x01]
    let bytes = assert_roundtrips("tr(@0/**,pk(@1/**))");
    let n = bytes.len();
    assert_eq!(
        &bytes[n - 7..],
        &[0x06, 0x32, 0x00, 0x0C, 0x1B, 0x32, 0x01],
        "expected [Tr][Plc][0][Check][PkK][Plc][1] tail, got {:02x?}",
        &bytes[n - 7..]
    );
}

#[test]
fn taproot_single_leaf_multi_a_round_trips() {
    // `tr(@0/**, multi_a(2, @1/**, @2/**, @3/**))` — single-leaf with a
    // 2-of-3 multi_a. multi_a is in the Coldcard subset.
    let bytes = assert_roundtrips("tr(@0/**,multi_a(2,@1/**,@2/**,@3/**))");
    // Pin: [Tr][Plc][0][MultiA][k=2][n=3][Plc][1][Plc][2][Plc][3]
    let n = bytes.len();
    let expected_tail: &[u8] = &[
        0x06, 0x32, 0x00, // Tr, Placeholder, idx 0 (internal key)
        0x1A, 0x02, 0x03, // MultiA, k=2, n=3
        0x32, 0x01, 0x32, 0x02, 0x32, 0x03, // 3 placeholder records
    ];
    assert_eq!(
        &bytes[n - expected_tail.len()..],
        expected_tail,
        "multi_a tail mismatch, got {:02x?}",
        &bytes[n - expected_tail.len()..]
    );
}

#[test]
fn taproot_nested_subset_round_trips() {
    // `tr(@0/**, or_d(pk(@1/**), and_v(v:older(144), pk(@2/**))))` —
    // exercises every Coldcard-allowed composition: `or_d`, `and_v`,
    // `pk` (= `c:pk_k`), `v:` wrapper, `older`. This is the canonical
    // Liana-style timelock recovery shape projected to taproot.
    let _ = assert_roundtrips("tr(@0/**,or_d(pk(@1/**),and_v(v:older(144),pk(@2/**))))");
}

// ---------------------------------------------------------------------------
// Subset rejections (D-2)
// ---------------------------------------------------------------------------

#[test]
fn taproot_rejects_out_of_subset_sha256() {
    // sha256 is not in the Coldcard subset. Wrap inside and_v(v:..., pk(...))
    // so the upstream miniscript parser doesn't reject for "all spend paths
    // must require a signature".
    let policy: WalletPolicy =
        "tr(@0/**,and_v(v:sha256(b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9),pk(@1/**)))"
            .parse()
            .expect("policy should parse");
    let err = policy.to_bytecode(&EncodeOptions::default()).unwrap_err();
    match err {
        Error::SubsetViolation { ref operator, .. } => {
            assert!(
                operator.contains("sha256"),
                "expected operator='sha256', got {operator:?}"
            );
        }
        other => panic!("expected SubsetViolation, got {other:?}"),
    }
}

#[test]
fn taproot_rejects_wrapper_alt_outside_subset() {
    // The `s:` wrapper is required by miniscript typing for `or_b` /
    // `and_b` / `thresh` children, but `or_b` / `and_b` / `thresh` are
    // themselves out-of-subset. Use a thresh fragment to drive the
    // rejection — miniscript surfaces the outer operator name first.
    let policy: WalletPolicy = "tr(@0/**,thresh(2,pk(@1/**),s:pk(@2/**),s:pk(@3/**)))"
        .parse()
        .expect("policy should parse");
    let err = policy.to_bytecode(&EncodeOptions::default()).unwrap_err();
    match err {
        Error::SubsetViolation { ref operator, .. } => {
            // The outer operator (`thresh`) is what the validator hits
            // first when walking the AST. We don't prescribe which is
            // reported, but we do prescribe that *some* out-of-subset
            // operator name is named.
            assert!(
                operator == "thresh" || operator.starts_with("s:"),
                "expected thresh or s: rejection, got {operator:?}"
            );
        }
        other => panic!("expected SubsetViolation, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Wire-format rejections (D-1, D-3)
// ---------------------------------------------------------------------------

#[test]
fn taproot_decodes_tag_taptree_routes_into_subtree_helper() {
    // v0.5 admits Tag::TapTree (0x08) as the multi-leaf inner-node
    // framing. Appending a bare 0x08 byte to a `tr(@0/**)` bytecode
    // therefore enters `decode_tap_subtree` — which immediately needs
    // the left child and surfaces `UnexpectedEnd` when the byte stream
    // terminates instead.
    //
    // (v0.4 used to reject Tag::TapTree with a v1+-deferred
    // PolicyScopeViolation; that rejection was removed in v0.5 Phase 2.
    // See `design/SPEC_v0_5_multi_leaf_taptree.md` §3 for the routing
    // semantics; Phase 6 adds the canonical N1-N9 negative fixtures.)
    let policy: WalletPolicy = "tr(@0/**)".parse().unwrap();
    let mut bytes = policy.to_bytecode(&EncodeOptions::default()).unwrap();
    bytes.push(0x08); // Tag::TapTree — multi-leaf inner-node framing
    let err = WalletPolicy::from_bytecode(&bytes).unwrap_err();
    match err {
        Error::InvalidBytecode {
            kind: md_codec::BytecodeErrorKind::UnexpectedEnd,
            ..
        } => {}
        other => panic!("expected InvalidBytecode/UnexpectedEnd, got {other:?}"),
    }
}

#[test]
fn taproot_rejects_nested_tr_inside_wsh() {
    // wsh() inner cannot be Tr — the tag is only valid at the top level.
    // Manually compose a bytecode stream: header + minimal shared-path
    // declaration + Tag::Wsh + Tag::Tr + ... and verify the decoder
    // rejects with PolicyScopeViolation about an invalid inner-fragment
    // tag (Tr is recognised but mis-placed).
    let policy: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
    let valid = policy.to_bytecode(&EncodeOptions::default()).unwrap();
    // Locate Tag::Wsh (0x05) in the valid bytecode and replace the byte
    // after it (the Wsh inner tag) with Tag::Tr (0x06) to force a nested
    // Tr that doesn't exist syntactically through the parser.
    let wsh_pos = valid
        .iter()
        .position(|&b| b == 0x05)
        .expect("encoded wsh must contain Tag::Wsh");
    // Synthesise: keep [..wsh_pos+1], then Tag::Tr+Placeholder+idx 0 to
    // simulate a nested tr() inside wsh. The decoder must reject without
    // panicking.
    let mut bytes = valid[..=wsh_pos].to_vec();
    bytes.extend_from_slice(&[0x06, 0x32, 0x00]); // Tr, Placeholder, idx 0
    let err = WalletPolicy::from_bytecode(&bytes).unwrap_err();
    match err {
        Error::PolicyScopeViolation(ref msg) => {
            assert!(
                msg.contains("inner-fragment") || msg.contains("Tr"),
                "expected inner-fragment / Tr rejection, got {msg:?}"
            );
        }
        other => panic!("expected PolicyScopeViolation, got {other:?}"),
    }
}
