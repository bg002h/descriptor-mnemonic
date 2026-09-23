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
    ORIGINLESS_SPENDABLE_TR, case, kofn_with_internal_key,
    near_miss_liana_recipe_over_different_leaves, self_referential_liana_key_descriptor,
    self_referential_liana_key_descriptor_at_leaf_use_site,
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
    // F-636: the same-use-site repeat of the recognised key names the dead
    // leaf too, and KEEPS the BIP-388 citation because here the paths also
    // overlap.
    assert!(
        err.contains("can never be satisfied"),
        "must name the dead leaf (F-636): {err}"
    );
}

/// Fix round 1 re-review Minor, folded rather than deferred. The SAME
/// recognised internal key reused at a DISJOINT leaf use-site
/// (`/<2;3>/*` against the internal key's `/<0;1>/*`) — the variant the
/// same-use-site test above cannot reach, because `retain` matches by
/// RENDERED TEXT and these two renderings differ.
///
/// Under the OLD ordering this was accepted at exit 0, emitting
/// `tr(UNSPENDABLE(liana),{pk(@0/<2;3>/*),pk(@1/...)})` — md handed the
/// operator a slot for a value it had ITSELF just proved unspendable, and
/// would have asked them to mint a card for a key with no private key. That
/// is the same defect class as I-2, hiding one use-site away from it.
///
/// F-636 (F-449 stage 2 Task 6): the refusal used to come from the generic
/// disjoint-multipath branch, whose message blamed md's one-path-per-slot
/// limit ("BIP 388 permits that shape ... UNSUPPORTED") -- inviting the
/// operator to reproduce a DEAD leaf in another tool. It now names the real
/// problem: the wallet's own provably-unspendable internal key sits at a
/// spending leaf, which no implementation can ever satisfy. The old
/// `UNSUPPORTED` assertion is RETIRED deliberately: that word is the
/// md-capability framing this follow-up removed.
#[test]
fn decompose_refuses_the_recognised_internal_key_reused_at_a_disjoint_use_site() {
    let err = md_err(&[
        "decompose",
        &self_referential_liana_key_descriptor_at_leaf_use_site("<2;3>"),
        "--emit",
        "template",
    ]);
    assert!(
        !err.contains("internal:"),
        "internal invariant leaked, not a real refusal: {err}"
    );
    assert!(
        err.contains("unspendable internal key") && err.contains("can never be satisfied"),
        "must name the dead leaf, not md's limit (F-636): {err}"
    );
    assert!(
        !err.contains("md's template surface is narrower") && !err.contains("UNSUPPORTED"),
        "the md-capability framing is exactly what F-636 removed: {err}"
    );
}

/// Task 8 step 5 (§6's pinned invariant, I6). SPEC §6 carries an invariant,
/// not a refusal: a `tr` with no leaf keys cannot be constructed, so
/// `sha256("")` can never become a shared chain code. This lives here
/// (md-cli, not md-codec) because the guard is the TEMPLATE PARSER's --
/// `md_codec::tree` has no opinion on whether a `tr` needs a leaf, only
/// `parse/template.rs`'s `@i`-placeholder scan does.
///
/// If this test ever starts asserting `code == 0` instead of a refusal,
/// `liana_unspendable_xpub` would derive `sha256("")` for every such wallet
/// and they would all share one internal key -- a funds-relevant collision,
/// not a cosmetic one.
///
/// Review round 1, M4: the first draft pinned only the KIND-0 spelling
/// (the raw NUMS hex). That form can never reach the hazard at all --
/// `liana_unspendable_xpub` is kind 1's recipe, and a kind-0 `tr` never
/// calls it, so a test built only from the NUMS-hex spelling proves the
/// guard through a door the danger cannot use. The MARKER form,
/// `tr(UNSPENDABLE(liana))`, is kind 1 -- the one shape that could actually
/// reach `liana_unspendable_xpub(&[])` if this refusal regressed -- so it is
/// the primary pin now. The kind-0 case stays too (harmless, and it is a
/// real refusal in its own right), but the marker case is the one that
/// matters.
#[test]
fn a_tr_with_no_leaf_keys_cannot_be_constructed() {
    let err = md_err(&["encode", "tr(UNSPENDABLE(liana))"]);
    assert!(
        err.contains("no @i placeholders"),
        "kind 1 (the marker spelling) must be refused for having no leaf \
         keys, not some other reason: {err}"
    );

    let err = md_err(&[
        "encode",
        "tr(50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0)",
    ]);
    assert!(
        err.contains("no @i placeholders"),
        "kind 0 (the raw NUMS hex) must also be refused for having no leaf \
         keys, not some other reason: {err}"
    );
}

