#![allow(missing_docs)]
#![cfg(feature = "cli-compiler")]
use assert_cmd::Command;

#[test]
fn compile_pk_segwitv0() {
    Command::cargo_bin("md")
        .unwrap()
        .args(["compile", "pk(@0)", "--context", "segwitv0"])
        .assert()
        .success()
        .stdout(predicates::str::starts_with("wsh("));
}

#[cfg(feature = "json")]
#[test]
fn compile_json() {
    Command::cargo_bin("md")
        .unwrap()
        .args(["compile", "pk(@0)", "--context", "segwitv0", "--json"])
        .assert()
        .success()
        .stdout(predicates::str::contains("\"schema\": \"md-cli/1\""))
        .stdout(predicates::str::contains("\"template\":"));
}

#[test]
fn compile_pk_tap_emits_keypath_only() {
    // Single-key tap policy collapses to key-path-only `tr(@0)`. The auto-NUMS
    // default in v0.17 does not change this — miniscript's compile_tr is
    // extract-first, so a key-extractable policy keeps the @N internal key.
    Command::cargo_bin("md")
        .unwrap()
        .args(["compile", "pk(@0)", "--context", "tap"])
        .assert()
        .success()
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
    Command::cargo_bin("md")
        .unwrap()
        .args([
            "compile",
            "or(pk(@0),and(pk(@1),older(144)))",
            "--context",
            "tap",
        ])
        .assert()
        .success()
        .stdout(predicates::str::starts_with(
            "tr(@0,and_v(v:pk(@1),older(144)))",
        ));
}

/// v0.17 — `--unspendable-key` is rejected for `--context segwitv0` because
/// wsh() has no internal-key concept.
#[test]
fn compile_segwitv0_rejects_unspendable_key() {
    Command::cargo_bin("md")
        .unwrap()
        .args([
            "compile",
            "pk(@0)",
            "--context",
            "segwitv0",
            "--unspendable-key",
            "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "--unspendable-key is only valid for --context tap",
        ));
}

/// v0.17 — empty `--unspendable-key` is rejected at dispatch with a clear
/// message pointing the user at the auto-NUMS default. Without this guard,
/// `Some("")` would silently flow into miniscript and produce a generic
/// compile error.
#[test]
fn compile_empty_unspendable_key_rejected() {
    Command::cargo_bin("md")
        .unwrap()
        .args([
            "compile",
            "pk(@0)",
            "--context",
            "tap",
            "--unspendable-key",
            "",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "--unspendable-key must not be empty",
        ));
}

/// v0.17 — explicit `--unspendable-key` with the BIP-341 NUMS H-point.
/// miniscript's compile_tr extract-first preserves @0 (the policy has an
/// extractable single-key spend), so output matches the auto-NUMS-default
/// case. Confirms the flag plumbing works end-to-end without regressing
/// the extract-first invariant.
#[test]
fn compile_pk_tap_with_explicit_nums_unspendable_key() {
    Command::cargo_bin("md")
        .unwrap()
        .args([
            "compile",
            "pk(@0)",
            "--context",
            "tap",
            "--unspendable-key",
            "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0",
        ])
        .assert()
        .success()
        .stdout(predicates::str::starts_with("tr(@0)"));
}

/// v0.18 Item G — `--unspendable-key <xpub>` is rejected at dispatch.
/// Pre-v0.18 the xpub form half-worked (compile rendered something, but
/// encode failed opaquely). v0.18 narrows the accepted forms to NUMS-hex-or-
/// omitted with a clear note that other forms are deferred to a future
/// version.
#[test]
fn compile_unspendable_key_rejects_xpub_form() {
    Command::cargo_bin("md")
        .unwrap()
        .args([
            "compile",
            "thresh(2,pk(@0),pk(@1),pk(@2))",
            "--context",
            "tap",
            "--unspendable-key",
            "xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "--unspendable-key currently only accepts the BIP-341 NUMS H-point literal hex",
        ))
        .stderr(predicates::str::contains("deferred to a future version"));
}

/// v0.18 Item G — arbitrary x-only-hex values that aren't the NUMS H-point
/// are also rejected. Verifies the guard is a strict equality check, not a
/// "looks like 64 hex chars" heuristic.
#[test]
fn compile_unspendable_key_rejects_non_nums_x_only_hex() {
    Command::cargo_bin("md")
        .unwrap()
        .args([
            "compile",
            "thresh(2,pk(@0),pk(@1),pk(@2))",
            "--context",
            "tap",
            "--unspendable-key",
            "0000000000000000000000000000000000000000000000000000000000000001",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "--unspendable-key currently only accepts the BIP-341 NUMS H-point literal hex",
        ));
}

/// v0.18 Item G — encode --from-policy applies the same NUMS-hex-only guard
/// as compile (uniform CLI dispatch). Pre-v0.18 the encode-side xpub form
/// failed with an opaque downstream error rather than a clean rejection.
#[test]
fn encode_from_policy_unspendable_key_rejects_xpub_form() {
    Command::cargo_bin("md")
        .unwrap()
        .args([
            "encode",
            "--from-policy",
            "thresh(2,pk(@0),pk(@1),pk(@2))",
            "--context",
            "tap",
            "--unspendable-key",
            "xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "--unspendable-key currently only accepts the BIP-341 NUMS H-point literal hex",
        ));
}
