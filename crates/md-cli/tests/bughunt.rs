#![allow(missing_docs)]
//! Bug-hunt triage rows (md-cli). Each test asserts the CORRECT behaviour, so
//! it FAILS on unmodified code when the reported bug is real.
//! Hunt report: design/agent-reports/bughunt-2026-10-31-md.md.

use assert_cmd::Command;

const XPUB_A: &str = "xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf";
const XPUB_B: &str = "xpub6DzhyrnFFYQ1HimDiM388xHnDiRPNdZJFBmmxge3Y1WWcHLtMJLfRuhRHqnQCPbTj3fGKTuKFLHzzwpJkp5Dtc3UtLKZKaVZe1yqMBXd6Vk";
const KEYED: &str = "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))";

fn md() -> Command {
    Command::cargo_bin("md").unwrap()
}

fn keyed_args(fp0: &str) -> Vec<String> {
    let a = format!("@0={XPUB_A}");
    let b = format!("@1={XPUB_B}");
    let f0 = format!("@0={fp0}");
    [
        "encode",
        KEYED,
        "--key",
        &a,
        "--key",
        &b,
        "--fingerprint",
        &f0,
        "--fingerprint",
        "@1=aabbccdd",
        "--path",
        "m/48h/0h/0h/2h",
        "--group-size",
        "0",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

fn keyed_cards() -> Vec<String> {
    let out = md().args(keyed_args("73c5da0a")).output().unwrap();
    assert!(out.status.success(), "fixture encode failed");
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect()
}

/// #1. `--json` must refuse a truncating relative timelock exactly as the text
/// path does (BIP-68 keeps only bits 0-15 and 22).
#[cfg(feature = "json")]
#[test]
fn bughunt_md_01_json_encode_refuses_truncated_timelock() {
    let t = "wsh(and_v(v:pk(@0/<0;1>/*),older(65536)))";
    let text = md().args(["encode", t]).output().unwrap();
    assert!(!text.status.success(), "control: text path must refuse");
    let json = md().args(["encode", "--json", t]).output().unwrap();
    assert!(
        !json.status.success(),
        "--json exited 0 for older(65536); stdout={}",
        String::from_utf8_lossy(&json.stdout)
    );
}

/// #3. `--json` must chunk a policy too large for one string, as the text path.
#[cfg(feature = "json")]
#[test]
fn bughunt_md_03_json_encode_chunks_large_policy() {
    let mut args = keyed_args("73c5da0a");
    args.insert(1, "--json".into());
    let out = md().args(&args).output().unwrap();
    assert!(
        out.status.success(),
        "--json failed where the text path chunks: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// #4. A multipath alternative >= 2^31 is hardened (BIP-32) and must be refused
/// AT THE LEXER with a hardened-derivation message, not by a downstream varint
/// limit. Fails closed today, so only the diagnosis is wrong.
#[test]
fn bughunt_md_04_multipath_hardened_range_refused_as_hardened() {
    let out = md()
        .args(["encode", "wsh(multi(2,@0/<2147483648;1>/*,@1/<0;1>/*))"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("hardened"),
        "refused, but not as hardened: {err}"
    );
}

/// #10. stdout carries only the artifact (SPEC P3 6a): every stdout line is a
/// card. OPEN PRODUCT QUESTION: `encode_with_policy_id_fingerprint_prints_two_lines`
/// (tests/cmd_encode.rs:153) pins the line on stdout today.
#[test]
fn bughunt_md_10_policy_id_fingerprint_not_on_stdout() {
    let out = md()
        .args(["encode", "wpkh(@0/<0;1>/*)", "--policy-id-fingerprint"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    for l in stdout.lines() {
        assert!(l.starts_with("md1"), "non-artifact line on stdout: {l:?}");
    }
}

/// #11. `inspect --json` of a keyed (wallet-policy) card must not call stdout a
/// keyless template.
#[cfg(feature = "json")]
#[test]
fn bughunt_md_11_inspect_json_keyed_card_not_called_keyless() {
    let cards = keyed_cards();
    let mut args = vec!["inspect".to_string(), "--json".to_string()];
    args.extend(cards);
    let out = md().args(&args).output().unwrap();
    assert!(out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!err.contains("keyless"), "keyed card called keyless: {err}");
}

/// #12. A sign character is not a hex digit: `+f+f+f+f` must be refused.
#[test]
fn bughunt_md_12_fingerprint_with_plus_signs_refused() {
    let out = md().args(keyed_args("+f+f+f+f")).output().unwrap();
    assert!(
        !out.status.success(),
        "`+f+f+f+f` accepted; stdout={}",
        String::from_utf8_lossy(&out.stdout)
    );
}

/// #14. A single-key card at the canonical origin gets no origins note.
/// OPEN PRODUCT QUESTION: what is "canonical" (BIP-84 path for wpkh assumed).
#[test]
fn bughunt_md_14_decode_single_key_canonical_card_no_origins_note() {
    let enc = md()
        .args(["encode", "wpkh(@0/<0;1>/*)", "--path", "m/84h/0h/0h"])
        .output()
        .unwrap();
    assert!(enc.status.success());
    let card = String::from_utf8(enc.stdout).unwrap().trim().to_string();
    let out = md().args(["decode", &card]).output().unwrap();
    assert!(out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!err.contains("key origins carried"), "note printed: {err}");
}
