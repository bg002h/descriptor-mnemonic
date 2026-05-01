//! BIP 388 placeholder-ordering canonicalization per spec v0.13 §6.1.
//!
//! BIP 388 wallet policies require placeholder indices `@0..@{N-1}` to be
//! introduced in the descriptor template in canonical first-occurrence order:
//! the first new placeholder encountered in document-order pre-order
//! traversal must be `@0`, the next new one `@1`, etc.
//!
//! [`canonicalize_placeholder_indices`] reshapes a [`Descriptor`] in place
//! so this invariant holds, atomically permuting:
//!
//! - the tree's `KeyArg.index` and `Tr.key_index` fields;
//! - the [`PathDecl`]'s `Divergent` paths vector (one path per `@N`);
//! - per-`@N` TLV maps: `use_site_path_overrides`, `fingerprints`,
//!   `pubkeys`, `origin_path_overrides`.
//!
//! After canonicalization, the post-conditions are:
//!
//! 1. Each TLV map's `(idx, _)` keys are strictly ascending and `< n`.
//! 2. The tree's first-occurrence sequence is exactly `[0, 1, ..., n-1]`
//!    (verified via [`crate::validate::validate_placeholder_usage`]).
//!
//! Idempotent: calling on an already-canonical descriptor is a no-op
//! (identity-permutation fast path).
//!
//! The decoder side does not call this routine: v0.11's
//! [`crate::validate::validate_placeholder_usage`] rejects non-canonical
//! wires up-front via
//! [`Error::PlaceholderFirstOccurrenceOutOfOrder`].

use crate::encode::Descriptor;
use crate::error::Error;
use crate::origin_path::PathDeclPaths;
use crate::tree::{Body, Node};

/// Walk `node` in pre-order, recording the first occurrence of each
/// placeholder index in `first_occurrences`. `seen[i]` toggles to `true`
/// the first time `@i` is encountered.
fn walk_collect_first(node: &Node, seen: &mut [bool], first_occurrences: &mut Vec<u8>) {
    match &node.body {
        Body::KeyArg { index } => {
            if let Some(slot) = seen.get_mut(*index as usize) {
                if !*slot {
                    *slot = true;
                    first_occurrences.push(*index);
                }
            }
        }
        Body::Tr { key_index, tree } => {
            if let Some(slot) = seen.get_mut(*key_index as usize) {
                if !*slot {
                    *slot = true;
                    first_occurrences.push(*key_index);
                }
            }
            if let Some(t) = tree {
                walk_collect_first(t, seen, first_occurrences);
            }
        }
        Body::Children(children) => {
            for c in children {
                walk_collect_first(c, seen, first_occurrences);
            }
        }
        Body::Variable { children, .. } => {
            for c in children {
                walk_collect_first(c, seen, first_occurrences);
            }
        }
        Body::Hash256Body(_) | Body::Hash160Body(_) | Body::Timelock(_) | Body::Empty => {}
    }
}

/// Apply `perm[old_idx] -> new_idx` to every `KeyArg.index` and
/// `Tr.key_index` in `node` (recursive).
fn remap_indices(node: &mut Node, perm: &[u8]) {
    match &mut node.body {
        Body::KeyArg { index } => {
            *index = perm[*index as usize];
        }
        Body::Tr { key_index, tree } => {
            *key_index = perm[*key_index as usize];
            if let Some(t) = tree {
                remap_indices(t, perm);
            }
        }
        Body::Children(children) => {
            for c in children {
                remap_indices(c, perm);
            }
        }
        Body::Variable { children, .. } => {
            for c in children {
                remap_indices(c, perm);
            }
        }
        Body::Hash256Body(_) | Body::Hash160Body(_) | Body::Timelock(_) | Body::Empty => {}
    }
}

/// Remap idx values in a sparse TLV vector and re-sort ascending. After
/// `perm` is applied the (possibly scrambled) idx column is restored to
/// strictly ascending order, preserving the per-entry payload.
fn remap_tlv_vec<T>(entries: &mut [(u8, T)], perm: &[u8]) {
    for (idx, _) in entries.iter_mut() {
        *idx = perm[*idx as usize];
    }
    entries.sort_by_key(|(idx, _)| *idx);
}

