#![allow(missing_docs)]

use assert_cmd::Command;
use predicates::prelude::*;

/// Abandon-mnemonic tpub at m/84'/1'/0' (BIP 84 testnet account, depth 3).
/// Same value as `parse::keys::ABANDON_TPUB_DEPTH3_BIP84` in the bin crate
/// (integration tests can't reach pub(crate) items there).
const TPUB_FIXTURE: &str = "tpubDC8msFGeGuwnKG9Upg7DM2b4DaRqg3CUZa5g8v2SRQ6K4NSkxUgd7HsL2XVWbVm39yBA4LAxysQAm397zwQSQoQgewGiYZqrA9DsP4zbQ1M";

#[test]
fn encode_template_only_emits_a_phrase() {
    let mut cmd = Command::cargo_bin("md").unwrap();
    cmd.args(["encode", "wpkh(@0/<0;1>/*)"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("md1"));
}

#[test]
fn encode_with_policy_id_fingerprint_prints_two_lines() {
    let mut cmd = Command::cargo_bin("md").unwrap();
    cmd.args(["encode", "wpkh(@0/<0;1>/*)", "--policy-id-fingerprint"])
        .assert()
        .success()
        .stdout(predicate::str::contains("policy-id-fingerprint: 0x"));
}

#[test]
fn encode_json_has_schema_and_phrase() {
    Command::cargo_bin("md")
        .unwrap()
        .args(["encode", "wpkh(@0/<0;1>/*)", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"schema\": \"md-cli/1\""))
        .stdout(predicate::str::contains("\"phrase\":"));
}

#[cfg(feature = "cli-compiler")]
#[test]
fn encode_from_policy_segwitv0() {
    Command::cargo_bin("md")
        .unwrap()
        .args(["encode", "--from-policy", "pk(@0)", "--context", "segwitv0"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("md1"));
}

/// v0.17 — end-to-end encode for the 2-of-3 hardware-wallet multisig
/// pattern. compile auto-NUMS → walk_tr emits Tag::TrUnspendable →
/// md-codec encodes wire format. Asserts the md1 phrase prefix.
#[cfg(feature = "cli-compiler")]
#[test]
fn encode_from_policy_thresh_2_of_3_tap() {
    Command::cargo_bin("md")
        .unwrap()
        .args([
            "encode",
            "--from-policy",
            "thresh(2,pk(@0),pk(@1),pk(@2))",
            "--context",
            "tap",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("md1"));
}

/// v0.17 — end-to-end encode for the inheritance / timelock pattern.
/// Exercises Axis 1 walker arms (AndV, Verify, Older) through the
/// encode pipeline. Output is a Tag::Tr (extract wins; @0 is internal
/// key) with a single-leaf and_v body.
#[cfg(feature = "cli-compiler")]
#[test]
fn encode_from_policy_inheritance_tap() {
    Command::cargo_bin("md")
        .unwrap()
        .args([
            "encode",
            "--from-policy",
            "or(pk(@0),and(pk(@1),older(144)))",
            "--context",
            "tap",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("md1"));
}

// Round-trip integration test (encode → decode/inspect verifying Tag::TrUnspendable
// reassembles correctly) is deferred to a v0.17.1 follow-up. The blocker is
// unrelated to v0.17: md-cli's existing canonicity gate requires explicit origin
// paths for non-canonical wrappers, but `--from-policy` emits @N without
// derivation suffixes. A proper round-trip test needs `--key @0=<xpub>` arguments
// for all placeholders. Tracked in design/FOLLOWUPS.md as
// `v0.17.1-from-policy-round-trip-integration`.

#[test]
fn encode_json_network_field_default_mainnet() {
    Command::cargo_bin("md")
        .unwrap()
        .args(["encode", "wpkh(@0/<0;1>/*)", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"network\": \"mainnet\""));
}

#[test]
fn encode_json_network_field_testnet() {
    Command::cargo_bin("md")
        .unwrap()
        .args([
            "encode",
            "wpkh(@0/<0;1>/*)",
            "--network",
            "testnet",
            "--key",
            &format!("@0={TPUB_FIXTURE}"),
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"network\": \"testnet\""));
}

#[test]
fn encode_rejects_tpub_under_default_mainnet() {
    Command::cargo_bin("md")
        .unwrap()
        .args([
            "encode",
            "wpkh(@0/<0;1>/*)",
            "--key",
            &format!("@0={TPUB_FIXTURE}"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("expected mainnet"));
}

/// v0.18 Item J — `--path` flag now actually affects encode output. Pre-v0.18
/// the value was destructured as `path: _` at main.rs:218 and silently dropped.
#[cfg(feature = "cli-compiler")]
#[test]
fn encode_with_explicit_path_populates_path_decl() {
    use std::process::Command as StdCommand;

    let baseline = StdCommand::new(env!("CARGO_BIN_EXE_md"))
        .args([
            "encode",
            "--from-policy",
            "thresh(2,pk(@0),pk(@1),pk(@2))",
            "--context",
            "tap",
        ])
        .output()
        .expect("baseline encode");
    let baseline_phrase = String::from_utf8(baseline.stdout)
        .unwrap()
        .trim()
        .to_string();

    let with_path = StdCommand::new(env!("CARGO_BIN_EXE_md"))
        .args([
            "encode",
            "--from-policy",
            "thresh(2,pk(@0),pk(@1),pk(@2))",
            "--context",
            "tap",
            "--path",
            "48'/0'/0'/2'",
        ])
        .output()
        .expect("with-path encode");
    let with_path_phrase = String::from_utf8(with_path.stdout)
        .unwrap()
        .trim()
        .to_string();

    assert!(baseline_phrase.starts_with("md1"));
    assert!(with_path_phrase.starts_with("md1"));
    assert_ne!(
        baseline_phrase, with_path_phrase,
        "explicit --path must change the encoded phrase (was silently dropped pre-v0.18)"
    );
}

/// v0.18 Item J — named-path forms (`bip44|48|49|84|86`) resolve via parse_path
/// and produce the same wire output as the literal equivalent.
#[cfg(feature = "cli-compiler")]
#[test]
fn encode_with_named_path_bip48() {
    use std::process::Command as StdCommand;

    let named = StdCommand::new(env!("CARGO_BIN_EXE_md"))
        .args([
            "encode",
            "--from-policy",
            "thresh(2,pk(@0),pk(@1),pk(@2))",
            "--context",
            "tap",
            "--path",
            "bip48",
        ])
        .output()
        .expect("named-path encode");
    let named_phrase = String::from_utf8(named.stdout).unwrap().trim().to_string();

    let literal = StdCommand::new(env!("CARGO_BIN_EXE_md"))
        .args([
            "encode",
            "--from-policy",
            "thresh(2,pk(@0),pk(@1),pk(@2))",
            "--context",
            "tap",
            "--path",
            "48'/0'/0'/2'",
        ])
        .output()
        .expect("literal-path encode");
    let literal_phrase = String::from_utf8(literal.stdout)
        .unwrap()
        .trim()
        .to_string();

    assert!(named_phrase.starts_with("md1"));
    assert_eq!(
        named_phrase, literal_phrase,
        "`--path bip48` must resolve to `48'/0'/0'/2'` (parse_path::parse_path_name)"
    );
}

/// v0.18 Item J — explicit --path overrides the inferred canonical default.
/// Different explicit paths produce different phrases.
#[cfg(feature = "cli-compiler")]
#[test]
fn encode_path_overrides_canonical_default() {
    use std::process::Command as StdCommand;

    let path_a = StdCommand::new(env!("CARGO_BIN_EXE_md"))
        .args([
            "encode",
            "--from-policy",
            "thresh(2,pk(@0),pk(@1),pk(@2))",
            "--context",
            "tap",
            "--path",
            "48'/0'/0'/2'",
        ])
        .output()
        .expect("path-A encode");
    let phrase_a = String::from_utf8(path_a.stdout).unwrap().trim().to_string();

    let path_b = StdCommand::new(env!("CARGO_BIN_EXE_md"))
        .args([
            "encode",
            "--from-policy",
            "thresh(2,pk(@0),pk(@1),pk(@2))",
            "--context",
            "tap",
            "--path",
            "86'/0'/0'",
        ])
        .output()
        .expect("path-B encode");
    let phrase_b = String::from_utf8(path_b.stdout).unwrap().trim().to_string();

    assert!(phrase_a.starts_with("md1"));
    assert!(phrase_b.starts_with("md1"));
    assert_ne!(
        phrase_a, phrase_b,
        "different explicit --path values must produce different encoded phrases"
    );
}
