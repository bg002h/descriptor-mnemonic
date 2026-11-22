//! FIX_bh1119_md_card_readers (bughunt-1119 row #40, bh-md-I3): `md repair`
//! emits each corrected card in the letter case of its input string. Each test
//! is named exactly as its TRACE row (`design/FIX_bh1119_md_card_readers.md`
//! section 4).
//!
//! Fixture provenance: the chunked set `C0`/`C1_CLEAN` was minted on the base by
//! `md encode 'wsh(sortedmulti(2,@0/<0;1>/*,...,@11/<0;1>/*))' --path bip48
//! --fingerprint @0=00000001 ... --fingerprint @11=0000000c`, then uppercased;
//! `V12_*` is `cli_repair_unsupported_version.rs` `V12_CLEAN` uppercased.

#![allow(missing_docs)]

use std::process::Command;

const S_DAMAGED: &str = "MD12QPQQXQQ8XTWHW4XWN4QH";
const S_CLEAN: &str = "MD1YQPQQXQQ8XTWHW4XWN4QH";
const C0: &str = "MD1F0C9LQSPZ6TVYYY4QQXPPC2CPYDZK0ZDTPNKQQQQQQQZYQQQQQZYQQQQQPNQQQQQPZQCJG0YYNDM69P4";
const C1_CLEAN: &str = "MD1F0C9LQSGQQQQP2SQQQQQESQQQQQWUQQQQQGSQQQQQYEQQQQQZ4QQQQQPWCQQQQQCQ7H9LLL0W28FY4";
const V12_DAMAGED: &str = "MD12ZFDSSSJJTVYYW2FDSSJ54QQXPPCGSCU5E7M9JGAWLHG";
const V12_UPPER: &str = "MD1UZFDSSSJJTVYYW2FDSSJ54QQXPPCGSCU5E7M9JGAWLHG";

/// `C1_CLEAN` with data position 7 (`G`) replaced by `7`.
fn c1_damaged() -> String {
    let mut s: Vec<char> = C1_CLEAN.chars().collect();
    assert_eq!(s[3 + 7], 'G', "fixture moved");
    s[3 + 7] = '7';
    s.into_iter().collect()
}

fn md(args: &[&str]) -> (i32, String, String) {
    let o = Command::new(assert_cmd::cargo::cargo_bin("md"))
        .args(args)
        .output()
        .expect("invoke md");
    (
        o.status.code().expect("md exited normally"),
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

fn json(args: &[&str]) -> serde_json::Value {
    let (code, out, err) = md(args);
    assert_eq!(code, 5, "stderr={err}");
    serde_json::from_str(out.trim()).expect("stdout parses as JSON")
}

#[test]
fn i3_r1_repair_upper_digit_damage_text_emits_upper_card() {
    let (code, out, _) = md(&["repair", S_DAMAGED]);
    assert_eq!(code, 5);
    assert_eq!(
        out,
        format!("# Repair report\n#   md1 chunk 0: 1 correction at position 0: '2' -> 'y'\n{S_CLEAN}\n")
    );
    let (dcode, dout, _) = md(&["decode", S_CLEAN]);
    assert_eq!(dcode, 0);
    assert_eq!(dout.lines().next(), Some("wpkh(@0/<0;1>/*)"));
}

#[test]
fn i3_r2_repair_upper_digit_damage_json_emits_upper_card() {
    let v = json(&["repair", "--json", S_DAMAGED]);
    assert_eq!(v["corrected_chunks"], serde_json::json!([S_CLEAN]));
    let r = &v["repairs"][0];
    assert_eq!(r["corrected_chunk"], S_CLEAN);
    assert_eq!(r["original_chunk"], S_DAMAGED);
    let p = &r["corrected_positions"];
    assert_eq!(p.as_array().expect("array").len(), 1);
    assert_eq!(p[0]["position"], 0);
    assert_eq!(p[0]["was"], "2");
    assert_eq!(p[0]["now"], "y");
}

#[test]
fn i3_r3_repair_upper_chunked_digit_damage_text_emits_upper_set() {
    let (dc, _, de) = md(&["decode", C0, C1_CLEAN]);
    assert_eq!(dc, 0, "fixture sanity: {de}");
    let d = c1_damaged();
    let (code, out, _) = md(&["repair", C0, &d]);
    assert_eq!(code, 5);
    assert_eq!(
        out,
        format!("# Repair report\n#   md1 chunk 1: 1 correction at position 7: '7' -> 'g'\n{C0}\n{C1_CLEAN}\n")
    );
    assert_eq!(md(&["decode", C0, C1_CLEAN]).0, 0);
}

#[test]
fn i3_r4_repair_upper_chunked_digit_damage_json_emits_upper_set() {
    let d = c1_damaged();
    let v = json(&["repair", "--json", C0, &d]);
    assert_eq!(v["corrected_chunks"], serde_json::json!([C0, C1_CLEAN]));
    let reps = v["repairs"].as_array().expect("repairs array");
    assert_eq!(reps.len(), 1);
    assert_eq!(reps[0]["chunk_index"], 1);
    assert_eq!(reps[0]["corrected_chunk"], C1_CLEAN);
}

#[test]
fn i3_r5_repair_set_keeps_each_strings_case() {
    let c0_lower = C0.to_ascii_lowercase();
    let d = c1_damaged();
    let (code, out, _) = md(&["repair", &c0_lower, &d]);
    assert_eq!(code, 5);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(&lines[2..], [c0_lower.as_str(), C1_CLEAN]);
    assert_eq!(md(&["decode", &c0_lower, C1_CLEAN]).0, 0);
}

#[test]
fn i3_r6_repair_upper_unsupported_version_emits_upper_card() {
    let (code, out, err) = md(&["repair", V12_DAMAGED]);
    assert_eq!(code, 5);
    assert_eq!(
        out,
        format!("# Repair report\n#   md1 chunk 0: 1 correction at position 0: '2' -> 'u'\n{V12_UPPER}\n")
    );
    assert!(
        err.contains("md: repair: corrected, but this build cannot read wire version 12 (accepted: 4, 8)"),
        "stderr={err}"
    );
}

#[test]
fn i3_c1_repair_upper_letter_damage_unchanged() {
    let (code, out, _) = md(&["repair", "MD1ZQPQQXQQ8XTWHW4XWN4QH"]);
    assert_eq!(code, 5);
    assert_eq!(out.lines().nth(2), Some(S_CLEAN));
}

#[test]
fn i3_c2_repair_lower_digit_damage_unchanged() {
    let (code, out, _) = md(&["repair", "md12qpqqxqq8xtwhw4xwn4qh"]);
    assert_eq!(code, 5);
    assert_eq!(out.lines().nth(2), Some("md1yqpqqxqq8xtwhw4xwn4qh"));
}

#[test]
fn i3_c3_repair_mixed_case_input_refused_unchanged() {
    let (code, out, err) = md(&["repair", "Md12qpqqxqq8xtwhw4xwn4qh"]);
    assert_eq!(code, 2);
    assert_eq!(out, "");
    assert_eq!(
        err.trim_end(),
        "md: repair: codex32 decode error: chunk 0: string mixes upper and lower case (BIP-173 forbids mixed case)"
    );
}
