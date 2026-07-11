#![allow(missing_docs)]

use md_codec::test_vectors as manifest;

use assert_cmd::Command;

fn encode(template: &str) -> String {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["encode", template])
        .output()
        .unwrap();
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .to_string()
}

fn decode(phrase: &str) -> String {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["decode", phrase])
        .output()
        .unwrap();
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .to_string()
}

#[test]
fn round_trip_each_manifest_entry() {
    for v in manifest::MANIFEST {
        if v.force_chunked {
            continue;
        } // multi-chunk handled separately
        // Path-carrying (non-canonical) vectors need the explicit origin at
        // encode time, else they mint a decode-rejecting card.
        let phrase = match v.path {
            Some(p) => encode_with_path(v.template, p),
            None => encode(v.template),
        };
        let back = decode(&phrase);
        assert_eq!(
            back, v.template,
            "round-trip mismatch for {}: got {} want {}",
            v.name, back, v.template
        );
    }
}

/// Encode a template with an explicit `--path` override. Required for
/// templates whose `canonical_origin` lookup returns None — most
/// `tr(@N, TapTree)` shapes fall in this bucket. (Templates with explicit
/// origin paths already in the placeholder syntax, like the manifest's
/// single-leaf vectors, satisfy the canonicity gate without a separate
/// `--path` override.) Without `--path` here, encode would fail at the
/// canonicity gate with `non-canonical wrapper requires explicit origin`,
/// not at the walker — false-pass on the wrong error path.
fn encode_with_path(template: &str, path: &str) -> String {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["encode", template, "--path", path])
        .output()
        .unwrap();
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .to_string()
}

/// v0.19 — round-trip a 2-leaf balanced multi-branch tap tree end-to-end:
/// walker → wire encode → wire decode → renderer. The decoded template must
/// equal the input string byte-for-byte.
#[test]
fn tap_two_leaf_round_trips() {
    let template = "tr(@0/<0;1>/*,{pk(@1/<0;1>/*),pk(@2/<0;1>/*)})";
    let phrase = encode_with_path(template, "48'/0'/0'/2'");
    assert!(
        phrase.starts_with("md1"),
        "encode produced phrase: {phrase}"
    );
    let decoded = decode(&phrase);
    assert_eq!(decoded, template, "round-trip mismatch");
}

/// v0.19 — round-trip a 4-leaf balanced nested multi-branch tap tree:
/// `tr(@0,{{pk(@1),pk(@2)},{pk(@3),pk(@4)}})`. Exercises the recursive
/// Tag::TapTree wire-encode/decode path with two layers of branching.
#[test]
fn tap_four_leaf_balanced_round_trips() {
    let template =
        "tr(@0/<0;1>/*,{{pk(@1/<0;1>/*),pk(@2/<0;1>/*)},{pk(@3/<0;1>/*),pk(@4/<0;1>/*)}})";
    let phrase = encode_with_path(template, "48'/0'/0'/2'");
    assert!(phrase.starts_with("md1"));
    let decoded = decode(&phrase);
    assert_eq!(decoded, template);
}

/// v0.19 — round-trip a 3-leaf left-unbalanced multi-branch tap tree:
/// `tr(@0,{pk(@1),{pk(@2),pk(@3)}})`. Asymmetric shape — one bare leaf
/// and one TapTree branch as siblings. Confirms the wire format handles
/// unbalanced trees correctly through both encode and decode.
#[test]
fn tap_three_leaf_unbalanced_round_trips() {
    let template = "tr(@0/<0;1>/*,{pk(@1/<0;1>/*),{pk(@2/<0;1>/*),pk(@3/<0;1>/*)}})";
    let phrase = encode_with_path(template, "48'/0'/0'/2'");
    assert!(phrase.starts_with("md1"));
    let decoded = decode(&phrase);
    assert_eq!(decoded, template);
}

