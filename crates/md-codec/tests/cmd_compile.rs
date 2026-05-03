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
