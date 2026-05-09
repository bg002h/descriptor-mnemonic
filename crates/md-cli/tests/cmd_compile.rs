#![allow(missing_docs)]

#![cfg(feature = "cli-compiler")]
use assert_cmd::Command;

#[test]
fn compile_pk_segwitv0() {
    Command::cargo_bin("md").unwrap()
        .args(["compile", "pk(@0)", "--context", "segwitv0"])
        .assert().success()
        .stdout(predicates::str::starts_with("wsh("));
}

#[test]
fn compile_json() {
    Command::cargo_bin("md").unwrap()
        .args(["compile", "pk(@0)", "--context", "segwitv0", "--json"])
        .assert().success()
        .stdout(predicates::str::contains("\"schema\": \"md-cli/1\""))
        .stdout(predicates::str::contains("\"template\":"));
}

#[test]
fn compile_pk_tap_emits_keypath_only() {
    // Single-key tap policy collapses to key-path-only `tr(@0)`. The auto-NUMS
    // default in v0.17 does not change this — miniscript's compile_tr is
    // extract-first, so a key-extractable policy keeps the @N internal key.
    Command::cargo_bin("md").unwrap()
        .args(["compile", "pk(@0)", "--context", "tap"])
        .assert().success()
        .stdout(predicates::str::starts_with("tr(@0)"));
}

/// v0.17 — 2-of-3 hardware-wallet multisig (the headline use case).
/// Auto-NUMS default emits `tr(<NUMS-hex>, multi_a(2,@0,@1,@2))`.
#[test]
fn compile_thresh_2_of_3_tap_auto_nums() {
    Command::cargo_bin("md").unwrap()
        .args(["compile", "thresh(2,pk(@0),pk(@1),pk(@2))", "--context", "tap"])
        .assert().success()
        .stdout(predicates::str::contains(
            "tr(50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0,multi_a(2,@0,@1,@2))",
        ));
}

/// v0.17 — inheritance / timelock pattern. Extract wins (@0 becomes
/// internal key); script-path leaf is `and_v(v:pk(@1),older(144))`.
#[test]
fn compile_or_pk_and_pk_older_tap() {
    Command::cargo_bin("md").unwrap()
        .args(["compile", "or(pk(@0),and(pk(@1),older(144)))", "--context", "tap"])
        .assert().success()
        .stdout(predicates::str::starts_with("tr(@0,and_v(v:pk(@1),older(144)))"));
}

/// v0.17 — `--unspendable-key` is rejected for `--context segwitv0` because
/// wsh() has no internal-key concept.
#[test]
fn compile_segwitv0_rejects_unspendable_key() {
    Command::cargo_bin("md").unwrap()
        .args([
            "compile", "pk(@0)", "--context", "segwitv0",
            "--unspendable-key", "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0",
        ])
        .assert().failure()
        .stderr(predicates::str::contains("--unspendable-key is only valid for --context tap"));
}

/// v0.17 — empty `--unspendable-key` is rejected at dispatch with a clear
/// message pointing the user at the auto-NUMS default. Without this guard,
/// `Some("")` would silently flow into miniscript and produce a generic
/// compile error.
#[test]
fn compile_empty_unspendable_key_rejected() {
    Command::cargo_bin("md").unwrap()
        .args(["compile", "pk(@0)", "--context", "tap", "--unspendable-key", ""])
        .assert().failure()
        .stderr(predicates::str::contains("--unspendable-key must not be empty"));
}

/// v0.17 — explicit `--unspendable-key` with the BIP-341 NUMS H-point.
/// miniscript's compile_tr extract-first preserves @0 (the policy has an
/// extractable single-key spend), so output matches the auto-NUMS-default
/// case. Confirms the flag plumbing works end-to-end without regressing
/// the extract-first invariant.
#[test]
fn compile_pk_tap_with_explicit_nums_unspendable_key() {
    Command::cargo_bin("md").unwrap()
        .args([
            "compile", "pk(@0)", "--context", "tap",
            "--unspendable-key", "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0",
        ])
        .assert().success()
        .stdout(predicates::str::starts_with("tr(@0)"));
}
