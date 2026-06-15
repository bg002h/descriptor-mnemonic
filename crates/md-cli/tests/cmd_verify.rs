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
fn verify_accepts_grouped_input() {
    // mstring display-grouping (SPEC §3.2): a separator-bearing card re-ingests.
    let template = "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))";
    let phrase = encode(template);
    let grouped = group5(&phrase);
    Command::cargo_bin("md")
        .unwrap()
        .args(["verify", &grouped, "--template", template])
        .assert()
        .code(0)
        .stdout(predicates::str::contains("OK"));
}

#[test]
fn verify_match_returns_0() {
    let template = "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))";
    let phrase = encode(template);
    Command::cargo_bin("md")
        .unwrap()
        .args(["verify", &phrase, "--template", template])
        .assert()
        .code(0)
        .stdout(predicates::str::contains("OK"));
}

#[test]
fn verify_mismatch_returns_1() {
    let template = "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))";
    let phrase = encode(template);
    let wrong = "wpkh(@0/<0;1>/*)";
    Command::cargo_bin("md")
        .unwrap()
        .args(["verify", &phrase, "--template", wrong])
        .assert()
        .code(1)
        .stderr(predicates::str::contains("MISMATCH"));
}