/// C-1 (whole-branch review, CRITICAL). `liana_internal_key_match` compared
/// only the extended KEY, ignoring the internal key's own origin,
/// derivation path and wildcard — so ANY use-site on a byte-matching xpub
/// was silently replaced by the marker. `UNSPENDABLE(liana)` DENOTES
/// `/<0;1>/*` with NO origin (SPEC §2 step 6: Liana derives the internal
/// key at exactly `0/i` and `1/i` in every port); anything else is a
/// GENUINELY DIFFERENT derivation, and recognising it anyway erases the
/// divergence between what the operator pasted and what the card says.
///
/// Every non-origin shape the review named as swallowed, built from
/// `preset-kofn-recovery-tr`'s own real key material (the recipe xpub is
/// genuine and byte-matches the leaves actually present — ONLY the internal
/// key's own use-site differs from `/<0;1>/*`): a different multipath
/// order, a different arity, a fixed single path, a hardened wildcard, no
/// wildcard at all, and a trailing fixed step after the multipath group.
/// Each must fall through to the ordinary annotated-slot path, unchanged
/// from base — decompose still succeeds (this is real, parseable,
/// mintable-shaped input, just not Liana's own recipe use-site), but the
/// output must never claim `UNSPENDABLE(liana)`.
#[test]
fn decompose_does_not_recognise_a_non_canonical_internal_key_use_site() {
    let xpub = case("preset-kofn-recovery-tr").expected_xpub;
    let cases: &[(&str, String)] = &[
        (
            "<2;3> -- different multipath alternatives",
            format!("{xpub}/<2;3>/*"),
        ),
        ("<1;0> -- reversed order", format!("{xpub}/<1;0>/*")),
        ("<0;1;2> -- extra alternative", format!("{xpub}/<0;1;2>/*")),
        ("/0/* -- fixed single path", format!("{xpub}/0/*")),
        (
            "/*h -- hardened wildcard, no multipath",
            format!("{xpub}/*h"),
        ),
        ("/0 -- no wildcard at all", format!("{xpub}/0")),
        (
            "/<0;1>/5/* -- trailing fixed step",
            format!("{xpub}/<0;1>/5/*"),
        ),
    ];
    for (label, internal_key_expr) in cases {
        let d = kofn_with_internal_key(internal_key_expr);
        let (out, err, code) = md(&["decompose", &d, "--emit", "template"]);
        assert_eq!(code, 0, "[{label}] decompose failed: {err}");
        assert!(
            !out.contains("UNSPENDABLE(liana)"),
            "[{label}] must NOT be recognised as Liana's own internal key: {out}"
        );
        assert!(
            out.contains("tr(@0/"),
            "[{label}] must fall through to an ordinary slot: {out}"
        );
    }
}

/// C-1's eighth named case: a PREFIXED ORIGIN on the internal key. Base
/// (`37367c1f`, before the Liana feature existed at all) already refused
/// this shape outright — `preset-kofn-recovery-tr`'s recipe xpub has
/// `depth == 0`, and attaching a 4-component origin path to a depth-0 xpub
/// trips the PRE-EXISTING, generic `check_depth_consistency` refusal, which
/// has nothing Liana-specific about it. Named as its own test because the
/// assertion shape differs from the seven above (a real refusal, not a
/// clean fall-through) and because `liana_internal_key_match`'s own
/// `origin.is_some()` check is what keeps this shape from EVER reaching
/// `UNSPENDABLE(liana)` in the first place, ahead of that depth check.
#[test]
fn decompose_refuses_an_internal_key_with_a_prefixed_origin_same_as_base() {
    let xpub = case("preset-kofn-recovery-tr").expected_xpub;
    let internal_key_expr = format!("[73c5da0a/48'/0'/0'/3']{xpub}/<0;1>/*");
    let d = kofn_with_internal_key(&internal_key_expr);
    let (out, err, code) = md(&["decompose", &d, "--emit", "template"]);
    assert_ne!(code, 0, "must be refused, same as base: stdout was {out}");
    assert!(
        !err.contains("internal:"),
        "internal invariant leaked, not a real refusal: {err}"
    );
    assert!(
        !out.contains("UNSPENDABLE(liana)"),
        "must never be labelled as Liana's own key on the way to refusing: {out}"
    );
}

