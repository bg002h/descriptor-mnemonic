//! md thresh / seedhammer I-1: an honest "any k of n" summary for `thresh`
//! (scratchpad DESIGN_thresh_summary.md, sections 1, 2 and 6).
//!
//! A `thresh` with `k < n` at a path root, every child spelled exactly as one
//! allowlisted key or timelock, is ONE any-of entry. A `k == n` `thresh` is a
//! conjunction only when every child is on the same list. Everything else
//! makes the shape incomplete, and `skeleton()` refuses it.
//!
//! The minted vectors come from `fixtures/policy_shape/minted.tsv`: name,
//! template, the concrete descriptor `md` minted, and the SkeletonKey
//! `md shape-key --descriptor` printed for it BEFORE this change. A kept key
//! must equal that literal, which is how "no identity changed" is proved.

mod common;

use common::{keyarg, node2, thresh_node, timelock, wrap};
use md_codec::descriptor_route::descriptor_from_text;
use md_codec::encode::Descriptor;
use md_codec::policy_shape::{AnyOf, Branch, Lock, LockKind, PolicyShape, policy_shape};
use md_codec::skeleton::{SkeletonError, skeleton, skeleton_key};
use md_codec::tag::Tag;
use md_codec::tree::{Body, InternalKey, Node};

// ─── the minted vectors ─────────────────────────────────────────────────────

const MINTED: &str = include_str!("fixtures/policy_shape/minted.tsv");

/// `(template, descriptor, pre-change SkeletonKey or the refusal it printed)`.
fn minted(name: &str) -> (&'static str, &'static str, &'static str) {
    for line in MINTED.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        if f[0] == name {
            return (f[1], f[2], f[3]);
        }
    }
    panic!("no minted vector {name}");
}

/// The pre-change SkeletonKey. The fixture writes U+001F as the visible `␟`.
fn pre_key(name: &str) -> String {
    minted(name).2.replace('␟', "\u{1f}")
}

fn minted_descriptor(name: &str) -> Descriptor {
    descriptor_from_text(minted(name).1).unwrap_or_else(|e| panic!("{name}: {e}"))
}

const OB: LockKind = LockKind::OlderBlocks;

fn lk(kind: LockKind, value: u32) -> Lock {
    Lock { kind, value }
}

/// The invariants every any-of entry holds (design section 2).
fn assert_any_of_invariants(name: &str, b: &Branch) {
    let a = b.any_of.expect("an any-of entry");
    assert!(1 <= a.k && a.k < a.n, "{name}: 1 <= k < n, got {a:?}");
    assert_eq!(
        usize::from(a.n),
        b.slots.len() + b.locks.len(),
        "{name}: n counts every condition"
    );
    assert!(b.hashlocks.is_empty(), "{name}: no hashlock");
    assert_eq!((b.k, b.n, b.sorted), (0, 0, false), "{name}: no multi k/n");
}

/// One in-scope minted vector: which entry is the any-of entry, its `(k, n)`,
/// its slots, its locks and the fewest keys that can spend; plus the slots of
/// every other entry (each a conjunction).
struct InScope {
    name: &'static str,
    at: usize,
    any_of: (u8, u8),
    slots: &'static [u8],
    locks: &'static [(LockKind, u32)],
    min_keys: u8,
    others: &'static [&'static [u8]],
}

