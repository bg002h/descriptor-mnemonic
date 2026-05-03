#![allow(missing_docs)]

use assert_cmd::Command;
use std::process::Command as StdCommand;

fn encode(template: &str) -> String {
    let out = StdCommand::new(assert_cmd::cargo::cargo_bin("md"))
        .args(["encode", template])
        .output()
        .unwrap();
    let s = String::from_utf8(out.stdout).unwrap();
    s.lines().next().unwrap().to_string()
}

#[test]
fn decode_round_trips_to_template() {
    let template = "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))";
    let phrase = encode(template);
    let mut cmd = Command::cargo_bin("md").unwrap();
    cmd.args(["decode", &phrase]).assert().success().stdout(predicates::str::contains(template));
}

#[test]
fn decode_json_emits_schema_and_descriptor() {
    let phrase = encode("wpkh(@0/<0;1>/*)");
    Command::cargo_bin("md").unwrap()
        .args(["decode", &phrase, "--json"])
        .assert().success()
        .stdout(predicates::str::contains("\"schema\": \"md-cli/1\""))
        .stdout(predicates::str::contains("\"descriptor\":"));
}