/// C-1's own funds-relevant reproduction, verbatim: the SAME descriptor's
/// truth (`--emit descriptor`) and its template (`--emit template`) must
/// keep describing the SAME wallet. Before the fix, a `/<2;3>/*` internal
/// key produced a template BYTE-IDENTICAL to the genuine `/<0;1>/*`
/// descriptor's — the divergence erased, two different wallets collapsed
/// into one card. After the fix, the two templates must DIFFER.
#[test]
fn a_non_canonical_use_site_produces_a_different_template_than_the_genuine_one() {
    // The genuine descriptor, straight from the fixture (not round-tripped
    // through `kofn_with_internal_key`, which asserts its substitution
    // actually changed something -- substituting the genuine text for
    // itself would trip that assertion for being a no-op, correctly).
    let genuine = case("preset-kofn-recovery-tr").descriptor_with_checksum;
    let xpub = case("preset-kofn-recovery-tr").expected_xpub;
    let divergent = kofn_with_internal_key(&format!("{xpub}/<2;3>/*"));
    assert_ne!(
        genuine, divergent,
        "test construction error: the two inputs must actually differ"
    );

    let (genuine_template, genuine_err, genuine_code) =
        md(&["decompose", &genuine, "--emit", "template"]);
    assert_eq!(genuine_code, 0, "genuine descriptor failed: {genuine_err}");
    let (divergent_template, divergent_err, divergent_code) =
        md(&["decompose", &divergent, "--emit", "template"]);
    assert_eq!(
        divergent_code, 0,
        "divergent descriptor failed: {divergent_err}"
    );

    assert_ne!(
        genuine_template, divergent_template,
        "a wallet whose internal key derives at 2/i must not emit the SAME template as one \
         that derives at 0/i -- that is the divergence being erased"
    );
}

/// I-1 (whole-branch review, Important). `UNSPENDABLE(liana)` is meaningful
/// ONLY as a `tr()` descriptor's own internal key. Before the fix, placing
/// it anywhere else was silently rewritten to the same synthetic hex as a
/// genuine internal-key marker by `substitute_synthetic`'s blind
/// `str::replace` (which has no notion of position), and then leaked
/// `"internal: synthetic key … not found in key map"` when `lookup_key`
/// failed to find that hex in the `@i` placeholder map — an internal
/// invariant string SPEC §4a requires never reach a user. Three placements
/// the review named: a tapleaf, inside `multi_a(...)`, and inside a
/// `wsh(...)` that carries no `tr()` at all.
#[test]
fn md_encode_refuses_the_marker_outside_the_internal_key_position_cleanly() {
    let cases: &[(&str, &str)] = &[
        (
            "at a tapleaf",
            "tr(UNSPENDABLE(liana),{pk(UNSPENDABLE(liana))})",
        ),
        (
            "inside multi_a",
            "tr(UNSPENDABLE(liana),{multi_a(1,UNSPENDABLE(liana))})",
        ),
        (
            "inside a wsh (no tr() at all)",
            "wsh(pk(UNSPENDABLE(liana)))",
        ),
    ];
    for (label, template) in cases {
        let err = md_err(&["encode", template, "--path", "bip48"]);
        assert!(
            !err.contains("internal:"),
            "[{label}] internal invariant leaked: {err}"
        );
        assert!(
            err.contains("UNSPENDABLE(liana)"),
            "[{label}] must name the marker: {err}"
        );
        assert!(
            err.contains("tr("),
            "[{label}] must name where it IS allowed: {err}"
        );
    }
}

