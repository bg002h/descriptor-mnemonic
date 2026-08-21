#![allow(missing_docs)]
//! F-219: `md decode` must tell an operator the key origins the card carries.
//!
//! The rendered template writes `@0/<0;1>/*` — the origin lives in the payload
//! and does not appear in that text. `md verify` proves the card carries it;
//! `decode` is the command someone restoring a plate actually runs, and it said
//! nothing.
//!
//! ON STDERR, and the tests below pin that: stdout is the template and is piped
//! into `verify`, `encode` and diffs. A fix that printed to stdout would break
//! every such pipeline to solve a display problem.

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
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| l.starts_with("md1"))
        .map(str::to_owned)
        .collect()
}

fn decode(c: &[String]) -> (String, String) {
    let out = md().arg("decode").args(c).output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn decode_reports_both_origins_on_stderr() {
    let (_, err) = decode(&cards(&[]));
    for p in ["@0: m/48'/0'/0'/2'", "@1: m/48'/0'/1'/2'"] {
        assert!(err.contains(p), "stderr is missing {p}:\n{err}");
    }
}

/// STDOUT MUST STAY THE TEMPLATE ALONE. This is the constraint that decides the
/// design, so it is asserted rather than assumed: exactly one line, and it
/// re-encodes — which is what a pipeline does with it.
#[test]
fn stdout_is_still_only_the_template() {
    let (out, _) = decode(&cards(&[]));
    let lines: Vec<&str> = out.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 1, "decode's stdout gained a line:\n{out}");
    assert!(
        lines[0].starts_with("wsh("),
        "stdout is not a template: {}",
        lines[0]
    );
    assert!(
        !out.contains("note:") && !out.contains("origins"),
        "the origins note leaked onto stdout, which pipelines consume:\n{out}"
    );
}

/// With fingerprints the note is the DESCRIPTOR spelling, so it can be compared
/// against a coordinator character for character.
#[test]
fn a_keyed_card_notes_descriptor_style_origins() {
    let (_, err) = decode(&cards(&[
        "--key",
        &format!("@0={K0}"),
        "--key",
        &format!("@1={K1}"),
        "--fingerprint",
        "@0=73c5da0a",
        "--fingerprint",
        "@1=73c5da0a",
    ]));
    assert!(
        err.contains("@0: [73c5da0a/48'/0'/0'/2']"),
        "keyed origins are not in descriptor form:\n{err}"
    );
    assert!(
        err.contains("@1: [73c5da0a/48'/0'/1'/2']"),
        "the SECOND slot is wrong or missing:\n{err}"
    );
}

/// The note must agree with `md inspect`, which grew the same information a
/// commit earlier. Two surfaces disagreeing about one card is the whole defect.
#[test]
fn the_note_agrees_with_inspect() {
    let c = cards(&[]);
    let (_, err) = decode(&c);
    let ins = md().arg("inspect").args(&c).output().unwrap();
    let text = String::from_utf8_lossy(&ins.stdout);
    for p in ["48'/0'/0'/2'", "48'/0'/1'/2'"] {
        assert!(
            err.contains(p) && text.contains(p),
            "decode and inspect disagree about {p}"
        );
    }
}
