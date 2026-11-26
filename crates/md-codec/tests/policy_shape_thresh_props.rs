//! md thresh / seedhammer I-1: the type-check property (DESIGN_thresh_summary
//! section 6, review C-1 fix 2). The allowlist is the only type check the
//! summary has, so every `thresh` it accepts must be one `to_miniscript`
//! accepts. If either test finds a counterexample, the list is wrong and the
//! design re-opens.

mod common;

use common::{
    assign_sequential_indices, canon, descriptor_with_pubkeys, keyarg, node2, node3, tr_node,
    wire_descriptor_strategy, wrap,
};
use md_codec::policy_shape::{AnyOf, policy_shape};
use md_codec::tag::Tag;
use md_codec::to_miniscript_descriptor;
use md_codec::tree::{Body, Node};
use proptest::prelude::*;

fn f() -> Node {
    Node {
        tag: Tag::False,
        body: Body::Empty,
    }
}

/// KEY: `pk`/`pkh`, bare or under one `c:`. `(node, is_pkh)`.
fn key() -> impl Strategy<Value = (Node, bool)> {
    (any::<bool>(), any::<bool>()).prop_map(|(pkh, checked)| {
        let k = keyarg(if pkh { Tag::PkH } else { Tag::PkK }, 0);
        (if checked { wrap(Tag::Check, k) } else { k }, pkh)
    })
}

/// T: all four operand kinds, in range.
fn t() -> impl Strategy<Value = Node> {
    prop_oneof![
        (1u32..=65535).prop_map(|v| (Tag::Older, v)),
        (1u32..=65535).prop_map(|v| (Tag::Older, 0x0040_0000 | v)),
        (1u32..500_000_000).prop_map(|v| (Tag::After, v)),
        (500_000_000u32..(1 << 31)).prop_map(|v| (Tag::After, v)),
    ]
    .prop_map(|(tag, v)| Node {
        tag,
        body: Body::Timelock(v),
    })
}

/// LOCK: `ln:T`, `un:T`, `nl:T`, `nu:T`.
fn lock() -> impl Strategy<Value = Node> {
    (0u8..4, t()).prop_map(|(form, t)| match form {
        0 => node2(Tag::OrI, f(), wrap(Tag::ZeroNotEqual, t)),
        1 => node2(Tag::OrI, wrap(Tag::ZeroNotEqual, t), f()),
        2 => wrap(Tag::ZeroNotEqual, node2(Tag::OrI, f(), t)),
        _ => wrap(Tag::ZeroNotEqual, node2(Tag::OrI, t, f())),
    })
}

fn first_child() -> impl Strategy<Value = Node> {
    prop_oneof![key().prop_map(|(k, _)| k), lock()]
}

fn later_child() -> impl Strategy<Value = Node> {
    prop_oneof![
        key()
            .prop_filter("s: over pk only", |(_, pkh)| !pkh)
            .prop_map(|(k, _)| wrap(Tag::Swap, k)),
        key().prop_map(|(k, _)| wrap(Tag::Alt, k)),
        lock().prop_map(|l| wrap(Tag::Swap, l)),
        lock().prop_map(|l| wrap(Tag::Alt, l)),
    ]
}

/// `(thresh, k, n)` drawn from the allowlist grammar only.
fn allowlisted_thresh() -> impl Strategy<Value = (Node, u8, u8)> {
    (2usize..=6)
        .prop_flat_map(|n| {
            (
                1..=n as u8,
                first_child(),
                proptest::collection::vec(later_child(), n - 1),
            )
        })
        .prop_map(|(k, first, rest)| {
            let n = 1 + rest.len() as u8;
            let mut children = vec![first];
            children.extend(rest);
            (common::thresh_node(k, children), k, n)
        })
}

fn pk() -> Node {
    keyarg(Tag::PkK, 0)
}

/// Every root shape the design lists for an any-of entry.
fn rooted(th: Node, root: u8) -> Node {
    match root {
        0 => wrap(Tag::Wsh, th),
        1 => wrap(Tag::Sh, wrap(Tag::Wsh, th)),
        2 => tr_node(true, 0, Some(th)),
        3 => wrap(Tag::Wsh, node2(Tag::OrD, th, pk())),
        4 => wrap(Tag::Wsh, node2(Tag::OrD, pk(), th)),
        5 => wrap(Tag::Wsh, node2(Tag::OrI, th, pk())),
        6 => wrap(Tag::Wsh, node2(Tag::OrI, pk(), th)),
        7 => wrap(Tag::Wsh, node2(Tag::OrB, pk(), wrap(Tag::Alt, th))),
        _ => wrap(Tag::Wsh, node3(Tag::AndOr, pk(), pk(), th)),
    }
}

/// Distinct placeholders in pre-order, then real keys for every `@i`.
/// `None` for a tree with no key at all (an md1 card holds at least one
/// placeholder); a locks-only `thresh` is still drawn under the roots that
/// add a `pk` partner.
fn keyed(mut tree: Node) -> Option<md_codec::encode::Descriptor> {
    let mut next = 0u8;
    assign_sequential_indices(&mut tree, &mut next);
    (next > 0).then(|| descriptor_with_pubkeys(tree))
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 512, ..ProptestConfig::default() })]

    #[test]
    fn every_any_of_tree_passes_to_miniscript(
        (th, k, n) in allowlisted_thresh(),
        root in 0u8..9,
    ) {
        let d = keyed(rooted(th, root));
        prop_assume!(d.is_some());
        let d = d.unwrap();
        let shape = policy_shape(&d);
        prop_assert!(shape.complete, "root {}: {:?}", root, d.tree);
        let any: Vec<_> = shape.branches.iter().filter_map(|b| b.any_of).collect();
        if k < n {
            prop_assert_eq!(any, vec![AnyOf { k, n }]);
        } else {
            prop_assert!(any.is_empty());
        }
        let c = canon(&d);
        prop_assert!(
            to_miniscript_descriptor(&c, 0).is_ok(),
            "to_miniscript refused {:?}: {:?}",
            c.tree,
            to_miniscript_descriptor(&c, 0).err()
        );
    }
}