/// Whole-branch fix round 2 (Important). `validate_marker_position` sliced
/// `&template[pos - 3..pos]` on a BYTE offset, so any multi-byte character
/// ending within three bytes before the marker panicked the process:
/// exit 101, an internal source path, and the user's own input echoed back,
/// where the base binary gave a clean exit-1 parse error.
///
/// These are not exotic inputs. Every character below is an ordinary paste
/// artefact from a document, a chat client or a PDF: a Euro sign, an em
/// dash, a non-breaking space, a smart quote, an accented letter, an emoji.
/// A descriptor is something operators PASTE, so this path sees them.
///
/// The all-ASCII tests added alongside the original fix stayed green
/// throughout, which is exactly why this needed its own case: a suite can be
/// 1499/1499 and still never have fed the function a non-ASCII byte.
#[test]
fn a_misplaced_marker_after_a_multibyte_char_refuses_cleanly_and_does_not_panic() {
    for probe in [
        "tr(x\u{20ac}yUNSPENDABLE(liana))", // €  — Euro sign
        "tr(a\u{2014}bUNSPENDABLE(liana))", // —  — em dash
        "tr(a\u{00a0}UNSPENDABLE(liana))",  // NBSP
        "tr(a\u{201c}UNSPENDABLE(liana))",  // "  — smart quote
        "tr(caf\u{e9}UNSPENDABLE(liana))",  // é
        "tr(a\u{1f600}UNSPENDABLE(liana))", // 😀 — 4-byte
    ] {
        let (out, err, code) = md(&["encode", probe, "--path", "bip48"]);
        assert_ne!(code, 101, "PANICKED on {probe:?}: {err}");
        assert_eq!(code, 1, "must be a clean parse refusal on {probe:?}: {err}");
        assert!(
            !err.contains("panicked") && !err.contains("src/parse/template.rs"),
            "leaked a panic or an internal source path on {probe:?}: {err}"
        );
        assert!(out.is_empty(), "refusal must emit nothing on stdout: {out}");
    }
}

/// F-638 through the operator CLI: a per-key override off `<0;1>` on an
/// otherwise-canonical kind-1 template names that key.
#[test]
fn encode_names_the_key_whose_use_site_diverged() {
    let err = md_err(&[
        "encode",
        "tr(UNSPENDABLE(liana),{pk(@0/<0;1>/*),pk(@1/<2;3>/*)})",
    ]);
    assert!(err.contains("@1"), "names the placeholder (F-638): {err}");
    assert!(
        !err.contains("@0"),
        "names only the one that diverged: {err}"
    );
}

/// F-641 (F-449 stage 2 Task 6). The position check used to ask whether the
/// marker was preceded by the three bytes `tr(`, so a NESTED `tr` and any
/// identifier ENDING in `tr` passed it -- and then failed downstream naming
/// the 64-hex synthetic key substituted for the marker, which the operator
/// never wrote. The check is now structural (the marker must be the first
/// argument of the OUTERMOST node, and that node must be `tr`), so both get
/// the same marker refusal as every other misplacement.
#[test]
fn a_marker_under_a_nested_or_lookalike_tr_names_the_marker_not_a_synthetic_key() {
    // `liana_synthetic_internal_key_hex()`'s value, as the old message leaked it.
    const SYNTHETIC: &str = "fa1446b119da8e010be1a88d3340f68fe4f21c439270c652c5434d04d3e92c98";
    for template in [
        "wsh(tr(UNSPENDABLE(liana),pk(@0/<0;1>/*)))",
        "xtr(UNSPENDABLE(liana),pk(@0/<0;1>/*))",
        "sh(tr(UNSPENDABLE(liana),pk(@0/<0;1>/*)))",
        // Embedded in a longer token: not a marker NODE, but the text
        // substitution would still rewrite it.
        "tr(UNSPENDABLE(liana),{pk(@0/<0;1>/*),pk(xUNSPENDABLE(liana))})",
        "tr(xUNSPENDABLE(liana),pk(@0/<0;1>/*))",
    ] {
        let err = md_err(&["encode", template]);
        assert!(
            !err.contains(SYNTHETIC),
            "leaked the synthetic key for {template}: {err}"
        );
        assert!(
            err.contains("meaningful only as a tr() descriptor's own internal key"),
            "the existing marker refusal, reused verbatim, for {template}: {err}"
        );
    }
    // The genuine position still encodes.
    let (out, err, code) = md(&["encode", "tr(UNSPENDABLE(liana),pk(@0/<0;1>/*))"]);
    assert_eq!(code, 0, "{err}");
    assert!(out.starts_with("md1"), "{out}");
}
