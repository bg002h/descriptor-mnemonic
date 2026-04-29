//! Hostile-input fixtures for v0.5 multi-leaf TapTree decoder.
//!
//! These tests construct bytecode directly (not via the encoder) to exercise
//! decoder edge cases that a well-behaved encoder would not produce. They
//! call the bytecode-tree-level decoder (`md_codec::bytecode::decode::decode_template`)
//! rather than `decode_string` so that:
//!
//! - the chunking/codex32 layer is out of scope (its size-bounded plans
//!   reject H4's 10K-byte recursion bomb on its own);
//! - the BIP 388 wallet-policy template re-derivation
//!   (`WalletPolicy::from_descriptor`, which enforces monotonic `@N` index
//!   ordering) is bypassed — H1's 129-leaf tree references many duplicate
//!   keys, which the underlying `Descriptor`/`TapTree` accept fine.
//!
//! Per `design/SPEC_v0_5_multi_leaf_taptree.md` §5 (H1-H5) and the
//! implementation plan Phase 6 Task 6.3.

use md_codec::bytecode::decode::decode_template;
use md_codec::bytecode::tag::Tag;
use md_codec::{Error, decode_bytecode};
use miniscript::descriptor::DescriptorPublicKey;
use std::str::FromStr;

/// 32 dummy `DescriptorPublicKey`s (matches `policy::all_dummy_keys`'s cap).
/// Used for the depth-stress tests so placeholder indices in the constructed
/// bytecode resolve cleanly. We re-define the small set here to keep this
/// test file self-contained (the upstream `all_dummy_keys` is a private
/// helper).
fn dummy_keys() -> Vec<DescriptorPublicKey> {
    // Use 32 distinct origin fingerprints so each index resolves to a
    // distinct key. The xpub itself is shared; only the origin fingerprint
    // differs (sufficient because the depth-stress tests don't exercise
    // origin-uniqueness). Path is `<0;1>/*` so leaf-substituted keys are
    // wildcard-multipath-derivable.
    const XPUB: &str = "xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz";
    (0..32u32)
        .map(|i| {
            let key_str = format!("[{:08x}/86'/0'/0']{XPUB}/<0;1>/*", i);
            DescriptorPublicKey::from_str(&key_str)
                .unwrap_or_else(|e| panic!("dummy key {i} parse failed: {e}"))
        })
        .collect()
}

/// Bytecode template prefix for `tr(KEY, ...)`: `[Tr][Placeholder][0]`.
/// (No bytecode header byte, no path declaration — those are not part of the
/// template tree consumed by `decode_template`.)
fn tr_template_prefix() -> Vec<u8> {
    vec![
        Tag::Tr.as_byte(),
        Tag::Placeholder.as_byte(),
        0u8, // internal-key placeholder index 0
    ]
}

/// Build a left-spine taptree template: `framings` `[TapTree]` framing bytes,
/// followed by 1 bottom-left leaf and `framings` right-children leaves
/// (encountered in DFS pre-order as the recursion unwinds).
///
/// Each leaf is `[Tag::PkK, Tag::Placeholder, idx]` where `idx` cycles in
/// `1..32` (placeholder 0 is reserved for the internal key). Reused indices
/// resolve to the same dummy key; that is harmless for the depth/structure
/// tests because we bypass BIP 388 re-derivation by calling `decode_template`
/// directly.
///
/// Result: a tree where the deepest leaf is at miniscript-depth `framings`
/// (after `TapTree::combine` post-increments depth on each unwind). For BIP
/// 341's TAPROOT_CONTROL_MAX_NODE_COUNT = 128, call with `framings = 128`
/// (legal); for boundary rejection, call with `129` (illegal).
fn build_left_spine_taptree_template(framings: usize) -> Vec<u8> {
    let mut out = tr_template_prefix();
    for _ in 0..framings {
        out.push(Tag::TapTree.as_byte());
    }
    // One bottom leaf at the deepest left position (miniscript-depth = framings).
    out.push(Tag::PkK.as_byte());
    out.push(Tag::Placeholder.as_byte());
    out.push(1u8);
    // `framings` right-children — one per framing — emitted in the order the
    // decoder consumes them as recursion unwinds. Cycle indices through 1..32
    // to stay within the bytecode-level decoder's 32-key cap.
    for i in 0..framings {
        out.push(Tag::PkK.as_byte());
        out.push(Tag::Placeholder.as_byte());
        let idx = ((i % 31) as u8) + 1;
        out.push(idx);
    }
    out
}

