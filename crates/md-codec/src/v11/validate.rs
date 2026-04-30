//! Decoder-side validation per spec §7.

use crate::v11::error::V11Error;
use crate::v11::tag::Tag;
use crate::v11::tree::{Body, Node};
use crate::v11::use_site_path::UseSitePath;

/// Validate the BIP 388 well-formedness of placeholder usage in the tree.
///
/// Enforces two invariants:
/// 1. Every `@i` for `0 ≤ i < n` appears at least once in the tree.
/// 2. The first occurrences (in pre-order traversal) of distinct placeholder
///    indices appear in canonical ascending order: `@0` before `@1` before `@2`, etc.
pub fn validate_placeholder_usage(root: &Node, n: u8) -> Result<(), V11Error> {
    let mut seen = vec![false; n as usize];
    let mut first_occurrences: Vec<u8> = Vec::new();
    walk_for_placeholders(root, &mut seen, &mut first_occurrences)?;
    // Each @i for 0 ≤ i < n must appear at least once.
    for (i, was_seen) in seen.iter().enumerate() {
        if !was_seen {
            return Err(V11Error::PlaceholderNotReferenced { idx: i as u8, n });
        }
    }
    // First occurrences must be in canonical ascending order.
    for (pos, idx) in first_occurrences.iter().enumerate() {
        if *idx as usize != pos {
            return Err(V11Error::PlaceholderFirstOccurrenceOutOfOrder {
                expected_first: pos as u8,
                got_first: *idx,
            });
        }
    }
    Ok(())
}

fn walk_for_placeholders(
    node: &Node,
    seen: &mut [bool],
    first_occurrences: &mut Vec<u8>,
) -> Result<(), V11Error> {
    match &node.body {
        Body::KeyArg { index } => {
            debug_assert!(
                (*index as usize) < seen.len(),
                "structural decode should have caught index >= n"
            );
            if (*index as usize) < seen.len() && !seen[*index as usize] {
                seen[*index as usize] = true;
                first_occurrences.push(*index);
            }
        }
        Body::Children(children) => {
            for c in children {
                walk_for_placeholders(c, seen, first_occurrences)?;
            }
        }
        Body::Variable { children, .. } => {
            for c in children {
                walk_for_placeholders(c, seen, first_occurrences)?;
            }
        }
        Body::Tr { key_index, tree } => {
            debug_assert!(
                (*key_index as usize) < seen.len(),
                "structural decode should have caught index >= n"
            );
            if (*key_index as usize) < seen.len() && !seen[*key_index as usize] {
                seen[*key_index as usize] = true;
                first_occurrences.push(*key_index);
            }
            if let Some(t) = tree {
                walk_for_placeholders(t, seen, first_occurrences)?;
            }
        }
        Body::Hash256Body(_) | Body::Hash160Body(_) | Body::Timelock(_) | Body::Empty => {}
    }
    Ok(())
}

/// Validate that all multipaths in shared default + overrides share the same alt-count.
///
/// Per spec §7, when multiple `UseSitePath` entries (the shared default plus any
/// per-`@N` overrides) carry a multipath group, all groups MUST have the same
/// number of alternatives.
pub fn validate_multipath_consistency(
    shared: &UseSitePath,
    overrides: &[(u8, UseSitePath)],
) -> Result<(), V11Error> {
    let mut seen_alt_count: Option<usize> = None;
    let candidates = std::iter::once(shared).chain(overrides.iter().map(|(_, p)| p));
    for path in candidates {
        if let Some(alts) = &path.multipath {
            match seen_alt_count {
                None => seen_alt_count = Some(alts.len()),
                Some(prev) if prev == alts.len() => {}
                Some(prev) => {
                    return Err(V11Error::MultipathAltCountMismatch {
                        expected: prev,
                        got: alts.len(),
                    });
                }
            }
        }
    }
    Ok(())
}

/// Validate that all leaves in a tap-script-tree are permitted-leaf tags per §6.3.1.
pub fn validate_tap_script_tree(node: &Node) -> Result<(), V11Error> {
    walk_tap_tree_leaves(node)
}

