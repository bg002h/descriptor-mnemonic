//! Lowering tests for `md_codec::compose` (SPEC_wallet_policy_composer.md §4, §5).
//!
//! Every expected template string below is the FIXED spelling the Go port must
//! reproduce byte for byte; a change here is a normative change.

use md_codec::canonicalize::canonicalize_placeholder_indices;
use md_codec::chunk::{reassemble, split};
use md_codec::compose::{
    ComposeError, Composed, Experimental, KeySet, Lock, MAX_PATHS, MAX_SLOTS, PathList, SlotOrigin,
    SpendPath, Wrapper, compose, compose_with, template_with_origins,
};
use md_codec::encode::{encode_md1_string, encode_payload};
use md_codec::origin_path::{OriginPath, PathComponent, PathDeclPaths};
use md_codec::render::descriptor_to_template;

const H1: [u8; 32] = [0xa8; 32];

fn keys(k: u8, n: u8) -> SpendPath {
    SpendPath {
        keys: Some(KeySet { k, n, sorted: true }),
        hash: None,
        lock: None,
    }
}

fn unsorted(k: u8, n: u8) -> SpendPath {
    SpendPath {
        keys: Some(KeySet {
            k,
            n,
            sorted: false,
        }),
        hash: None,
        lock: None,
    }
}

fn with_lock(mut p: SpendPath, lock: Lock) -> SpendPath {
    p.lock = Some(lock);
    p
}

fn with_hash(mut p: SpendPath, h: [u8; 32]) -> SpendPath {
    p.hash = Some(h);
    p
}

fn keyless(h: [u8; 32], lock: Option<Lock>) -> SpendPath {
    SpendPath {
        keys: None,
        hash: Some(h),
        lock,
    }
}

fn list(wrapper: Wrapper, paths: Vec<SpendPath>) -> PathList {
    PathList { wrapper, paths }
}

fn hardened(values: &[u32]) -> OriginPath {
    OriginPath {
        components: values
            .iter()
            .map(|v| PathComponent {
                hardened: true,
                value: *v,
            })
            .collect(),
    }
}

// ---- §4e structural refusals -------------------------------------------------

#[test]
fn compose_refuses_an_empty_path_list() {
    let err = compose(&list(Wrapper::Wsh, vec![])).unwrap_err();
    assert_eq!(err, ComposeError::NoPaths);
}

#[test]
fn compose_refuses_more_than_max_paths() {
    let paths: Vec<SpendPath> = (0..(MAX_PATHS + 1)).map(|_| keys(1, 1)).collect();
    let err = compose(&list(Wrapper::Wsh, paths)).unwrap_err();
    assert_eq!(err, ComposeError::TooManyPaths { got: MAX_PATHS + 1 });
}

#[test]
fn compose_refuses_a_policy_with_no_keyed_path() {
    let err = compose(&list(Wrapper::Wsh, vec![keyless(H1, None)])).unwrap_err();
    assert_eq!(err, ComposeError::NoKeyedPath);
}

#[test]
fn compose_refuses_a_lock_only_path() {
    let lock_only = SpendPath {
        keys: None,
        hash: None,
        lock: Some(Lock::OlderBlocks(100)),
    };
    let err = compose(&list(Wrapper::Wsh, vec![keys(1, 1), lock_only])).unwrap_err();
    assert_eq!(err, ComposeError::LockOnlyPath { path: 1 });
}

#[test]
fn compose_refuses_a_keyless_path_under_tr() {
    let err = compose(&list(Wrapper::Tr, vec![keys(2, 3), keyless(H1, None)])).unwrap_err();
    assert_eq!(err, ComposeError::KeylessUnderTr { path: 1 });
}

#[test]
fn compose_refuses_bad_thresholds() {
    assert_eq!(
        compose(&list(Wrapper::Wsh, vec![keys(0, 2)])).unwrap_err(),
        ComposeError::BadThreshold {
            path: 0,
            k: 0,
            n: 2
        }
    );
    assert_eq!(
        compose(&list(Wrapper::Wsh, vec![keys(3, 2)])).unwrap_err(),
        ComposeError::BadThreshold {
            path: 0,
            k: 3,
            n: 2
        }
    );
    assert_eq!(
        compose(&list(Wrapper::Wsh, vec![keys(1, 10)])).unwrap_err(),
        ComposeError::BadThreshold {
            path: 0,
            k: 1,
            n: 10
        }
    );
}

