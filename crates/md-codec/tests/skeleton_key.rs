//! Task 4: `Skeleton` and `SkeletonKey` — membership is exhaustive and
//! fixed by the design (root + inner_wsh + template + fp_partition +
//! key_partition + key_path_kind), `keys_present` is deliberately not in
//! the key, and an incomplete walk yields no key at all.
mod common;
use md_codec::skeleton::{skeleton, skeleton_key};

#[test]
fn seated_and_unseated_do_not_share_a_key() {
    let unseated = common::kofn_recovery();
    let seated = common::seated(&[
        (0, [0xaa; 4]),
        (1, [0xbb; 4]),
        (2, [0xcc; 4]),
        (3, [0xdd; 4]),
    ]);
    // Distinct fingerprints => every slot its own group => same partition
    // shape as template-only? NO: template-only partitions to nothing. The
    // key differs, and that is correct -- identity is part of the key.
    assert_ne!(
        skeleton_key(&skeleton(&unseated).unwrap()),
        skeleton_key(&skeleton(&seated).unwrap())
    );
}

#[test]
fn a_nums_key_path_and_an_unspendable_xpub_do_not_share_a_key() {
    let nums = common::tr_nums_two_leaves();
    let xpub = common::tr_unspendable_xpub_two_leaves();
    assert_ne!(
        skeleton_key(&skeleton(&nums).unwrap()),
        skeleton_key(&skeleton(&xpub).unwrap()),
        "Nunchuk treats these as different wallets (F-449)"
    );
}

#[test]
fn sh_wsh_and_bare_sh_do_not_share_a_key() {
    // sh(wsh) is NOT a ScriptKind value; it is root=Sh plus inner_wsh=true.
    assert_ne!(
        skeleton_key(&skeleton(&common::sh_wsh_2of3()).unwrap()),
        skeleton_key(&skeleton(&common::bare_sh_2of3()).unwrap())
    );
}

#[test]
fn the_serialization_is_stable_and_documented() {
    let k = skeleton_key(&skeleton(&common::kofn_recovery()).unwrap());
    let s = k.as_str();
    assert!(
        s.contains('\u{001F}'),
        "template and partitions are separated: {s}"
    );
    assert_eq!(
        skeleton_key(&skeleton(&common::kofn_recovery()).unwrap()).as_str(),
        s,
        "the key is a pure function of the descriptor"
    );
}

#[test]
fn an_incomplete_walk_yields_no_key() {
    let d = common::unclassifiable();
    assert!(
        skeleton(&d).is_err(),
        "PolicyShape.complete=false must not produce a key -- a partial \
         decomposition keyed as if whole is a false evidence match"
    );
}

/// Not one of the plan's five prescribed tests -- added to make the hard
/// constraint machine-checkable rather than merely stated: `keys_present`
/// MUST be read from raw TLV presence (`Descriptor::is_wallet_policy`),
/// never inferred from partition emptiness. `seated()` is exactly the
/// fixture that tells the two derivations apart: it sets ONLY the
/// Fingerprints TLV, never Pubkeys, so `is_wallet_policy()` -- which checks
/// Pubkeys alone -- reports `false`, while `fp_partition` is very much
/// non-empty (real fingerprint data was measured). A `keys_present` wrongly
/// inferred from partition emptiness would report `true` here instead.
#[test]
fn keys_present_is_read_from_tlv_presence_not_partition_emptiness() {
    let d = common::seated(&[
        (0, [0xaa; 4]),
        (1, [0xbb; 4]),
        (2, [0xcc; 4]),
        (3, [0xdd; 4]),
    ]);
    let s = skeleton(&d).unwrap();
    assert!(
        s.fp_partition.iter().any(|p| !p.is_empty()),
        "fixture sanity: fp_partition must be non-empty for this to be a real test: {:?}",
        s.fp_partition
    );
    assert!(
        !s.keys_present,
        "seated() carries only Fingerprints, never Pubkeys -- is_wallet_policy() is false"
    );
}
