//! Stage 1b task 4: §2's derivation -- Liana's unspendable-xpub recipe
//! (`md_codec::nums::liana_unspendable_xpub`), checked against all eight
//! vendored evidence cases in `tests/fixtures/liana/cases.json`
//! (`scripts/vendor-liana-evidence.sh`).
//!
//! The recipe is Liana's own, `liana/src/descriptors/analysis.rs:398-430`:
//! sha256 over each leaf key's 33-byte compressed pubkey, concatenated in
//! descriptor left-to-right (wire) order -- NOT sorted, NOT deduplicated.
//! Sorting/deduplicating is the abandoned bitcoin/bips PR #1746 recipe, a
//! DIFFERENT wallet -- `sorting_or_deduplicating_produces_a_DIFFERENT_xpub`
//! below pins that they diverge.

use bitcoin::Network;
use md_codec::nums::liana_unspendable_xpub;

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/common/vendored.rs"
));
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/common/liana.rs"
));

#[test]
fn the_recipe_reproduces_every_golden_xpub() {
    for c in all_cases() {
        // all EIGHT, not just the four accepted
        let leaves: Vec<[u8; 33]> = c.leaf_pubkeys();
        assert_eq!(
            liana_unspendable_xpub(&leaves, Network::Bitcoin).to_string(),
            c.expected_xpub,
            "{}",
            c.name
        );
    }
}

#[test]
fn sorting_or_deduplicating_produces_a_different_xpub() {
    // Liana's recipe is NOT sorted and NOT deduplicated. The abandoned BIPs
    // PR #1746 recipe is, and it is a different wallet.
    let c = case("preset-kofn-recovery-tr");
    let mut l = c.leaf_pubkeys();
    let straight = liana_unspendable_xpub(&l, Network::Bitcoin).to_string();
    l.sort();
    l.dedup();
    assert_ne!(
        straight,
        liana_unspendable_xpub(&l, Network::Bitcoin).to_string()
    );
}

#[test]
fn a_synthetic_duplicate_leaf_changes_the_xpub_when_removed() {
    // `sorting_or_deduplicating_produces_a_different_xpub` above pins that
    // Liana's recipe is not sorted -- but it does NOT pin the "not
    // deduplicated" half: its `l.sort(); l.dedup();` call removes zero
    // elements, because `preset-kofn-recovery-tr`'s four leaf pubkeys are
    // already distinct. Review round 1 confirmed this by inserting a bare
    // `.dedup()` call into `liana_unspendable_xpub` itself and finding all
    // four original tests stayed green.
    //
    // NO VENDORED case can pin the dedup half, and this is not a fixture
    // gap to fill -- it is structural. md1 refuses a key slot from
    // appearing twice in ANY taptree it can express, for two independent
    // reasons: same-path reuse is forbidden by BIP 388's disjointness
    // rule, and disjoint-path reuse is forbidden by F-417's one-path-per-
    // key-slot rule (see `md encode` refusing both shapes in the task 4
    // brief). So no `cases.json` entry -- vendored today or added later --
    // will ever contain a duplicate leaf pubkey, and a test built only
    // from vendored cases structurally cannot exercise this half.
    //
    // `liana_unspendable_xpub`'s contract is about the SLICE it is
    // handed -- `&[[u8; 33]]` -- not about what md1 can express, so this
    // pins it with a slice built by hand: a real pubkey repeated
    // adjacently. Do not "fix" this by switching it to a vendored case;
    // that would silently un-pin this half again, exactly as happened
    // before this test existed.
    let a = case("preset-kofn-recovery-tr").leaf_pubkeys()[0];
    let with_duplicate = vec![a, a];
    let deduplicated = vec![a];
    assert_ne!(
        liana_unspendable_xpub(&with_duplicate, Network::Bitcoin).to_string(),
        liana_unspendable_xpub(&deduplicated, Network::Bitcoin).to_string(),
        "a duplicated leaf pubkey must change the chain code -- \
         sha256(a||a) must differ from sha256(a)"
    );
}

#[test]
fn the_chain_code_depends_on_the_keys_not_the_tree() {
    // kofn-recovery, tiered-recovery and decaying-multisig are three
    // different trees over the same four keys in the same order and share
    // one internal key. decaying-multisig is the only NESTED taptree in
    // the evidence, so it is what proves the traversal is depth-first
    // left-to-right.
    let names = [
        "preset-kofn-recovery-tr",
        "preset-tiered-recovery-tr",
        "preset-decaying-multisig-tr",
    ];
    let xs: Vec<String> = names
        .iter()
        .map(|n| case(n).expected_xpub.clone())
        .collect();
    assert_eq!(xs[0], xs[1]);
    assert_eq!(xs[1], xs[2]);
}

#[test]
fn a_tpub_wallet_derives_a_tpub_internal_key() {
    // SPEC §2 step 5's testnet branch is TRANSCRIBED, not measured -- all
    // eight evidence descriptors are mainnet. This is the vector that
    // measures it.
    assert!(
        liana_unspendable_xpub(
            &case("preset-kofn-recovery-tr").leaf_pubkeys(),
            Network::Testnet
        )
        .to_string()
        .starts_with("tpub")
    );
}
