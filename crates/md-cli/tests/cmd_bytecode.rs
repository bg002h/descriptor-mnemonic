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
fn bytecode_accepts_grouped_input() {
    // mstring display-grouping (SPEC §3.2): a separator-bearing card re-ingests.
    let phrase = encode("wpkh(@0/<0;1>/*)");
    let grouped = group5(&phrase);
    Command::cargo_bin("md")
        .unwrap()
        .args(["bytecode", &grouped])
        .assert()
        .success()
        .stdout(predicates::str::contains("payload-bits:"));
}

#[test]
fn bytecode_prints_hex_and_lengths() {
    let phrase = encode("wpkh(@0/<0;1>/*)");
    Command::cargo_bin("md")
        .unwrap()
        .args(["bytecode", &phrase])
        .assert()
        .success()
        .stdout(predicates::str::contains("payload-bits:"))
        .stdout(predicates::str::contains("payload-bytes:"))
        .stdout(predicates::str::contains("hex:"));
}

#[cfg(feature = "json")]
#[test]
fn bytecode_json_has_payload_fields() {
    let phrase = encode("wpkh(@0/<0;1>/*)");
    Command::cargo_bin("md")
        .unwrap()
        .args(["bytecode", &phrase, "--json"])
        .assert()
        .success()
        .stdout(predicates::str::contains("\"schema\": \"md-cli/1\""))
        .stdout(predicates::str::contains("\"payload_bytes\":"));
}
