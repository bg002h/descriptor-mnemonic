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
use md_codec::encode_payload;
use md_codec::nums::liana_unspendable_xpub;
use md_codec::tree::Node;
use md_codec::use_site_path::{Alternative, UseSitePath};
use md_codec::{Error, OriginPath, PathComponent, PathDecl, PathDeclPaths, Tag, TlvSection};

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

// Stage 1b task 7: SPEC §6's ENCODE-side refusals for a wire-kind-1 (Liana
// unspendable) internal key. Two of the four live here, because they need
// only `encode_payload` (public) and `kind1_from_vector`
// (`tests/common/liana.rs`, spliced in above) -- the other two need
// crate-private surface only reachable from a unit test inside `src/`
// (`crates/md-codec/src/encode.rs`'s `unspendable_shape_tests` module).

/// A kind-1 tr whose taptree contains a `sortedmulti_a` leaf -- the shape §6
/// row 1 refuses. VERIFIED SHAPE: `keyed_tr_sortedmulti_a` is NOT this, it is
/// `tr(@0/...,sortedmulti_a(...))` with a SPENDABLE internal key, so
/// `kind1_from_vector`'s kind-0 assert would panic on it. The NUMS-rooted
/// vector is `keyed_compose_tr_sole_sortedmulti_a`:
/// `tr({NUMS},sortedmulti_a(2,@0,@1,@2))`, use-site `<0;1>/*` (its own
/// `.descriptor.json` confirms both), so `kind1_from_vector`'s swap is legal
/// and this fixture trips ONLY row 1, never row 2.
fn tr_liana_with_sortedmulti_a_leaf() -> Descriptor {
    kind1_from_vector("keyed_compose_tr_sole_sortedmulti_a")
}

/// A kind-1 tr whose slots sit at a non-canonical use site. md1 carries ONE
/// path per slot (F-417), so this is a whole-descriptor change, not
/// per-leaf. `keyed_compose_tr_nums_three_leaves`'s own `.descriptor.json`
/// confirms its taptree has no `sortedmulti_a` leaf (its multi-family node is
/// `MultiA`, not `SortedMultiA`) and its own use-site is already the
/// canonical `<0;1>/*`, so overriding it below is what makes this fixture
/// trip ONLY row 2, never row 1.
///
/// `UseSitePath` has NO `parse` and NO `FromStr` -- its surface is
/// `standard_multipath()`, `write()`, `read()` (`use_site_path.rs:70-78`).
/// Build it by struct literal; the fields are public.
fn tr_liana_at_use_site(a: u32, b: u32) -> Descriptor {
    let mut d = kind1_from_vector("keyed_compose_tr_nums_three_leaves");
    d.use_site_path = UseSitePath {
        multipath: Some(vec![
            Alternative {
                hardened: false,
                value: a,
            },
            Alternative {
                hardened: false,
                value: b,
            },
        ]),
        wildcard_hardened: false,
    };
    d
}

/// A kind-1 `tr()` nested inside `wsh(...)` -- the shape §6 row 4 refuses.
/// `wsh(tr(...))` is not a real BIP-380 shape and `kind1_from_vector` has no
/// vendored vector to swap (every vendored `tr` is a ROOT `tr`), so this is
/// hand-built from `Node`/`Body` directly, same as
/// `crates/md-codec/src/encode.rs`'s in-crate fixtures. Structurally
/// encodable regardless: `write_node`'s `Body::Children` arm recurses
/// generically over any child tag, and the root-tag allow-list
/// (`Sh|Wsh|Wpkh|Pkh|Tr`) is a DECODE-side check (`decode.rs:97-104`), not an
/// encode-side one -- this fixture must be refused before it ever reaches
/// that check.
fn wsh_wrapping_tr_liana() -> Descriptor {
    Descriptor {
        n: 1,
        path_decl: PathDecl {
            n: 1,
            paths: PathDeclPaths::Shared(OriginPath {
                components: vec![
                    PathComponent {
                        hardened: true,
                        value: 48,
                    },
                    PathComponent {
                        hardened: true,
                        value: 0,
                    },
                    PathComponent {
                        hardened: true,
                        value: 0,
                    },
                    PathComponent {
                        hardened: true,
                        value: 2,
                    },
                ],
            }),
        },
        use_site_path: UseSitePath::standard_multipath(),
        tree: Node {
            tag: Tag::Wsh,
            body: Body::Children(vec![Node {
                tag: Tag::Tr,
                body: Body::Tr {
                    internal_key: InternalKey::LianaUnspendable,
                    tree: Some(Box::new(Node {
                        tag: Tag::PkK,
                        body: Body::KeyArg { index: 0 },
                    })),
                },
            }]),
        },
        tlv: TlvSection::new_empty(),
    }
}

#[test]
fn kind_1_with_a_sortedmulti_a_leaf_is_refused_at_mint() {
    let d = tr_liana_with_sortedmulti_a_leaf();
    assert!(matches!(
        encode_payload(&d),
        Err(Error::UnspendableWithSortedMultiA)
    ));
}

#[test]
fn kind_1_off_the_canonical_use_site_is_refused() {
    assert!(matches!(
        encode_payload(&tr_liana_at_use_site(2, 3)),
        Err(Error::UnspendableUseSiteNotCanonical)
    ));
}

#[test]
fn kind_1_nested_under_wsh_is_refused() {
    assert!(matches!(
        encode_payload(&wsh_wrapping_tr_liana()),
        Err(Error::UnspendableNotRootTr)
    ));
}
