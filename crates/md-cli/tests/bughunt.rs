#![allow(missing_docs)]
//! Bug-hunt triage rows (md-cli). Each test asserts the CORRECT behaviour, so
//! it FAILS on unmodified code when the reported bug is real.
//! Hunt report: design/agent-reports/bughunt-2026-10-31-md.md.

use assert_cmd::Command;

const XPUB_A: &str = "xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf";
const XPUB_B: &str = "xpub6DzhyrnFFYQ1HimDiM388xHnDiRPNdZJFBmmxge3Y1WWcHLtMJLfRuhRHqnQCPbTj3fGKTuKFLHzzwpJkp5Dtc3UtLKZKaVZe1yqMBXd6Vk";
const KEYED: &str = "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))";
/// R-4 expected stderr for a `<2147483648;1>` alternative on `@0`.
const HARDENED_2_31: &str = "md: template parse error: @0 multipath alt `2147483648` is hardened; \
hardened derivation is impossible on a watch-only (xpub) card\n";
const WATCH_ONLY_NOTE: &str = "note: stdout is watch-only — public keys only, cannot spend";
const KEYLESS_NOTE: &str = "note: stdout is a keyless descriptor template (no keys)";

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
#[ignore = "owed to FIX-A (md bug hunt row 01)"]
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
    // R-1: stderr and exit code byte-identical to the text path; no stdout.
    assert_eq!(json.status.code(), Some(1));
    assert_eq!(json.status.code(), text.status.code());
    assert_eq!(json.stderr, text.stderr);
    assert!(json.stdout.is_empty());
}

/// #3. `--json` must chunk a policy too large for one string, as the text path.
#[cfg(feature = "json")]
#[test]
#[ignore = "owed to FIX-A (md bug hunt row 03)"]
fn bughunt_md_03_json_encode_chunks_large_policy() {
    let mut args = keyed_args("73c5da0a");
    args.insert(1, "--json".into());
    let out = md().args(&args).output().unwrap();
    assert!(
        out.status.success(),
        "--json failed where the text path chunks: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    // R-3: the `--force-chunked` JSON shape, chunks identical to the text path.
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let expected = serde_json::json!({
        "chunk_set_id": "0x95075",
        "chunks": keyed_cards(),
        "network": "mainnet",
        "schema": "md-cli/2",
    });
    assert_eq!(v, expected);
}

/// #4. A multipath alternative >= 2^31 is hardened (BIP-32) and must be refused
/// AT THE LEXER with a hardened-derivation message, not by a downstream varint
/// limit. Fails closed today, so only the diagnosis is wrong.
#[test]
#[ignore = "owed to FIX-A (md bug hunt row 04)"]
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
    // R-4: the existing hardened-alt text, exit 1.
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(err, HARDENED_2_31);
    // Architect ruling Q3: 2^32 is not a child index at all, so it keeps
    // today's "not a bare unsigned integer" text (pins against a u64 lexer).
    let big = md()
        .args(["encode", "wpkh(@0/<4294967296;1>/*)"])
        .output()
        .unwrap();
    assert_eq!(big.status.code(), Some(1));
    assert!(big.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&big.stderr),
        "md: template parse error: @0 multipath alt `4294967296` is not a bare unsigned integer\n"
    );
}

/// #4 (R-4), `md address`: the shared lexer refuses alt 2^31 as hardened.
/// Today it derives `bc1q0qgl7sdpw0vvdn3g0kha052hkjugcm8zujqhea` and exits 0.
#[test]
fn bughunt_md_04b_address_multipath_hardened_range_refused_as_hardened() {
    let out = md()
        .args([
            "address",
            "--template",
            "wpkh(@0/<2147483648;1>/*)",
            "--key",
            &format!("@0={XPUB_A}"),
            "--count",
            "1",
        ])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(1),
        "stdout={}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(out.stdout.is_empty());
    assert_eq!(String::from_utf8_lossy(&out.stderr), HARDENED_2_31);
}

/// #4 (R-4), `md descriptor`: the shared lexer refuses alt 2^31 as hardened.
/// Today it prints `wpkh(<xpub>/<2147483648;1>/*)#26hvecv9` and exits 0.
#[test]
fn bughunt_md_04c_descriptor_multipath_hardened_range_refused_as_hardened() {
    let out = md()
        .args([
            "descriptor",
            "--template",
            "wpkh(@0/<2147483648;1>/*)",
            "--key",
            &format!("@0={XPUB_A}"),
        ])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(1),
        "stdout={}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(out.stdout.is_empty());
    assert_eq!(String::from_utf8_lossy(&out.stderr), HARDENED_2_31);
}

const VARINT_2_29: &str = "md: codec error: varint value 536870912 exceeds single-extension range \
(max 2^29 - 1)\n";

