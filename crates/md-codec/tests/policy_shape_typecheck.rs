//! md review r2 I-1: a card that is not typed miniscript has no summary and
//! no key. `or_b(pk(@0), thresh(2,…))` was summarised as "1 key / any 2 of 3"
//! and keyed, yet spends with NO signature (the thresh's first CHECKSIG eats
//! the 0 left by `pk(@0)`, and BOOLOR takes an attacker's 1). The allowlist
//! vouches for a `thresh` node only; the type check vouches for the context.
//!
//! Display and identity only: these cards still decode, re-encode to the same
//! bytes and keep their ids.

mod common;

use common::{UNTYPED_CARDS, keyarg, node2, thresh_node, untyped_trees, wrap};
use md_codec::decode::decode_md1_string;
use md_codec::encode::Descriptor;
use md_codec::policy_shape::{policy_shape, policy_type_checks};
use md_codec::skeleton::{SkeletonError, skeleton};
use md_codec::tag::Tag;
use md_codec::tree::{Body, InternalKey, Node};

fn pk(i: u8) -> Node {
    keyarg(Tag::PkK, i)
}
fn s(x: Node) -> Node {
    wrap(Tag::Swap, x)
}
fn a(x: Node) -> Node {
    wrap(Tag::Alt, x)
}
fn v(x: Node) -> Node {
    wrap(Tag::Verify, x)
}
fn wsh(x: Node) -> Node {
    wrap(Tag::Wsh, x)
}
fn three(first: u8) -> Node {
    thresh_node(2, vec![pk(first), s(pk(first + 1)), s(pk(first + 2))])
}
fn nums(x: Node) -> Node {
    Node {
        tag: Tag::Tr,
        body: Body::Tr {
            internal_key: InternalKey::NumsPoint,
            tree: Some(Box::new(x)),
        },
    }
}

fn template(tree: Node) -> Descriptor {
    common::descriptor_from_tree(tree, true)
}

#[test]
fn the_review_cards_decode_but_get_no_summary_and_no_key() {
    for (_, card) in UNTYPED_CARDS {
        let d = decode_md1_string(card).expect("the card still decodes");
        assert!(!policy_type_checks(&d), "{card}");
        assert!(!policy_shape(&d).complete, "{card}");
        assert_eq!(
            skeleton(&d).unwrap_err(),
            SkeletonError::NotMiniscript,
            "{card}"
        );
        // Identity untouched: it re-encodes to the same payload and keeps ids.
        let (bytes, bits) = md_codec::encode::encode_payload_unadmitted(&d).unwrap();
        let back = md_codec::decode::decode_payload(&bytes, bits).unwrap();
        assert_eq!(back, d, "{card}");
        md_codec::identity::compute_md1_encoding_id(&d).unwrap();
        md_codec::identity::compute_wallet_descriptor_template_id(&d).unwrap();
    }
}

#[test]
fn every_untyped_tree_is_incomplete_and_keyless() {
    let accepted: Vec<&str> = untyped_trees()
        .into_iter()
        .filter(|(_, tree)| policy_type_checks(&template(tree.clone())))
        .map(|(name, _)| name)
        .collect();
    assert!(
        accepted.is_empty(),
        "accepted as valid miniscript: {accepted:?}"
    );
    for (name, tree) in untyped_trees() {
        let d = template(tree);
        assert!(!policy_shape(&d).complete, "{name}");
        assert_eq!(
            skeleton(&d).unwrap_err(),
            SkeletonError::NotMiniscript,
            "{name}"
        );
    }
}

/// The typed spelling of the same policies stays summarised.
#[test]
fn the_typed_spellings_stay_complete() {
    for (name, tree) in [
        ("or_b_a_thresh", wsh(node2(Tag::OrB, pk(0), a(three(1))))),
        ("or_b_pk_a_pk", wsh(node2(Tag::OrB, pk(0), a(pk(1))))),
        ("or_d_pk_pk", wsh(node2(Tag::OrD, pk(0), pk(1)))),
        ("and_v_v_pk_pk", wsh(node2(Tag::AndV, v(pk(0)), pk(1)))),
        ("tr_leaf_or_b", nums(node2(Tag::OrB, pk(0), a(pk(1))))),
    ] {
        let d = template(tree);
        assert!(policy_type_checks(&d), "{name}");
        assert!(policy_shape(&d).complete, "{name}");
    }
}

#[test]
fn the_not_miniscript_refusal_text() {
    assert_eq!(
        SkeletonError::NotMiniscript.to_string(),
        "this policy is not valid miniscript (it fails a miniscript type, timelock or script-limit \
         rule), so md shows no spend-path summary for it; no key can be computed from it"
    );
}