/// An ARBITRARY child: a chain of 0..=3 wrappers drawn from every wrapper
/// md's wire can carry (`a: s: c: v: n: j: d: l: u:`), over a key, a
/// timelock with any operand (including 0 and >= 2^31), or `0`/`1`. Most
/// draws are untypable; the property is that the walk accepts none of those.
fn arbitrary_child() -> impl Strategy<Value = Node> {
    let leaf = prop_oneof![
        Just(keyarg(Tag::PkK, 0)),
        Just(keyarg(Tag::PkH, 0)),
        prop_oneof![Just(0u32), 1u32..=70000, Just(1 << 31), any::<u32>()].prop_map(|v| Node {
            tag: Tag::Older,
            body: Body::Timelock(v),
        }),
        prop_oneof![Just(0u32), 1u32..(1 << 31), any::<u32>()].prop_map(|v| Node {
            tag: Tag::After,
            body: Body::Timelock(v),
        }),
        Just(f()),
        Just(Node {
            tag: Tag::True,
            body: Body::Empty,
        }),
    ];
    (leaf, proptest::collection::vec(0u8..9, 0..=3))
        .prop_map(|(n, chain)| chain.into_iter().fold(n, one_more_wrapper))
}

/// One wrapper of any kind around `n`.
fn one_more_wrapper(n: Node, w: u8) -> Node {
    match w {
        0 => wrap(Tag::Alt, n),
        1 => wrap(Tag::Swap, n),
        2 => wrap(Tag::Check, n),
        3 => wrap(Tag::Verify, n),
        4 => wrap(Tag::ZeroNotEqual, n),
        5 => wrap(Tag::NonZero, n),
        6 => wrap(Tag::DupIf, n),
        7 => node2(Tag::OrI, f(), n),
        _ => node2(Tag::OrI, n, f()),
    }
}

/// A child NEAR the allowlist: a listed spelling (in either position), the
/// same with one more wrapper of any kind, or an arbitrary chain. Uniform
/// arbitrary chains almost never give a `thresh` the walk accepts, so a
/// wrong list entry would go unseen.
fn near_child() -> impl Strategy<Value = Node> {
    let listed = || prop_oneof![first_child(), later_child()];
    prop_oneof![
        3 => listed(),
        2 => (listed(), 0u8..9).prop_map(|(n, w)| one_more_wrapper(n, w)),
        1 => arbitrary_child(),
    ]
}

/// A `thresh` over 1..=4 near-allowlist children, `k` mostly in range, at
/// any root of [`rooted`]: the trees the existing wire generator almost
/// never produces with a complete shape.
fn arbitrary_thresh_tree() -> impl Strategy<Value = md_codec::encode::Descriptor> {
    (proptest::collection::vec(near_child(), 1..=4), 0u8..9)
        .prop_flat_map(|(children, root)| {
            let n = children.len() as u8;
            (
                prop_oneof![4 => 1..=n, 1 => 0u8..=5],
                Just(children),
                Just(root),
            )
        })
        .prop_filter_map("has a key", |(k, children, root)| {
            keyed(rooted(common::thresh_node(k, children), root))
        })
}

/// Every `thresh` node in `n`.
fn threshes<'a>(n: &'a Node, out: &mut Vec<&'a Node>) {
    if n.tag == Tag::Thresh {
        out.push(n);
    }
    match &n.body {
        Body::Children(c) | Body::Variable { children: c, .. } => {
            c.iter().for_each(|x| threshes(x, out))
        }
        Body::Tr { tree: Some(t), .. } => threshes(t, out),
        _ => {}
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 16384, ..ProptestConfig::default() })]

    /// Over arbitrary decoded trees: in a COMPLETE shape every `thresh` was
    /// accepted by the walk (an any-of root or a `k == n` conjunction), so
    /// each one, alone under `wsh` and as a NUMS taproot leaf, must
    /// type-check. Stated over the node, not the whole tree, because the walk
    /// does not vouch for the rest of the tree.
    #[test]
    fn an_accepted_thresh_node_type_checks_on_its_own(
        d in prop_oneof![wire_descriptor_strategy(), arbitrary_thresh_tree().boxed()]
    ) {
        if policy_shape(&d).complete {
            let mut found = Vec::new();
            threshes(&d.tree, &mut found);
            for th in found {
                for tree in [wrap(Tag::Wsh, th.clone()), tr_node(true, 0, Some(th.clone()))] {
                    let Some(d) = keyed(tree) else { continue };
                    let c = canon(&d);
                    prop_assert!(
                        to_miniscript_descriptor(&c, 0).is_ok(),
                        "accepted thresh refused: {:?}: {:?}",
                        th,
                        to_miniscript_descriptor(&c, 0).err()
                    );
                }
            }
        }
    }
}
