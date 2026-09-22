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
        .stdout(predicates::str::contains("\"schema\": \"md-cli/2\""))
        .stdout(predicates::str::contains("\"payload_bytes\":"));
}

/// F-594: `md bytecode` refused a card `md encode` had just minted, with a bare
/// codec error — `non-canonical wrapper requires explicit origin for @0`. The
/// operator's natural conclusion is that their plate is corrupt. It is not:
/// `md decode` reads the same card and reports the origin as
/// «unspecified — supply on restore», and `md bytecode` has no flag that could
/// supply one, so the refusal was a dead end.
///
/// This file's own sibling comment in `cmd/bytecode.rs` states the principle it
/// violated: a plate "must still READ, or the refusal has taken away the only
/// tool that could tell its holder what they have."
///
/// The exit code and the error type are deliberately unchanged — only guidance
/// is added — and both named routes were RUN before being printed.
///
/// MUTATION: delete the `eprintln!` -> the first two assertions fail while the
/// exit-code assertion still passes, which is the point: the verdict was never
/// what was wrong.
#[test]
fn a_template_card_without_an_origin_is_not_reported_as_corrupt() {
    let minted = Command::cargo_bin("md")
        .unwrap()
        .args([
            "encode",
            "wsh(and_v(v:sha256(2d711642b726b04401627ca9fbac32f5c8530fb1903cc4db02258717921a4881),pk(@0/<0;1>/*)))",
            "--group-size",
            "0",
        ])
        .output()
        .unwrap();
    assert!(minted.status.success(), "mint failed");
    let md1 = String::from_utf8_lossy(&minted.stdout)
        .lines()
        .find(|l| l.starts_with("md1"))
        .expect("no md1 minted")
        .to_string();

    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["bytecode", &md1])
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("NOT corrupt"),
        "the refusal still reads as a corrupt plate:\n{err}"
    );
    assert!(
        err.contains("md decode") && err.contains("--path"),
        "the refusal names neither way forward:\n{err}"
    );
    // The verdict itself was never the defect.
    assert!(!out.status.success(), "the card must still be refused");

    // Control: the SAME policy minted WITH an origin reads clean and silent.
    let with_origin = Command::cargo_bin("md")
        .unwrap()
        .args([
            "encode",
            "wsh(and_v(v:sha256(2d711642b726b04401627ca9fbac32f5c8530fb1903cc4db02258717921a4881),pk(@0/<0;1>/*)))",
            "--path",
            "bip84",
            "--group-size",
            "0",
        ])
        .output()
        .unwrap();
    let md1b = String::from_utf8_lossy(&with_origin.stdout)
        .lines()
        .find(|l| l.starts_with("md1"))
        .expect("no md1")
        .to_string();
    let ok = Command::cargo_bin("md")
        .unwrap()
        .args(["bytecode", &md1b])
        .output()
        .unwrap();
    assert!(
        ok.status.success(),
        "a card with an origin must still decode"
    );
    assert!(
        !String::from_utf8_lossy(&ok.stderr).contains("NOT corrupt"),
        "the hint fires on a card that decoded fine"
    );
}