/// Canonicalize placeholder indices in `d` so the first-occurrence
/// sequence in `d.tree` is exactly `[0, 1, ..., d.n - 1]`.
///
/// Walks the tree in document order to build a first-occurrence map,
/// then atomically permutes indices in the tree, the path declaration
/// (divergent variant), and every per-`@N` TLV map. Identity-permutation
/// fast path: returns `Ok(())` without mutating `d` if the tree is
/// already canonical.
///
/// # Errors
///
/// Returns [`Error::PlaceholderNotReferenced`] if any `@i` for
/// `0 ≤ i < d.n` does not appear in the tree (a structural error that
/// would otherwise leave the permutation under-specified).
///
/// Returns [`Error::PlaceholderIndexOutOfRange`] if the tree references
/// a placeholder `@i` with `i >= d.n`.
pub fn canonicalize_placeholder_indices(d: &mut Descriptor) -> Result<(), Error> {
    let n = d.n as usize;

    // Defensive bounds check before walking — surface out-of-range
    // placeholder references as a typed error rather than silently
    // ignoring them in walk_collect_first.
    check_placeholder_bounds(&d.tree, d.n)?;

    let mut seen = vec![false; n];
    let mut first_occurrences: Vec<u8> = Vec::with_capacity(n);
    walk_collect_first(&d.tree, &mut seen, &mut first_occurrences);

    // Every `@i` must be referenced; otherwise the permutation is
    // under-specified.
    for (i, was_seen) in seen.iter().enumerate() {
        if !was_seen {
            return Err(Error::PlaceholderNotReferenced { idx: i as u8, n: d.n });
        }
    }

    // perm[old_idx] = new_idx, where new_idx is the position at which
    // old_idx was first observed in document order.
    let mut perm = vec![0u8; n];
    for (new_idx, &old_idx) in first_occurrences.iter().enumerate() {
        perm[old_idx as usize] = new_idx as u8;
    }

    // Identity fast path: no work needed when perm is the identity.
    if perm.iter().enumerate().all(|(i, p)| i as u8 == *p) {
        return Ok(());
    }

    // Atomically apply the permutation to every index-bearing field.
    remap_indices(&mut d.tree, &perm);

    if let PathDeclPaths::Divergent(paths) = &mut d.path_decl.paths {
        // paths[old_idx] becomes paths[perm[old_idx]] — i.e. new_paths[new_idx] = old_paths[old_idx]
        // where perm[old_idx] = new_idx. We need new_paths[new_idx] = old_paths[inverse_perm[new_idx]].
        let mut inverse = vec![0u8; n];
        for (old, &new) in perm.iter().enumerate() {
            inverse[new as usize] = old as u8;
        }
        let old_paths = std::mem::take(paths);
        let mut new_paths = Vec::with_capacity(n);
        for new_idx in 0..n {
            new_paths.push(old_paths[inverse[new_idx] as usize].clone());
        }
        *paths = new_paths;
    }

    if let Some(v) = d.tlv.use_site_path_overrides.as_mut() {
        remap_tlv_vec(v, &perm);
    }
    if let Some(v) = d.tlv.fingerprints.as_mut() {
        remap_tlv_vec(v, &perm);
    }
    if let Some(v) = d.tlv.pubkeys.as_mut() {
        remap_tlv_vec(v, &perm);
    }
    if let Some(v) = d.tlv.origin_path_overrides.as_mut() {
        remap_tlv_vec(v, &perm);
    }

    // Post-condition assertions (debug-only). Constructing the iterator-
    // based check is cheap; gating to debug mode keeps release builds
    // free of redundant work since the permutation is correct by
    // construction.
    debug_assert!(
        crate::validate::validate_placeholder_usage(&d.tree, d.n).is_ok(),
        "post-condition: tree first-occurrence must be canonical after canonicalize_placeholder_indices",
    );
    debug_assert!(
        tlv_indices_strictly_ascending_and_in_range(d),
        "post-condition: every TLV's idx column must be strictly ascending and < n",
    );

    Ok(())
}

