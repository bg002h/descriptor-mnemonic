//! F-449 stage 2 Task 2c (SPEC §8.9's stage-2 row): `md repair` KEEPS a
//! correction on a card whose wire version this build does not support, and
//! reports it distinctly from the atomic-fail exit -- exit 5
//! (REPAIR_APPLIED), never 2, and never 0.
//!
//! Before this task (measured at 25acb33c) the v12 card below exited 2 with
//! nothing on stdout: the correction was computed and discarded.
//!
//! Fixtures mirror md-codec's `tests/correct_chunks.rs`, where the recipe is
//! written down.

#![allow(missing_docs)]

use std::process::Command as StdCommand;

fn md(args: &[&str]) -> (String, String, i32) {
    let out = StdCommand::new(assert_cmd::cargo::cargo_bin("md"))
        .args(args)
        .output()
        .expect("invoke md");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().expect("md exited normally"),
    )
}

/// Version 12, one correctable error at data position 0.
const V12_ONE_ERROR: &str = "md1qzfdsssjjtvyyw2fdssj54qqxppcgscu5e7m9jgawlhg";
/// The same card, corrected: BCH-valid, zero errors, version 12.
const V12_CLEAN: &str = "md1uzfdsssjjtvyyw2fdssj54qqxppcgscu5e7m9jgawlhg";
/// Version 4, the same payload, five errors: beyond t = 4.
fn v4_uncorrectable() -> String {
    let mut s: Vec<char> = "md1qzfdsssjjtvyyw2fdssj54qqxppcgsc276kwwfnzntuh"
        .chars()
        .collect();
    for (i, pos) in [5usize, 9, 14, 20, 27].iter().enumerate() {
        s[*pos] = if s[*pos] == 'q' {
            'p'
        } else {
            ['q', 'z', 'r', 'y', 'x'][i]
        };
    }
    s.into_iter().collect()
}
/// A `md-codec-v0.16.2` string (pre-v0.30: same HRP and BCH constant, header
/// reads as version 0), as the plan cites it (R4 M-1). MEASURED: it is
/// BCH-CLEAN -- `correct_chunks` returns zero corrections -- not "one error
/// at data position 7" as the plan says. So it is the clean-legacy control
/// here, and `legacy_one_error` injects the error the plan meant.
const LEGACY_CLEAN: &str = "md1qppqqxzxpp29gtcfh4dhmh72l6atuttfxe3cw2xenm";
/// `LEGACY_CLEAN` with one substitution at data position 7 (`x` -> `q`).
fn legacy_one_error() -> String {
    let mut s: Vec<char> = LEGACY_CLEAN.chars().collect();
    assert_eq!(s[3 + 7], 'x', "fixture moved");
    s[3 + 7] = 'q';
    s.into_iter().collect()
}

#[test]
fn a_correction_on_an_unsupported_version_is_kept_and_exits_5() {
    let (out, err, code) = md(&["repair", V12_ONE_ERROR]);
    // 1. stdout carries the corrected string.
    assert!(
        out.lines().any(|l| l == V12_CLEAN),
        "the corrected card must reach stdout: {out}"
    );
    assert!(out.contains("position 0: 'q' -> 'u'"), "the report: {out}");
    // 2. exit 5 (REPAIR_APPLIED), distinct from the atomic-fail 2.
    assert_eq!(code, 5, "stderr: {err}");
    // 3. stderr names the version AND the accepted set.
    assert!(
        err.contains("wire version 12") && err.contains("accepted: 4, 8"),
        "{err}"
    );
    assert!(err.contains("newer md"), "v12 is even and above 8: {err}");
    // No output-class advisory: there is no descriptor to classify (R3 M-4).
    assert!(!err.contains("note: stdout is"), "{err}");
}

#[test]
fn the_json_report_keeps_the_d27_shape() {
    let (out, err, code) = md(&["repair", "--json", V12_ONE_ERROR]);
    assert_eq!(code, 5, "{err}");
    let v: serde_json::Value =
        serde_json::from_str(&out).unwrap_or_else(|e| panic!("not JSON ({e}): {out}"));
    assert_eq!(v["corrected_chunks"][0], V12_CLEAN, "{v}");
    assert_eq!(v["repairs"].as_array().map(Vec::len), Some(1), "{v}");
}

#[test]
fn an_uncorrectable_v4_control_still_exits_2_with_empty_stdout() {
    // 4.
    let bad = v4_uncorrectable();
    let (out, err, code) = md(&["repair", &bad]);
    assert_eq!(code, 2, "{err}");
    assert!(
        out.is_empty(),
        "D28: nothing on stdout in the atomic-fail case: {out}"
    );
}

#[test]
fn a_clean_v12_card_exits_2_with_empty_stdout() {
    // 5. The control for a reused `any_correction ? 5 : 0` (R3 M-6): a card
    // with nothing to correct and a version this build cannot read is NOT
    // "already valid".
    let (out, err, code) = md(&["repair", V12_CLEAN]);
    assert_eq!(code, 2, "{err}");
    assert!(out.is_empty(), "{out}");
    assert!(err.contains("got 12"), "{err}");
}

#[test]
fn a_legacy_card_is_corrected_without_the_newer_md_advice() {
    // 6. (R4 M-1): pre-v0.30 cards pass BCH and reach this branch, and no
    // newer md reads version 0.
    let (out, err, code) = md(&["repair", &legacy_one_error()]);
    assert_eq!(code, 5, "{err}");
    assert!(out.contains("position 7: 'q' -> 'x'"), "{out}");
    assert!(out.lines().any(|l| l == LEGACY_CLEAN), "{out}");
    assert!(err.contains("wire version 0"), "{err}");
    assert!(
        !err.contains("newer md"),
        "version 0 is not from the future: {err}"
    );
    assert!(err.contains("pre-v0.30"), "{err}");
    assert!(!out.is_empty(), "the corrected card: {out}");
}

#[test]
fn a_clean_legacy_card_exits_2_with_empty_stdout() {
    let (out, err, code) = md(&["repair", LEGACY_CLEAN]);
    assert_eq!(code, 2, "{err}");
    assert!(out.is_empty(), "{out}");
}
