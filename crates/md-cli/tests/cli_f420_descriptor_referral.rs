//! F-420 (mnemonic-engrave FOLLOWUPS, journey walk W2): `md encode` on a
//! concrete descriptor or a BlueWallet `Key: value` export used to dead-end
//! with only "template contains no @i placeholders" — no recognition, no
//! referral, from the tool named for descriptors. The refusal names what the
//! input is and where to take it. These run the real binary because the walk
//! measured the real binary.
//!
//! **The two arms now refer to two different tools**
//! (REVIEW-converter-whole-diff-r1 I3). When F-420 shipped, md could not read
//! a descriptor and both arms pointed at `me sysw pack` in a sibling repo. The
//! wallet-form-converter cycle shipped `md decompose`, so md IS the tool that
//! takes a concrete descriptor and sending its holder to another binary is now
//! a false record at the exact moment they need the feature. The BlueWallet
//! arm is unchanged and still correct: `decompose` takes a descriptor, not a
//! `Key: value` export.

#![allow(missing_docs)]

use assert_cmd::Command;

/// The BlueWallet arm's referral — a `Key: value` export is still `me`'s job.
const REFERRAL: &str = "me sysw pack --as <descriptor|md1>";

/// The concrete-descriptor arm's referral — this repo, this binary.
const DECOMPOSE_REFERRAL: &str = "md decompose";

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
fn concrete_descriptor_is_refused_and_sent_to_md_decompose() {
    let template = format!("wpkh([4bbaa801/84'/0'/0']{XPUB}/<0;1>/*)");
    let (stdout, stderr, ok) = run(&["encode", &template]);
    assert!(!ok, "a concrete descriptor must refuse");
    assert!(
        stderr.contains(DECOMPOSE_REFERRAL),
        "expected the referral to name md decompose; got: {stderr}"
    );
    assert!(
        !stderr.contains("me sysw pack"),
        "md reads descriptors itself now — this must not send the operator to \
         another binary in another repo; got: {stderr}"
    );
    assert!(
        stderr.contains("concrete wallet descriptor"),
        "expected the input named; got: {stderr}"
    );
    assert!(stdout.is_empty(), "refusals write nothing to stdout");
}

/// NON-VACUOUS: the command the referral prints must actually work on the
/// input that drew it. A referral naming a verb that refuses this descriptor
/// would be a worse dead-end than the terse refusal it replaced.
#[test]
fn the_referred_command_actually_reads_that_descriptor() {
    let descriptor = format!("wpkh([4bbaa801/84'/0'/0']{XPUB}/<0;1>/*)");
    let (stdout, stderr, ok) = run(&["decompose", &descriptor, "--emit", "template"]);
    assert!(
        ok,
        "md decompose refused the input it was referred: {stderr}"
    );
    assert!(
        stdout.contains("@0"),
        "the referred command must yield the @i template md encode wants; got: {stdout}"
    );
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
