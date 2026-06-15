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

/// Insert a comma every 5 chars to simulate a grouped/transcribed card.
/// Comma is the SPEC §3.2 separator md-codec's codex32 layer does NOT already
/// tolerate (it strips whitespace/hyphen via D11), so this genuinely exercises
/// the md-cli intake strip (`strip_md1_inputs`).
fn group5(s: &str) -> String {
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && i % 5 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[test]
fn decode_accepts_grouped_input() {
    // mstring display-grouping (SPEC §3.2): a separator-bearing card re-ingests.
    let template = "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))";
    let phrase = encode(template);
    let grouped = group5(&phrase);
    Command::cargo_bin("md")
        .unwrap()
        .args(["decode", &grouped])
        .assert()
        .success()
        .stdout(predicates::str::contains(template));
}

#[test]
fn decode_round_trips_to_template() {
    let template = "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))";
    let phrase = encode(template);
    let mut cmd = Command::cargo_bin("md").unwrap();
    cmd.args(["decode", &phrase])
        .assert()
        .success()
        .stdout(predicates::str::contains(template));
}

#[cfg(feature = "json")]
#[test]
fn decode_json_emits_schema_and_descriptor() {
    let phrase = encode("wpkh(@0/<0;1>/*)");
    Command::cargo_bin("md")
        .unwrap()
        .args(["decode", &phrase, "--json"])
        .assert()
        .success()
        .stdout(predicates::str::contains("\"schema\": \"md-cli/1\""))
        .stdout(predicates::str::contains("\"descriptor\":"));
}
