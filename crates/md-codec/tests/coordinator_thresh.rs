//! md thresh / seedhammer I-1: the coordinator clauses over an any-of entry
//! (DESIGN_thresh_summary.md section 4). "Mixed" is an any-of entry with at
//! least one timelock: it can be satisfied without its lock, so "no unlocked
//! path" must not fire on it, and a choice of locks alone with no key is "a
//! path with no key".

mod common;

use common::{keyarg, node2, thresh_node, timelock, wrap};
use md_codec::coordinator::{CoordinatorVerdict, Form, Verdict, verdicts};
use md_codec::descriptor_route::descriptor_from_text;
use md_codec::skeleton::{Skeleton, skeleton};
use md_codec::tag::Tag;
use md_codec::tree::{Body, Node};

const MINTED: &str = include_str!("fixtures/policy_shape/minted.tsv");

fn minted_text(name: &str) -> String {
    MINTED
        .lines()
        .map(|l| l.split('\t').collect::<Vec<_>>())
        .find(|f| f[0] == name)
        .unwrap_or_else(|| panic!("no minted vector {name}"))[2]
        .to_string()
}

fn keyed_text(text: &str) -> Skeleton {
    let d = descriptor_from_text(text).unwrap_or_else(|e| panic!("{e}: {text}"));
    skeleton(&d).unwrap_or_else(|e| panic!("{e}"))
}

fn minted(name: &str) -> Skeleton {
    keyed_text(&minted_text(name))
}

/// The minted descriptor with every fingerprint set to the first one (one
/// signer's seed in every slot), checksum dropped.
fn shared_fp(name: &str) -> Skeleton {
    let t = minted_text(name);
    let body = t.split('#').next().unwrap();
    let body = ["bbbbbbb2", "ccccccc3", "ddddddd4", "eeeeeee5"]
        .iter()
        .fold(body.to_string(), |b, fp| b.replace(fp, "aaaaaaa1"));
    keyed_text(&body)
}

/// The reason classes `coordinator` refuses with, over every run.
fn refusals(vs: &[CoordinatorVerdict], coordinator: &str) -> Vec<&'static str> {
    vs.iter()
        .filter(|v| v.coordinator.id.0 == coordinator)
        .filter_map(|v| match &v.verdict {
            Verdict::Refuses { reason, .. } => Some(reason.class),
            _ => None,
        })
        .collect()
}

fn liana(s: &Skeleton) -> Vec<&'static str> {
    refusals(&verdicts(s, Some(Form::Multipath)), "liana")
}

/// L11: a mixed entry can be spent without its lock, so it is an unlocked
/// path. Before this change Liana "refused (no unlocked path)", which is
/// false.
#[test]
fn no_unlocked_path_does_not_fire_on_a_mixed_any_of() {
    for name in [
        "thresh_decay_2of3_lock",
        "thresh_bip388_3of4_lock",
        "thresh_compiler_3of4_lock",
        "thresh_pkh_children",
        "thresh_lock_first",
    ] {
        let r = liana(&minted(name));
        assert!(!r.contains(&"no unlocked path"), "{name}: {r:?}");
        assert!(r.is_empty(), "{name}: Liana is unproven, got {r:?}");
    }
}

/// L11 still fires where the lock is required: `k == n`.
#[test]
fn no_unlocked_path_still_fires_on_k_equal_n_with_a_lock() {
    let r = liana(&minted("thresh_kn_3of3_lock"));
    assert_eq!(r, vec!["no unlocked path"], "{r:?}");
}

/// L10 fires on an absolute lock among the conditions.
#[test]
fn an_absolute_lock_among_the_conditions_still_refuses() {
    let r = liana(&minted("thresh_after_2of3"));
    assert_eq!(r, vec!["an absolute lock"], "{r:?}");
}

/// L2/N1/C1: an any-of whose timelocks alone reach k needs no key.
#[test]
fn a_keyless_choice_is_a_path_with_no_key() {
    let keyless = minted("thresh_keyless_1of2");
    // Height and time cannot both be met, but the clause errs toward warning.
    let mixed_kinds = skeleton(&common::descriptor_from_tree(
        wrap(
            Tag::Wsh,
            thresh_node(
                2,
                vec![
                    keyarg(Tag::PkK, 0),
                    wrap(Tag::Swap, ln(timelock(Tag::After, 800000))),
                    wrap(Tag::Swap, ln(timelock(Tag::After, 1700000000))),
                ],
            ),
        ),
        true,
    ))
    .expect("complete");
    for (name, s) in [
        ("keyless_1of2", &keyless),
        ("mixed_after_kinds", &mixed_kinds),
    ] {
        let v = verdicts(s, Some(Form::Chain0));
        for c in ["liana", "nunchuk", "core"] {
            let r = refusals(&v, c);
            assert!(
                r.contains(&"a path with no key"),
                "{name}: {c} refuses {r:?}"
            );
        }
    }
}

fn ln(t: Node) -> Node {
    node2(
        Tag::OrI,
        Node {
            tag: Tag::False,
            body: Body::Empty,
        },
        wrap(Tag::ZeroNotEqual, t),
    )
}

/// L3: an all-keys any-of reads the same key set as the equivalent `multi`,
/// so one signer twice is refused; a mixed entry is skipped.
#[test]
fn one_signer_twice_fires_on_all_keys_and_skips_a_mixed_entry() {
    let r = liana(&shared_fp("thresh_2of3_keys"));
    assert!(r.contains(&"one signer twice in a path"), "{r:?}");
    let r = liana(&shared_fp("thresh_decay_2of3_lock"));
    assert!(!r.contains(&"one signer twice in a path"), "{r:?}");
}

/// Unchanged: an all-keys any-of with no lock has no locked path.
#[test]
fn an_all_keys_any_of_still_has_no_locked_path() {
    for name in ["thresh_1of3_keys", "thresh_under_or_d", "thresh_tr_leaf"] {
        let r = liana(&minted(name));
        assert!(r.contains(&"no locked path"), "{name}: {r:?}");
    }
}
