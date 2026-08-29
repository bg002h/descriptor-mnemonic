//! F-420 (mnemonic-engrave FOLLOWUPS, journey walk W2): `md encode` on a
//! concrete descriptor or a BlueWallet `Key: value` export used to dead-end
//! with only "template contains no @i placeholders" — no recognition, no
//! referral, from the tool NAMED for descriptors. The refusal now names what
//! the input is and refers to `me sysw pack --as <descriptor|md1>`. These run
//! the real binary because the walk measured the real binary.

#![allow(missing_docs)]

use assert_cmd::Command;

const REFERRAL: &str = "me sysw pack --as <descriptor|md1>";

const XPUB: &str = "xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz";

fn run(args: &[&str]) -> (String, String, bool) {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(args)
        .output()
        .unwrap();
    (
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
        out.status.success(),
    )
}

#[test]
fn concrete_descriptor_is_refused_with_the_referral() {
    let template = format!("wpkh([4bbaa801/84'/0'/0']{XPUB}/<0;1>/*)");
    let (stdout, stderr, ok) = run(&["encode", &template]);
    assert!(!ok, "a concrete descriptor must refuse");
    assert!(
        stderr.contains(REFERRAL),
        "expected referral; got: {stderr}"
    );
    assert!(
        stderr.contains("concrete wallet descriptor"),
        "expected the input named; got: {stderr}"
    );
    assert!(stdout.is_empty(), "refusals write nothing to stdout");
}

#[test]
fn bluewallet_file_is_refused_with_the_referral() {
    let content = format!(
        "Name: our wallet\nPolicy: 2 of 2\nDerivation: m/48'/0'/0'/2'\nFormat: P2WSH\nDC567276: {XPUB}\n"
    );
    let (stdout, stderr, ok) = run(&["encode", &content]);
    assert!(!ok, "a BlueWallet export must refuse");
    assert!(
        stderr.contains(REFERRAL),
        "expected referral; got: {stderr}"
    );
    assert!(
        stderr.contains("BlueWallet"),
        "expected the file shape named; got: {stderr}"
    );
    assert!(stdout.is_empty(), "refusals write nothing to stdout");
}

#[test]
fn neither_shape_keeps_the_terse_refusal_without_a_referral() {
    let (stdout, stderr, ok) = run(&["encode", "hello world"]);
    assert!(!ok);
    assert!(
        stderr.contains("template contains no @i placeholders"),
        "got: {stderr}"
    );
    assert!(
        !stderr.contains("me sysw pack"),
        "no referral without a recognized shape; got: {stderr}"
    );
    assert!(stdout.is_empty());
}