const IN_SCOPE: &[InScope] = &[
    InScope {
        name: "thresh_bip388_3of4_lock",
        at: 0,
        any_of: (3, 4),
        slots: &[0, 1, 2],
        locks: &[(OB, 12960)],
        min_keys: 2,
        others: &[],
    },
    InScope {
        name: "thresh_compiler_3of4_lock",
        at: 0,
        any_of: (3, 4),
        slots: &[0, 1, 2],
        locks: &[(OB, 12960)],
        min_keys: 2,
        others: &[],
    },
    InScope {
        name: "thresh_decay_2of3_lock",
        at: 0,
        any_of: (2, 3),
        slots: &[0, 1],
        locks: &[(OB, 52560)],
        min_keys: 1,
        others: &[],
    },
    InScope {
        name: "thresh_after_2of3",
        at: 0,
        any_of: (2, 3),
        slots: &[0, 1],
        locks: &[(LockKind::AfterHeight, 800000)],
        min_keys: 1,
        others: &[],
    },
    InScope {
        name: "thresh_pkh_children",
        at: 0,
        any_of: (2, 3),
        slots: &[0, 1],
        locks: &[(OB, 100)],
        min_keys: 1,
        others: &[],
    },
    InScope {
        name: "thresh_lock_first",
        at: 0,
        any_of: (2, 3),
        slots: &[0, 1],
        locks: &[(OB, 100)],
        min_keys: 1,
        others: &[],
    },
    InScope {
        name: "thresh_alt_children",
        at: 0,
        any_of: (2, 3),
        slots: &[0, 1],
        locks: &[(OB, 100)],
        min_keys: 1,
        others: &[],
    },
    InScope {
        name: "thresh_un_lock",
        at: 0,
        any_of: (2, 3),
        slots: &[0, 1],
        locks: &[(OB, 100)],
        min_keys: 1,
        others: &[],
    },
    InScope {
        name: "thresh_1of3_keys",
        at: 0,
        any_of: (1, 3),
        slots: &[0, 1, 2],
        locks: &[],
        min_keys: 1,
        others: &[],
    },
    InScope {
        name: "thresh_2of3_keys",
        at: 0,
        any_of: (2, 3),
        slots: &[0, 1, 2],
        locks: &[],
        min_keys: 2,
        others: &[],
    },
    InScope {
        name: "thresh_sh_wsh",
        at: 0,
        any_of: (2, 3),
        slots: &[0, 1, 2],
        locks: &[],
        min_keys: 2,
        others: &[],
    },
    InScope {
        name: "thresh_under_or_d",
        at: 1,
        any_of: (2, 3),
        slots: &[1, 2, 3],
        locks: &[],
        min_keys: 2,
        others: &[&[0]],
    },
    InScope {
        name: "thresh_under_or_b",
        at: 1,
        any_of: (2, 3),
        slots: &[1, 2, 3],
        locks: &[],
        min_keys: 2,
        others: &[&[0]],
    },
    InScope {
        name: "thresh_under_or_i",
        at: 1,
        any_of: (2, 3),
        slots: &[1, 2, 3],
        locks: &[],
        min_keys: 2,
        others: &[&[0]],
    },
    InScope {
        name: "thresh_andor_z",
        at: 1,
        any_of: (2, 3),
        slots: &[2, 3, 4],
        locks: &[],
        min_keys: 2,
        others: &[&[0, 1]],
    },
    InScope {
        name: "thresh_tr_leaf",
        at: 1,
        any_of: (2, 3),
        slots: &[1, 2, 3],
        locks: &[],
        min_keys: 2,
        others: &[&[0]],
    },
    InScope {
        name: "thresh_tr_two_leaves",
        at: 2,
        any_of: (2, 3),
        slots: &[2, 3, 4],
        locks: &[],
        min_keys: 2,
        others: &[&[0], &[1]],
    },
    InScope {
        name: "thresh_tr_nums_leaf",
        at: 0,
        any_of: (2, 3),
        slots: &[0, 1],
        locks: &[(OB, 52560)],
        min_keys: 1,
        others: &[],
    },
    InScope {
        name: "thresh_keyless_1of2",
        at: 0,
        any_of: (1, 2),
        slots: &[0],
        locks: &[(OB, 52560)],
        min_keys: 0,
        others: &[],
    },
];

