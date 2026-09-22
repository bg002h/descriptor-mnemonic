//! Stage 1b task 6 (SPEC §4a): the INPUT side. Kind 1 (SPEC §2/§3d) already
//! exists on the wire (tasks 1-4) and renders (task 5); this task makes `md`
//! RECOGNISE it, on the three surfaces that need it and only there:
//!
//! | surface | can it recompute §2? | rule |
//! | --- | --- | --- |
//! | `md decompose` | yes — it holds the real leaf keys | recompute; byte match -> kind 1, NO slot |
//! | the template grammar | n/a | `UNSPENDABLE(liana)` parses AND yields kind 1 |
//! | `md encode` with a literal xpub | no | refuse, naming both working spellings |
//!
//! Every test here shells out to the `md` binary, so it lives in md-cli's own
//! `tests/`, not md-codec's — md-codec's test root cannot host a subprocess
//! test. `crates/md-cli/tests/liana_cases.rs` is md-cli's OWN reader over the
//! SAME vendored `cases.json` md-codec's tests read (that file's own header
//! comment explains why a second reader is needed rather than reusing
//! md-codec's `include!`d one).

#![allow(missing_docs)]

#[path = "liana_cases.rs"]
mod liana_cases;
use liana_cases::{
    ORIGINLESS_SPENDABLE_TR, case, near_miss_liana_recipe_over_different_leaves,
    self_referential_liana_key_descriptor,
};

use std::process::Command as StdCommand;

/// Run `md` with `args`, returning `(stdout, stderr, exit code)`. Every call
/// site must bind and check all three — a bare `md(&[...]);` asserts nothing
/// (this exact class reached this plan three times already).
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

/// `md`, asserting the run FAILED, returning stderr.
fn md_err(args: &[&str]) -> String {
    let (out, err, code) = md(args);
    assert_ne!(code, 0, "expected failure, got success with stdout: {out}");
    err
}

#[test]
fn decompose_recognises_a_real_liana_descriptor_and_gives_it_no_slot() {
    let (out, err, code) = md(&[
        "decompose",
        &case("preset-kofn-recovery-tr").descriptor_with_checksum,
        "--emit",
        "template",
    ]);
    assert_eq!(code, 0, "decompose failed: {err}");
    assert!(out.contains("UNSPENDABLE(liana)"), "got {out}");
    // Brief's own snippet checked `out.contains("state NO origin")`, which
    // can never fail: `cmd/decompose.rs`'s own doc comment ("stdout is the
    // machine contract; notes and advisories go to stderr") means a note
    // NEVER reaches stdout under any `--emit` mode, so that assertion was
    // vacuously true regardless of whether the recogniser worked. Checking
    // `err` instead is the version that can actually fail — this is exactly
    // the class of test the brief warns has "promised more than it
    // delivered" three times already on this stage.
    assert!(
        !err.contains("state NO origin"),
        "must not become a phantom slot: {err}"
    );
}

#[test]
fn decompose_keeps_todays_behaviour_for_an_origin_less_key_that_is_not_lianas() {
    // G-8: the phantom property is NO ORIGIN, not "not Liana's". A real
    // spendable internal key whose owner recorded no origin is @0 and
    // correct; refusing it would also lock out libnunchuk's PR-1746 form.
    let (out, err, code) = md(&["decompose", ORIGINLESS_SPENDABLE_TR, "--emit", "template"]);
    assert_eq!(code, 0, "decompose failed: {err}");
    assert!(out.contains("tr(@0/"), "must still be a slot: {out}");
    // Same stream correction as the test above: the origin-less note is on
    // stderr, never stdout.
    assert!(
        err.contains("state NO origin"),
        "must still annotate: {err}"
    );
}

#[test]
fn md_encode_refuses_a_literal_xpub_in_the_internal_key_position_cleanly() {
    let err = md_err(&[
        "encode",
        &case("preset-kofn-recovery-tr").descriptor_with_checksum,
    ]);
    assert!(
        !err.contains("internal:"),
        "internal invariant leaked: {err}"
    );
    assert!(
        err.contains("UNSPENDABLE(liana)"),
        "must name the working spelling: {err}"
    );
    assert!(
        err.contains("decompose"),
        "must name the other working route: {err}"
    );
}

