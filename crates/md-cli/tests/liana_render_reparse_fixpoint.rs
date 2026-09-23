//! Task 8 step 5b, item 7: the render-reparse fixpoint for a wire-kind-1
//! (Liana unspendable) `tr()`. §8 items 2 and 7 were §9-assigned to this
//! stage but the C3 fold silently dropped item 7 while moving item 2's
//! evidence leg to stage 2 -- this file restores it.
//!
//! A fixpoint is ENCODE -> RENDER -> RE-ENCODE -> IDENTICAL. It lives in
//! md-cli, not md-codec: it shells out to the `md` binary (md-codec's test
//! root has no `CARGO_BIN_EXE_md` and no CLI runner in dev-deps), and it is
//! its OWN file rather than sharing one with a md-codec test fence, because
//! an earlier draft's shared fence made `plan-api-check.sh`'s cross-crate
//! report flag the whole fence -- one fence with two crate homes is exactly
//! how a helper ends up called from where it cannot be reached.
//!
//! Two earlier drafts of the item-7 idea failed the same way, worth naming
//! so a future edit does not reintroduce either:
//!
//!   draft 1: one test that only re-parsed  -> never called the renderer at
//!            all, so a renderer regression was invisible to it.
//!   draft 2: two tests, one per crate      -> the md-codec half discarded
//!            `md()`'s return, and `md(args) -> (String, String, i32)` does
//!            NOT panic on a nonzero exit (see `md` below), so a silent
//!            failure could not fail the test; and the two template strings
//!            compared across the crate split never corresponded to the
//!            same wallet in the first place.
//!
//! Doing all three steps through the CLI, in ONE test, keeps every step
//! asserting and keeps both halves (encode's template grammar, decode's
//! renderer) exercised against each other rather than against themselves.

#![allow(missing_docs)]

use std::process::Command as StdCommand;

/// Run `md` with `args`, returning `(stdout, stderr, exit code)`. Does NOT
/// panic on a nonzero exit -- every call site must bind and check `code`
/// itself, per `cli_bip388_double_wildcard.rs:35-45`'s own helper (this
/// file's `md` is the identical pattern, reproduced narrowly here since a
/// test-root helper is not visible across integration-test binaries).
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

/// Doing BOTH halves as a real round trip: encode a marker-form template,
/// decode it back through the real renderer, and re-encode what the
/// renderer produced. `keyed_compose_tr_nums_three_leaves` (an earlier
/// draft's fixture) renders a NESTED four-key tree with `older`/`multi_a`/
/// `after` -- not this flat two-key literal -- which is why this test
/// builds its own template rather than reusing a vendored vector's name.
#[test]
fn item_7_the_render_reparse_fixpoint_covers_tr_kind_1() {
    let tpl = "tr(UNSPENDABLE(liana),{pk(@0/48'/0'/0'/3'/<0;1>/*),pk(@1/48'/0'/1'/3'/<0;1>/*)})";

    // 1. ENCODE the marker form.
    let (md1, err, code) = md(&["encode", tpl, "--path", "bip48"]);
    assert_eq!(code, 0, "encode failed: {err}");

    // 2. RENDER it back through the real renderer. `md decode` emits the
    //    plain BIP-388 template by default.
    let (rendered, err, code) = md(&["decode", md1.trim()]);
    assert_eq!(code, 0, "decode failed: {err}");
    assert!(
        rendered.contains("UNSPENDABLE(liana)"),
        "the renderer dropped the marker: {rendered}"
    );
    assert!(
        !rendered.contains("50929b74"),
        "kind 1 must not render as the raw NUMS hex: {rendered}"
    );

    // 3. RE-ENCODE what the renderer produced. This is the fixpoint, and it
    //    is what `md-cli/src/format/text.rs:269-274` asserts for wsh and
    //    cannot currently assert for tr kind 1.
    let (md1b, err, code) = md(&["encode", rendered.trim(), "--path", "bip48"]);
    assert_eq!(code, 0, "the rendered template did NOT re-parse: {err}");
    assert_eq!(md1.trim(), md1b.trim(), "round trip is not a fixpoint");
}
