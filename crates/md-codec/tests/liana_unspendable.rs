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
