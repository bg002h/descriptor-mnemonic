//! Task 1: port of the branch-decomposition walk (`md/policy_shape.go`),
//! pinning the slot-set extension and the honesty contract's refusal.
mod common;
use common::{keyarg, multikeys, node2, timelock, wrap};
use md_codec::policy_shape::{KeyPathKind, policy_shape};
use md_codec::tag::Tag;

/// `wsh(or_d(multi(2,@0,@1,@2), and_v(v:pkh(@3), older(26280))))`
fn kofn_recovery() -> md_codec::encode::Descriptor {
    let primary = multikeys(Tag::Multi, 2, vec![0, 1, 2]);
    let recovery = node2(
        Tag::AndV,
        wrap(Tag::Verify, keyarg(Tag::Pkh, 3)),
        timelock(Tag::Older, 26280),
    );
    let tree = wrap(Tag::Wsh, node2(Tag::OrD, primary, recovery));
    common::descriptor_of(tree, 4)
}

#[test]
fn branch_retains_its_slot_set_not_just_a_count() {
    let shape = policy_shape(&kofn_recovery());
    assert!(
        shape.complete,
        "the walk must classify every node of a shipped preset"
    );
    assert_eq!(shape.key_path, KeyPathKind::NotTaproot);
    assert_eq!(shape.branches.len(), 2, "or_d yields two spend paths");

    // THE EXTENSION: which slots, not how many.
    assert_eq!(shape.branches[0].slots, vec![0, 1, 2]);
    assert_eq!(shape.branches[1].slots, vec![3]);
    // The count the Go original kept is still derivable, and must agree.
    assert_eq!(shape.branches[0].slots.len(), 3);
}

/// If the walk meets something it cannot classify it must report
/// `complete = false`, so the caller shows the honest-minimal screen instead
/// of a partial (and therefore misleading) decomposition.
///
/// DEVIATION FROM THE BRIEF'S DRAFT: the brief proposed a bare `thresh` over
/// keys at the wsh root. Traced against the Go original (`md/policy_shape.go`
/// `collect`'s `tagThresh` arm, `splitBranches`'s `default` arm calling
/// `branchOf`), that shape does NOT refuse -- `thresh(2,pk_k(@0),pk_k(@1),
/// pk_k(@2))` is exactly the "combinatorial-but-still-one-branch" case the
/// Go doc comment on `splitBranches` describes, and it decodes to one
/// Complete=true branch with Keys=3, K=0/N=0. The brief's own step 6 text
/// anticipated this ("If the Go original DOES classify this shape, pick
/// whatever node it refuses").
///
/// The fork's OWN test, `TestPolicyShapeRefusesAnUnknownTag`
/// (md/policy_shape_test.go:175-189), pins the actual refusing shape: a
/// `tagTr` node nested inside a `wsh` script. `collect` has no case for
/// `Tag::Tr` (or `Tag::TapTree`) -- its doc comment says so explicitly:
/// "tagTr and tagTapTree cannot appear inside a branch ... Refuse rather
/// than guess." That is what this test pins, ported 1:1 from the Go test.
#[test]
fn an_unclassifiable_node_sets_complete_false_and_yields_no_branches() {
    let tr_leaf = common::tr_node(true, 0, None);
    let tree = wrap(Tag::Wsh, tr_leaf);
    let shape = policy_shape(&common::descriptor_of(tree, 1));
    assert!(!shape.complete);
    assert!(
        shape.branches.is_empty(),
        "an incomplete walk must not hand back a partial decomposition"
    );
}
