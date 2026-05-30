//! Shared generators + helpers for the md-codec test-hardening suite.
//! Consumed by proptest_roundtrip.rs and bch_adversarial.rs via `mod common;`.
#![allow(dead_code, unused_imports)]

use md_codec::canonicalize::canonicalize_placeholder_indices;
use md_codec::encode::Descriptor;
use md_codec::origin_path::{OriginPath, PathComponent, PathDecl, PathDeclPaths};
use md_codec::tag::Tag;
use md_codec::tlv::TlvSection;
use md_codec::tree::{Body, Node};
use md_codec::use_site_path::UseSitePath;
use proptest::prelude::*;

fn divergent_path(n: u8, depth: u8) -> PathDecl {
    let paths = (0..n)
        .map(|c| OriginPath {
            components: (0..depth)
                .map(|i| PathComponent {
                    hardened: true,
                    value: (c as u32) * 100 + (i as u32) + 1,
                })
                .collect(),
        })
        .collect();
    PathDecl {
        n,
        paths: PathDeclPaths::Divergent(paths),
    }
}

fn wrap(tag: Tag, inner: Node) -> Node {
    Node {
        tag,
        body: Body::Children(vec![inner]),
    }
}
fn keyarg(tag: Tag, index: u8) -> Node {
    Node {
        tag,
        body: Body::KeyArg { index },
    }
}
fn multikeys(tag: Tag, k: u8, indices: Vec<u8>) -> Node {
    Node {
        tag,
        body: Body::MultiKeys { k, indices },
    }
}

/// n biased to the kiw-width boundaries (exercises kiw 0..5).
fn n_strategy() -> impl Strategy<Value = u8> {
    prop_oneof![
        Just(1u8),
        Just(2),
        Just(3),
        Just(4),
        Just(5),
        Just(8),
        Just(9),
        Just(15),
        Just(16),
        Just(17),
        Just(31),
        Just(32),
        2u8..=32,
    ]
}

/// Bounded-recursion tr() taptree: internal TapTree{Children(2)}; leaves from the
/// permitted allow-list. Leaves reference indices in 1..=max (keypath is @0);
/// descriptor_from_tree renumbers to contiguous 0..n.
fn taptree_strategy(max_key_index: u8) -> impl Strategy<Value = Node> {
    let leaf = prop_oneof![
        (1u8..=max_key_index).prop_map(|i| keyarg(Tag::PkK, i)),
        (1u8..=max_key_index).prop_map(|i| keyarg(Tag::PkH, i)),
        (1u8..=max_key_index).prop_map(|i| multikeys(Tag::MultiA, 1, vec![i])),
        (1u32..=65535).prop_map(|t| Node {
            tag: Tag::Older,
            body: Body::Timelock(t)
        }),
    ];
    leaf.prop_recursive(3, 8, 2, |inner| {
        (inner.clone(), inner).prop_map(|(l, r)| Node {
            tag: Tag::TapTree,
            body: Body::Children(vec![l, r]),
        })
    })
}

/// Distinct placeholder indices referenced by a tree (KeyArg + MultiKeys +
/// non-NUMS Tr.key_index), so n can be derived.
fn referenced_indices(node: &Node, out: &mut std::collections::BTreeSet<u8>) {
    match &node.body {
        Body::KeyArg { index } => {
            out.insert(*index);
        }
        Body::MultiKeys { indices, .. } => {
            out.extend(indices.iter().copied());
        }
        Body::Tr {
            is_nums,
            key_index,
            tree,
        } => {
            if !is_nums {
                out.insert(*key_index);
            }
            if let Some(t) = tree {
                referenced_indices(t, out);
            }
        }
        Body::Children(cs) => {
            for c in cs {
                referenced_indices(c, out);
            }
        }
        Body::Variable { children, .. } => {
            for c in children {
                referenced_indices(c, out);
            }
        }
        _ => {}
    }
}

/// Rewrite every placeholder index through `perm` (old->new). NUMS Tr.key_index
/// is left untouched (no wire repr), matching referenced_indices.
fn renumber_tree(node: &mut Node, perm: &std::collections::BTreeMap<u8, u8>) {
    match &mut node.body {
        Body::KeyArg { index } => {
            *index = perm[&*index];
        }
        Body::MultiKeys { indices, .. } => {
            for i in indices.iter_mut() {
                *i = perm[&*i];
            }
        }
        Body::Tr {
            is_nums,
            key_index,
            tree,
        } => {
            if !*is_nums {
                *key_index = perm[&*key_index];
            }
            if let Some(t) = tree {
                renumber_tree(t, perm);
            }
        }
        Body::Children(cs) => {
            for c in cs.iter_mut() {
                renumber_tree(c, perm);
            }
        }
        Body::Variable { children, .. } => {
            for c in children.iter_mut() {
                renumber_tree(c, perm);
            }
        }
        _ => {}
    }
}

