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

/// F-A3: `--force-long-code` is now a hard error (the long BCH code was
/// removed in v0.12.0). The flag stays in the clap surface but referencing it
/// exits non-zero with an explanatory message.
#[test]
fn encode_force_long_code_hard_errors() {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["encode", "wpkh(@0/<0;1>/*)", "--force-long-code"])
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "--force-long-code must exit non-zero"
    );
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("long BCH code was removed in v0.12.0"),
        "expected long-code-removed message; got: {stderr}"
    );
    // Nothing on stdout — the error fires before any card is emitted.
    assert!(
        out.stdout.is_empty(),
        "no stdout on hard error; got: {:?}",
        String::from_utf8_lossy(&out.stdout)
    );
}

/// F-A3: without the flag, encode is unchanged (emits the md1 card).
#[test]
fn encode_without_force_long_code_unchanged() {
    Command::cargo_bin("md")
        .unwrap()
        .args(["encode", "wpkh(@0/<0;1>/*)"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("md1"));
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

/// cycle-4 H6, restated for F-136: an over-cap payload must never become a
/// single string — it must become a valid CHUNK SET.
///
/// H6's property is FAIL-CLOSED: `wrap_payload` refuses an over-length payload
/// rather than emit an un-decodable, aliasing-prone single string. That
/// property is unchanged and still enforced in the codec. What changed is the
/// CLI's response to it: this used to be a hard error naming
/// `--force-chunked`, and now it chunks automatically (F-136).
///
/// The old test asserted the ERROR, which is the mechanism, not the guarantee.
/// This asserts the guarantee — no single over-cap string is ever emitted, and
/// what IS emitted decodes back — so it keeps holding across the change and
/// would still catch a regression that emitted one long string.
#[test]
fn md_encode_never_emits_an_oversize_single_string() {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args([
            "encode",
            "wpkh(@0/<0;1>/*)",
            "--network",
            "testnet",
            "--key",
            &format!("@0={TPUB_FIXTURE}"),
            "--group-size",
            "0",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "an over-cap policy now chunks rather than failing: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let chunks: Vec<&str> = stdout.lines().filter(|l| l.starts_with("md1")).collect();
    assert!(
        chunks.len() > 1,
        "over-cap output must be a chunk SET, never one long string; got {} line(s)",
        chunks.len()
    );
    // Every chunk is within the regular code's reach, which is the whole point
    // of chunking rather than emitting one over-length string.
    for c in &chunks {
        assert!(
            c.len() <= 93 + 4,
            "chunk of {} chars exceeds the codex32 regular code envelope: {c}",
            c.len()
        );
    }
    // And it decodes back — the fail-closed guarantee is about never emitting
    // something un-decodable, so read-back is the assertion that matters.
    let mut dec = vec!["decode".to_string()];
    dec.extend(chunks.iter().map(|c| (*c).to_string()));
    Command::cargo_bin("md")
        .unwrap()
        .args(&dec)
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

/// An 11-key xpub from the pathological-wallet corpus, enough to push a keyed
/// policy well past the single-string cap.
const KEYED_XPUB_A: &str = "xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf";
const KEYED_XPUB_B: &str = "xpub6DzhyrnFFYQ1HimDiM388xHnDiRPNdZJFBmmxge3Y1WWcHLtMJLfRuhRHqnQCPbTj3fGKTuKFLHzzwpJkp5Dtc3UtLKZKaVZe1yqMBXd6Vk";

fn keyed_policy_args() -> Vec<String> {
    vec![
        "encode".into(),
        "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))".into(),
        "--key".into(),
        format!("@0={KEYED_XPUB_A}"),
        "--key".into(),
        format!("@1={KEYED_XPUB_B}"),
        "--fingerprint".into(),
        "@0=73c5da0a".into(),
        "--fingerprint".into(),
        "@1=aabbccdd".into(),
        "--path".into(),
        "m/48h/0h/0h/2h".into(),
        "--group-size".into(),
        "0".into(),
    ]
}

/// A policy over the single-string cap CHUNKS AUTOMATICALLY (F-136).
///
/// It used to be a hard error telling the operator to retry with
/// `--force-chunked`. Every keyed wallet policy is over the cap — 246 data
/// symbols against a limit of 80 — so the first encounter with a real
/// multisig looked like the policy was unsupported. Two docs described the
/// dispatch as automatic while the encoder refused; the docs were right about
/// the intent and the code was what disagreed.
#[test]
fn a_policy_over_the_single_string_cap_chunks_without_the_flag() {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(keyed_policy_args())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "an over-cap policy must encode, not refuse: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let chunks: Vec<&str> = stdout.lines().filter(|l| l.starts_with("md1")).collect();
    assert!(
        chunks.len() > 1,
        "must be chunked, got {} line(s)",
        chunks.len()
    );
    assert!(
        stdout.contains("chunk-set-id:"),
        "auto-chunked output must carry the chunk-set-id header, like --force-chunked"
    );
}

/// Auto-chunking is byte-identical to what `--force-chunked` produced, so the
/// change is which INPUTS are accepted, never which bytes come out.
#[test]
fn auto_chunked_output_equals_force_chunked_output() {
    let auto = Command::cargo_bin("md")
        .unwrap()
        .args(keyed_policy_args())
        .output()
        .unwrap();
    let mut forced_args = keyed_policy_args();
    forced_args.push("--force-chunked".into());
    let forced = Command::cargo_bin("md")
        .unwrap()
        .args(forced_args)
        .output()
        .unwrap();
    assert!(auto.status.success() && forced.status.success());
    assert_eq!(
        String::from_utf8_lossy(&auto.stdout),
        String::from_utf8_lossy(&forced.stdout),
        "auto-chunking must emit exactly what --force-chunked emits"
    );
}

/// A SHORT policy still emits ONE string by default — auto-chunking triggers on
/// overflow only, and does not change the common case.
#[test]
fn a_short_policy_still_emits_a_single_string() {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["encode", "wpkh(@0/<0;1>/*)", "--group-size", "0"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        stdout.lines().filter(|l| l.starts_with("md1")).count(),
        1,
        "a short policy must stay a single string"
    );
    assert!(
        !stdout.contains("chunk-set-id:"),
        "no chunk header for a single string"
    );
}

/// `--force-chunked` keeps its documented meaning: chunk even a SHORT policy.
#[test]
fn force_chunked_still_chunks_a_short_policy() {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args([
            "encode",
            "wpkh(@0/<0;1>/*)",
            "--group-size",
            "0",
            "--force-chunked",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("chunk-set-id:"),
        "the flag must still force chunking"
    );
}