#[test]
fn in_scope_minted_thresh_is_one_any_of_entry_and_keeps_its_key() {
    for v in IN_SCOPE {
        let d = minted_descriptor(v.name);
        let shape = policy_shape(&d);
        assert!(shape.complete, "{}: {shape:?}", v.name);
        assert_eq!(shape.branches.len(), v.others.len() + 1, "{}", v.name);
        let b = &shape.branches[v.at];
        assert_eq!(
            b.any_of,
            Some(AnyOf {
                k: v.any_of.0,
                n: v.any_of.1
            }),
            "{}",
            v.name
        );
        assert_eq!(b.slots, v.slots, "{}", v.name);
        let locks: Vec<Lock> = v.locks.iter().map(|&(k, x)| lk(k, x)).collect();
        assert_eq!(b.locks, locks, "{}", v.name);
        assert_eq!(b.any_of_min_keys(), Some(v.min_keys), "{}", v.name);
        assert_any_of_invariants(v.name, b);
        let others: Vec<&Branch> = shape
            .branches
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != v.at)
            .map(|(_, b)| b)
            .collect();
        for (o, want) in others.iter().zip(v.others) {
            assert_eq!(o.any_of, None, "{}", v.name);
            assert_eq!(o.any_of_min_keys(), None, "{}", v.name);
            assert_eq!(&o.slots, want, "{}", v.name);
        }
        let key = skeleton_key(&skeleton(&d).unwrap_or_else(|e| panic!("{}: {e}", v.name)));
        assert_eq!(key.as_str(), pre_key(v.name), "{}: the key moved", v.name);
    }
}

/// `k == n` with every child on the list: still a conjunction, key unchanged.
#[test]
fn k_equal_n_on_the_list_stays_a_conjunction_and_keeps_its_key() {
    for (name, slots, locks) in [
        ("thresh_kn_2of2", vec![0u8, 1], vec![]),
        ("thresh_kn_3of3_lock", vec![0, 1], vec![lk(OB, 52560)]),
    ] {
        let d = minted_descriptor(name);
        let shape = policy_shape(&d);
        assert!(shape.complete, "{name}");
        assert_eq!(shape.branches.len(), 1, "{name}");
        let b = &shape.branches[0];
        assert_eq!(b.any_of, None, "{name}");
        assert_eq!(b.slots, slots, "{name}");
        assert_eq!(b.locks, locks, "{name}");
        let key = skeleton_key(&skeleton(&d).unwrap());
        assert_eq!(key.as_str(), pre_key(name), "{name}: the key moved");
    }
}

/// Mintable shapes outside the list: incomplete, and no key. Each but the
/// `v:thresh` under `or_c` had a key before this change.
#[test]
fn minted_thresh_outside_the_list_refuses_the_summary_and_the_key() {
    for name in [
        "thresh_neg_under_and",
        "thresh_neg_multi_child",
        "thresh_neg_nested",
        "thresh_neg_compound_child",
        "thresh_neg_hash_child_jtv",
        "thresh_neg_v_thresh_under_or_c",
        "thresh_neg_sn_pk",
        "thresh_neg_sndv_older",
        "thresh_neg_kn_multi_children",
    ] {
        let d = minted_descriptor(name);
        assert!(!policy_shape(&d).complete, "{name}");
        assert!(
            matches!(skeleton(&d), Err(SkeletonError::IncompleteShape)),
            "{name}"
        );
    }
}

// ─── hand-built trees ───────────────────────────────────────────────────────