/// #4 (R-4, architect ruling Q1), `md address`: an alt in [2^29, 2^31) passes
/// the lexer and the codec refuses it with VarintOverflow. Control: 2^29 - 1
/// derives. Today alt 536870912 derives `bc1qghsy27425xapuv6gltah6uf2q9w5usrjucr5uy`
/// and exits 0. GREEN on FIX-B's head.
#[test]
fn bughunt_md_04d_address_alt_above_2_29_refused_as_varint_overflow() {
    let run = |alt: &str| {
        md().args([
            "address",
            "--template",
            &format!("wpkh(@0/<{alt};1>/*)"),
            "--key",
            &format!("@0={XPUB_A}"),
            "--count",
            "1",
        ])
        .output()
        .unwrap()
    };
    let out = run("536870912");
    assert_eq!(
        out.status.code(),
        Some(1),
        "stdout={}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(out.stdout.is_empty());
    assert_eq!(String::from_utf8_lossy(&out.stderr), VARINT_2_29);
    let ctl = run("536870911");
    assert_eq!(ctl.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&ctl.stdout),
        "bc1qd4366caaldhw2sx6970x8zqn5f9s8yzjm84kt2\n"
    );
}

/// #4 (R-4, architect ruling Q1), `md descriptor`: as 04d. Control: 2^29 - 1
/// prints a descriptor ending `/<536870911;1>/*)#rjkyn63n`. Today alt
/// 536870912 prints a descriptor ending `#akxwxgma` and exits 0.
#[test]
fn bughunt_md_04e_descriptor_alt_above_2_29_refused_as_varint_overflow() {
    let run = |alt: &str| {
        md().args([
            "descriptor",
            "--template",
            &format!("wpkh(@0/<{alt};1>/*)"),
            "--key",
            &format!("@0={XPUB_A}"),
        ])
        .output()
        .unwrap()
    };
    let out = run("536870912");
    assert_eq!(
        out.status.code(),
        Some(1),
        "stdout={}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(out.stdout.is_empty());
    assert_eq!(String::from_utf8_lossy(&out.stderr), VARINT_2_29);
    let ctl = run("536870911");
    assert_eq!(ctl.status.code(), Some(0));
    assert!(
        String::from_utf8_lossy(&ctl.stdout).ends_with("/<536870911;1>/*)#rjkyn63n\n"),
        "stdout={}",
        String::from_utf8_lossy(&ctl.stdout)
    );
}

/// #10. stdout carries only the artifact (SPEC P3 6a): every stdout line is a
/// card. OPEN PRODUCT QUESTION: `encode_with_policy_id_fingerprint_prints_two_lines`
/// (tests/cmd_encode.rs:153) pins the line on stdout today.
#[test]
#[ignore = "owed to FIX-A (md bug hunt row 10)"]
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
    // R-10: the card alone on stdout; the unchanged line moves to stderr.
    assert_eq!(stdout, "md1yqpqqxqq8xtwhw4xwn4qh\n");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.lines()
            .any(|l| l == "policy-id-fingerprint: 0x3ace1082"),
        "fingerprint line not on stderr: {err}"
    );
}

/// #11. `inspect --json` of a keyed (wallet-policy) card must not call stdout a
/// keyless template.
#[cfg(feature = "json")]
#[test]
#[ignore = "owed to FIX-A (md bug hunt row 11)"]
fn bughunt_md_11_inspect_json_keyed_card_not_called_keyless() {
    let cards = keyed_cards();
    let mut args = vec!["inspect".to_string(), "--json".to_string()];
    args.extend(cards);
    let out = md().args(&args).output().unwrap();
    assert!(out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!err.contains("keyless"), "keyed card called keyless: {err}");
    // R-11: encode's WatchOnly note instead.
    assert!(err.lines().any(|l| l == WATCH_ONLY_NOTE), "{err}");
}

/// #11 (R-11), text path: `md inspect` of a keyed card must not call stdout a
/// keyless template either; it prints encode's WatchOnly note.
#[test]
fn bughunt_md_11b_inspect_text_keyed_card_not_called_keyless() {
    let cards = keyed_cards();
    let mut args = vec!["inspect".to_string()];
    args.extend(cards);
    let out = md().args(&args).output().unwrap();
    assert!(out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!err.contains("keyless"), "keyed card called keyless: {err}");
    assert!(err.lines().any(|l| l == WATCH_ONLY_NOTE), "{err}");
}

/// #12. A sign character is not a hex digit: `+f+f+f+f` must be refused.
#[test]
#[ignore = "owed to FIX-A (md bug hunt row 12)"]
fn bughunt_md_12_fingerprint_with_plus_signs_refused() {
    let out = md().args(keyed_args("+f+f+f+f")).output().unwrap();
    assert!(
        !out.status.success(),
        "`+f+f+f+f` accepted; stdout={}",
        String::from_utf8_lossy(&out.stdout)
    );
    // R-12: the existing per-byte wording, exit 1.
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&out.stderr),
        "md: --fingerprint @0: invalid hex byte: +f\n"
    );
}

/// #14. A single-key card at the canonical origin gets no origins note.
/// OPEN PRODUCT QUESTION: what is "canonical" (BIP-84 path for wpkh assumed).
#[test]
#[ignore = "owed to FIX-A (md bug hunt row 14)"]
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
    // R-14: the note and the `--path` hint are both suppressed; stdout and the
    // output-class advisory are unchanged.
    assert_eq!(String::from_utf8_lossy(&out.stdout), "wpkh(@0/<0;1>/*)\n");
    assert_eq!(err, format!("{KEYLESS_NOTE}\n"));
}