/// v0.4.2 generic bug-class probe: every template here must satisfy a
/// stronger property than `decode(encode(t)) == t` — namely
/// `encode(decode(encode(t))) == encode(t)`. This catches asymmetries
/// where the renderer emits text that miniscript-rs cannot re-parse, OR
/// re-parses to a different AST that produces a different wire payload,
/// EVEN IF the decoded text differs from the input by some valid
/// canonical-form drift. The original-vs-decoded equality assertion in
/// `round_trip_each_manifest_entry` catches text drift but not
/// "decoded text doesn't re-parse" (it stops at decode); this test
/// closes that gap.
///
/// Each entry is `(test_name, template, optional_explicit_path)`. Path
/// is `Some(...)` for non-canonical wrappers (`canonical_origin` returns
/// `None` for these and the encode-side canonicity gate would otherwise
/// reject the template).
///
/// Concrete shape that surfaced this test (2026-05-10): the user's
/// inheritance/multi-tier-recovery policy
/// `wsh(andor(pkh(@0),after(N),or_i(and_v(v:pkh(@1),older(M)),and_v(v:pkh(@2),older(K)))))`
/// encoded fine but decoded to non-canonical `pk_h(...)` (type K) where
/// the input had `pkh(...)` (type B); `v:pk_h(...)` is type-invalid in
/// miniscript so the decoded text wouldn't re-parse. The
/// renderer was patched in `format/text.rs::render_node` (Tag::PkH arm)
/// and `render_wrapper_chain` (Check(PkH) shorthand collapse).
#[test]
fn reencode_round_trip_curated_shapes() {
    let cases: &[(&str, &str, Option<&str>)] = &[
        // Bug-surfacing case: pkh fragment inside andor / or_i / and_v
        // wrappers with both absolute and relative timelocks.
        (
            "inheritance_andor_or_i_pkh",
            "wsh(andor(pkh(@0/<0;1>/*),after(1200000),or_i(and_v(v:pkh(@1/<0;1>/*),older(4032)),and_v(v:pkh(@2/<0;1>/*),older(32768)))))",
            Some("48'/0'/0'/2'"),
        ),
        // Companion: pkh inside a simpler `and_v(v:pkh(...), older(...))`
        // (the inheritance-pattern tap-leaf shape from v0.18 prep). Wsh
        // context here, not tap, so exercises the wsh `pkh` rendering
        // arm specifically.
        (
            "wsh_and_v_v_pkh_older",
            "wsh(and_v(v:pkh(@0/<0;1>/*),older(144)))",
            Some("48'/0'/0'/2'"),
        ),
        // Sanity: a wsh-wrapped bare pkh inside or_i (no v: wrapper) —
        // confirms bare Tag::PkH renders as `pkh(...)` in B-position.
        (
            "wsh_or_i_pkh_pkh",
            "wsh(or_i(pkh(@0/<0;1>/*),pkh(@1/<0;1>/*)))",
            Some("48'/0'/0'/2'"),
        ),
        // Tap-leaf variant of the v: + pkh shape (the original v0.18-prep
        // shape that didn't surface the bug because tap-leaves go through
        // a different rendering path; included for parity coverage).
        (
            "tap_leaf_and_v_v_pkh_older",
            "tr(@0/<0;1>/*,and_v(v:pkh(@1/<0;1>/*),older(144)))",
            Some("48'/0'/0'/2'"),
        ),
    ];
    for (name, template, path) in cases {
        let phrase = match path {
            Some(p) => encode_with_path(template, p),
            None => encode(template),
        };
        assert!(
            phrase.starts_with("md1"),
            "[{name}] encode produced non-md1 phrase: {phrase}"
        );
        let decoded = decode(&phrase);
        let phrase_again = match path {
            Some(p) => encode_with_path(&decoded, p),
            None => encode(&decoded),
        };
        assert_eq!(
            phrase, phrase_again,
            "[{name}] re-encode of decoded text produced a different phrase: \
             original={phrase}, decoded={decoded}, re-encoded={phrase_again}"
        );
    }
}