fn f() -> Node {
    Node {
        tag: Tag::False,
        body: Body::Empty,
    }
}
fn pk(i: u8) -> Node {
    keyarg(Tag::PkK, i)
}
fn pkh(i: u8) -> Node {
    keyarg(Tag::PkH, i)
}
fn c(x: Node) -> Node {
    wrap(Tag::Check, x)
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
fn n(x: Node) -> Node {
    wrap(Tag::ZeroNotEqual, x)
}
fn j(x: Node) -> Node {
    wrap(Tag::NonZero, x)
}
fn d(x: Node) -> Node {
    wrap(Tag::DupIf, x)
}
fn l(x: Node) -> Node {
    node2(Tag::OrI, f(), x)
}
fn u(x: Node) -> Node {
    node2(Tag::OrI, x, f())
}
fn older(x: u32) -> Node {
    timelock(Tag::Older, x)
}
fn after(x: u32) -> Node {
    timelock(Tag::After, x)
}
fn wsh(x: Node) -> Node {
    wrap(Tag::Wsh, x)
}
fn tr_leaf(x: Node) -> Node {
    Node {
        tag: Tag::Tr,
        body: Body::Tr {
            internal_key: InternalKey::NumsPoint,
            tree: Some(Box::new(x)),
        },
    }
}
fn th(k: u8, children: Vec<Node>) -> Node {
    thresh_node(k, children)
}
/// `thresh(2, pk(@0), s:pk(@1), <third>)`.
fn two_of(third: Node) -> Node {
    th(2, vec![pk(0), s(pk(1)), third])
}
fn sha() -> Node {
    Node {
        tag: Tag::Sha256,
        body: Body::Hash256Body([7u8; 32]),
    }
}

fn shape_of(tree: Node) -> PolicyShape {
    policy_shape(&common::descriptor_of(tree, 5))
}

#[test]
fn hand_built_in_scope_trees() {
    // Timelocks only: any 1 of 2, no key needed.
    let shape = shape_of(wsh(th(1, vec![l(n(older(10))), s(u(n(older(20))))])));
    assert!(shape.complete);
    let b = &shape.branches[0];
    assert_eq!(b.any_of, Some(AnyOf { k: 1, n: 2 }));
    assert!(b.slots.is_empty());
    assert_eq!(b.locks, vec![lk(OB, 10), lk(OB, 20)]);
    assert_eq!(b.any_of_min_keys(), Some(0));
    assert_any_of_invariants("locks_only", b);

    // A height and a time that cannot both be met: on the list, min 0.
    let shape = shape_of(wsh(th(
        2,
        vec![pk(0), s(l(n(after(800000)))), s(l(n(after(1700000000))))],
    )));
    assert!(shape.complete);
    let b = &shape.branches[0];
    assert_eq!(b.any_of, Some(AnyOf { k: 2, n: 3 }));
    assert_eq!(b.slots, vec![0]);
    assert_eq!(
        b.locks,
        vec![
            lk(LockKind::AfterHeight, 800000),
            lk(LockKind::AfterTime, 1700000000)
        ]
    );
    assert_eq!(b.any_of_min_keys(), Some(0));

    // Check-wrapped keys are the same fragments as bare ones.
    let shape = shape_of(wsh(th(2, vec![c(pk(0)), s(c(pk(1))), a(c(pkh(2)))])));
    assert!(shape.complete);
    assert_eq!(shape.branches[0].any_of, Some(AnyOf { k: 2, n: 3 }));
    assert_eq!(shape.branches[0].slots, vec![0, 1, 2]);
    assert_eq!(shape.branches[0].any_of_min_keys(), Some(2));

    // R1 N-2: `older(0x400000)` is in range (units flag, 0 units) but delays
    // nothing; `md encode` mints it today. The summary still counts it as a
    // timelock; min keys is unaffected (it already assumes the lock is met).
    let shape = shape_of(wsh(th(
        2,
        vec![pk(0), s(pk(1)), s(l(n(older(0x0040_0000))))],
    )));
    assert!(shape.complete);
    let b = &shape.branches[0];
    assert_eq!(b.any_of, Some(AnyOf { k: 2, n: 3 }));
    assert_eq!(b.locks, vec![lk(LockKind::OlderUnits, 0)]);
    assert_eq!(b.any_of_min_keys(), Some(1));

    // k == n under `and_v`: still a conjunction.
    let shape = shape_of(wsh(node2(
        Tag::AndV,
        v(pk(3)),
        th(2, vec![pk(0), s(pk(1))]),
    )));
    assert!(shape.complete);
    assert_eq!(shape.branches.len(), 1);
    assert_eq!(shape.branches[0].any_of, None);
    assert_eq!(shape.branches[0].slots, vec![0, 1, 3]);
}

/// One per excluded spelling or position (design section 6, review C-1).
#[test]
fn hand_built_trees_outside_the_list_are_incomplete() {
    let three = || th(2, vec![pk(0), s(pk(1)), s(pk(2))]);
    let cases: Vec<(&str, Node)> = vec![
        ("av_pk", wsh(two_of(a(v(pk(2)))))),
        ("sv_pk", wsh(two_of(s(v(pk(2)))))),
        (
            "sl_older",
            wsh(th(3, vec![pk(0), s(pk(1)), s(pk(2)), s(l(older(2)))])),
        ),
        ("su_older", wsh(two_of(s(u(older(2)))))),
        ("a_older", wsh(two_of(a(older(2))))),
        ("an_older", wsh(two_of(a(n(older(2)))))),
        ("slj_older", wsh(two_of(s(l(j(older(144))))))),
        ("sjl_older", wsh(two_of(s(j(l(older(144))))))),
        ("aj_pk", wsh(two_of(a(j(pk(2)))))),
        ("an_pk", wsh(two_of(a(n(pk(2)))))),
        ("al_pk", wsh(two_of(a(l(pk(2)))))),
        ("s_pkh", wsh(two_of(s(pkh(2))))),
        ("s_first", wsh(th(2, vec![s(pk(0)), s(pk(1)), s(pk(2))]))),
        ("bare_second", wsh(th(2, vec![pk(0), pk(1), s(pk(2))]))),
        ("adv_older_wsh", wsh(two_of(a(d(v(older(2))))))),
        ("adv_older_tr", tr_leaf(two_of(a(d(v(older(2))))))),
        ("ss_pk", wsh(two_of(s(s(pk(2)))))),
        ("sc_c_pk", wsh(two_of(s(c(c(pk(2))))))),
        ("older_zero", wsh(two_of(s(l(n(older(0))))))),
        ("older_bit31", wsh(two_of(s(l(n(older(1 << 31))))))),
        ("ori_two_zero", wsh(two_of(s(node2(Tag::OrI, f(), f()))))),
        ("hash_child_bare", wsh(two_of(a(sha())))),
        ("root_alt", wsh(a(three()))),
        ("root_verify", wsh(v(three()))),
        ("or_b_first_alt", wsh(node2(Tag::OrB, a(three()), a(pk(3))))),
        ("or_d_alt", wsh(node2(Tag::OrD, pk(3), a(three())))),
        ("kn_av_pk", wsh(th(3, vec![pk(0), s(pk(1)), a(v(pk(2)))]))),
        (
            "kn_sl_older",
            wsh(th(3, vec![pk(0), s(pk(1)), s(l(older(2)))])),
        ),
        (
            "kn_under_and_av_pk",
            wsh(node2(
                Tag::AndV,
                v(pk(3)),
                th(3, vec![pk(0), s(pk(1)), a(v(pk(2)))]),
            )),
        ),
        ("dup_slot", wsh(th(2, vec![pk(0), s(pk(0)), s(pk(1))]))),
        ("k_zero", wsh(th(0, vec![pk(0), s(pk(1))]))),
        ("k_above_n", wsh(th(3, vec![pk(0), s(pk(1))]))),
    ];
    for (name, tree) in cases {
        let shape = shape_of(tree);
        assert!(!shape.complete, "thresh_neg_{name}: {shape:?}");
    }
}

/// The skeleton's refusal names every cause (review M-2).
#[test]
fn the_incomplete_shape_refusal_names_every_cause() {
    assert_eq!(
        SkeletonError::IncompleteShape.to_string(),
        "md cannot break this policy into spend paths: it has a threshold or an or that md \
         does not summarise, or a fragment md does not know; no key can be computed from a \
         partial decomposition"
    );
}

// ─── nothing on the wire moves ──────────────────────────────────────────────

/// The parked A1 patch's keys, so the pins below are the values it captured
/// on the code before any thresh change.
const XPUBS: [&str; 3] = [
    "xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf",
    "xpub6BosfCnifzxcFwrSzQiqu2DBVTshkCXacvNsWGYJVVhhawA7d4R5WSWGFNbi8Aw6ZRc1brxMyWMzG3DSSSSoekkudhUd9yLb6qx39T9nMdj",
    "xpub6EddmgK6uMbst7251zES359MJzfz3o2wTJWeefTzSiPJf1FUg6easA2Uk3jUbztBffWS4Dg2oWjhomvU2jJKr4EZukDfxVxb4VJva3jXGAK",
];

fn keyed(tree: Node, n: u8) -> Descriptor {
    use md_codec::origin_path::{OriginPath, PathComponent, PathDecl, PathDeclPaths};
    use std::str::FromStr as _;
    let mut d = common::descriptor_of(tree, n);
    d.path_decl = PathDecl {
        n,
        paths: PathDeclPaths::Shared(OriginPath {
            components: [48, 0, 0, 2]
                .into_iter()
                .map(|value| PathComponent {
                    hardened: true,
                    value,
                })
                .collect(),
        }),
    };
    d.tlv.pubkeys = Some(
        (0..n)
            .map(|i| {
                let x = bitcoin::bip32::Xpub::from_str(XPUBS[usize::from(i)]).unwrap();
                let mut p = [0u8; 65];
                p[..32].copy_from_slice(x.chain_code.as_bytes());
                p[32..].copy_from_slice(&x.public_key.serialize());
                (i, p)
            })
            .collect(),
    );
    d
}

fn hex(b: &[u8]) -> String {
    use std::fmt::Write as _;
    b.iter().fold(String::new(), |mut out, x| {
        let _ = write!(out, "{x:02x}");
        out
    })
}

/// Card bytes, decode, ids and the first receive address of a decaying
/// 2-of-2 and a 1-of-3, pinned from the code before this change.
#[test]
fn wire_decode_ids_and_addresses_are_unchanged() {
    let cases: [(&str, Node, u8, &str); 2] = [
        (
            "2-of-key-key-lock",
            wsh(th(2, vec![pk(0), s(pk(1)), s(l(n(older(52560))))])),
            2,
            PIN_2OF3_LOCK,
        ),
        (
            "1-of-3-keys",
            wsh(th(1, vec![pk(0), s(pk(1)), s(pk(2))])),
            3,
            PIN_1OF3,
        ),
    ];
    for (name, tree, n, want) in cases {
        let d = keyed(tree, n);
        let (bytes, bits) = md_codec::encode::encode_payload(&d).unwrap();
        let back = md_codec::decode::decode_payload(&bytes, bits).unwrap();
        assert_eq!(back, d, "{name}: decode round trip");
        let enc = md_codec::identity::compute_md1_encoding_id(&d).unwrap();
        let pol = md_codec::identity::compute_wallet_policy_id(&d).unwrap();
        let tpl = md_codec::identity::compute_wallet_descriptor_template_id(&d).unwrap();
        let addr = d
            .derive_address(0, 0, bitcoin::Network::Bitcoin)
            .unwrap()
            .assume_checked();
        let got = format!(
            "{} {} {} {} {addr}",
            hex(&bytes),
            hex(enc.as_bytes()),
            hex(pol.as_bytes()),
            hex(tpl.as_bytes())
        );
        assert_eq!(got, want, "{name}");
    }
}

const PIN_2OF3_LOCK: &str = "2052d84212a0018268222871539989270000335405704977418f942c150e1dfd72815721a1e8509fd436bc1a422ecefd047f86fc5aaec6043477ebf7ee6fa1e6d327fa8db892261277d6a65aca9fc1bfb317a4eb0bb93e533da4bc190a2680111d31fadfdc905f2a7f6ce77c6f109919116f253d4344521903774c910fcf07fa96886ea794f0d5caed9afe30b44b83f7e213bb92930e7df4bd 2f8d53cf5268732749f5f48c65c180c6 bcf9e1b1c6123cf54a76dd9bbfd4bc26 da0a944d51de9a3e004daf20324751b4 bc1qumcehq8cht43nw5y8u0yeucu5gqhtjpy9djwsvt7p6yncjlhk08qpsur3k";
const PIN_1OF3: &str = "2092d84212a0018268022838a4e2a15e1e2ee831f28582a1c3bfae502ae4343d0a13fa86d7834845d9dfa08ff0df8b55d8c0868efd7efdcdf43cda64ff51b71244c24efad4cb5953f837f662f49d617727ca53da4bc190a2680111d31fadfdc905f2a7f6ce77c6f109919116f253d4344521903774c910fcf07fa96886ea794f0d5caed9afe30b44b83f7e213bb92930e7df4bdbc7e50ac3a21ca51d28e06b5d5908c5248cb81dd700fba84adb8669258d2ad0fc0ee79d56ff3ea5e2e1cfd5d46933d981f8a4a997b144cf443f899a1f49ae6d4ca8 a87359b43ca99a44eda7daa182cacb0f dd8cad8eec6baa48c84ef4f20348df31 09280eb823398f0c3bea20a11b0928e0 bc1q7h6ccjkzwlzfqlnfn47r4rp07tsugr8c7z64p9e3lydzyx2sq0fs3y9gjn";
