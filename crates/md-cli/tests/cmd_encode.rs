#![allow(missing_docs)]

use assert_cmd::Command;
use predicates::prelude::*;

/// Abandon-mnemonic tpub at m/84'/1'/0' (BIP 84 testnet account, depth 3).
/// Same value as `parse::keys::ABANDON_TPUB_DEPTH3_BIP84` in the bin crate
/// (integration tests can't reach pub(crate) items there).
const TPUB_FIXTURE: &str = "tpubDC8msFGeGuwnKG9Upg7DM2b4DaRqg3CUZa5g8v2SRQ6K4NSkxUgd7HsL2XVWbVm39yBA4LAxysQAm397zwQSQoQgewGiYZqrA9DsP4zbQ1M";

#[test]
fn encode_template_only_emits_a_phrase() {
    let mut cmd = Command::cargo_bin("md").unwrap();
    cmd.args(["encode", "wpkh(@0/<0;1>/*)"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("md1"));
}

#[test]
fn encode_with_policy_id_fingerprint_prints_two_lines() {
    let mut cmd = Command::cargo_bin("md").unwrap();
    cmd.args(["encode", "wpkh(@0/<0;1>/*)", "--policy-id-fingerprint"])
        .assert()
        .success()
        .stdout(predicate::str::contains("policy-id-fingerprint: 0x"));
}

#[test]
fn encode_json_has_schema_and_phrase() {
    Command::cargo_bin("md").unwrap()
        .args(["encode", "wpkh(@0/<0;1>/*)", "--json"])
        .assert().success()
        .stdout(predicate::str::contains("\"schema\": \"md-cli/1\""))
        .stdout(predicate::str::contains("\"phrase\":"));
}

#[cfg(feature = "cli-compiler")]
#[test]
fn encode_from_policy_segwitv0() {
    Command::cargo_bin("md").unwrap()
        .args(["encode", "--from-policy", "pk(@0)", "--context", "segwitv0"])
        .assert().success()
        .stdout(predicate::str::starts_with("md1"));
}

#[test]
fn encode_json_network_field_default_mainnet() {
    Command::cargo_bin("md").unwrap()
        .args(["encode", "wpkh(@0/<0;1>/*)", "--json"])
        .assert().success()
        .stdout(predicate::str::contains("\"network\": \"mainnet\""));
}

#[test]
fn encode_json_network_field_testnet() {
    Command::cargo_bin("md").unwrap()
        .args(["encode", "wpkh(@0/<0;1>/*)", "--network", "testnet",
               "--key", &format!("@0={TPUB_FIXTURE}"), "--json"])
        .assert().success()
        .stdout(predicate::str::contains("\"network\": \"testnet\""));
}

#[test]
fn encode_rejects_tpub_under_default_mainnet() {
    Command::cargo_bin("md").unwrap()
        .args(["encode", "wpkh(@0/<0;1>/*)", "--key", &format!("@0={TPUB_FIXTURE}")])
        .assert().code(1)
        .stderr(predicate::str::contains("expected mainnet"));
}
