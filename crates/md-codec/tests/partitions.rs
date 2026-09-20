//! Task 3: the two partitions over key identity — `fp_partition` (per spend
//! path, for Liana's `DuplicateOriginSamePath`) and `key_partition`
//! (whole-policy, for its `DuplicateKey`) — plus the absence rule that an
//! all-zero fingerprint/xpub sentinel never groups with another occurrence
//! of itself.
mod common;
use md_codec::policy_shape::{fp_partition, key_partition, policy_shape};

#[test]
fn slots_sharing_a_fingerprint_group_within_a_path() {
    // Path 0 seats @0 and @1 from ONE seed, @2 from another.
    let d = common::seated(&[
        (0, [0xaa; 4]),
        (1, [0xaa; 4]),
        (2, [0xbb; 4]),
        (3, [0xcc; 4]),
    ]);
    let p = fp_partition(&d, &policy_shape(&d));
    assert_eq!(p[0], vec![vec![0u8, 1], vec![2]], "path 0: {p:?}");
    assert_eq!(p[1], vec![vec![3u8]], "path 1: {p:?}");
}

#[test]
fn an_absent_fingerprint_is_its_own_singleton() {
    // md-codec's ABSENT sentinel is all-zero; two absent slots are NOT known
    // to share a signer, and grouping them would assert a relation nobody
    // measured.
    let d = common::seated(&[
        (0, [0x00; 4]),
        (1, [0x00; 4]),
        (2, [0xbb; 4]),
        (3, [0xcc; 4]),
    ]);
    let p = fp_partition(&d, &policy_shape(&d));
    assert_eq!(
        p[0],
        vec![vec![0u8], vec![1], vec![2]],
        "absent fingerprints never join: {p:?}"
    );
}

#[test]
fn key_partition_groups_by_xpub_and_origin_path_across_the_whole_policy() {
    // @0 and @3 are the SAME key at the same origin, in different paths.
    let d = common::seated_same_key(&[0, 3]);
    let kp = key_partition(&d);
    assert!(kp.contains(&vec![0u8, 3]), "whole-policy grouping: {kp:?}");
}

#[test]
fn a_template_only_card_partitions_to_nothing() {
    let d = common::kofn_recovery(); // template-only, no Pubkeys TLV
    assert!(
        fp_partition(&d, &policy_shape(&d))
            .iter()
            .all(|p| p.is_empty())
    );
    assert!(key_partition(&d).is_empty());
}