#[test]
fn compose_refuses_a_thirty_third_slot() {
    // 3 × 9 + 6 = 33 slots.
    let paths = vec![keys(9, 9), keys(9, 9), keys(9, 9), keys(6, 6)];
    let err = compose(&list(Wrapper::Wsh, paths)).unwrap_err();
    assert_eq!(
        err,
        ComposeError::TooManySlots {
            got: 33,
            max: MAX_SLOTS
        }
    );
}

#[test]
fn compose_admits_exactly_thirty_two_slots() {
    // 3 × 9 + 5 = 32 slots. Passes only once the lowering exists (Task 2).
    let paths = vec![keys(9, 9), keys(9, 9), keys(9, 9), keys(5, 5)];
    assert!(compose(&list(Wrapper::Wsh, paths)).is_ok());
}

#[test]
fn compose_refuses_legacy_wrappers_outside_the_single_sorted_multi_shape() {
    for w in [Wrapper::Sh, Wrapper::ShWsh] {
        assert_eq!(
            compose(&list(w, vec![keys(1, 1)])).unwrap_err(),
            ComposeError::LegacyWrapperShape
        );
        assert_eq!(
            compose(&list(w, vec![keys(2, 3), keys(1, 1)])).unwrap_err(),
            ComposeError::LegacyWrapperShape
        );
        assert_eq!(
            compose(&list(w, vec![with_lock(keys(2, 3), Lock::OlderBlocks(10))])).unwrap_err(),
            ComposeError::LegacyWrapperShape
        );
        assert_eq!(
            compose(&list(w, vec![unsorted(2, 3)])).unwrap_err(),
            ComposeError::LegacyWrapperShape
        );
    }
}

#[test]
fn compose_refuses_lock_operands_outside_the_consensus_bands() {
    // Each case pins the BAND NAMED in the refusal, not only that a refusal fired.
    let cases: &[(Lock, &str)] = &[
        (Lock::OlderBlocks(0), "older in blocks needs 1..=65535"),
        (
            Lock::OlderUnits(0),
            "older in 512-second units needs 1..=65535",
        ),
        (Lock::AfterHeight(0), "after height needs 1..=499999999"),
        (
            Lock::AfterHeight(500_000_000),
            "after height needs 1..=499999999",
        ),
        (
            Lock::AfterTime(499_999_999),
            "after time needs 500000000..=2147483647",
        ),
        (
            Lock::AfterTime(2_147_483_648),
            "after time needs 500000000..=2147483647",
        ),
    ];
    for (lock, why) in cases {
        let err = compose(&list(Wrapper::Wsh, vec![with_lock(keys(1, 1), *lock)])).unwrap_err();
        assert_eq!(
            err,
            ComposeError::LockOutOfRange { path: 0, why },
            "{lock:?}"
        );
    }
}

#[test]
fn lock_operand_bands_are_inclusive_at_both_ends() {
    // BIP-68 / BIP-65 / BIP-379 boundaries, straight from `Lock::operand`; no
    // lowering involved, so this passes from Task 1.
    use md_codec::tag::Tag;
    assert_eq!(Lock::OlderBlocks(1).operand(), Ok((Tag::Older, 1)));
    assert_eq!(Lock::OlderBlocks(65535).operand(), Ok((Tag::Older, 65535)));
    assert_eq!(Lock::OlderUnits(1).operand(), Ok((Tag::Older, 0x0040_0001)));
    assert_eq!(
        Lock::OlderUnits(65535).operand(),
        Ok((Tag::Older, 0x0040_ffff))
    );
    assert_eq!(Lock::AfterHeight(1).operand(), Ok((Tag::After, 1)));
    assert_eq!(
        Lock::AfterHeight(499_999_999).operand(),
        Ok((Tag::After, 499_999_999))
    );
    assert_eq!(
        Lock::AfterTime(500_000_000).operand(),
        Ok((Tag::After, 500_000_000))
    );
    assert_eq!(
        Lock::AfterTime(2_147_483_647).operand(),
        Ok((Tag::After, 2_147_483_647))
    );
    assert!(
        Lock::OlderUnits(0).operand().is_err(),
        "0x400000 alone is a lock of ZERO units, i.e. none (filed md-older-zero-time-units-not-refused)"
    );
}
