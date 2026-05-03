use assert_cmd::Command;
use std::process::Command as StdCommand;

fn encode(template: &str) -> String {
    let out = StdCommand::new(assert_cmd::cargo::cargo_bin("md"))
        .args(["encode", template])
        .output().unwrap();
    String::from_utf8(out.stdout).unwrap().lines().next().unwrap().to_string()
}

#[test]
fn inspect_prints_all_fields() {
    let phrase = encode("wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))");
    Command::cargo_bin("md").unwrap()
        .args(["inspect", &phrase])
        .assert().success()
        .stdout(predicates::str::contains("template:"))
        .stdout(predicates::str::contains("md1-encoding-id:"))
        .stdout(predicates::str::contains("wallet-policy-id-fingerprint: 0x"));
}

#[test]
fn inspect_json_has_schema_and_descriptor() {
    let phrase = encode("wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))");
    Command::cargo_bin("md").unwrap()
        .args(["inspect", &phrase, "--json"])
        .assert().success()
        .stdout(predicates::str::contains("\"schema\": \"md-cli/1\""))
        .stdout(predicates::str::contains("\"wallet_policy_id\":"));
}
