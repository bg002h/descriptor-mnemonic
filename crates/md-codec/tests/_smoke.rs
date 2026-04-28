//! Smoke test for the shared `tests/common/mod.rs` helpers. Each helper
//! is exercised by at least one Phase 6 bucket below; this file verifies
//! the foundation compiles and the simplest helper round-trips.

mod common;

#[test]
fn round_trip_assert_works_on_simplest_policy() {
    common::round_trip_assert("wsh(pk(@0/**))");
}

#[test]
fn assert_structural_eq_passes_for_equal_policies() {
    use std::str::FromStr;
    use md_codec::WalletPolicy;
    let p1 = WalletPolicy::from_str("wsh(pk(@0/**))").unwrap();
    let p2 = WalletPolicy::from_str("wsh(pk(@0/**))").unwrap();
    common::assert_structural_eq(&p1, &p2);
}

#[test]
fn corrupt_n_changes_n_chars() {
    use md_codec::{EncodeOptions, WalletPolicy, encode};
    let p: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
    let backup = encode(&p, &EncodeOptions::default()).unwrap();
    let original = backup.chunks[0].raw.clone();
    let code = backup.chunks[0].code;
    let corrupted = common::corrupt_n(&original, 3, 0xDEADBEEF, code);
    let diff_count = original
        .chars()
        .zip(corrupted.chars())
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(diff_count, 3, "corrupt_n should change exactly 3 chars");
}
