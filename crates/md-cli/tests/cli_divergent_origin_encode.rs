//! Divergent per-key origins: they WORK, and the wrong syntax must say so.
//!
//! md's template placeholder syntax puts a key's origin AFTER `@i`, as path
//! components — `@0/48'/0'/0'/2'/<0;1>/*`. `lex_placeholders`' regex captures it
//! (`crates/md-cli/src/parse/template.rs`, capture 2), and `make_path_decl`
//! emits `PathDeclPaths::Divergent` whenever the per-placeholder origins differ.
//!
//! WHY THESE TESTS EXIST. On 2026-08-15 a downstream consumer (the SeedHammer
//! firmware's byte-identity gate, which derives an expected md1 through this
//! CLI) concluded that md "cannot encode divergent origins", because it fed md a
//! DESCRIPTOR-style origin — `[fingerprint/path]@0/…` — and got:
//!
//!   md: template parse error: internal: synthetic key [73c5da0a not found in
//!       key map (rendered: [73c5da0a/48'/0'/0'/2']xpub6DXuQW1FgeHbfmex…)
//!
//! That message is why the wrong conclusion was reachable: it says `internal:`,
//! so it reads as a tool bug rather than a rejected input, and it appears only
//! AFTER md has rendered the descriptor correctly, so everything looks right up
//! to the failure.
//!
//! The near-miss worth recording: the obvious "fix" — teaching `lookup_key` to
//! strip the bracket — makes md ACCEPT the foreign syntax and SILENTLY DROP the
//! origins (verified: `path_decl` comes back empty). That converts a loud
//! wrong-syntax error into a quiet wrong-policy, on a funds-relevant encoder.
//! The round-trip test below is what caught it; an encode-succeeds assertion
//! alone passes.

use assert_cmd::Command;
use predicates::prelude::*;

/// Two accounts of ONE master — the shape a shared `--path` cannot express,
/// since the keys differ only in their account component.
const DIVERGENT: &str = "wsh(sortedmulti(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/1'/2'/<0;1>/*))";

/// Descriptor-style origins. NOT md template syntax.
const BRACKETED: &str = "wsh(sortedmulti(2,\
    [73c5da0a/48h/0h/0h/2h]@0/<0;1>/*,\
    [73c5da0a/48h/0h/1h/2h]@1/<0;1>/*))";

#[test]
fn divergent_per_key_origins_encode() {
    Command::cargo_bin("md")
        .unwrap()
        .args(["encode", DIVERGENT, "--force-chunked"])
        .assert()
        .success()
        .stdout(predicate::str::contains("md1"));
}

/// The property with funds attached: both origins must survive a round trip,
/// DISTINCT. An encoder that flattened them to a shared origin would still
/// "encode successfully" while writing a policy that spends from the wrong
/// account.
#[test]
fn divergent_origins_survive_a_decode_round_trip() {
    let enc = Command::cargo_bin("md")
        .unwrap()
        .args(["encode", DIVERGENT, "--force-chunked"])
        .output()
        .expect("md encode runs");
    assert!(
        enc.status.success(),
        "encode failed: {}",
        String::from_utf8_lossy(&enc.stderr)
    );
    let md1s: Vec<String> = String::from_utf8_lossy(&enc.stdout)
        .lines()
        .filter(|l| l.starts_with("md1"))
        .map(str::to_string)
        .collect();
    assert!(!md1s.is_empty(), "encode produced no md1 chunks");

    let mut dec = Command::cargo_bin("md").unwrap();
    dec.arg("decode");
    for c in &md1s {
        dec.arg(c);
    }
    dec.arg("--json");
    let out = dec.output().expect("md decode runs");
    assert!(
        out.status.success(),
        "decode failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);

    assert!(
        text.contains("Divergent"),
        "decoded path_decl is not Divergent, so the two origins were flattened \
         into one and the policy is not the one requested:\n{text}"
    );
    for want in ["48'/0'/0'/2'", "48'/0'/1'/2'"] {
        assert!(
            text.contains(want),
            "decoded output does not carry origin {want}:\n{text}"
        );
    }
}

/// Descriptor-style bracketed origins must be REFUSED CLEARLY — never accepted
/// (which would silently drop the origins) and never reported as `internal:`
/// (which sends the reader hunting a tool bug instead of fixing their input).
#[test]
fn bracketed_descriptor_origins_are_refused_with_the_correct_syntax_named() {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["encode", BRACKETED, "--force-chunked"])
        .output()
        .expect("md encode runs");

    assert!(
        !out.status.success(),
        "bracketed descriptor origins were ACCEPTED; md would silently drop them"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("internal:"),
        "a wrong-syntax input is reported as an internal error, which reads as a \
         tool bug rather than a rejected input:\n{stderr}"
    );
    assert!(
        stderr.contains("@0/48'") || stderr.contains("after the placeholder"),
        "the refusal does not name the correct syntax, so the reader cannot act \
         on it:\n{stderr}"
    );
}
