//! Unit tests for md-signer-compat.

use std::sync::Arc;

use bitcoin::hashes::Hash as _;
use md_codec::test_helpers::{dummy_key_a, dummy_key_b};
use miniscript::{DescriptorPublicKey, Miniscript, Tap, Terminal, Threshold};

use crate::{COLDCARD_TAP, LEDGER_TAP, validate, validate_tap_tree};

#[test]
fn coldcard_admits_documented_pk_shape() {
    let key = dummy_key_a();
    let pk_k = Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::PkK(key)).unwrap();
    // pk(K) desugars to c:pk_k(K); the AST has Check around PkK.
    let c_pk =
        Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::Check(Arc::new(pk_k))).unwrap();
    validate(&COLDCARD_TAP, &c_pk, Some(0)).expect("c:pk_k must admit under COLDCARD_TAP");
}

#[test]
fn coldcard_rejects_thresh_with_operator_name() {
    let key = dummy_key_a();
    let pk_k = Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::PkK(key)).unwrap();
    let c_pk = Arc::new(
        Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::Check(Arc::new(pk_k))).unwrap(),
    );
    let thresh_term = Terminal::Thresh(Threshold::new(1, vec![c_pk]).unwrap());
    let thresh_ms = Miniscript::<DescriptorPublicKey, Tap>::from_ast(thresh_term).unwrap();
    let err = validate(&COLDCARD_TAP, &thresh_ms, Some(2)).unwrap_err();
    match err {
        md_codec::Error::SubsetViolation {
            operator,
            leaf_index,
            ..
        } => {
            assert_eq!(operator, "thresh");
            assert_eq!(leaf_index, Some(2));
        }
        other => panic!("expected SubsetViolation, got {other:?}"),
    }
}

#[test]
fn ledger_admits_relative_timelock_multisig_shape() {
    use miniscript::RelLockTime;
    let key_a = dummy_key_a();
    let key_b = dummy_key_b();
    let multi_a = Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::MultiA(
        Threshold::new(2, vec![key_a, key_b]).unwrap(),
    ))
    .unwrap();
    let v_multi_a =
        Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::Verify(Arc::new(multi_a)))
            .unwrap();
    let older = Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::Older(
        RelLockTime::from_consensus(144).unwrap(),
    ))
    .unwrap();
    let and_v = Terminal::AndV(Arc::new(v_multi_a), Arc::new(older));
    let and_v_ms = Miniscript::<DescriptorPublicKey, Tap>::from_ast(and_v).unwrap();
    validate(&LEDGER_TAP, &and_v_ms, Some(0))
        .expect("and_v(v:multi_a, older(n)) must admit under LEDGER_TAP");
}

#[test]
fn ledger_rejects_sha256() {
    use bitcoin::hashes::sha256;
    let h = sha256::Hash::from_byte_array([0xAA; 32]);
    let sha = Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::Sha256(h)).unwrap();
    let err = validate(&LEDGER_TAP, &sha, Some(1)).unwrap_err();
    match err {
        md_codec::Error::SubsetViolation {
            operator,
            leaf_index,
            ..
        } => {
            assert_eq!(operator, "sha256");
            assert_eq!(leaf_index, Some(1));
        }
        other => panic!("expected SubsetViolation, got {other:?}"),
    }
}

