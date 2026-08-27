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
    //
    // P3 §6c / D4 moved the grouped form off stdout and onto the stderr
    // engraving card. The DEFAULT is unchanged -- space/5 -- and this test
    // still pins it; only the stream it is read from moved.
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["encode", "wpkh(@0/<0;1>/*)"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let line = String::from_utf8(out.stderr)
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

/// P3 §6c — `--separator` is WHITESPACE-ONLY, and `hyphen` and `comma` are
/// refused rather than silently accepted.
///
/// This test replaces `encode_separator_hyphen`, which pinned `--separator
/// hyphen` at exit 0 with a hyphen-grouped card. The reason the option goes is
/// cross-tool: `mt`'s decoder strips whitespace and nothing else, so a
/// hyphen-grouped string is one `mt`'s own verbs refuse -- after the plates are
/// cut. A rule that is safe per-tool and unsafe across tools is exactly the one
/// an operator carries between tools.
///
/// BOTH retired keywords AND both retired literal chars are asserted. The
/// literals are the half a narrow fix misses: `parse_separator` accepted
/// `"hyphen"` and `"-"` through the same match arm, and removing only the
/// keyword would leave `--separator -` working.
#[test]
fn encode_refuses_the_retired_separators() {
    for arg in ["hyphen", "-", "comma", ","] {
        let out = Command::cargo_bin("md")
            .unwrap()
            .args(["encode", "wpkh(@0/<0;1>/*)", "--separator", arg])
            .output()
            .unwrap();
        let code = out.status.code();
        assert_eq!(
            code,
            Some(2),
            "--separator {arg:?} must be refused at clap-usage exit 2; got {code:?}"
        );
        let stderr = String::from_utf8(out.stderr).unwrap();
        // §6h: the remedy must be EXECUTABLE -- it names what replaced the
        // retired value, not merely that it is gone.
        assert!(
            stderr.contains("--separator space"),
            "the refusal must name what replaced it; got {stderr:?}"
        );
        assert!(
            stderr.contains("--group-size 0"),
            "and the other way to get an unbroken card; got {stderr:?}"
        );
    }
}

/// The control: whitespace still works, by keyword AND by literal, and still
/// shapes the card.
#[test]
fn encode_still_accepts_whitespace_separators() {
    for arg in ["space", " "] {
        let out = Command::cargo_bin("md")
            .unwrap()
            .args(["encode", "wpkh(@0/<0;1>/*)", "--separator", arg])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "--separator {arg:?} must still work: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(
            stderr.contains("md1yq pqqxq q8xtw hw4xw n4qh") && stderr.contains("separator: space"),
            "got {stderr:?}"
        );
    }
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
    let stderr = String::from_utf8_lossy(&out.stderr);
    let chunks: Vec<&str> = stdout.lines().filter(|l| l.starts_with("md1")).collect();
    assert!(
        chunks.len() > 1,
        "must be chunked, got {} line(s)",
        chunks.len()
    );
    // P3 §6a moved the header to STDERR; it is still emitted, and this test
    // still checks that, because a chunk set with no chunk-set-id anywhere is
    // a card whose pieces cannot be told apart from another card's.
    assert!(
        stderr.contains("chunk-set-id:"),
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
    let stderr = String::from_utf8_lossy(&out.stderr);
    // P3 §6a: the header is on stderr now. Chunking is still what is being
    // asserted -- the header is the only observable that distinguishes a
    // forced-chunk run of a SHORT policy from an ordinary single-string one.
    assert!(
        stderr.contains("chunk-set-id:"),
        "the flag must still force chunking"
    );
    assert!(
        !stdout.contains("chunk-set-id:"),
        "and it must not be on stdout"
    );
}

/// `md encode` refuses an `older()` that BIP-68 consensus will not enforce.
///
/// The codec stays lenient on purpose — rust-miniscript accepts these values
/// and `proptest_to_miniscript` pins that, because a codec must round-trip
/// whatever the descriptor layer accepts. The refusal belongs on the AUTHORING
/// surface: this command mints an artifact that gets engraved in metal and read
/// for years, and `older(210000)` locks for 13392 blocks while `older(65536)`
/// locks for nothing at all.
///
/// Same split `mnemonic-toolkit` specified — blocking gate when authoring,
/// advisory on intake.
#[test]
fn encode_refuses_a_relative_timelock_consensus_would_truncate() {
    const NUMS: &str = "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0";
    let policy =
        |n: u32| format!("tr({NUMS},{{pk(@0/<0;1>/*),and_v(v:older({n}),pk(@1/<0;1>/*))}})");
    let enc = |n: u32| {
        Command::cargo_bin("md")
            .unwrap()
            .args([
                "encode",
                &policy(n),
                "--path",
                "m/270028h/0h/0h/0h",
                "--group-size",
                "0",
            ])
            .output()
            .unwrap()
    };

    // Faithful: the whole 16-bit block range encodes.
    for n in [1u32, 32_768, 65_535] {
        assert!(
            enc(n).status.success(),
            "older({n}) is faithful and must encode"
        );
    }
    // Faithful time-based: bit 22 set, value still inside 16 bits.
    assert!(
        enc((1 << 22) | 1_000).status.success(),
        "a 512s-unit lock must encode"
    );

    // Truncating: refused, and the message says what consensus would do.
    for (n, enforced) in [(65_536u32, "0"), (210_000, "13392"), (420_000, "26784")] {
        let out = enc(n);
        assert!(!out.status.success(), "older({n}) must be refused");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(
            err.contains(enforced),
            "the refusal must name what consensus enforces ({enforced}): {err}"
        );
        assert!(
            err.contains("after()"),
            "the refusal must point at the fix (an absolute after()): {err}"
        );
    }
}

const NUMS_KEY: &str = "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0";

/// `--experimental` admits a spend path that requires no signature.
///
/// rust-miniscript refuses these by default ("All spend paths must require a
/// signature"), but that is a SAFETY POLICY, not a language rule — the script
/// is well-formed and valid, and the library ships `ExtParams` precisely so
/// individual rules can be relaxed. A hashlock-plus-timelock recovery tier is a
/// legitimate design; refusing to express it is a tooling limit, not a
/// consensus one.
#[test]
fn experimental_admits_a_keyless_spend_path() {
    let policy = format!(
        "tr({NUMS_KEY},{{pk(@0/<0;1>/*),and_v(v:after(1383520),sha256(\
         a84dce40975727c398023cfbd50d5db3b9662375521d0f1ac62dbd829b9a08ad))}})"
    );
    let run = |extra: &[&str]| {
        let mut args = vec![
            "encode",
            &policy,
            "--path",
            "m/270028h/0h/0h/0h",
            "--group-size",
            "0",
        ];
        args.extend_from_slice(extra);
        Command::cargo_bin("md")
            .unwrap()
            .args(&args)
            .output()
            .unwrap()
    };

    let refused = run(&[]);
    assert!(
        !refused.status.success(),
        "a keyless path must be refused by default"
    );
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains("must require a signature"),
        "the default refusal must name the signature rule"
    );

    let allowed = run(&["--experimental"]);
    assert!(
        allowed.status.success(),
        "--experimental must admit it: {}",
        String::from_utf8_lossy(&allowed.stderr)
    );
    assert!(
        String::from_utf8_lossy(&allowed.stdout)
            .lines()
            .any(|l| l.starts_with("md1")),
        "it must actually encode"
    );
    // Loud, every time — the card carries no record that a flag made it.
    let warn = String::from_utf8_lossy(&allowed.stderr);
    assert!(warn.contains("--experimental relaxed"), "must warn: {warn}");
    assert!(
        warn.contains("BEARER ACCESS"),
        "must name the engraving hazard: {warn}"
    );
}

