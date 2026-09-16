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

/// F-582: `md verify` MISMATCHed a CORRECT plate set and named two numbers and
/// no cause. The most common cause is a flag the operator did not repeat:
/// fingerprints are part of the payload, so verifying a plate minted with
/// `--fingerprint` against a template without it mismatches on size.
///
/// The failure mode is the reaction, not the message. An operator who makes
/// verification pass by re-minting WITHOUT `--fingerprint` gets a set that two
/// gates bless and a third had already condemned as unseatable -- so the hint
/// points at the template side and explicitly says not to re-mint.
///
/// MUTATION: drop the hint -> the first assertion fails. MUTATION: emit it
/// unconditionally -> the WITH-fingerprint control still passes (it exits OK
/// and prints no mismatch), so the size-direction assertion is what pins it.
#[test]
fn a_mismatch_caused_by_missing_fingerprints_says_so() {
    const TMPL: &str = "wsh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))";
    const K: [&str; 3] = [
        "xpub6EddmgK6uMbst7251zES359MJzfz3o2wTJWeefTzSiPJf1FUg6easA2Uk3jUbztBffWS4Dg2oWjhomvU2jJKr4EZukDfxVxb4VJva3jXGAK",
        "xpub6EPimu5ztRjy6dfECvb9AQNAadNHN8qumZYQhvdPWh9U9xJ9XMi328aXAFMGHwc11wZWrmdwsHUWJn5in6BHywGrbJx5ZLUWe8xhNPRf5kL",
        "xpub6EVA2mZiCBtGYbGM5eEs42k8tso4QekUog2t8U14UDyjUYpKByLYcXB8t3yVAvapQymRSK2MDM84cspZa5udQEEUSVGo28RJJt47eeC846C",
    ];
    const FP: [&str; 3] = ["@0=aabbccdd", "@1=11223344", "@2=55667788"];

    let dir = tempfile::tempdir().unwrap();
    let plate = dir.path().join("fp.md1");

    // Mint WITH fingerprints.
    let minted = Command::cargo_bin("md")
        .unwrap()
        .args(["encode", TMPL, "--group-size", "0"])
        .args(["--key", &format!("@0={}", K[0])])
        .args(["--key", &format!("@1={}", K[1])])
        .args(["--key", &format!("@2={}", K[2])])
        .args(["--fingerprint", FP[0]])
        .args(["--fingerprint", FP[1]])
        .args(["--fingerprint", FP[2]])
        .output()
        .unwrap();
    assert!(minted.status.success(), "mint failed");
    let md1: String = String::from_utf8_lossy(&minted.stdout)
        .lines()
        .filter(|l| l.starts_with("md1"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!md1.is_empty(), "no md1 minted");
    std::fs::write(&plate, &md1).unwrap();

    // Verify WITHOUT them: mismatches, and must explain why.
    let bare = Command::cargo_bin("md")
        .unwrap()
        .args(["verify", "--template", TMPL])
        .args(["--key", &format!("@0={}", K[0])])
        .args(["--key", &format!("@1={}", K[1])])
        .args(["--key", &format!("@2={}", K[2])])
        .args(["--in", &plate.display().to_string()])
        .output()
        .unwrap();
    assert!(
        !bare.status.success(),
        "a fingerprinted plate verified without them"
    );
    let err = String::from_utf8_lossy(&bare.stderr);
    assert!(
        err.contains("--fingerprint"),
        "the mismatch never names the flag that explains it:\n{err}"
    );
    assert!(
        err.contains("Do not re-mint"),
        "the mismatch does not warn against the dangerous fix:\n{err}"
    );

    // Control: with the same fingerprints it verifies, so the plate was right
    // all along and the hint described a real cause.
    Command::cargo_bin("md")
        .unwrap()
        .args(["verify", "--template", TMPL])
        .args(["--key", &format!("@0={}", K[0])])
        .args(["--key", &format!("@1={}", K[1])])
        .args(["--key", &format!("@2={}", K[2])])
        .args(["--fingerprint", FP[0]])
        .args(["--fingerprint", FP[1]])
        .args(["--fingerprint", FP[2]])
        .args(["--in", &plate.display().to_string()])
        .assert()
        .success();
}