#[test]
fn accepts_taptree_with_leaves_at_miniscript_depth_128() {
    // H1: 128 framings + leaves => deepest leaf at miniscript-depth 128
    // (BIP 341 max, legal). The gate `depth > 128` in decode_tap_subtree
    // fires only when entering a recursive call with depth == 129, i.e.
    // reading a hypothetical 129th `[TapTree]` byte. With exactly 128
    // framings, the 128th call's left-recursion enters at depth=129 but
    // immediately reads a LEAF (not another `[TapTree]`), so the gate
    // never fires.
    let template = build_left_spine_taptree_template(128);
    let keys = dummy_keys();
    let descriptor =
        decode_template(&template, &keys).expect("BIP 341 max-depth left-spine tree must decode");

    // Walk the descriptor's TapTree leaves and confirm:
    //   1. there are 1 + 128 = 129 leaves (one bottom-left + one per framing)
    //   2. the deepest leaf is at miniscript-depth 128 (BIP 341 boundary)
    use miniscript::Descriptor;
    let tr = match descriptor {
        Descriptor::Tr(tr) => tr,
        other => panic!("expected Descriptor::Tr, got {other:?}"),
    };
    let leaves: Vec<_> = match tr.tap_tree() {
        Some(tt) => tt.leaves().collect(),
        None => panic!("expected multi-leaf TapTree, got KeyOnly"),
    };
    assert_eq!(
        leaves.len(),
        129,
        "left-spine of 128 framings produces 129 leaves (1 bottom + 128 right)"
    );
    let max_depth = leaves
        .iter()
        .map(|l| l.depth())
        .max()
        .expect("at least one leaf");
    assert_eq!(
        max_depth, 128,
        "expected max miniscript-depth 128 (BIP 341 boundary)"
    );
}

#[test]
fn rejects_taptree_at_miniscript_depth_129() {
    // H2: 129 framings — the gate fires at recursion-depth=129 reading the
    // 129th `[TapTree]` byte. Expected: PolicyScopeViolation with the
    // depth-128 message.
    let template = build_left_spine_taptree_template(129);
    let keys = dummy_keys();
    let err = decode_template(&template, &keys).expect_err("129-deep tree must reject");
    let msg = format!("{err}");
    assert!(
        matches!(err, Error::PolicyScopeViolation(_)),
        "expected PolicyScopeViolation, got: {err:?}"
    );
    assert!(
        msg.contains("TapTree depth exceeds BIP 341 consensus maximum"),
        "expected depth-128 PolicyScopeViolation message, got: {msg}"
    );
}

#[test]
fn rejects_taptree_with_truncated_subtree() {
    // H3: `[Tr][Placeholder][0][TapTree]` then EOF — cursor runs out reading
    // the left child. Use the bytecode-level decoder (`decode_bytecode`)
    // since it exercises the same path; the truncation is at the template
    // tree, not the header. Build a complete bytecode (header + path decl +
    // truncated template) so `decode_bytecode` parses the framing then
    // surfaces the missing-children error from `decode_tap_subtree`.
    let bytecode = vec![
        0x00, // bytecode header
        Tag::SharedPath.as_byte(),
        0x04, // BIP 86 indicator
        Tag::Tr.as_byte(),
        Tag::Placeholder.as_byte(),
        0u8,
        Tag::TapTree.as_byte(),
    ];
    let err = decode_bytecode(&bytecode).expect_err("truncated subtree must reject");
    let msg = format!("{err:?}");
    assert!(
        matches!(err, Error::InvalidBytecode { .. }),
        "expected InvalidBytecode, got: {err:?}"
    );
    assert!(
        msg.contains("UnexpectedEnd") || msg.contains("Truncated"),
        "expected UnexpectedEnd/Truncated InvalidBytecode kind, got: {msg}"
    );
}

#[test]
fn rejects_deeply_nested_recursion_bomb() {
    // H4: 10K `[TapTree]` bytes with no leaves — must reject at recursion-
    // depth 129 cleanly, BEFORE stack overflow (rationale: peek-before-
    // recurse + depth gate fires at the BIP 341 boundary, well below
    // stack-overflow risk).
    let mut template = tr_template_prefix();
    for _ in 0..10_000 {
        template.push(Tag::TapTree.as_byte());
    }
    let keys = dummy_keys();
    let err = decode_template(&template, &keys).expect_err("recursion bomb must reject");
    let msg = format!("{err}");
    assert!(
        matches!(err, Error::PolicyScopeViolation(_)),
        "expected PolicyScopeViolation (depth gate fires before stack overflow), got: {err:?}"
    );
    assert!(
        msg.contains("TapTree depth exceeds BIP 341 consensus maximum"),
        "expected depth-128 violation message, got: {msg}"
    );
}

#[test]
fn rejects_taptree_unrecognized_inner_at_depth() {
    // H5: `[Tr][Placeholder][0][TapTree][TapTree][unallocated_byte]` — at
    // recursion-depth 3, peek sees an unrecognised byte; helper returns
    // InvalidBytecode { kind: UnknownTag } with offset pointing at the
    // unrecognised byte.
    let mut template = tr_template_prefix();
    template.push(Tag::TapTree.as_byte());
    template.push(Tag::TapTree.as_byte());
    template.push(0xff_u8); // unallocated tag byte
    let keys = dummy_keys();
    let err = decode_template(&template, &keys).expect_err("unknown-tag at depth must reject");
    let msg = format!("{err:?}");
    assert!(
        matches!(err, Error::InvalidBytecode { .. }),
        "expected InvalidBytecode, got: {err:?}"
    );
    assert!(
        msg.contains("UnknownTag"),
        "expected UnknownTag in InvalidBytecode kind, got: {msg}"
    );
}
