use assert_cmd::Command;
use predicates::prelude::*;

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
