//! v0.17 Phase 0 V1 — pre-implementation behavior canary.
//!
//! Locks down the exact pre-Phase-1 / pre-Phase-3 failure modes for the
//! three template shapes v0.17 will need to flip to success. Each test
//! either asserts success today (V1.a control) or pins the exact error
//! string today (V1.b, V1.c). When Phase 2 (Axis 1 walker extension)
//! and Phase 3 (Axis 2 NUMS recognition) land, V1.b and V1.c MUST be
//! updated to assert success — that update is the canary that the
//! implementation actually changed the behavior the plan claimed.

#![allow(missing_docs)]

use assert_cmd::Command;
use predicates::prelude::*;

/// V1.a — control: single-leaf bare-pk tap template encodes cleanly today.
/// Phase 4 must keep this passing (no regression on the existing supported shape).
/// No `--keys` is passed; encode uses synthetic placeholder keys when none supplied.
#[test]
fn v017_v1_a_single_leaf_bare_pk_encodes() {
    Command::cargo_bin("md").unwrap()
        .args(["encode", "tr(@0,pk(@1))"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("md1"));
}

/// V1.b — Axis 1 success case. PRE-Phase-2 this failed with
/// `unsupported miniscript fragment: and_v` because `walk_miniscript_node`
/// only wired PkK/PkH/Multi/MultiA/Check. Phase 2 added Terminal::AndV +
/// Terminal::Older + Terminal::Verify arms; the canary fired and was
/// flipped to assert-success in the same commit (TDD red→green; keeps
/// each commit's test suite green). The historical pre-Phase-2 stderr
/// `unsupported miniscript fragment: and_v(v:pk(<resolved-xpub>),older(144))`
/// is preserved as a comment for git-history readers.
#[test]
fn v017_v1_b_and_v_inheritance_pattern_encodes() {
    Command::cargo_bin("md").unwrap()
        .args(["encode", "tr(@0,and_v(v:pk(@1),older(144)))"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("md1"));
}

/// V1.c — pre-Phase-3 failure mode: literal x-only hex in tr() internal-key
/// position is rejected by `walk_tr` because lookup_key only knows about
/// `@N`-derived synthetic xpubs. Phase 3's NUMS recognition (walk_tr branch
/// on `t.internal_key().to_string() == NUMS_H_POINT_X_ONLY_HEX`) will FORCE
/// this test to fail; **update this test in Phase 5 (not Phase 3)** to
/// assert success once the full integration suite is wired. Failing here
/// mid-cycle is the canary that Axis 2 shipped.
#[test]
fn v017_v1_c_nums_pre_phase_3_synthetic_key_not_found() {
    Command::cargo_bin("md").unwrap()
        .args(["encode", "tr(50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0,multi_a(2,@0,@1,@2))"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "synthetic key 50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0 not found in key map",
        ));
}