/// `--experimental` relaxes ONLY the signature rule. The other four sanity
/// rules still apply, because relaxing those admits scripts that are
/// UNSPENDABLE rather than merely unguaranteed — a different, worse class.
#[test]
fn experimental_still_enforces_the_other_sanity_rules() {
    let cases = [
        (
            "timelock mixing",
            format!(
                "tr({NUMS_KEY},{{pk(@0/<0;1>/*),and_v(v:after(1000),\
                 and_v(v:after(1600000000),pk(@1/<0;1>/*)))}})"
            ),
            "heightlock and timelock",
        ),
        (
            "repeated keys",
            format!("tr({NUMS_KEY},{{pk(@0/<0;1>/*),and_v(v:pk(@1/<0;1>/*),pk(@1/<0;1>/*))}})"),
            "repeated pubkeys",
        ),
    ];
    for (label, policy, needle) in cases {
        let out = Command::cargo_bin("md")
            .unwrap()
            .args([
                "encode",
                &policy,
                "--path",
                "m/270028h/0h/0h/0h",
                "--group-size",
                "0",
                "--experimental",
            ])
            .output()
            .unwrap();
        assert!(
            !out.status.success(),
            "{label} must still be refused under --experimental"
        );
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(
            err.contains(needle),
            "{label}: refusal must name the rule, got: {err}"
        );
        assert!(
            err.contains("relaxes ONLY the signature rule"),
            "{label}: the refusal must state what --experimental does and does not do"
        );
    }
}

// ──────────────────────────────────────────────────────────────────────────
// P3 §6a — `encode`'s stdout is the canonical artifact and NOTHING else
// ──────────────────────────────────────────────────────────────────────────

