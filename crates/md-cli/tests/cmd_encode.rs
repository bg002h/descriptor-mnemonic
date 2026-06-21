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

/// The canonical unbroken md1 for `wpkh(@0/<0;1>/*)` (wire canary; same value
/// pinned in `smoke.rs`). Grouping is a display layer over this string.
const WPKH_UNBROKEN: &str = "md1yqpqqxqq8xtwhw4xwn4qh";

#[test]
fn encode_default_groups_space_5() {
    // mstring display-grouping (SPEC §3): default = space/5, single line.
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["encode", "wpkh(@0/<0;1>/*)"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let line = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .to_string();
    assert_eq!(
        line.chars().nth(5),
        Some(' '),
        "expected a space after the first 5 chars; got {line:?}"
    );
    let unbroken: String = line.chars().filter(|c| *c != ' ').collect();
    assert_eq!(
        unbroken, WPKH_UNBROKEN,
        "space-stripped grouped form must equal the canonical md1"
    );
}

#[test]
fn encode_unbroken_group_size_0() {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["encode", "wpkh(@0/<0;1>/*)", "--group-size", "0"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let line = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .to_string();
    assert!(
        !line.contains(' ') && !line.contains('-') && !line.contains(','),
        "--group-size 0 must be unbroken; got {line:?}"
    );
    assert_eq!(line, WPKH_UNBROKEN);
}

#[test]
fn encode_separator_hyphen() {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["encode", "wpkh(@0/<0;1>/*)", "--separator", "hyphen"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let line = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .to_string();
    assert_eq!(
        line.chars().nth(5),
        Some('-'),
        "expected a hyphen after the first 5 chars; got {line:?}"
    );
}

#[test]
fn encode_rejects_bad_separator() {
    Command::cargo_bin("md")
        .unwrap()
        .args(["encode", "wpkh(@0/<0;1>/*)", "--separator", "bogus"])
        .assert()
        .code(2);
}

#[test]
fn encode_with_policy_id_fingerprint_prints_two_lines() {
    let mut cmd = Command::cargo_bin("md").unwrap();
    cmd.args(["encode", "wpkh(@0/<0;1>/*)", "--policy-id-fingerprint"])
        .assert()
        .success()
        .stdout(predicate::str::contains("policy-id-fingerprint: 0x"));
}

#[cfg(feature = "json")]
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

/// v0.20 — `--path` against a raw template (no `--from-policy`). The Phase 1
/// `--path` tests are all `#[cfg(feature = "cli-compiler")]` and exercise
/// `--from-policy`, so this path was previously unpinned in CI without the
/// feature flag. Asserts the override produces a different phrase than the
/// no-path baseline. Closes followup `v0.18-phase-1-low-2-cli-path-non-from-policy-test-gate`.
#[test]
fn encode_with_explicit_path_raw_template_differs_from_baseline() {
    let baseline = Command::cargo_bin("md")
        .unwrap()
        .args(["encode", "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))"])
        .output()
        .unwrap();
    assert!(baseline.status.success(), "baseline encode failed");
    let baseline_phrase = String::from_utf8(baseline.stdout)
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .to_string();

    let with_path = Command::cargo_bin("md")
        .unwrap()
        .args([
            "encode",
            "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))",
            "--path",
            "84'/0'/0'",
        ])
        .output()
        .unwrap();
    assert!(with_path.status.success(), "--path encode failed");
    let with_path_phrase = String::from_utf8(with_path.stdout)
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .to_string();

    assert!(baseline_phrase.starts_with("md1"));
    assert!(with_path_phrase.starts_with("md1"));
    assert_ne!(
        baseline_phrase, with_path_phrase,
        "expected --path override to change the encoded phrase"
    );
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

#[cfg(feature = "json")]
#[test]
fn encode_json_network_field_default_mainnet() {
    Command::cargo_bin("md")
        .unwrap()
        .args(["encode", "wpkh(@0/<0;1>/*)", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"network\": \"mainnet\""));
}

#[cfg(feature = "json")]
#[test]
fn encode_json_network_field_testnet() {
    // cycle-4 H6: a keyed wallet-policy descriptor (65-byte xpub TLV) exceeds
    // the 80-data-symbol single-string cap → use the chunked path; the `network`
    // JSON field (the assertion under test) is emitted in both forms.
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
            "--force-chunked",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"network\": \"testnet\""));
}

/// cycle-4 H6: a keyed wallet-policy descriptor overflows the codex32 regular
/// code's 80-data-symbol single-string cap, so the default (non-chunked)
/// `md encode` fails closed (non-zero exit) and directs the user to chunked
/// encoding; `--force-chunked` is the live remedy and succeeds.
#[test]
fn md_encode_default_rejects_oversize() {
    // Default single-string path → reject, message names `--force-chunked`.
    Command::cargo_bin("md")
        .unwrap()
        .args([
            "encode",
            "wpkh(@0/<0;1>/*)",
            "--network",
            "testnet",
            "--key",
            &format!("@0={TPUB_FIXTURE}"),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--force-chunked"));

    // The chunked remedy succeeds.
    Command::cargo_bin("md")
        .unwrap()
        .args([
            "encode",
            "wpkh(@0/<0;1>/*)",
            "--network",
            "testnet",
            "--key",
            &format!("@0={TPUB_FIXTURE}"),
            "--force-chunked",
        ])
        .assert()
        .success();
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

/// v0.18 Phase 5 — Item F end-to-end round-trip for the 2-of-3 hardware-
/// wallet multisig pattern (the headline use case). Encodes via
/// `--from-policy` with an explicit `--path` (Phase 1's --path fix is the
/// enabler — without it, the canonicity gate rejects the descriptor on
/// decode). Decodes the resulting phrase and asserts the rendered
/// template contains the NUMS hex (Phase 3's sentinel rule rendered as
/// the literal x-only key) and the multi_a body. Resolves the
/// `v0.17.1-from-policy-round-trip-integration` carryover FOLLOWUP.
#[cfg(feature = "cli-compiler")]
#[test]
fn encode_decode_roundtrip_thresh_2_of_3_tap_with_explicit_path() {
    use std::process::Command as StdCommand;

    let encode_out = StdCommand::new(env!("CARGO_BIN_EXE_md"))
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
        .expect("encode");
    let phrase = String::from_utf8(encode_out.stdout)
        .unwrap()
        .trim()
        .to_string();
    assert!(
        phrase.starts_with("md1"),
        "encode must produce an md1 phrase, got: {phrase}"
    );

    let decode_out = StdCommand::new(env!("CARGO_BIN_EXE_md"))
        .args(["decode", &phrase])
        .output()
        .expect("decode");
    let template = String::from_utf8(decode_out.stdout)
        .unwrap()
        .trim()
        .to_string();

    assert!(
        template.contains("50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0"),
        "decoded template must include NUMS hex (Tag::Tr+sentinel rendered \
         as tr(<NUMS>, ...)). Got: {template}"
    );
    assert!(
        template.contains("multi_a(2,@0"),
        "decoded template must include multi_a(2,@0... body. Got: {template}"
    );
}

/// v0.18 Phase 5 — Item F end-to-end round-trip for the inheritance/
/// timelock pattern. Exercises Phase 4a walker arms (AndV + Verify +
/// Older) through the full encode → decode pipeline.
#[cfg(feature = "cli-compiler")]
#[test]
fn encode_decode_roundtrip_inheritance_pattern_with_explicit_path() {
    use std::process::Command as StdCommand;

    let encode_out = StdCommand::new(env!("CARGO_BIN_EXE_md"))
        .args([
            "encode",
            "--from-policy",
            "or(pk(@0),and(pk(@1),older(144)))",
            "--context",
            "tap",
            "--path",
            "86'/0'/0'",
        ])
        .output()
        .expect("encode");
    let phrase = String::from_utf8(encode_out.stdout)
        .unwrap()
        .trim()
        .to_string();
    assert!(phrase.starts_with("md1"));

    let decode_out = StdCommand::new(env!("CARGO_BIN_EXE_md"))
        .args(["decode", &phrase])
        .output()
        .expect("decode");
    let template = String::from_utf8(decode_out.stdout)
        .unwrap()
        .trim()
        .to_string();

    // Inheritance pattern: tr(@0, and_v(v:pk(@1), older(144)))
    // - tr() with extracted @0 as internal key (miniscript prefers extraction
    //   over the auto-NUMS fallback).
    // - and_v with verify-wrapped pk and older timelock.
    assert!(
        template.starts_with("tr(@0"),
        "decoded must start with tr(@0 (extracted @0 internal key). Got: {template}"
    );
    assert!(
        template.contains("and_v(v:pk(@1"),
        "decoded must include and_v(v:pk(@1 body. Got: {template}"
    );
    assert!(
        template.contains("older(144)"),
        "decoded must include older(144). Got: {template}"
    );
}