/// Build a Descriptor: collect referenced indices, RENUMBER the tree to contiguous
/// 0..n, then derive n + path-decl. Explicit-origin shapes get a Divergent path.
fn descriptor_from_tree(mut tree: Node, explicit_origin: bool) -> Descriptor {
    let mut set = std::collections::BTreeSet::new();
    referenced_indices(&tree, &mut set);
    let perm: std::collections::BTreeMap<u8, u8> = set
        .iter()
        .enumerate()
        .map(|(rank, &old)| (old, rank as u8))
        .collect();
    renumber_tree(&mut tree, &perm);
    let n = set.len() as u8;
    let path_decl = if explicit_origin {
        divergent_path(n, 3)
    } else {
        PathDecl {
            n,
            paths: PathDeclPaths::Shared(OriginPath {
                components: vec![PathComponent {
                    hardened: true,
                    value: 84,
                }],
            }),
        }
    };
    Descriptor {
        n,
        path_decl,
        use_site_path: UseSitePath::standard_multipath(),
        tree,
        tlv: TlvSection::new_empty(),
    }
}

pub fn descriptor_strategy() -> BoxedStrategy<Descriptor> {
    let single_sig = prop_oneof![
        Just(keyarg(Tag::Wpkh, 0)),
        Just(keyarg(Tag::Pkh, 0)),
        Just(Node {
            tag: Tag::Tr,
            body: Body::Tr {
                is_nums: false,
                key_index: 0,
                tree: None
            }
        }),
    ]
    .prop_map(|t| descriptor_from_tree(t, false));

    let sh_wpkh =
        Just(wrap(Tag::Sh, keyarg(Tag::Wpkh, 0))).prop_map(|t| descriptor_from_tree(t, false));

    let multisig = (
        n_strategy(),
        1u8..=32u8,
        prop::sample::select(vec![Tag::Multi, Tag::SortedMulti]),
    )
        .prop_filter("k<=n", |(n, k, _)| k <= n)
        .prop_map(|(n, k, mtag)| {
            let inner = multikeys(mtag, k, (0..n).collect());
            descriptor_from_tree(wrap(Tag::Wsh, inner), true)
        });

    let sh_wsh = (n_strategy(), 1u8..=32u8)
        .prop_filter("k<=n", |(n, k)| k <= n)
        .prop_map(|(n, k)| {
            let inner = wrap(Tag::Wsh, multikeys(Tag::SortedMulti, k, (0..n).collect()));
            descriptor_from_tree(wrap(Tag::Sh, inner), true)
        });

    let sh_sortedmulti = (n_strategy(), 1u8..=32u8)
        .prop_filter("k<=n", |(n, k)| k <= n)
        .prop_map(|(n, k)| {
            let inner = multikeys(Tag::SortedMulti, k, (0..n).collect());
            descriptor_from_tree(wrap(Tag::Sh, inner), true)
        });

    let tr_multi_a = (2u8..=16u8, 1u8..=16u8)
        .prop_filter("k<=n-1", |(n, k)| *k <= n - 1)
        .prop_map(|(n, k)| {
            let leaf = multikeys(Tag::MultiA, k, (1..n).collect());
            let tree = Node {
                tag: Tag::Tr,
                body: Body::Tr {
                    is_nums: false,
                    key_index: 0,
                    tree: Some(Box::new(leaf)),
                },
            };
            descriptor_from_tree(tree, true)
        });

    let tr_taptree = (2u8..=8u8).prop_flat_map(|max| {
        taptree_strategy(max).prop_map(move |tt| {
            let tree = Node {
                tag: Tag::Tr,
                body: Body::Tr {
                    is_nums: false,
                    key_index: 0,
                    tree: Some(Box::new(tt)),
                },
            };
            descriptor_from_tree(tree, true)
        })
    });

    prop_oneof![
        single_sig,
        sh_wpkh,
        multisig,
        sh_wsh,
        sh_sortedmulti,
        tr_multi_a,
        tr_taptree
    ]
    .boxed()
}

/// canonicalize a descriptor (the fixpoint helper).
pub fn canon(d: &Descriptor) -> Descriptor {
    let mut c = d.clone();
    canonicalize_placeholder_indices(&mut c).expect("strategy descriptors are canonicalizable");
    c
}

/// flip one codex32 symbol at data-part position `pos` (post-"md1") of a chunk.
pub fn corrupt_chunk_at(chunk: &str, pos: usize, xor_mask: u8) -> String {
    const A: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";
    let mut chars: Vec<char> = chunk.chars().collect();
    let idx = 3 + pos;
    assert!(
        idx < chars.len(),
        "corrupt position {pos} past data-part (chunk len {})",
        chars.len()
    );
    let sym = A
        .iter()
        .position(|&b| b == (chars[idx] as u8).to_ascii_lowercase())
        .unwrap() as u8;
    chars[idx] = A[((sym ^ (xor_mask & 0x1F)) & 0x1F) as usize] as char;
    chars.into_iter().collect()
}