/// The `chunk-set-id:` header is on STDERR. §6a rules `encode`'s stdout to be
/// the artifact alone, and the header is what made `md encode | me sysw pack`
/// need a `grep`.
///
/// BOTH halves are load-bearing and neither is redundant. The absence half
/// alone cannot tell "moved to stderr" from "deleted", and the existing control
/// `a_short_policy_still_emits_a_single_string` asserts an ABSENCE too — so an
/// implementation that dropped the chunk-set-id entirely would leave every
/// other test in this file green. The stderr assertion is the only thing in the
/// suite that would notice.
#[test]
fn the_chunk_set_id_header_is_on_stderr_not_stdout() {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["encode", "wpkh(@0/<0;1>/*)", "--force-chunked"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        !stdout.contains("chunk-set-id:"),
        "the header must not be on stdout; got {stdout:?}"
    );
    assert!(
        stderr.contains("chunk-set-id: 0x"),
        "the header must still be EMITTED, on stderr; got {stderr:?}"
    );
    // And nothing else moved onto stdout in its place.
    for l in stdout.lines() {
        assert!(
            l.starts_with("md1"),
            "encode stdout must be md1 lines only; got {l:?}"
        );
    }
}

/// The same rule on the AUTO-chunking path, which is the one a real keyed
/// wallet policy takes — `--force-chunked` is a flag a pipeline never passes.
#[test]
fn the_auto_chunked_header_is_on_stderr_too() {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(keyed_policy_args())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(!stdout.contains("chunk-set-id:"), "stdout: {stdout:?}");
    assert!(stderr.contains("chunk-set-id: 0x"), "stderr: {stderr:?}");
}

// ──────────────────────────────────────────────────────────────────────────
// P3 §6c / D4 — stdout is the UNBROKEN artifact; the grouped form moves to a
// stderr engraving card
// ──────────────────────────────────────────────────────────────────────────

/// The single-string case, asserted as byte-equality on stdout rather than as
/// "no space at position 5". Equality is what a pipeline sees.
#[test]
fn encode_stdout_is_unbroken_and_the_card_is_on_stderr() {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["encode", "wpkh(@0/<0;1>/*)"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert_eq!(
        stdout,
        format!("{WPKH_UNBROKEN}\n"),
        "stdout must be the canonical artifact, unbroken, and nothing else"
    );
    // The card: the grouped string first (it is the thing a human transcribes),
    // then the two flags that shaped it, then the existing advisory last.
    assert!(
        stderr.contains("md1yq pqqxq q8xtw hw4xw n4qh"),
        "the grouped form must survive, on the card; got {stderr:?}"
    );
    assert!(stderr.contains("group size: 5"), "got {stderr:?}");
    assert!(stderr.contains("separator: space"), "got {stderr:?}");
    assert!(
        stderr
            .trim_end()
            .ends_with("note: stdout is a keyless descriptor template (no keys)"),
        "the card ends with the existing output-class advisory; got {stderr:?}"
    );
}

/// `--group-size` and `--separator` shape the CARD and never stdout. A run with
/// non-default grouping must leave stdout byte-identical to the default run.
#[test]
fn grouping_flags_move_the_card_and_not_stdout() {
    let run = |extra: &[&str]| {
        let mut args = vec!["encode", "wpkh(@0/<0;1>/*)"];
        args.extend_from_slice(extra);
        let out = Command::cargo_bin("md")
            .unwrap()
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success());
        (
            String::from_utf8(out.stdout).unwrap(),
            String::from_utf8(out.stderr).unwrap(),
        )
    };
    let (base_out, _) = run(&[]);
    let (g4_out, g4_err) = run(&["--group-size", "4"]);
    let (g0_out, g0_err) = run(&["--group-size", "0"]);
    assert_eq!(base_out, g4_out, "--group-size must not reach stdout");
    assert_eq!(base_out, g0_out, "--group-size 0 must not reach stdout");
    assert!(
        g4_err.contains("md1y qpqq xqq8 xtwh w4xw n4qh") && g4_err.contains("group size: 4"),
        "the card must follow --group-size; got {g4_err:?}"
    );
    assert!(
        g0_err.contains("group size: 0") && g0_err.contains(WPKH_UNBROKEN),
        "group size 0 renders the card unbroken; got {g0_err:?}"
    );
}

/// The chunked case. Every stdout line is a bare md1 token carrying NO
/// whitespace — which is exactly what `me sysw pack` classifies a record by,
/// and the property whose absence made `md encode | me sysw pack` need a
/// `grep` plus a `--group-size 0`.
#[test]
fn chunked_stdout_is_bare_md1_tokens_and_the_card_carries_every_chunk() {
    let mut args = keyed_policy_args();
    // Drop the fixture's own `--group-size 0`, so this runs the DEFAULT.
    let g = args.iter().position(|a| a == "--group-size").unwrap();
    args.drain(g..g + 2);
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(lines.len() > 1, "expected a chunk set; got {lines:?}");
    for l in &lines {
        assert!(l.starts_with("md1"), "non-artifact line on stdout: {l:?}");
        assert!(
            !l.contains(char::is_whitespace),
            "stdout must be unbroken; got {l:?}"
        );
    }
    // The card carries the grouped form of EVERY chunk, not just the first.
    for l in &lines {
        let grouped: String = l
            .chars()
            .collect::<Vec<_>>()
            .chunks(5)
            .map(|c| c.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            stderr.contains(&grouped),
            "the card is missing a chunk: {grouped:?}\nstderr: {stderr}"
        );
    }
    assert!(stderr.contains("group size: 5") && stderr.contains("separator: space"));
}