fn walk_tap_tree_leaves(node: &Node) -> Result<(), V11Error> {
    if matches!(node.tag, Tag::TapTree) {
        if let Body::Children(children) = &node.body {
            for c in children {
                walk_tap_tree_leaves(c)?;
            }
        }
        Ok(())
    } else {
        // This is a leaf — validate per §6.3.1.
        if is_forbidden_leaf_tag(node.tag) {
            return Err(V11Error::ForbiddenTapTreeLeaf {
                tag: node.tag.codes().0,
            });
        }
        Ok(())
    }
}

fn is_forbidden_leaf_tag(tag: Tag) -> bool {
    matches!(
        tag,
        Tag::Wpkh | Tag::Tr | Tag::Wsh | Tag::Sh | Tag::Pkh | Tag::Multi | Tag::SortedMulti
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v11::tag::Tag;
    use crate::v11::tree::{Body, Node};

    #[test]
    fn placeholder_usage_ok_for_2_of_3() {
        let root = Node {
            tag: Tag::SortedMulti,
            body: Body::Variable {
                k: 2,
                children: (0..3)
                    .map(|i| Node {
                        tag: Tag::PkK,
                        body: Body::KeyArg { index: i },
                    })
                    .collect(),
            },
        };
        validate_placeholder_usage(&root, 3).unwrap();
    }

    #[test]
    fn placeholder_usage_rejects_unreferenced() {
        let root = Node {
            tag: Tag::SortedMulti,
            body: Body::Variable {
                k: 1,
                children: vec![
                    Node { tag: Tag::PkK, body: Body::KeyArg { index: 0 } },
                    Node { tag: Tag::PkK, body: Body::KeyArg { index: 1 } },
                ],
            },
        };
        assert!(matches!(
            validate_placeholder_usage(&root, 3),
            Err(V11Error::PlaceholderNotReferenced { idx: 2, n: 3 })
        ));
    }

    #[test]
    fn placeholder_usage_rejects_out_of_order_first_occurrences() {
        let root = Node {
            tag: Tag::SortedMulti,
            body: Body::Variable {
                k: 1,
                children: vec![
                    Node { tag: Tag::PkK, body: Body::KeyArg { index: 1 } },
                    Node { tag: Tag::PkK, body: Body::KeyArg { index: 0 } },
                ],
            },
        };
        assert!(matches!(
            validate_placeholder_usage(&root, 2),
            Err(V11Error::PlaceholderFirstOccurrenceOutOfOrder { .. })
        ));
    }

    #[test]
    fn multipath_consistency_ok_when_all_match() {
        let shared = UseSitePath::standard_multipath();
        let overrides = vec![(1u8, UseSitePath::standard_multipath())];
        validate_multipath_consistency(&shared, &overrides).unwrap();
    }

    #[test]
    fn multipath_consistency_rejects_mismatched_alt_counts() {
        use crate::v11::use_site_path::Alternative;
        let shared = UseSitePath::standard_multipath();
        let overrides = vec![(
            1u8,
            UseSitePath {
                multipath: Some(vec![
                    Alternative { hardened: false, value: 0 },
                    Alternative { hardened: false, value: 1 },
                    Alternative { hardened: false, value: 2 },
                ]),
                wildcard_hardened: false,
            },
        )];
        assert!(matches!(
            validate_multipath_consistency(&shared, &overrides),
            Err(V11Error::MultipathAltCountMismatch { expected: 2, got: 3 })
        ));
    }

    #[test]
    fn tap_tree_leaf_rejects_wsh() {
        let leaf = Node { tag: Tag::Wsh, body: Body::Children(vec![]) };
        assert!(matches!(
            validate_tap_script_tree(&leaf),
            Err(V11Error::ForbiddenTapTreeLeaf { .. })
        ));
    }

    #[test]
    fn tap_tree_leaf_accepts_pk_k() {
        let leaf = Node {
            tag: Tag::PkK,
            body: Body::KeyArg { index: 0 },
        };
        validate_tap_script_tree(&leaf).unwrap();
    }
}
