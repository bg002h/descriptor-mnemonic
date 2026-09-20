//! Task 2: the abstracting render mode — `descriptor_to_abstract_template`
//! replaces lock values and hash digests with per-kind equality classes
//! (`kind#class`) so two policies differing only in which concrete lock
//! value or digest they use can be compared.

mod common;
use md_codec::render::descriptor_to_abstract_template;

#[test]
fn equal_lock_values_share_a_class_and_different_ones_do_not() {
    // wsh(or_i(and_v(v:pkh(@0),older(26280)),
    //          or_i(and_v(v:pkh(@1),older(1000)),
    //               and_v(v:pkh(@2),older(26280)))))
    let d = common::three_older_descriptor(26280, 1000, 26280);
    let t = descriptor_to_abstract_template(&d).unwrap();
    assert!(t.contains("older(older-blocks#1)"), "got {t}");
    assert!(t.contains("older(older-blocks#2)"), "got {t}");
    assert_eq!(
        t.matches("older-blocks#1").count(),
        2,
        "the two equal values share class 1: {t}"
    );
    assert!(
        !t.contains("26280"),
        "no literal lock value may survive: {t}"
    );
}

#[test]
fn digests_carry_a_class_too_symmetric_with_locks() {
    let d = common::two_sha256_descriptor([0x11; 32], [0x11; 32]);
    let t = descriptor_to_abstract_template(&d).unwrap();
    assert_eq!(
        t.matches("sha256(#1)").count(),
        2,
        "two branches committing to the SAME digest share a class: {t}"
    );
    assert!(!t.contains("1111"), "no literal digest may survive: {t}");
}

#[test]
fn the_use_site_is_kept() {
    let d = common::kofn_recovery_with_use_site();
    let t = descriptor_to_abstract_template(&d).unwrap();
    assert!(
        t.contains("/<0;1>/*"),
        "the key is looked up against evidence whose template field keeps it: {t}"
    );
}
