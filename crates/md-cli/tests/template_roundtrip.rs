#![allow(missing_docs)]

mod manifest {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../md-codec/tests/vectors/manifest.rs"
    ));
}

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
        let phrase = encode(v.template);
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
