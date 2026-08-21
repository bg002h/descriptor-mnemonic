#![allow(missing_docs)]
//! F-219: `md inspect`'s TEXT output must show the per-key origins its own
//! `--json` has carried all along.
//!
//! The origin is what a signer uses to FIND its key. A card carries one per
//! `@N`; `md verify` proves they are there; and until this landed the only way
//! to read them back was `--json`, which is not where an operator restoring a
//! plate will look.
//!
//! THE GATE IS AGREEMENT BETWEEN THE TWO SURFACES, not the presence of a
//! string. Asserting only that text mentions a path would let the two drift
//! apart again in some other direction, which is the defect this is about.

use assert_cmd::Command;

const K0: &str = "xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf";
const K1: &str = "xpub6DzhyrnFFYQ1HimDiM388xHnDiRPNdZJFBmmxge3Y1WWcHLtMJLfRuhRHqnQCPbTj3fGKTuKFLHzzwpJkp5Dtc3UtLKZKaVZe1yqMBXd6Vk";
const DIVERGENT: &str = "wsh(sortedmulti(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/1'/2'/<0;1>/*))";

fn md() -> Command {
    Command::cargo_bin("md").unwrap()
}

fn cards(extra: &[&str]) -> Vec<String> {
    let out = md()
        .args(["encode", DIVERGENT])
        .args(extra)
        .args(["--group-size", "0", "--force-chunked"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "encode failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| l.starts_with("md1"))
        .map(str::to_owned)
        .collect()
}

fn inspect(cards: &[String], json: bool) -> String {
    let mut c = md();
    c.arg("inspect");
    if json {
        c.arg("--json");
    }
    let out = c.args(cards).output().unwrap();
    assert!(
        out.status.success(),
        "inspect failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Every origin `--json` reports must appear in the text output too.
#[test]
fn text_shows_every_origin_that_json_does() {
    let c = cards(&[]);
    let js: serde_json::Value = serde_json::from_str(&inspect(&c, true)).unwrap();
    let decl = &js["descriptor"]["path_decl"];
    let paths: Vec<String> = decl["data"]
        .as_array()
        .expect("path_decl.data should be an array for a Divergent card")
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        paths.len(),
        2,
        "fixture must be Divergent, or this test proves nothing about per-key origins"
    );
    assert_ne!(paths[0], paths[1], "the two origins must DIFFER");

    let text = inspect(&c, false);
    assert!(
        text.contains("origins:"),
        "the text output has no origins section:\n{text}"
    );
    for p in &paths {
        // The text form drops the leading `m/` when a fingerprint is present,
        // so compare on the component tail that both surfaces share.
        let tail = p.trim_start_matches("m/");
        assert!(
            text.contains(tail),
            "origin {p} is in --json but not in the text output:\n{text}"
        );
    }
}

/// With fingerprints, the text form is the DESCRIPTOR spelling — the thing an
/// operator compares against a coordinator, character for character.
#[test]
fn a_keyed_card_prints_descriptor_style_origins() {
    let c = cards(&[
        "--key",
        &format!("@0={K0}"),
        "--key",
        &format!("@1={K1}"),
        "--fingerprint",
        "@0=73c5da0a",
        "--fingerprint",
        "@1=73c5da0a",
    ]);
    let text = inspect(&c, false);
    assert!(
        text.contains("@0: [73c5da0a/48'/0'/0'/2']"),
        "keyed origins are not in descriptor form:\n{text}"
    );
    assert!(
        text.contains("@1: [73c5da0a/48'/0'/1'/2']"),
        "the SECOND slot's origin is wrong or missing:\n{text}"
    );
}

/// A keyless template has no fingerprint to show, and must not invent one.
#[test]
fn a_keyless_card_prints_the_bare_path() {
    let text = inspect(&cards(&[]), false);
    assert!(
        text.contains("@0: m/48'/0'/0'/2'"),
        "keyless origins are not in bare-path form:\n{text}"
    );
    assert!(
        !text.contains("@0: ["),
        "a fingerprint was printed for a card that carries none:\n{text}"
    );
}

/// The origins reported must be the ones the card actually re-encodes to — the
/// only claim that matters. `md verify` is the arbiter.
#[test]
fn the_reported_origins_are_the_ones_the_card_carries() {
    let c = cards(&[]);
    let ok = md()
        .args(["verify", "--template", DIVERGENT])
        .args(&c)
        .output()
        .unwrap();
    assert!(
        ok.status.success(),
        "the fixture card does not re-encode to its own template"
    );
    // A card built from a DIFFERENT origin set must not verify against this
    // template — otherwise `verify` would agree with anything and the check
    // above would be worthless.
    let other = md()
        .args([
            "encode",
            "wsh(sortedmulti(2,@0/48'/0'/7'/2'/<0;1>/*,@1/48'/0'/8'/2'/<0;1>/*))",
            "--group-size",
            "0",
            "--force-chunked",
        ])
        .output()
        .unwrap();
    let other_cards: Vec<String> = String::from_utf8_lossy(&other.stdout)
        .lines()
        .filter(|l| l.starts_with("md1"))
        .map(str::to_owned)
        .collect();
    let bad = md()
        .args(["verify", "--template", DIVERGENT])
        .args(&other_cards)
        .output()
        .unwrap();
    assert!(
        !bad.status.success(),
        "verify accepted a card whose origins differ — it agrees with anything"
    );
}
