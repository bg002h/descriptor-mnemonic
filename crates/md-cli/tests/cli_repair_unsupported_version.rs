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

// ---------------------------------------------------------------------------
// Whole-branch review fix wave (F-449 stage 2).
// ---------------------------------------------------------------------------

/// I-1: a SET mixing single-string cards reads each one as a chunk header, so
/// a card this build reads perfectly well reports a bit-shifted "version"
/// (10 here). Before the guard, 0.19.0 exited 5 and advised "take the
/// corrected card to a newer md ... wire version 10"; 0.18.0 exited 2. The
/// exit-5 branch applies to single-string input only (ruling 7).
#[test]
fn a_mixed_set_of_single_string_cards_is_not_an_unsupported_version() {
    // Ae: `md encode "wsh(multi(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/1'/2'/<0;1>/*))"`
    // with one substitution at data position 9. B: `md encode
    // "tr(UNSPENDABLE(liana),{pk(@0/<0;1>/*),pk(@1/<0;1>/*)})"`. Both decode alone.
    let (out, err, code) = md(&[
        "repair",
        "md15pfdsssjjvvyyw2sqrqscy9zsn0mkdw0fzr7",
        "md1gppqqxq799p20d5hxuzu2c9la",
    ]);
    assert_eq!(
        code, 2,
        "the old atomic-fail exit, not REPAIR_APPLIED: {err}"
    );
    assert!(out.is_empty(), "D28: nothing on stdout: {out}");
    assert!(
        !err.contains("newer md"),
        "no card here has version 10: {err}"
    );

    // A chunked string first does not change that (the chunked v9 card from
    // md-codec's `correct_chunks.rs`, followed by single-string B).
    let (out, err, code) = md(&[
        "repair",
        "md1q4frpqq9q2tvyyy5jmpprj5qqcyxppgqwcudeey7atgd5",
        "md1gppqqxq799p20d5hxuzu2c9la",
    ]);
    assert_eq!(code, 2, "a set with a single-string card in it: {err}");
    assert!(out.is_empty(), "{out}");
}

/// RULING 7 (re-review NEW-1): the corrected-but-unsupported branch is
/// SINGLE-STRING only. A genuine multi-chunk set at an unsupported version
/// (all three chunk headers rewritten to 12, BCH re-wrapped; one correctable
/// error in chunk 1) exits 2 with empty stdout, as in 0.18.0: a build cannot
/// read the chunk-header layout of a version it does not support, so it
/// cannot tell this set from unrelated chunks. The stated limitation.
#[test]
fn a_multi_chunk_set_at_an_unsupported_version_exits_2() {
    // `md encode` of an 8-key wsh multi with 8 fingerprints: three v4 chunks.
    let v4 = [
        "md1fsyk8pq9p6tvyyy5jmpprjjtvyy49ykcgfw2fdssnj2fdssnk2gh20njr9zxysyn",
        "md1fsyk8pq2mpp855jmpp8u4qqxppsfc989mse3sq3zyg3zfzyg3zywcpgwuu5knxhy",
        "md1fsyk8pq3rxvenxd5g3zygj924242kkvenxvm8wamhwlc3zyg3qqlprv3746tu2us",
    ];
    let mut damaged: Vec<String> = v4
        .iter()
        .map(|c| {
            let (mut bytes, bits) = md_codec::codex32::unwrap_string(c).unwrap();
            bytes[0] = (12 << 4) | (bytes[0] & 0x0f);
            md_codec::codex32::wrap_payload(&bytes, bits).unwrap()
        })
        .collect();
    let mut chars: Vec<char> = damaged[1].chars().collect();
    chars[10] = if chars[10] == 'q' { 'p' } else { 'q' };
    damaged[1] = chars.into_iter().collect();
    let mut args = vec!["repair"];
    args.extend(damaged.iter().map(String::as_str));
    let (out, err, code) = md(&args);
    assert_eq!(code, 2, "{err}");
    assert!(out.is_empty(), "{out}");
    assert!(!err.contains("newer md"), "{err}");
}

/// Re-review NEW-1: two GENUINE chunk-of-1 cards from UNRELATED wallets
/// (`md encode --force-chunked "wsh(pk(@0/48'/0'/0'/2'/<0;1>/*))"`, its
/// header version rewritten to 12 and one error added; and `md encode
/// --force-chunked "wsh(pk(@0/48'/1'/9'/2'/<0;1>/*))"`, a different chunk
/// set at v4). Every string is individually chunked, so the first guard let
/// it through to exit 5 and "take the corrected card to a newer md". No md
/// will ever reassemble these two strings.
#[test]
fn two_unrelated_chunks_are_not_an_unsupported_version() {
    let (out, err, code) = md(&[
        "repair",
        "md1ep5e0pqpqztvyyy4qqxpzs7j5uasr0kygh6",
        "md1f8v7jqqpqztvywjv4qqxpzsqa49qwd4lpae92",
    ]);
    assert_eq!(code, 2, "{err}");
    assert!(out.is_empty(), "D28: nothing on stdout: {out}");
    assert!(!err.contains("newer md"), "{err}");
}

/// M-1: the PARITY half of the advice. A chunked card whose header version
/// reads 9 (odd, above 8) must NOT be sent to "a newer md": no release uses
/// an odd version. Fixtures from md-codec's `tests/correct_chunks.rs`.
#[test]
fn an_odd_version_above_8_is_not_sent_to_a_newer_md() {
    let (out, err, code) = md(&["repair", "md1q4frpqq9q2tvyyy5jmpprj5qqcyxppgqwcudeey7atgd5"]);
    assert_eq!(code, 5, "{err}");
    assert!(
        out.lines()
            .any(|l| l == "md1n4frpqq9q2tvyyy5jmpprj5qqcyxppgqwcudeey7atgd5"),
        "{out}"
    );
    assert!(err.contains("wire version 9"), "{err}");
    assert!(!err.contains("newer md"), "9 is odd: {err}");
    assert!(err.contains("pre-v0.30 or misread"), "{err}");
}

/// M-9: an all-uppercase card (QR alphanumeric form) is corrected in its own
/// case. Before, the lowercase corrected char made the output MIXED case,
/// which md refuses to read (BIP-173). Covers the success path, and the
/// exit-5 path shares `apply_corrections`.
#[test]
fn an_uppercase_card_is_corrected_in_uppercase_and_still_reads() {
    let (out, err, code) = md(&["repair", "MD15PFDSSSJJVVYYW2SQRQSCY9ZSN0MKDW0FZR7"]);
    assert_eq!(code, 5, "{err}");
    let fixed = out.lines().last().expect("corrected card");
    assert_eq!(fixed, "MD15PFDSSSJJTVYYW2SQRQSCY9ZSN0MKDW0FZR7");
    let (_, err, code) = md(&["decode", fixed]);
    assert_eq!(code, 0, "the corrected card must decode: {err}");

    let (out, err, code) = md(&["repair", &V12_ONE_ERROR.to_uppercase()]);
    assert_eq!(code, 5, "{err}");
    assert!(out.lines().any(|l| l == V12_CLEAN.to_uppercase()), "{out}");
}