/// Per spec §4.6 acceptance: every entry in COLDCARD_TAP and LEDGER_TAP
/// allowlists must be a name md-codec's operator-naming hook can produce
/// (catches typos in the allowlist constants).
///
/// For each leaf-shaped operator, build a minimal Tap miniscript whose
/// root produces that name, then validate it under a single-entry
/// allowlist. Wrappers and compound operators (`c:`, `v:`, `or_d`,
/// `and_v`) cannot appear in isolation at the AST root — they're
/// validated indirectly through the COLDCARD/LEDGER admit/reject tests
/// above plus the hand-AST coverage tests in md-codec.
#[test]
fn allowlist_entries_are_recognized_by_naming_hook() {
    use md_codec::bytecode::encode::validate_tap_leaf_subset_with_allowlist;

    fn check(allowed_operators: &[&str]) {
        for op in allowed_operators {
            let leaf = match *op {
                "pk_k" => Some(
                    Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::PkK(dummy_key_a()))
                        .unwrap(),
                ),
                "pk_h" => Some(
                    Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::PkH(dummy_key_a()))
                        .unwrap(),
                ),
                "multi_a" => Some(
                    Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::MultiA(
                        Threshold::new(1, vec![dummy_key_a()]).unwrap(),
                    ))
                    .unwrap(),
                ),
                "sortedmulti_a" => Some(
                    Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::SortedMultiA(
                        Threshold::new(1, vec![dummy_key_a()]).unwrap(),
                    ))
                    .unwrap(),
                ),
                "older" => Some(
                    Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::Older(
                        miniscript::RelLockTime::from_consensus(144).unwrap(),
                    ))
                    .unwrap(),
                ),
                "after" => Some(
                    Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::After(
                        miniscript::AbsLockTime::from_consensus(1000).unwrap(),
                    ))
                    .unwrap(),
                ),
                // Wrappers and compound operators — exercised indirectly.
                "c:" | "v:" | "or_d" | "and_v" => None,
                other => panic!(
                    "allowlist entry {other:?} not recognised by the typo guard \
                     (extend the match arm if you added a new operator)"
                ),
            };

            if let Some(ms) = leaf {
                let single = [*op];
                validate_tap_leaf_subset_with_allowlist(&ms, &single, Some(0)).unwrap_or_else(
                    |e| {
                        panic!(
                            "allowlist entry {op:?} should be admitted by single-entry \
                             allowlist; got {e:?}"
                        )
                    },
                );
            }
        }
    }
    check(COLDCARD_TAP.allowed_operators);
    check(LEDGER_TAP.allowed_operators);
}

/// Per Phase 1 reviewer IMP-1: multi-leaf DFS-pre-order leaf-index
/// attribution coverage. Build a 3-leaf TapTree where leaf 1 (left of
/// right subtree, DFS pre-order index 1) contains an out-of-subset
/// `sha256(...)`. `validate_tap_tree` walks every leaf with its derived
/// DFS-pre-order index; the rejection must surface `leaf_index = Some(1)`.
///
/// Closes FOLLOWUPS `v07-tap-leaf-iterator-with-index-coverage`.
#[test]
fn validate_tap_tree_attributes_violation_to_dfs_pre_order_index() {
    use bitcoin::hashes::sha256;
    use miniscript::descriptor::TapTree;

    // 3-leaf tree with shape `{leaf_0, {leaf_1_sha256_violator, leaf_2}}`:
    //   - leaf_0 (pre-order index 0): pk_k(a) — admitted under COLDCARD_TAP
    //     after wrapping with c: (B-type at leaf root).
    //   - leaf_1 (pre-order index 1): bare sha256 — out of subset.
    //   - leaf_2 (pre-order index 2): pk_k(b) wrapped in c:.
    let pk_a =
        Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::PkK(dummy_key_a())).unwrap();
    let leaf_0 = Arc::new(
        Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::Check(Arc::new(pk_a))).unwrap(),
    );
    let leaf_1 = Arc::new(
        Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::Sha256(
            sha256::Hash::from_byte_array([0xCC; 32]),
        ))
        .unwrap(),
    );
    let pk_b =
        Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::PkK(dummy_key_b())).unwrap();
    let leaf_2 = Arc::new(
        Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::Check(Arc::new(pk_b))).unwrap(),
    );

    let right_subtree = TapTree::combine(TapTree::leaf(leaf_1), TapTree::leaf(leaf_2)).unwrap();
    let tree = TapTree::combine(TapTree::leaf(leaf_0), right_subtree).unwrap();

    let err = validate_tap_tree(&COLDCARD_TAP, &tree).unwrap_err();
    match err {
        md_codec::Error::SubsetViolation {
            operator,
            leaf_index,
            ..
        } => {
            assert!(
                operator.contains("sha256"),
                "expected sha256 in operator, got {operator:?}"
            );
            assert_eq!(
                leaf_index,
                Some(1),
                "DFS pre-order index of the offending leaf must be 1"
            );
        }
        other => panic!("expected SubsetViolation, got {other:?}"),
    }
}