/// v0.4.2 manifest-wide variant of `reencode_round_trip_curated_shapes`.
/// Catches the same bug class for any future manifest entry whose
/// decoded text doesn't round-trip through re-encode. Manifest entries
/// don't currently carry an explicit path; templates that need one
/// (none today) would trip the canonicity gate and fail loudly here,
/// signaling that the manifest needs a path field.
#[test]
fn reencode_round_trip_each_manifest_entry() {
    for v in manifest::MANIFEST {
        if v.force_chunked {
            continue;
        }
        // Path-carrying (non-canonical) vectors need the explicit origin on
        // every encode pass, else the re-encode would trip the decode gate.
        let (phrase, decoded, phrase_again) = match v.path {
            Some(p) => {
                let phrase = encode_with_path(v.template, p);
                let decoded = decode(&phrase);
                let again = encode_with_path(&decoded, p);
                (phrase, decoded, again)
            }
            None => {
                let phrase = encode(v.template);
                let decoded = decode(&phrase);
                let again = encode(&decoded);
                (phrase, decoded, again)
            }
        };
        assert_eq!(
            phrase, phrase_again,
            "[{}] re-encode of decoded text produced a different phrase: \
             original={phrase}, decoded={decoded}, re-encoded={phrase_again}",
            v.name
        );
    }
}

/// Pins the `pkh(K)` shorthand round-trip end-to-end for the `wsh(pkh(K))`
/// shape. v0.30 SPEC §5.1 (Q12 — walker normalization) emits BARE
/// `Tag::PkH` at the key-leaf position regardless of context (segwitv0 +
/// tap-leaf alike); pre-v0.30 the walker emitted `Tag::Check(Tag::PkH(K))`
/// in segwitv0 and the renderer's `render_wrapper_chain` collapse arm
/// reconstructed `pkh(K)` from the wrapped wire shape. Post-v0.30 the
/// `render_node` PkH arm renders `pkh(K)` directly from the bare wire form;
/// no Check-wrapped wire shape is produced. The round-trip target string is
/// invariant across both wire shapes — this test pins the user-visible
/// behavior. Companion walker-shape unit test:
/// `parse::template::root_tests::pkh_key_leaf_bare_on_wire` (pins the bare
/// wire invariant directly). Companion to the `inheritance_andor_or_i_pkh`
/// curated case (same arm under `or_i` siblings).
#[test]
fn wsh_pkh_shorthand_collapse_round_trips() {
    let template = "wsh(pkh(@0/<0;1>/*))";
    let phrase = encode_with_path(template, "84'/0'/0'");
    assert!(
        phrase.starts_with("md1"),
        "encode produced phrase: {phrase}"
    );
    let decoded = decode(&phrase);
    assert_eq!(decoded, template, "round-trip mismatch");
    let phrase_again = encode_with_path(&decoded, "84'/0'/0'");
    assert_eq!(
        phrase, phrase_again,
        "re-encode of decoded text produced a different phrase: \
         original={phrase}, decoded={decoded}, re-encoded={phrase_again}"
    );
}

/// v0.30 Phase E (Q12) — Tr tap-leaf `pk(@1)` round-trips end-to-end. The
/// walker emits bare `Tag::PkK` (no enclosing `Tag::Check`); the renderer
/// reconstructs the `pk(...)` shorthand directly from the bare wire form.
/// Companion walker-shape unit test:
/// `parse::template::tr_tests::tr_tap_leaf_bare_pk_on_wire` (pins the
/// wire invariant).
#[test]
fn tr_tap_leaf_bare_pk_round_trip() {
    let template = "tr(@0/<0;1>/*,pk(@1/<0;1>/*))";
    let phrase = encode_with_path(template, "48'/0'/0'/2'");
    assert!(
        phrase.starts_with("md1"),
        "encode produced phrase: {phrase}"
    );
    let decoded = decode(&phrase);
    assert_eq!(decoded, template, "round-trip mismatch");
    let phrase_again = encode_with_path(&decoded, "48'/0'/0'/2'");
    assert_eq!(
        phrase, phrase_again,
        "re-encode of decoded text produced a different phrase: \
         original={phrase}, decoded={decoded}, re-encoded={phrase_again}"
    );
}
