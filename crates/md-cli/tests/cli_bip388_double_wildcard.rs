//! F-410 item 1 — `@i/**` is accepted as BIP-388 sugar for `@i/<0;1>/*`.
//!
//! WHAT WAS WRONG. BIP-388 ("Wallet Policies") defines `/**` as shorthand for
//! `/<0;1>/*`. `md encode` refused it, and refused it with a message that was
//! wrong on its face:
//!
//! ```text
//! md: template parse error: @0: derivation steps after the multipath group
//!     are not representable in md1
//! ```
//!
//! `/**` contains no derivation steps and no multipath group. The refusal was a
//! LEXER ACCIDENT: the placeholder regex consumed `@0/` + `*` as the wildcard
//! and left the second `*` as unconsumed path residue, so the M5 residue check
//! — which exists to refuse post-multipath fixed steps — fired on a template
//! that has none.
//!
//! THE ACCEPTANCE IS BYTE-IDENTITY, NOT "IT STOPS ERRORING". `/**` is sugar, so
//! the artifact it mints must be the same bytes as the desugared spelling, with
//! the same flags. A `/**` that encoded to *something* would be a new dialect;
//! only equality makes it sugar. The fix desugars the template BEFORE both the
//! lexer and the synthetic-key substitution see it, so the equality is
//! structural rather than a coincidence two regexes have to keep agreeing on.
//!
//! WHAT THIS CHANGE IS NOT. It is not a general loosening. The two genuine
//! refusals in this neighbourhood — a fixed step AFTER a multipath group, and a
//! HARDENED multipath alternative (un-derivable on a watch-only xpub, so a
//! permanently un-restorable card) — are re-asserted here so a later reader
//! cannot mistake `/**` acceptance for either of them being relaxed.

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

/// The artifact bytes only — stdout, which P3 §6a makes the canonical artifact
/// and nothing else.
fn encode_ok(template: &str) -> String {
    let (stdout, stderr, code) = md(&["encode", "--group-size", "0", template]);
    assert_eq!(code, 0, "encode `{template}` failed: {stderr}");
    assert!(
        stdout.starts_with("md1"),
        "encode `{template}` emitted no card: {stdout:?} / {stderr}"
    );
    stdout
}

#[test]
fn double_wildcard_is_byte_identical_to_explicit_multipath() {
    // THE acceptance test for item 1.
    assert_eq!(
        encode_ok("wpkh(@0/**)"),
        encode_ok("wpkh(@0/<0;1>/*)"),
        "`/**` must mint the SAME bytes as `/<0;1>/*` — anything else is a new \
         dialect, not BIP-388 sugar"
    );
}

#[test]
fn double_wildcard_after_an_origin_is_byte_identical() {
    // The origin declaration is a separate capture group; sugar must survive it.
    assert_eq!(
        encode_ok("wsh(multi(2,@0/48'/0'/0'/2'/**,@1/48'/0'/0'/2'/**))"),
        encode_ok("wsh(multi(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/0'/2'/<0;1>/*))"),
    );
}

#[test]
fn double_wildcard_mixed_with_explicit_spelling_is_byte_identical() {
    // Per-placeholder, not per-template: one slot sugared, one spelled out.
    assert_eq!(
        encode_ok("wsh(multi(2,@0/48'/0'/0'/2'/**,@1/48'/0'/0'/2'/<0;1>/*))"),
        encode_ok("wsh(multi(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/0'/2'/<0;1>/*))"),
    );
}

#[test]
fn double_wildcard_card_round_trips_through_decode() {
    // Byte-identity already implies this, but decoding states the OTHER half:
    // the card that comes back is a multipath template, not a bare `/*` collapse
    // that happened to hash the same.
    let card = encode_ok("wpkh(@0/**)");
    let (stdout, stderr, code) = md(&["decode", card.trim()]);
    assert_eq!(code, 0, "decode failed: {stderr}");
    assert!(
        stdout.contains("<0;1>/*"),
        "decoded template must carry the multipath the sugar stands for; \
         full stdout: {stdout}\nfull stderr: {stderr}"
    );
}

// ─── the refusals that MUST survive this change ───────────────────────────

#[test]
fn post_multipath_fixed_step_is_still_refused() {
    let (_, stderr, code) = md(&["encode", "wpkh(@0/<2;3>/0/*)"]);
    assert_ne!(
        code, 0,
        "a fixed step after the multipath group must refuse"
    );
    assert!(
        stderr.contains("final derivation step"),
        "must still refuse for the multipath-not-final reason: {stderr}"
    );
}

#[test]
fn hardened_multipath_is_still_refused() {
    let (_, stderr, code) = md(&["encode", "wpkh(@0/<0';1'>/*)"]);
    assert_ne!(code, 0, "a hardened multipath alt must refuse");
    assert!(
        stderr.contains("hardened"),
        "must still refuse for the hardened-alt reason: {stderr}"
    );
}

#[test]
fn double_wildcard_that_is_not_the_final_step_is_still_refused() {
    // The sugar is only sugar when `/**` ENDS the placeholder. `/**/0` is not a
    // BIP-388 form and must keep hitting the residue reject rather than being
    // quietly rewritten into something that parses.
    let (_, stderr, code) = md(&["encode", "wpkh(@0/**/0)"]);
    assert_ne!(code, 0, "`/**/0` must refuse; stderr was: {stderr}");
}