#[test]
fn the_marker_parses_in_a_template_and_yields_kind_1() {
    let (md1, err, code) = md(&[
        "encode",
        "tr(UNSPENDABLE(liana),{pk(@0/<0;1>/*),pk(@1/<0;1>/*)})",
        "--path",
        "bip48",
    ]);
    assert_eq!(code, 0, "encode failed: {err}");
    let d = md_codec::decode::decode_md1_string(md1.trim()).expect("decode");
    assert!(matches!(
        d.tree.body,
        md_codec::tree::Body::Tr {
            internal_key: md_codec::tree::InternalKey::LianaUnspendable,
            ..
        }
    ));
}

/// `md encode` on a literal xpub in the internal-key position, but reached
/// via a TEMPLATE with `@N`-placeholder leaves rather than a fully concrete
/// descriptor — the OTHER shape that used to leak the internal-error message
/// (`walk_tr`'s `lookup_key` failure, not the earlier no-placeholders refusal
/// the previous test exercises). Both shapes must be clean.
#[test]
fn md_encode_refuses_a_literal_internal_key_inside_an_otherwise_valid_template() {
    let xpub = &case("preset-kofn-recovery-tr").expected_xpub;
    let err = md_err(&[
        "encode",
        &format!("tr({xpub}/<0;1>/*,{{pk(@0/<0;1>/*),pk(@1/<0;1>/*)}})"),
        "--path",
        "bip48",
    ]);
    assert!(
        !err.contains("internal:"),
        "internal invariant leaked: {err}"
    );
    assert!(
        err.contains("UNSPENDABLE(liana)"),
        "must name the working spelling: {err}"
    );
    assert!(
        err.contains("decompose"),
        "must name the other working route: {err}"
    );
}

/// I-1 fix round 1 (Important, M17 on Task 9's mutation list). Weakening
/// `liana_internal_key_match`'s discriminating property from FULL byte
/// equality (`actual_xpub == recomputed`) to a NUMS-pubkey/depth-0
/// PATTERN-MATCH leaves the suite at 1475/1475 green — because every
/// existing test hands the recogniser a key that legitimately matches its
/// OWN tree, and a pattern-match accepts every one of those too (it is a
/// strict superset of the real check on inputs the real check accepts). The
/// only shape that separates the two is the near miss this test pins: a
/// real, valid Liana recipe output — genuine NUMS pubkey, depth 0, zero
/// parent fingerprint, zero child number — computed over a DIFFERENT leaf
/// set than the one actually present. A positive-only suite cannot see a
/// weakening here BY CONSTRUCTION; only a case the real check must REJECT
/// and a pattern-match would WRONGLY ACCEPT can gate it.
#[test]
fn decompose_does_not_recognise_lianas_own_recipe_computed_over_the_wrong_leaves() {
    let (out, err, code) = md(&[
        "decompose",
        &near_miss_liana_recipe_over_different_leaves(),
        "--emit",
        "template",
    ]);
    assert_eq!(code, 0, "decompose failed: {err}");
    assert!(out.contains("tr(@0/"), "must fall through to a slot: {out}");
    assert!(
        !out.contains("UNSPENDABLE(liana)"),
        "must NOT be wrongly relabelled as this wallet's own recipe: {out}"
    );
    assert!(
        err.contains("state NO origin"),
        "must still annotate as a phantom slot: {err}"
    );
}

/// I-2 fix round 1 (Important). The recognised Liana-unspendable internal
/// key also appears, byte-identically, as a `pk()` tapleaf key — the shape
/// that exposed `decompose/mod.rs`'s retain (which matches by RENDERED
/// TEXT, not tree position) running BEFORE `check_no_repeated_key` instead
/// of after: with the ordering bug, the retain would drop BOTH occurrences
/// (the internal key's and the leaf's) before the repeat check ever saw
/// either, and the leaf key would silently vanish from the slot set and the
/// mint commands with no refusal at all. The fixed ordering must still
/// refuse this — BIP-388's disjointness rule, since both occurrences share
/// the identical `/<0;1>/*` use-site.
#[test]
fn decompose_still_refuses_a_repeat_when_the_repeat_is_the_recognised_internal_key() {
    let err = md_err(&[
        "decompose",
        &self_referential_liana_key_descriptor(),
        "--emit",
        "template",
    ]);
    assert!(
        !err.contains("internal:"),
        "internal invariant leaked, not a real refusal: {err}"
    );
    assert!(
        err.contains("BIP 388"),
        "must be the BIP-388 repeat refusal, not something else: {err}"
    );
}