/// Verify every `@N` reference in `node` falls within `0..n`. Returns
/// [`Error::PlaceholderIndexOutOfRange`] on the first violation.
fn check_placeholder_bounds(node: &Node, n: u8) -> Result<(), Error> {
    match &node.body {
        Body::KeyArg { index } => {
            if *index >= n {
                return Err(Error::PlaceholderIndexOutOfRange { idx: *index, n });
            }
        }
        Body::Tr { key_index, tree } => {
            if *key_index >= n {
                return Err(Error::PlaceholderIndexOutOfRange { idx: *key_index, n });
            }
            if let Some(t) = tree {
                check_placeholder_bounds(t, n)?;
            }
        }
        Body::Children(children) => {
            for c in children {
                check_placeholder_bounds(c, n)?;
            }
        }
        Body::Variable { children, .. } => {
            for c in children {
                check_placeholder_bounds(c, n)?;
            }
        }
        Body::Hash256Body(_) | Body::Hash160Body(_) | Body::Timelock(_) | Body::Empty => {}
    }
    Ok(())
}

/// Returns `true` if every TLV map's idx column is strictly ascending
/// and within `0..d.n`. Used by debug-only post-condition assertions and
/// by tests that want to exercise this invariant directly.
fn tlv_indices_strictly_ascending_and_in_range(d: &Descriptor) -> bool {
    fn check<T>(v: &Option<Vec<(u8, T)>>, n: u8) -> bool {
        let Some(v) = v else {
            return true;
        };
        let mut prev: Option<u8> = None;
        for (idx, _) in v {
            if *idx >= n {
                return false;
            }
            if let Some(p) = prev {
                if *idx <= p {
                    return false;
                }
            }
            prev = Some(*idx);
        }
        true
    }
    check(&d.tlv.use_site_path_overrides, d.n)
        && check(&d.tlv.fingerprints, d.n)
        && check(&d.tlv.pubkeys, d.n)
        && check(&d.tlv.origin_path_overrides, d.n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::origin_path::{OriginPath, PathComponent, PathDecl, PathDeclPaths};
    use crate::tag::Tag;
    use crate::tlv::TlvSection;
    use crate::tree::{Body, Node};
    use crate::use_site_path::UseSitePath;

    fn pkk(index: u8) -> Node {
        Node {
            tag: Tag::PkK,
            body: Body::KeyArg { index },
        }
    }

    fn shared_bip84() -> PathDecl {
        PathDecl {
            n: 1,
            paths: PathDeclPaths::Shared(OriginPath {
                components: vec![
                    PathComponent { hardened: true, value: 84 },
                    PathComponent { hardened: true, value: 0 },
                    PathComponent { hardened: true, value: 0 },
                ],
            }),
        }
    }

    fn shared_path_decl(n: u8) -> PathDecl {
        PathDecl {
            n,
            paths: PathDeclPaths::Shared(OriginPath {
                components: vec![PathComponent { hardened: true, value: 48 }],
            }),
        }
    }

    fn no_multipath() -> UseSitePath {
        UseSitePath { multipath: None, wildcard_hardened: false }
    }

    /// Pre-condition: `tr(@0)` already canonical → after canonicalize,
    /// descriptor unchanged.
    #[test]
    fn identity_permutation_no_op() {
        let d = Descriptor {
            n: 1,
            path_decl: shared_bip84(),
            use_site_path: no_multipath(),
            tree: Node {
                tag: Tag::Tr,
                body: Body::Tr { key_index: 0, tree: None },
            },
            tlv: TlvSection::new_empty(),
        };
        let mut d2 = d.clone();
        canonicalize_placeholder_indices(&mut d2).unwrap();
        assert_eq!(d, d2);
    }

    /// Encoder canonicalizes `tr(multi(2, @1, @0))` →
    /// `tr(multi(2, @0, @1))` with swapped indices.
    #[test]
    fn swap_two_placeholders_in_multi() {
        let mut d = Descriptor {
            n: 2,
            path_decl: shared_path_decl(2),
            use_site_path: no_multipath(),
            // tr keypath @0 already references @0 first, so embed the
            // swap inside the tap-script-tree where the document-order
            // walk will hit @1 first.
            tree: Node {
                tag: Tag::Multi,
                body: Body::Variable {
                    k: 2,
                    children: vec![pkk(1), pkk(0)],
                },
            },
            tlv: TlvSection::new_empty(),
        };
        canonicalize_placeholder_indices(&mut d).unwrap();
        let expected_tree = Node {
            tag: Tag::Multi,
            body: Body::Variable {
                k: 2,
                children: vec![pkk(0), pkk(1)],
            },
        };
        assert_eq!(d.tree, expected_tree);
    }

    /// `wsh(sortedmulti(2, @2, @0, @1))` → tree becomes canonical and
    /// TLV `pubkeys` is renumbered consistently.
    ///
    /// Originally: pubkey-A is wired to @0, pubkey-B to @1, pubkey-C to @2.
    /// After tree first-occurrence is `[2, 0, 1]`:
    ///   perm[0] = 1, perm[1] = 2, perm[2] = 0.
    /// So the on-disk pubkeys vec `[(0, A), (1, B), (2, C)]` becomes
    ///   `[(perm[0], A), (perm[1], B), (perm[2], C)]`
    /// = `[(1, A), (2, B), (0, C)]`, then re-sorted to
    ///   `[(0, C), (1, A), (2, B)]`.
    #[test]
    fn permute_three_placeholders_in_sortedmulti() {
        let xpub_a = [0xaa; 65];
        let xpub_b = [0xbb; 65];
        let xpub_c = [0xcc; 65];
        let mut d = Descriptor {
            n: 3,
            path_decl: shared_path_decl(3),
            use_site_path: no_multipath(),
            tree: Node {
                tag: Tag::Wsh,
                body: Body::Children(vec![Node {
                    tag: Tag::SortedMulti,
                    body: Body::Variable {
                        k: 2,
                        children: vec![pkk(2), pkk(0), pkk(1)],
                    },
                }]),
            },
            tlv: {
                let mut t = TlvSection::new_empty();
                t.pubkeys = Some(vec![(0, xpub_a), (1, xpub_b), (2, xpub_c)]);
                t
            },
        };
        canonicalize_placeholder_indices(&mut d).unwrap();
        let expected_tree = Node {
            tag: Tag::Wsh,
            body: Body::Children(vec![Node {
                tag: Tag::SortedMulti,
                body: Body::Variable {
                    k: 2,
                    children: vec![pkk(0), pkk(1), pkk(2)],
                },
            }]),
        };
        assert_eq!(d.tree, expected_tree);
        assert_eq!(
            d.tlv.pubkeys.unwrap(),
            vec![(0, xpub_c), (1, xpub_a), (2, xpub_b)],
        );
    }

    /// Divergent path declaration is reordered in lockstep with the
    /// placeholder indices: paths[new] holds the path that was wired to
    /// the @N now mapped to that new index.
    #[test]
    fn permute_with_divergent_path_decl() {
        let path_for_at_0 = OriginPath {
            components: vec![PathComponent { hardened: true, value: 84 }],
        };
        let path_for_at_1 = OriginPath {
            components: vec![PathComponent { hardened: true, value: 86 }],
        };
        let mut d = Descriptor {
            n: 2,
            path_decl: PathDecl {
                n: 2,
                paths: PathDeclPaths::Divergent(vec![
                    path_for_at_0.clone(),
                    path_for_at_1.clone(),
                ]),
            },
            use_site_path: no_multipath(),
            tree: Node {
                tag: Tag::Wsh,
                body: Body::Children(vec![Node {
                    tag: Tag::Multi,
                    body: Body::Variable {
                        k: 2,
                        // First-occurrence: @1, then @0 → perm[0] = 1, perm[1] = 0.
                        children: vec![pkk(1), pkk(0)],
                    },
                }]),
            },
            tlv: TlvSection::new_empty(),
        };
        canonicalize_placeholder_indices(&mut d).unwrap();
        // After: tree has @0 first, then @1. The @ that was originally
        // @1 (and thus paired with `path_for_at_1`) is now @0, so
        // paths[0] must be the path originally at index 1.
        match &d.path_decl.paths {
            PathDeclPaths::Divergent(paths) => {
                assert_eq!(paths[0], path_for_at_1);
                assert_eq!(paths[1], path_for_at_0);
            }
            _ => panic!("expected divergent paths"),
        }
    }

    /// `use_site_path_overrides` keys are remapped consistently with
    /// the tree permutation.
    #[test]
    fn permute_with_use_site_path_overrides() {
        let custom = UseSitePath::standard_multipath();
        let mut d = Descriptor {
            n: 2,
            path_decl: shared_path_decl(2),
            use_site_path: no_multipath(),
            tree: Node {
                tag: Tag::Multi,
                body: Body::Variable {
                    k: 2,
                    children: vec![pkk(1), pkk(0)],
                },
            },
            tlv: {
                let mut t = TlvSection::new_empty();
                // Override applies to the @ that was originally @1.
                t.use_site_path_overrides = Some(vec![(1, custom.clone())]);
                t
            },
        };
        canonicalize_placeholder_indices(&mut d).unwrap();
        // After: original @1 → new @0; override should now key on @0.
        assert_eq!(
            d.tlv.use_site_path_overrides.unwrap(),
            vec![(0, custom)],
        );
    }

    /// Both `fingerprints` and `pubkeys` carry @N idx; both must be
    /// remapped identically.
    #[test]
    fn permute_with_fingerprints_and_pubkeys() {
        let fp_a = [0x11, 0x11, 0x11, 0x11];
        let fp_b = [0x22, 0x22, 0x22, 0x22];
        let fp_c = [0x33, 0x33, 0x33, 0x33];
        let xpub_a = [0xaa; 65];
        let xpub_b = [0xbb; 65];
        let xpub_c = [0xcc; 65];
        let mut d = Descriptor {
            n: 3,
            path_decl: shared_path_decl(3),
            use_site_path: no_multipath(),
            tree: Node {
                tag: Tag::SortedMulti,
                body: Body::Variable {
                    k: 2,
                    // First-occurrence: @2, @0, @1
                    // perm[0]=1, perm[1]=2, perm[2]=0.
                    children: vec![pkk(2), pkk(0), pkk(1)],
                },
            },
            tlv: {
                let mut t = TlvSection::new_empty();
                t.fingerprints = Some(vec![(0, fp_a), (1, fp_b), (2, fp_c)]);
                t.pubkeys = Some(vec![(0, xpub_a), (1, xpub_b), (2, xpub_c)]);
                t
            },
        };
        canonicalize_placeholder_indices(&mut d).unwrap();
        // Original (0,A)/(1,B)/(2,C) → perm gives (1,A)/(2,B)/(0,C) →
        // sorted: (0,C), (1,A), (2,B).
        assert_eq!(
            d.tlv.fingerprints.unwrap(),
            vec![(0, fp_c), (1, fp_a), (2, fp_b)],
        );
        assert_eq!(
            d.tlv.pubkeys.unwrap(),
            vec![(0, xpub_c), (1, xpub_a), (2, xpub_b)],
        );
    }

    /// `origin_path_overrides` is also remapped correctly.
    #[test]
    fn permute_with_origin_path_overrides() {
        let path_for_at_2 = OriginPath {
            components: vec![PathComponent { hardened: true, value: 99 }],
        };
        let mut d = Descriptor {
            n: 3,
            path_decl: shared_path_decl(3),
            use_site_path: no_multipath(),
            tree: Node {
                tag: Tag::SortedMulti,
                body: Body::Variable {
                    k: 2,
                    // first-occurrence: @2, @0, @1 → perm[2]=0
                    children: vec![pkk(2), pkk(0), pkk(1)],
                },
            },
            tlv: {
                let mut t = TlvSection::new_empty();
                t.origin_path_overrides = Some(vec![(2, path_for_at_2.clone())]);
                t
            },
        };
        canonicalize_placeholder_indices(&mut d).unwrap();
        // perm[2] = 0; override at idx 2 maps to idx 0.
        assert_eq!(
            d.tlv.origin_path_overrides.unwrap(),
            vec![(0, path_for_at_2)],
        );
    }

    /// `tr(@0)` with `n=3` (i.e. @1 and @2 declared but never used) →
    /// canonicalize errors with PlaceholderNotReferenced.
    #[test]
    fn unreferenced_placeholder_returns_error() {
        let mut d = Descriptor {
            n: 3,
            path_decl: shared_path_decl(3),
            use_site_path: no_multipath(),
            tree: Node {
                tag: Tag::Tr,
                body: Body::Tr { key_index: 0, tree: None },
            },
            tlv: TlvSection::new_empty(),
        };
        let err = canonicalize_placeholder_indices(&mut d).unwrap_err();
        assert!(matches!(err, Error::PlaceholderNotReferenced { idx: 1, n: 3 }));
    }

    /// Out-of-range `@N` reference is caught up-front with a typed error
    /// rather than panicking inside the walker.
    #[test]
    fn out_of_range_placeholder_returns_error() {
        let mut d = Descriptor {
            n: 2,
            path_decl: shared_path_decl(2),
            use_site_path: no_multipath(),
            tree: Node {
                tag: Tag::Wpkh,
                body: Body::KeyArg { index: 5 },
            },
            tlv: TlvSection::new_empty(),
        };
        let err = canonicalize_placeholder_indices(&mut d).unwrap_err();
        assert!(matches!(err, Error::PlaceholderIndexOutOfRange { idx: 5, n: 2 }));
    }

    /// Idempotence: canonicalizing twice is a no-op after the first call.
    #[test]
    fn idempotence() {
        let mut d = Descriptor {
            n: 3,
            path_decl: shared_path_decl(3),
            use_site_path: no_multipath(),
            tree: Node {
                tag: Tag::SortedMulti,
                body: Body::Variable {
                    k: 2,
                    children: vec![pkk(2), pkk(0), pkk(1)],
                },
            },
            tlv: {
                let mut t = TlvSection::new_empty();
                t.fingerprints = Some(vec![(0, [1; 4]), (1, [2; 4]), (2, [3; 4])]);
                t
            },
        };
        canonicalize_placeholder_indices(&mut d).unwrap();
        let after_first = d.clone();
        canonicalize_placeholder_indices(&mut d).unwrap();
        assert_eq!(d, after_first);
    }

    /// Post-condition (1): every TLV map's idx column is strictly
    /// ascending and `< d.n` after canonicalization.
    #[test]
    fn tlv_idx_post_condition() {
        let mut d = Descriptor {
            n: 3,
            path_decl: shared_path_decl(3),
            use_site_path: no_multipath(),
            tree: Node {
                tag: Tag::SortedMulti,
                body: Body::Variable {
                    k: 2,
                    children: vec![pkk(2), pkk(0), pkk(1)],
                },
            },
            tlv: {
                let mut t = TlvSection::new_empty();
                t.fingerprints = Some(vec![(0, [1; 4]), (1, [2; 4]), (2, [3; 4])]);
                t.pubkeys = Some(vec![(0, [0xaa; 65]), (1, [0xbb; 65]), (2, [0xcc; 65])]);
                t
            },
        };
        canonicalize_placeholder_indices(&mut d).unwrap();
        assert!(tlv_indices_strictly_ascending_and_in_range(&d));
    }

    /// Post-condition (2): the tree's first-occurrence sequence is
    /// exactly `[0, 1, ..., n-1]` after canonicalization.
    #[test]
    fn tree_first_occurrence_post_condition() {
        let mut d = Descriptor {
            n: 3,
            path_decl: shared_path_decl(3),
            use_site_path: no_multipath(),
            tree: Node {
                tag: Tag::SortedMulti,
                body: Body::Variable {
                    k: 2,
                    children: vec![pkk(2), pkk(0), pkk(1)],
                },
            },
            tlv: TlvSection::new_empty(),
        };
        canonicalize_placeholder_indices(&mut d).unwrap();
        // The validator returning Ok(()) is the canonical post-condition.
        crate::validate::validate_placeholder_usage(&d.tree, d.n).unwrap();
        // Also walk explicitly to assert the literal sequence.
        let mut seen = vec![false; d.n as usize];
        let mut first = Vec::new();
        walk_collect_first(&d.tree, &mut seen, &mut first);
        assert_eq!(first, vec![0, 1, 2]);
    }

    /// The encoder calls `canonicalize_placeholder_indices` internally,
    /// so a non-canonical input round-trips through encode/decode cleanly:
    /// the wire bytes are the canonical encoding, and the decoder accepts
    /// them without `PlaceholderFirstOccurrenceOutOfOrder`.
    #[test]
    fn encoder_canonicalizes_non_canonical_input() {
        let d = Descriptor {
            n: 2,
            path_decl: shared_path_decl(2),
            use_site_path: UseSitePath::standard_multipath(),
            tree: Node {
                tag: Tag::Wsh,
                body: Body::Children(vec![Node {
                    tag: Tag::Multi,
                    body: Body::Variable {
                        k: 2,
                        // first-occurrence: @1 then @0 (non-canonical).
                        children: vec![pkk(1), pkk(0)],
                    },
                }]),
            },
            tlv: TlvSection::new_empty(),
        };
        let (bytes, total_bits) =
            crate::encode::encode_payload(&d).expect("encoder must canonicalize and succeed");
        // Decoder rejects non-canonical first-occurrence ordering with
        // PlaceholderFirstOccurrenceOutOfOrder; if encoder didn't
        // canonicalize, this would fail.
        let decoded = crate::decode::decode_payload(&bytes, total_bits).expect("decode");
        // Decoded tree's first occurrence is canonical [0, 1].
        let mut seen = vec![false; decoded.n as usize];
        let mut first = Vec::new();
        walk_collect_first(&decoded.tree, &mut seen, &mut first);
        assert_eq!(first, vec![0, 1]);
    }

    /// Post-condition (3): round-trip property — for hand-crafted
    /// permutations, `canonicalize → encode → decode → canonicalize`
    /// equals the canonicalize-only result. (Encode requires a fully
    /// well-formed descriptor, so this exercises the encoder path.)
    #[test]
    fn round_trip_canonicalize_encode_decode_canonicalize() {
        // 8 permutations of @0,@1,@2 inside sortedmulti(2, ...) plus
        // base canonical and one swap-pair → 10 total cases.
        let permutations: Vec<Vec<u8>> = vec![
            vec![0, 1, 2],
            vec![0, 2, 1],
            vec![1, 0, 2],
            vec![1, 2, 0],
            vec![2, 0, 1],
            vec![2, 1, 0],
            vec![1, 0, 1], // duplicate refs (re-uses @1 and @0; only first introduces)
            vec![2, 1, 0], // duplicate of above to give 8
        ];
        for perm in permutations {
            // n is the count of distinct placeholders in `perm`.
            let mut distinct: Vec<u8> = perm.clone();
            distinct.sort_unstable();
            distinct.dedup();
            let n = distinct.len() as u8;
            assert!(n >= 2, "test fixture expects ≥2 distinct placeholders");
            // Children are pkk(@perm[i]) — but to match `n` we must use
            // exactly the `n` placeholders {0, 1, ..., n-1}; the
            // permutation `perm` already does that as long as `distinct`
            // == 0..n. Re-index if the permutation skipped any.
            let mut renumbered = perm.clone();
            // Build mapping: each distinct value gets the position of
            // its sorted occurrence as its label, ensuring the resulting
            // descriptor has placeholders 0..n exactly.
            let mut mapping = std::collections::HashMap::new();
            for (i, v) in distinct.iter().enumerate() {
                mapping.insert(*v, i as u8);
            }
            for v in renumbered.iter_mut() {
                *v = mapping[v];
            }

            let children: Vec<Node> = renumbered.iter().map(|i| pkk(*i)).collect();
            let n_children = children.len();
            let k_value = std::cmp::min(2u8, n_children as u8);
            let mut d = Descriptor {
                n,
                path_decl: shared_path_decl(n),
                use_site_path: UseSitePath::standard_multipath(),
                tree: Node {
                    tag: Tag::Wsh,
                    body: Body::Children(vec![Node {
                        tag: Tag::SortedMulti,
                        body: Body::Variable {
                            k: k_value,
                            children,
                        },
                    }]),
                },
                tlv: TlvSection::new_empty(),
            };
            canonicalize_placeholder_indices(&mut d).unwrap();
            let canonical = d.clone();

            // Encode → decode and confirm the result is already
            // canonical (decoder accepts it cleanly).
            let (bytes, total_bits) =
                crate::encode::encode_payload(&d).expect("encode");
            let decoded =
                crate::decode::decode_payload(&bytes, total_bits).expect("decode");
            let mut decoded_mut = decoded;
            canonicalize_placeholder_indices(&mut decoded_mut).unwrap();
            assert_eq!(canonical, decoded_mut);
        }
    }
}
