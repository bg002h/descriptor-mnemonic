#![allow(missing_docs)]

use assert_cmd::Command;
use std::process::Command as StdCommand;

fn encode(template: &str) -> String {
    let out = StdCommand::new(assert_cmd::cargo::cargo_bin("md"))
        .args(["encode", template])
        .output()
        .unwrap();
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .to_string()
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
fn inspect_accepts_grouped_input() {
    // mstring display-grouping (SPEC §3.2): a separator-bearing card re-ingests.
    let phrase = encode("wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))");
    let grouped = group5(&phrase);
    Command::cargo_bin("md")
        .unwrap()
        .args(["inspect", &grouped])
        .assert()
        .success()
        .stdout(predicates::str::contains("template:"));
}

#[test]
fn inspect_prints_all_fields() {
    let phrase = encode("wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))");
    Command::cargo_bin("md")
        .unwrap()
        .args(["inspect", &phrase])
        .assert()
        .success()
        .stdout(predicates::str::contains("template:"))
        .stdout(predicates::str::contains("md1-encoding-id:"))
        .stdout(predicates::str::contains(
            "wallet-policy-id-fingerprint: 0x",
        ));
}

#[cfg(feature = "json")]
#[test]
fn inspect_json_has_schema_and_descriptor() {
    let phrase = encode("wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))");
    Command::cargo_bin("md")
        .unwrap()
        .args(["inspect", &phrase, "--json"])
        .assert()
        .success()
        .stdout(predicates::str::contains("\"schema\": \"md-cli/1\""))
        .stdout(predicates::str::contains("\"wallet_policy_id\":"));
}
